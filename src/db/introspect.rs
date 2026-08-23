use std::fmt;

use anyhow::{Result, bail};
use postgres::Client;

/// A column whose type is `ltree`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LtreeColumn {
    pub schema: String,
    pub table: String,
    pub column: String,
}

impl fmt::Display for LtreeColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.schema, self.table, self.column)
    }
}

/// Lists every `ltree` column in the database, ordered so output is stable.
///
/// System schemas are excluded so the result only contains columns a user could
/// reasonably want to render. The `ltree` type lives in whichever schema its
/// extension was installed into, so we match it by name rather than a fixed OID.
pub fn list_ltree_columns(client: &mut Client) -> Result<Vec<LtreeColumn>> {
    let rows = client.query(LIST_LTREE_COLUMNS_SQL, &[])?;
    Ok(rows
        .into_iter()
        .map(|row| LtreeColumn {
            schema: row.get(0),
            table: row.get(1),
            column: row.get(2),
        })
        .collect())
}

const LIST_LTREE_COLUMNS_SQL: &str = "\
    SELECT n.nspname, c.relname, a.attname \
    FROM pg_attribute a \
    JOIN pg_class c ON c.oid = a.attrelid \
    JOIN pg_namespace n ON n.oid = c.relnamespace \
    JOIN pg_type t ON t.oid = a.atttypid \
    WHERE t.typname = 'ltree' \
      AND a.attnum > 0 \
      AND NOT a.attisdropped \
      AND c.relkind IN ('r', 'v', 'm', 'p', 'f') \
      AND n.nspname NOT IN ('pg_catalog', 'information_schema') \
      AND n.nspname NOT LIKE 'pg_toast%' \
    ORDER BY n.nspname, c.relname, a.attname";

/// Resolves which column holds the hierarchy.
///
/// When a table has several `ltree` columns and none was named, this fails with
/// the candidates listed rather than picking one.
pub fn resolve_column(
    client: &mut Client,
    table: &str,
    column: Option<&str>,
) -> Result<LtreeColumn> {
    let (schema, table) = split_table(table);

    let candidates: Vec<LtreeColumn> = list_ltree_columns(client)?
        .into_iter()
        .filter(|c| c.table == table && schema.is_none_or(|s| c.schema == s))
        .collect();

    if let Some(column) = column {
        return candidates
            .into_iter()
            .find(|c| c.column == column)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "column {:?} on table {:?} is not of type ltree \
                     (or does not exist). Run `ltree2mmd tables` to list ltree columns.",
                    column,
                    table
                )
            });
    }

    match candidates.as_slice() {
        [] => bail!(
            "table {:?} has no column of type ltree. \
             Run `ltree2mmd tables` to list ltree columns.",
            table
        ),
        [only] => Ok(only.clone()),
        many => {
            let listed = many
                .iter()
                .map(|c| format!("  {c}"))
                .collect::<Vec<_>>()
                .join("\n");
            bail!(
                "table {table:?} has more than one ltree column; \
                 pass --path-column to pick one:\n{listed}"
            )
        }
    }
}

/// Splits an optionally schema-qualified table name into `(schema, table)`.
fn split_table(table: &str) -> (Option<&str>, &str) {
    match table.split_once('.') {
        Some((schema, table)) => (Some(schema), table),
        None => (None, table),
    }
}
