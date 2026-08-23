//! End-to-end tests that run the built binary.

mod common;

use std::process::Command;

use crate::common::Schema;

#[test]
fn tables_subcommand_lists_the_configured_ltree_column() {
    let dsn = skip_without_db!();
    let mut schema = Schema::new(&dsn);
    schema.exec(&format!(
        "CREATE TABLE {s}.catalog (id serial primary key, path ltree not null);",
        s = schema.name
    ));

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
    let expected = format!("{}.catalog.path", schema.name);
    assert!(
        stdout.lines().any(|line| line == expected),
        "expected {expected:?} in tables output; got:\n{stdout}"
    );
}

#[test]
fn stdin_mode_renders_without_touching_a_database() {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(env!("CARGO_BIN_EXE_ltree2mmd"))
        .arg("-")
        .env_remove("DATABASE_URL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn the ltree2mmd binary");

    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(b"a\na.b\na.b.c\n")
        .expect("write paths to stdin");

    let output = child.wait_with_output().expect("wait for ltree2mmd");
    assert!(output.status.success(), "status={:?}", output.status);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout,
        "flowchart TD\n    \
         n0[\"a\"]\n    \
         n1[\"b\"]\n    \
         n2[\"c\"]\n    \
         n0 --> n1\n    \
         n1 --> n2\n"
    );
}
