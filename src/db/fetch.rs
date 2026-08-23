use anyhow::{Context, Result};
use postgres::Client;
use postgres::types::ToSql;

use crate::core::path::LtreePath;
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
    client: &mut Client,
    column: &LtreeColumn,
    label_column: Option<&str>,
    filter: &Filter,
) -> Result<Vec<Row>> {
    let ltree_schema = ltree_schema(client, column)?;
    let query = build_query(column, label_column, filter, &ltree_schema)?;

    let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();
    if let Some(root) = &query.root {
        params.push(root);
    }
    if let Some(max_level) = &query.max_level {
        params.push(max_level);
    }

    let rows = client
        .query(&query.sql, &params)
        .with_context(|| format!("reading {column}"))?;

    rows.into_iter()
        .map(|row| {
            let text: String = row.get(0);
            let path = LtreePath::parse(&text)
                .with_context(|| format!("parsing path {text:?} from {column}"))?;
            let label = if label_column.is_some() {
                row.get::<_, Option<String>>(1)
            } else {
                None
            };
            Ok(Row { path, label })
        })
        .collect()
}

/// The SQL and the values bound to it, kept together so both can be tested.
#[derive(Debug, PartialEq, Eq)]
struct Query {
    sql: String,
    root: Option<String>,
    max_level: Option<i32>,
}

fn build_query(
    column: &LtreeColumn,
    label_column: Option<&str>,
    filter: &Filter,
    ltree_schema: &str,
) -> Result<Query> {
    let root = filter
        .root
        .as_deref()
        .map(|root| {
            LtreePath::parse(root).with_context(|| format!("--root {root:?} is not an ltree path"))
        })
        .transpose()?;

    let ext = quote_ident(ltree_schema);
    let path = quote_ident(&column.column);

    let mut select = format!("{path}::text");
    if let Some(label) = label_column {
        select.push_str(&format!(", {}::text", quote_ident(label)));
    }

    let mut conditions = vec![format!("{path} IS NOT NULL")];
    if root.is_some() {
        conditions.push(format!("{path} OPERATOR({ext}.<@) $1::text::{ext}.ltree"));
    }

    // The root sits at output level zero; without one, the top-level labels do.
    let max_level = filter.depth.map(|depth| {
        let base = root.as_ref().map_or(1, LtreePath::nlevel) as u64;
        i32::try_from(base + u64::from(depth)).unwrap_or(i32::MAX)
    });
    if max_level.is_some() {
        let index = if root.is_some() { 2 } else { 1 };
        conditions.push(format!("{ext}.nlevel({path}) <= ${index}::int4"));
    }

    let sql = format!(
        "SELECT {select} FROM {schema}.{table} WHERE {conditions} ORDER BY {path}",
        schema = quote_ident(&column.schema),
        table = quote_ident(&column.table),
        conditions = conditions.join(" AND "),
    );

    Ok(Query {
        sql,
        root: root.map(|root| root.to_string()),
        max_level,
    })
}

/// The schema holding the `ltree` extension, taken from the column's own type
/// so that `nlevel` and `<@` can be qualified instead of relying on
/// `search_path`.
fn ltree_schema(client: &mut Client, column: &LtreeColumn) -> Result<String> {
    let row = client
        .query_opt(
            LTREE_SCHEMA_SQL,
            &[&column.schema, &column.table, &column.column],
        )
        .with_context(|| format!("locating the ltree extension for {column}"))?
        .with_context(|| format!("column {column} no longer exists"))?;
    Ok(row.get(0))
}

const LTREE_SCHEMA_SQL: &str = "\
    SELECT tn.nspname \
    FROM pg_attribute a \
    JOIN pg_class c ON c.oid = a.attrelid \
    JOIN pg_namespace cn ON cn.oid = c.relnamespace \
    JOIN pg_type t ON t.oid = a.atttypid \
    JOIN pg_namespace tn ON tn.oid = t.typnamespace \
    WHERE cn.nspname = $1 \
      AND c.relname = $2 \
      AND a.attname = $3 \
      AND a.attnum > 0 \
      AND NOT a.attisdropped \
      AND t.typname = 'ltree'";

/// Quotes an identifier the way Postgres does, so names holding quotes, dots or
/// uppercase letters survive.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column() -> LtreeColumn {
        LtreeColumn {
            schema: "public".into(),
            table: "catalog".into(),
            column: "path".into(),
        }
    }

    fn build(label: Option<&str>, filter: &Filter) -> Query {
        build_query(&column(), label, filter, "public").expect("query builds")
    }

    #[test]
    fn unfiltered_query_selects_the_path_as_text_and_orders_by_it() {
        let query = build(None, &Filter::default());

        assert_eq!(
            query.sql,
            "SELECT \"path\"::text FROM \"public\".\"catalog\" \
             WHERE \"path\" IS NOT NULL ORDER BY \"path\""
        );
        assert_eq!(query.root, None);
        assert_eq!(query.max_level, None);
    }

    #[test]
    fn label_column_is_selected_as_text() {
        let query = build(Some("name"), &Filter::default());
        assert!(
            query
                .sql
                .starts_with("SELECT \"path\"::text, \"name\"::text FROM")
        );
    }

    #[test]
    fn root_becomes_a_bound_containment_filter() {
        let query = build(
            None,
            &Filter {
                root: Some("a.b".into()),
                depth: None,
            },
        );

        assert!(
            query
                .sql
                .contains("\"path\" OPERATOR(\"public\".<@) $1::text::\"public\".ltree"),
            "sql: {}",
            query.sql
        );
        assert_eq!(query.root.as_deref(), Some("a.b"));
    }

    #[test]
    fn depth_becomes_an_nlevel_bound_relative_to_the_root() {
        let query = build(
            None,
            &Filter {
                root: Some("a.b".into()),
                depth: Some(2),
            },
        );

        assert!(
            query
                .sql
                .contains("\"public\".nlevel(\"path\") <= $2::int4"),
            "sql: {}",
            query.sql
        );
        assert_eq!(query.max_level, Some(4));
    }

    #[test]
    fn depth_without_a_root_counts_from_the_top_level() {
        let query = build(
            None,
            &Filter {
                root: None,
                depth: Some(1),
            },
        );

        assert!(
            query
                .sql
                .contains("\"public\".nlevel(\"path\") <= $1::int4"),
            "sql: {}",
            query.sql
        );
        assert_eq!(query.max_level, Some(2));
    }

    #[test]
    fn saturating_depth_does_not_overflow() {
        let query = build(
            None,
            &Filter {
                root: None,
                depth: Some(u32::MAX),
            },
        );
        assert_eq!(query.max_level, Some(i32::MAX));
    }

    #[test]
    fn identifiers_with_quotes_and_dots_are_quoted_not_concatenated() {
        let column = LtreeColumn {
            schema: "we\"ird".into(),
            table: "my.table".into(),
            column: "the\"path".into(),
        };
        let query =
            build_query(&column, Some("la\"bel"), &Filter::default(), "ext").expect("query builds");

        assert_eq!(
            query.sql,
            "SELECT \"the\"\"path\"::text, \"la\"\"bel\"::text \
             FROM \"we\"\"ird\".\"my.table\" \
             WHERE \"the\"\"path\" IS NOT NULL ORDER BY \"the\"\"path\""
        );
    }

    #[test]
    fn an_invalid_root_is_rejected_before_the_query_runs() {
        let err = build_query(
            &column(),
            None,
            &Filter {
                root: Some("a..b".into()),
                depth: None,
            },
            "public",
        )
        .expect_err("a malformed root must fail");

        assert!(err.to_string().contains("--root"), "message: {err}");
    }
}
