use anyhow::Result;
use postgres::Client;

/// Opens a read-only session.
///
/// Connection details are resolved from the explicit DSN, then `DATABASE_URL`,
/// then the standard libpq `PG*` variables. The session sets a statement
/// timeout and pins `search_path`; the crate contains no write path at all.
pub fn connect(_dsn: Option<&str>) -> Result<Client> {
    todo!()
}
