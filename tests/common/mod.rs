//! Shared harness for the tests that need a real Postgres.
#![allow(dead_code)]

use std::process::id;
use std::sync::atomic::{AtomicU32, Ordering};

use ltree2viz::db::connect::connect;
use postgres::{Client, NoTls};

/// Returns the test connection string, or `None` when the suite should skip.
pub fn test_dsn() -> Option<String> {
    std::env::var("TEST_DATABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
}

static SCHEMA_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A throwaway schema, dropped when the guard is dropped.
pub struct Schema {
    pub name: String,
    setup: Client,
    dsn: String,
}

impl Schema {
    pub fn new(dsn: &str) -> Self {
        let n = SCHEMA_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!("ltree2viz_test_{}_{}", id(), n);
        let mut setup = Client::connect(dsn, NoTls).expect("setup connection");
        ensure_ltree(&mut setup);
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

    pub fn exec(&mut self, sql: &str) {
        self.setup.batch_execute(sql).expect("setup sql");
    }

    /// A read-only client of the crate, as the binary would open.
    pub fn reader(&self) -> Client {
        connect(Some(&self.dsn)).expect("read-only connect")
    }
}

/// `CREATE EXTENSION IF NOT EXISTS` is not concurrency-safe: two tests racing on
/// it leave one with a duplicate-key error, so the outcome is what matters.
fn ensure_ltree(client: &mut Client) {
    if client
        .batch_execute("CREATE EXTENSION IF NOT EXISTS ltree;")
        .is_ok()
    {
        return;
    }
    client
        .query_one("SELECT 1 FROM pg_extension WHERE extname = 'ltree'", &[])
        .expect("ltree extension is installed");
}

impl Drop for Schema {
    fn drop(&mut self) {
        let _ = self
            .setup
            .batch_execute(&format!("DROP SCHEMA IF EXISTS {} CASCADE;", self.name));
    }
}

#[macro_export]
macro_rules! skip_without_db {
    () => {
        match $crate::common::test_dsn() {
            Some(dsn) => dsn,
            None => {
                eprintln!("skipping: set TEST_DATABASE_URL to run database tests");
                return;
            }
        }
    };
}

/// Points a DSN at a different database on the same server.
pub fn dsn_for_database(dsn: &str, database: &str) -> Option<String> {
    let (prefix, _) = dsn.split_once('?').unwrap_or((dsn, ""));
    let (base, _) = prefix.rsplit_once('/')?;
    Some(format!("{base}/{database}"))
}

/// A throwaway database, used when the schema-level harness is not enough —
/// notably to install `ltree` somewhere other than `public`.
pub struct Database {
    name: String,
    admin: Client,
    pub dsn: String,
}

impl Database {
    /// Returns `None` when the test role may not create databases.
    pub fn new(dsn: &str, setup_sql: &str) -> Option<Self> {
        let n = SCHEMA_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!("ltree2viz_db_{}_{}", id(), n);
        let mut admin = Client::connect(dsn, NoTls).expect("admin connection");
        admin
            .batch_execute(&format!("CREATE DATABASE {name}"))
            .ok()?;

        let dsn = dsn_for_database(dsn, &name)?;
        let mut setup = Client::connect(&dsn, NoTls).expect("connect to the new database");
        setup.batch_execute(setup_sql).expect("setup sql");

        Some(Self { name, admin, dsn })
    }

    pub fn reader(&self) -> Client {
        connect(Some(&self.dsn)).expect("read-only connect")
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = self
            .admin
            .batch_execute(&format!("DROP DATABASE IF EXISTS {} (FORCE)", self.name));
    }
}
