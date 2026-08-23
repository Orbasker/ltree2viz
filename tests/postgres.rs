//! Integration tests that hit a real Postgres with the `ltree` extension.
//!
//! Every test in here is `#[ignore]` so `cargo test` stays fast for
//! contributors without docker. CI runs them explicitly with
//! `cargo test -- --ignored`. The DSN is taken from
//! `LTREE2MMD_TEST_PG_DSN`; when it is unset the tests skip with a note
//! rather than fail, so a fresh clone doesn't fail the ignored suite.
//!
//! Each test carves its own schema so they can run in parallel and clean
//! up after themselves. The schema is dropped at the end of the test.

use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use ltree2mmd::core::path::LtreePath;
use ltree2mmd::db::fetch::{Filter, fetch};
use ltree2mmd::db::introspect::{list_ltree_columns, resolve_column};
use postgres::{Client, NoTls};

const DSN_ENV: &str = "LTREE2MMD_TEST_PG_DSN";

/// Returns a client, or `None` when no DSN is configured so the test can
/// skip cleanly. `#[ignore]` gates the tests on `--ignored`, this gates
/// them on the DSN actually being set at that point.
fn client_or_skip(test_name: &str) -> Option<Client> {
    let Ok(dsn) = std::env::var(DSN_ENV) else {
        eprintln!("skipping {test_name}: {DSN_ENV} is not set");
        return None;
    };
    let client = Client::connect(&dsn, NoTls)
        .unwrap_or_else(|error| panic!("{test_name}: connect to {dsn}: {error}"));
    Some(client)
}

static SCHEMA_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A short unique schema name so tests do not collide when parallelised.
fn fresh_schema(prefix: &str) -> String {
    let n = SCHEMA_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    format!("ltree2mmd_test_{prefix}_{pid}_{n}")
}

/// Owns a schema and drops it when the guard is dropped, so a panicking
/// assertion never leaves state behind.
struct Schema {
    client: Client,
    name: String,
}

impl Schema {
    fn create(mut client: Client, name: String) -> Self {
        client
            .batch_execute("CREATE EXTENSION IF NOT EXISTS ltree")
            .expect("create extension ltree");
        client
            .batch_execute(&format!(
                "DROP SCHEMA IF EXISTS {name} CASCADE; CREATE SCHEMA {name};"
            ))
            .expect("create schema");
        Self { client, name }
    }

    fn exec(&mut self, sql: &str) {
        self.client.batch_execute(sql).expect("exec");
    }

    fn client(&mut self) -> &mut Client {
        &mut self.client
    }
}

impl Drop for Schema {
    fn drop(&mut self) {
        let _ = self
            .client
            .batch_execute(&format!("DROP SCHEMA IF EXISTS {} CASCADE", self.name));
    }
}

fn paths(rows: &[ltree2mmd::core::tree::Row]) -> Vec<String> {
    rows.iter().map(|row| row.path.to_string()).collect()
}

#[test]
#[ignore = "requires a Postgres with ltree; set LTREE2MMD_TEST_PG_DSN and run with --ignored"]
fn auto_detects_the_single_ltree_column() {
    let Some(client) = client_or_skip("auto_detects_the_single_ltree_column") else {
        return;
    };
    let schema_name = fresh_schema("auto");
    let mut schema = Schema::create(client, schema_name.clone());
    schema.exec(&format!(
        "CREATE TABLE {schema_name}.catalog (id serial primary key, path ltree not null);
         INSERT INTO {schema_name}.catalog (path) VALUES ('a'), ('a.b');"
    ));

    let table = format!("{schema_name}.catalog");
    let column = resolve_column(schema.client(), &table, None).expect("resolve");
    assert_eq!(column.schema, schema_name);
    assert_eq!(column.table, "catalog");
    assert_eq!(column.column, "path");
}

#[test]
#[ignore = "requires a Postgres with ltree; set LTREE2MMD_TEST_PG_DSN and run with --ignored"]
fn ambiguous_column_error_names_every_candidate() {
    let Some(client) = client_or_skip("ambiguous_column_error_names_every_candidate") else {
        return;
    };
    let schema_name = fresh_schema("amb");
    let mut schema = Schema::create(client, schema_name.clone());
    schema.exec(&format!(
        "CREATE TABLE {schema_name}.catalog (id serial primary key, primary_path ltree, secondary_path ltree);"
    ));

    let table = format!("{schema_name}.catalog");
    let error =
        resolve_column(schema.client(), &table, None).expect_err("ambiguous columns must fail");
    let message = format!("{error:#}");

    assert!(
        message.contains("primary_path") && message.contains("secondary_path"),
        "both candidates should be listed, got: {message}"
    );
    assert!(
        message.contains("--path-column"),
        "the error should tell the user how to pick, got: {message}"
    );
}

#[test]
#[ignore = "requires a Postgres with ltree; set LTREE2MMD_TEST_PG_DSN and run with --ignored"]
fn root_and_depth_are_pushed_into_the_query() {
    let Some(client) = client_or_skip("root_and_depth_are_pushed_into_the_query") else {
        return;
    };
    let schema_name = fresh_schema("filter");
    let mut schema = Schema::create(client, schema_name.clone());
    schema.exec(&format!(
        "CREATE TABLE {schema_name}.catalog (id serial primary key, path ltree not null);
         INSERT INTO {schema_name}.catalog (path) VALUES
             ('top'),
             ('top.a'),
             ('top.a.x'),
             ('top.a.x.deep'),
             ('top.b'),
             ('elsewhere'),
             ('elsewhere.leaf');"
    ));

    let column = resolve_column(schema.client(), &format!("{schema_name}.catalog"), None)
        .expect("resolve column");

    // No filter: every row comes back.
    let all = fetch(schema.client(), &column, None, &Filter::default()).expect("fetch all");
    assert_eq!(all.len(), 7);

    // Root prunes everything outside the subtree.
    let root_only = fetch(
        schema.client(),
        &column,
        None,
        &Filter {
            root: Some("top".into()),
            depth: None,
        },
    )
    .expect("fetch root");
    let mut got = paths(&root_only);
    got.sort();
    assert_eq!(
        got,
        vec!["top", "top.a", "top.a.x", "top.a.x.deep", "top.b"]
    );

    // Root + depth: depth counts levels below the root itself.
    let root_depth = fetch(
        schema.client(),
        &column,
        None,
        &Filter {
            root: Some("top".into()),
            depth: Some(1),
        },
    )
    .expect("fetch depth");
    let mut got = paths(&root_depth);
    got.sort();
    assert_eq!(got, vec!["top", "top.a", "top.b"]);

    // Depth without a root counts from the top level.
    let depth_only = fetch(
        schema.client(),
        &column,
        None,
        &Filter {
            root: None,
            depth: Some(0),
        },
    )
    .expect("fetch depth without root");
    let mut got = paths(&depth_only);
    got.sort();
    assert_eq!(got, vec!["elsewhere", "top"]);
}

#[test]
#[ignore = "requires a Postgres with ltree; set LTREE2MMD_TEST_PG_DSN and run with --ignored"]
fn tables_subcommand_lists_the_configured_ltree_column() {
    let Some(client) = client_or_skip("tables_subcommand_lists_the_configured_ltree_column") else {
        return;
    };
    let schema_name = fresh_schema("tables");
    let mut schema = Schema::create(client, schema_name.clone());
    schema.exec(&format!(
        "CREATE TABLE {schema_name}.catalog (id serial primary key, path ltree not null);"
    ));

    // Sanity: the library sees the column too.
    let listed = list_ltree_columns(schema.client()).expect("list");
    assert!(
        listed
            .iter()
            .any(|c| c.schema == schema_name && c.table == "catalog" && c.column == "path"),
        "expected {schema_name}.catalog.path in {listed:?}"
    );

    let dsn = std::env::var(DSN_ENV).expect("DSN checked above");
    let output = Command::new(env!("CARGO_BIN_EXE_ltree2mmd"))
        .arg("tables")
        .env("DATABASE_URL", &dsn)
        .output()
        .expect("run the ltree2mmd binary");

    assert!(
        output.status.success(),
        "ltree2mmd tables failed: status={:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected_line = format!("{schema_name}.catalog.path");
    assert!(
        stdout.lines().any(|line| line == expected_line),
        "expected {expected_line:?} in tables output; got:\n{stdout}"
    );
}

#[test]
#[ignore = "requires a Postgres with ltree; set LTREE2MMD_TEST_PG_DSN and run with --ignored"]
fn fetch_selects_the_path_as_text_so_it_survives_any_ltree_oid() {
    // The ltree type OID is not stable across databases, so `fetch` casts
    // the column to text before decoding. If someone "simplifies" the SQL
    // by dropping the `::text` cast, `row.get::<_, String>(0)` panics with
    // a type-mismatch — this test guards that.
    let Some(client) =
        client_or_skip("fetch_selects_the_path_as_text_so_it_survives_any_ltree_oid")
    else {
        return;
    };
    let schema_name = fresh_schema("cast");
    let mut schema = Schema::create(client, schema_name.clone());
    schema.exec(&format!(
        "CREATE TABLE {schema_name}.catalog (id serial primary key, path ltree not null);
         INSERT INTO {schema_name}.catalog (path) VALUES ('root'), ('root.child');"
    ));

    let column = resolve_column(schema.client(), &format!("{schema_name}.catalog"), None)
        .expect("resolve column");
    let rows = fetch(schema.client(), &column, None, &Filter::default()).expect("fetch");

    let mut got = paths(&rows);
    got.sort();
    assert_eq!(got, vec!["root", "root.child"]);

    // Sanity-check the escape hatch too: reading the same column *without*
    // the cast fails to decode as text, which is exactly why the cast has
    // to stay in `fetch`.
    let raw = schema
        .client()
        .query(
            &format!("SELECT path FROM {schema_name}.catalog LIMIT 1"),
            &[],
        )
        .expect("raw query runs");
    let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: String = raw[0].get(0);
    }));
    assert!(
        attempt.is_err(),
        "raw ltree should not decode as String; if this ever starts \
         succeeding the `::text` cast in fetch.rs is no longer load-bearing"
    );

    // And parsing what we did get through `fetch` still round-trips.
    let path = LtreePath::parse(&rows[0].path.to_string()).expect("round-trip");
    assert_eq!(path.to_string(), rows[0].path.to_string());
}
