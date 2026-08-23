mod common;

use ltree2viz::core::tree::Row;
use ltree2viz::db::fetch::{Filter, fetch};
use ltree2viz::db::introspect::LtreeColumn;

use crate::common::Schema;

/// 500 roots, 50 children under `root1`, and a leaf under each of those.
const ROW_COUNT: usize = 600;

fn seed(schema: &mut Schema, table: &str) {
    let s = &schema.name.clone();
    schema.exec(&format!(
        "CREATE TABLE \"{s}\".\"{table}\" (id serial, path ltree, name text);
         INSERT INTO \"{s}\".\"{table}\" (path, name)
           SELECT ('root' || i)::ltree, 'name' || i FROM generate_series(1, 500) i;
         INSERT INTO \"{s}\".\"{table}\" (path, name)
           SELECT ('root1.' || i)::ltree, 'child' || i FROM generate_series(1, 50) i;
         INSERT INTO \"{s}\".\"{table}\" (path, name)
           SELECT ('root1.' || i || '.leaf')::ltree, NULL FROM generate_series(1, 50) i;"
    ));
}

fn column(schema: &Schema, table: &str) -> LtreeColumn {
    LtreeColumn {
        schema: schema.name.clone(),
        table: table.to_string(),
        column: "path".into(),
    }
}

fn paths(rows: &[Row]) -> Vec<String> {
    rows.iter().map(|row| row.path.to_string()).collect()
}

#[test]
fn fetches_every_row_when_unfiltered() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    seed(&mut schema, "big");

    let mut reader = schema.reader();
    let rows = fetch(
        &mut reader,
        &column(&schema, "big"),
        None,
        &Filter::default(),
    )
    .expect("fetch");

    assert_eq!(rows.len(), ROW_COUNT);
    assert!(rows.iter().all(|row| row.label.is_none()));
}

#[test]
fn root_filter_is_applied_by_the_database() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    seed(&mut schema, "big");

    let mut reader = schema.reader();
    let rows = fetch(
        &mut reader,
        &column(&schema, "big"),
        None,
        &Filter {
            root: Some("root1".into()),
            depth: None,
        },
    )
    .expect("fetch");

    // `root1` itself, its 50 children, and their 50 leaves — not the other 499
    // roots, which never leave the server.
    assert_eq!(rows.len(), 101);
    assert!(
        rows.iter().all(
            |row| row.path.to_string() == "root1" || row.path.to_string().starts_with("root1.")
        ),
        "unexpected paths: {:?}",
        paths(&rows)
    );
}

#[test]
fn depth_is_counted_from_the_root() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    seed(&mut schema, "big");

    let mut reader = schema.reader();
    let root_only = fetch(
        &mut reader,
        &column(&schema, "big"),
        None,
        &Filter {
            root: Some("root1".into()),
            depth: Some(0),
        },
    )
    .expect("fetch");
    assert_eq!(paths(&root_only), ["root1"]);

    let one_level = fetch(
        &mut reader,
        &column(&schema, "big"),
        None,
        &Filter {
            root: Some("root1".into()),
            depth: Some(1),
        },
    )
    .expect("fetch");
    assert_eq!(one_level.len(), 51);
    assert!(one_level.iter().all(|row| row.path.nlevel() <= 2));
}

#[test]
fn depth_without_a_root_bounds_the_whole_table() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    seed(&mut schema, "big");

    let mut reader = schema.reader();
    let rows = fetch(
        &mut reader,
        &column(&schema, "big"),
        None,
        &Filter {
            root: None,
            depth: Some(0),
        },
    )
    .expect("fetch");

    assert_eq!(rows.len(), 500);
    assert!(rows.iter().all(|row| row.path.nlevel() == 1));
}

#[test]
fn rows_are_ordered_by_path() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    seed(&mut schema, "big");

    let mut reader = schema.reader();
    let filter = Filter {
        root: Some("root1".into()),
        depth: None,
    };
    let first = fetch(&mut reader, &column(&schema, "big"), None, &filter).expect("fetch");
    let second = fetch(&mut reader, &column(&schema, "big"), None, &filter).expect("fetch");

    assert_eq!(paths(&first), paths(&second));

    let mut sorted = first.iter().map(|row| row.path.clone()).collect::<Vec<_>>();
    sorted.sort();
    assert_eq!(
        sorted.iter().map(ToString::to_string).collect::<Vec<_>>(),
        paths(&first)
    );
}

#[test]
fn label_column_is_read_and_nulls_become_none() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    seed(&mut schema, "big");

    let mut reader = schema.reader();
    let rows = fetch(
        &mut reader,
        &column(&schema, "big"),
        Some("name"),
        &Filter {
            root: Some("root1.1".into()),
            depth: None,
        },
    )
    .expect("fetch");

    assert_eq!(paths(&rows), ["root1.1", "root1.1.leaf"]);
    assert_eq!(rows[0].label.as_deref(), Some("child1"));
    assert_eq!(rows[1].label, None);
}

#[test]
fn null_paths_are_excluded() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    let s = schema.name.clone();
    schema.exec(&format!(
        "CREATE TABLE \"{s}\".sparse (path ltree);
         INSERT INTO \"{s}\".sparse VALUES ('a'), (NULL), ('a.b');"
    ));

    let mut reader = schema.reader();
    let rows = fetch(
        &mut reader,
        &column(&schema, "sparse"),
        None,
        &Filter::default(),
    )
    .expect("fetch");

    assert_eq!(paths(&rows), ["a", "a.b"]);
}

#[test]
fn table_names_with_dots_and_quotes_are_handled() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    let s = schema.name.clone();
    schema.exec(&format!(
        "CREATE TABLE \"{s}\".\"od.d\" (\"pa.th\" ltree, \"la\"\"bel\" text);
         INSERT INTO \"{s}\".\"od.d\" VALUES ('a', 'A'), ('a.b', 'B');
         CREATE TABLE \"{s}\".\"qu\"\"oted\" (path ltree);
         INSERT INTO \"{s}\".\"qu\"\"oted\" VALUES ('x'), ('x.y');"
    ));

    let mut reader = schema.reader();

    let dotted = fetch(
        &mut reader,
        &LtreeColumn {
            schema: schema.name.clone(),
            table: "od.d".into(),
            column: "pa.th".into(),
        },
        Some("la\"bel"),
        &Filter::default(),
    )
    .expect("fetch from a dotted table");
    assert_eq!(paths(&dotted), ["a", "a.b"]);
    assert_eq!(dotted[0].label.as_deref(), Some("A"));

    let quoted = fetch(
        &mut reader,
        &column(&schema, "qu\"oted"),
        None,
        &Filter {
            root: Some("x".into()),
            depth: None,
        },
    )
    .expect("fetch from a quoted table");
    assert_eq!(paths(&quoted), ["x", "x.y"]);
}

#[test]
fn a_malformed_root_is_rejected() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    seed(&mut schema, "big");

    let mut reader = schema.reader();
    let err = fetch(
        &mut reader,
        &column(&schema, "big"),
        None,
        &Filter {
            root: Some("a..b".into()),
            depth: None,
        },
    )
    .expect_err("a malformed root must fail");

    assert!(err.to_string().contains("--root"), "message: {err}");
}

#[test]
fn works_when_the_ltree_extension_lives_outside_search_path() {
    let dsn = skip_without_db!();
    let Some(database) = common::Database::new(
        &dsn,
        "CREATE SCHEMA ext;
         CREATE EXTENSION ltree SCHEMA ext;
         CREATE TABLE public.t (path ext.ltree);
         INSERT INTO public.t VALUES ('a'), ('a.b'), ('a.b.c'), ('z');",
    ) else {
        eprintln!("skipping: the test role may not create databases");
        return;
    };

    let mut reader = database.reader();
    let rows = fetch(
        &mut reader,
        &LtreeColumn {
            schema: "public".into(),
            table: "t".into(),
            column: "path".into(),
        },
        None,
        &Filter {
            root: Some("a".into()),
            depth: Some(1),
        },
    )
    .expect("fetch with ltree outside search_path");

    assert_eq!(paths(&rows), ["a", "a.b"]);
}
