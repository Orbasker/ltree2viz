mod common;

use ltree2viz::db::introspect::{LtreeColumn, list_ltree_columns, resolve_column};

use crate::common::Schema;

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
    assert!(msg.contains("ltree2viz tables"), "message: {msg}");
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
