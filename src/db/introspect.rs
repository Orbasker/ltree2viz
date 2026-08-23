use anyhow::Result;
use postgres::Client;

/// A column whose type is `ltree`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LtreeColumn {
    pub schema: String,
    pub table: String,
    pub column: String,
}

pub fn list_ltree_columns(_client: &mut Client) -> Result<Vec<LtreeColumn>> {
    todo!()
}

/// Resolves which column holds the hierarchy.
///
/// When a table has several `ltree` columns and none was named, this fails with
/// the candidates listed rather than picking one.
pub fn resolve_column(
    _client: &mut Client,
    _table: &str,
    _column: Option<&str>,
) -> Result<LtreeColumn> {
    todo!()
}
