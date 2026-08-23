use std::env;

use anyhow::{Context, Result, bail};
use postgres::{Client, Config, NoTls};

/// Statement timeout for the read-only session, in milliseconds.
const STATEMENT_TIMEOUT_MS: u32 = 30_000;

/// Opens a read-only session.
///
/// Connection details are resolved from the explicit DSN, then `DATABASE_URL`,
/// then the standard libpq `PG*` variables. The session sets a statement
/// timeout and pins `search_path`; the crate contains no write path at all.
pub fn connect(dsn: Option<&str>) -> Result<Client> {
    let mut client = open(dsn)?;
    init_session(&mut client)?;
    Ok(client)
}

fn open(dsn: Option<&str>) -> Result<Client> {
    if let Some(dsn) = dsn {
        return Client::connect(dsn, NoTls).context("connecting to database with the provided DSN");
    }

    if let Ok(url) = env::var("DATABASE_URL")
        && !url.is_empty()
    {
        return Client::connect(&url, NoTls).context("connecting via DATABASE_URL");
    }

    config_from_env()?
        .connect(NoTls)
        .context("connecting via the PG* environment variables")
}

/// Builds a connection config from the libpq `PG*` variables.
///
/// Fails when none of them is set, since that means there is no connection
/// information anywhere.
fn config_from_env() -> Result<Config> {
    let mut config = Config::new();
    let mut found_any = false;

    if let Ok(host) = env::var("PGHOST") {
        config.host(&host);
        found_any = true;
    }
    if let Ok(port) = env::var("PGPORT") {
        let port = port
            .parse()
            .with_context(|| format!("PGPORT is not a valid port number: {port:?}"))?;
        config.port(port);
        found_any = true;
    }
    if let Ok(user) = env::var("PGUSER") {
        config.user(&user);
        found_any = true;
    }
    if let Ok(password) = env::var("PGPASSWORD") {
        config.password(&password);
        found_any = true;
    }
    if let Ok(dbname) = env::var("PGDATABASE") {
        config.dbname(&dbname);
        found_any = true;
    }

    if !found_any {
        bail!(
            "no database connection information found. \
             Provide --dsn <URL>, set DATABASE_URL, or set the libpq \
             environment variables (PGHOST, PGPORT, PGUSER, PGPASSWORD, PGDATABASE)."
        );
    }

    Ok(config)
}

/// Pins the session to read-only, bounds statement runtime, and fixes
/// `search_path` so name resolution does not depend on the caller's environment.
fn init_session(client: &mut Client) -> Result<()> {
    client
        .batch_execute(&format!(
            "BEGIN READ ONLY;\n\
             SET statement_timeout = {STATEMENT_TIMEOUT_MS};\n\
             SET search_path = pg_catalog, public;"
        ))
        .context("initializing read-only session")?;
    Ok(())
}
