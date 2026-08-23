//! Integration tests for ltree column introspection.
//!
//! These need a real Postgres with the `ltree` extension available. Set
//! `TEST_DATABASE_URL` to a connection string the test may create schemas in;
//! without it the tests skip so `cargo test` stays green on a bare checkout.

use std::process::id;
use std::sync::atomic::{AtomicU32, Ordering};

use ltree2mmd::db::connect::connect;
use ltree2mmd::db::introspect::{LtreeColumn, list_ltree_columns, resolve_column};
use postgres::{Client, NoTls};

/// Returns the test connection string, or `None` when the suite should skip.
fn test_dsn() -> Option<String> {
    std::env::var("TEST_DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
}

static SCHEMA_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A throwaway schema, dropped when the guard is dropped.
struct Schema {
    name: String,
    setup: Client,
    dsn: String,
}

impl Schema {
    fn new(dsn: &str) -> Self {
        let n = SCHEMA_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!("ltree2mmd_test_{}_{}", id(), n);
        let mut setup = Client::connect(dsn, NoTls).expect("setup connection");
        setup
            .batch_execute("CREATE EXTENSION IF NOT EXISTS ltree;")
            .expect("ensure ltree extension");
        setup
            .batch_execute(&format!(
                "DROP SCHEMA IF EXISTS {name} CASCADE; CREATE SCHEMA {name};"
            ))
            .expect("create schema");
        Self {
            name,
            setup,
            dsn: dsn.to_string(),
        }
    }

    fn exec(&mut self, sql: &str) {
        self.setup.batch_execute(sql).expect("setup sql");
    }

    /// A read-only client of the crate, as the binary would open.
    fn reader(&self) -> postgres::Client {
        connect(Some(&self.dsn)).expect("read-only connect")
    }
}

impl Drop for Schema {
    fn drop(&mut self) {
        let _ = self
            .setup
            .batch_execute(&format!("DROP SCHEMA IF EXISTS {} CASCADE;", self.name));
    }
}

macro_rules! skip_without_db {
    () => {
        match test_dsn() {
            Some(dsn) => dsn,
            None => {
                eprintln!("skipping: set TEST_DATABASE_URL to run introspection tests");
                return;
            }
        }
    };
}

#[test]
fn auto_detects_the_single_ltree_column() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    schema.exec(&format!(
        "CREATE TABLE {s}.catalog (id int, path ltree, name text);",
        s = schema.name
    ));

    let mut reader = schema.reader();
    let resolved = resolve_column(&mut reader, &format!("{}.catalog", schema.name), None)
        .expect("auto-detect should succeed");

    assert_eq!(
        resolved,
        LtreeColumn {
            schema: schema.name.clone(),
            table: "catalog".into(),
            column: "path".into(),
        }
    );
}

#[test]
fn ambiguous_columns_fail_and_list_both() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    schema.exec(&format!(
        "CREATE TABLE {s}.tree (id int, path ltree, mirror ltree);",
        s = schema.name
    ));

    let mut reader = schema.reader();
    let err = resolve_column(&mut reader, &format!("{}.tree", schema.name), None)
        .expect_err("two ltree columns must be ambiguous");
    let msg = err.to_string();

    assert!(msg.contains("more than one"), "message: {msg}");
    assert!(
        msg.contains(&format!("{}.tree.path", schema.name)),
        "message: {msg}"
    );
    assert!(
        msg.contains(&format!("{}.tree.mirror", schema.name)),
        "message: {msg}"
    );
}

#[test]
fn no_ltree_column_fails_and_mentions_tables_command() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    schema.exec(&format!(
        "CREATE TABLE {s}.flat (id int, name text);",
        s = schema.name
    ));

    let mut reader = schema.reader();
    let err = resolve_column(&mut reader, &format!("{}.flat", schema.name), None)
        .expect_err("a table with no ltree column must fail");
    let msg = err.to_string();

    assert!(msg.contains("no column of type ltree"), "message: {msg}");
    assert!(msg.contains("ltree2mmd tables"), "message: {msg}");
}

#[test]
fn explicit_column_is_validated() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    schema.exec(&format!(
        "CREATE TABLE {s}.tree (id int, path ltree, mirror ltree, name text);",
        s = schema.name
    ));

    let mut reader = schema.reader();
    let table = format!("{}.tree", schema.name);

    let resolved = resolve_column(&mut reader, &table, Some("mirror")).expect("named ltree column");
    assert_eq!(resolved.column, "mirror");

    let err = resolve_column(&mut reader, &table, Some("name"))
        .expect_err("a non-ltree column must be rejected");
    assert!(
        err.to_string().contains("not of type ltree"),
        "message: {err}"
    );
}

#[test]
fn list_includes_created_columns_and_excludes_system_schemas() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    schema.exec(&format!(
        "CREATE TABLE {s}.a (p ltree); CREATE TABLE {s}.b (q ltree);",
        s = schema.name
    ));

    let mut reader = schema.reader();
    let all = list_ltree_columns(&mut reader).expect("list");

    assert!(
        all.iter()
            .any(|c| c.schema == schema.name && c.table == "a" && c.column == "p")
    );
    assert!(
        all.iter()
            .any(|c| c.schema == schema.name && c.table == "b" && c.column == "q")
    );
    assert!(
        all.iter()
            .all(|c| c.schema != "pg_catalog" && c.schema != "information_schema"),
        "system schemas must be excluded"
    );
}
