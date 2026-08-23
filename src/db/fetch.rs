use anyhow::Result;
use postgres::Client;

use crate::core::tree::Row;
use crate::db::introspect::LtreeColumn;

/// Filters pushed down into the query rather than applied after fetching.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub root: Option<String>,
    pub depth: Option<u32>,
}

/// Reads the hierarchy.
///
/// The path column is always selected as `::text`. The `ltree` type OID comes
/// from an extension and is not stable across databases, so binary decoding of
/// it cannot be relied on.
pub fn fetch(
    _client: &mut Client,
    _column: &LtreeColumn,
    _label_column: Option<&str>,
    _filter: &Filter,
) -> Result<Vec<Row>> {
    todo!()
}
