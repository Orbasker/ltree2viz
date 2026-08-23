use std::collections::HashMap;

use crate::core::path::LtreePath;

/// Assigns sequential `n0..nN` identifiers.
///
/// Identifiers are never derived from labels. That keeps Mermaid's reserved
/// words (`end`, `graph`, `class`, `o`, `x`), identifiers with a leading digit,
/// and version-dependent `ltree` label characters out of the generated syntax.
#[derive(Debug, Default)]
pub struct IdMap {
    pub next: usize,
    pub ids: HashMap<LtreePath, String>,
}

impl IdMap {
    /// Returns the identifier for `path`, assigning the next `nN` on first use.
    pub fn id_for(&mut self, path: &LtreePath) -> &str {
        if !self.ids.contains_key(path) {
            let id = format!("n{}", self.next);
            self.next += 1;
            self.ids.insert(path.clone(), id);
        }
        &self.ids[path]
    }
}

/// Escapes a label for use inside a quoted Mermaid node.
///
/// `#` is escaped first so the entities introduced for the other characters are
/// not themselves re-escaped.
pub fn escape_label(label: &str) -> String {
    label
        .replace('#', "#35;")
        .replace('"', "#quot;")
        .replace('<', "#lt;")
        .replace('>', "#gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(text: &str) -> LtreePath {
        LtreePath::from_labels(text.split('.'))
    }

    #[test]
    fn ids_are_sequential_and_stable() {
        let mut map = IdMap::default();
        let a = path("a");
        let b = path("a.b");

        assert_eq!(map.id_for(&a), "n0");
        assert_eq!(map.id_for(&b), "n1");
        // Re-querying an existing path returns the same id.
        assert_eq!(map.id_for(&a), "n0");
        assert_eq!(map.id_for(&b), "n1");
    }

    #[test]
    fn ids_are_never_derived_from_labels() {
        // A label that is a Mermaid reserved word still gets a synthetic id.
        let mut map = IdMap::default();
        assert_eq!(map.id_for(&path("end")), "n0");
        assert_eq!(map.id_for(&path("graph")), "n1");
    }

    #[test]
    fn escape_quotes_and_angle_brackets() {
        assert_eq!(escape_label(r#"a"b"#), "a#quot;b");
        assert_eq!(escape_label("<tag>"), "#lt;tag#gt;");
    }

    #[test]
    fn escape_hash_is_not_double_escaped() {
        assert_eq!(escape_label("#"), "#35;");
        assert_eq!(escape_label(r#"#""#), "#35;#quot;");
    }

    #[test]
    fn plain_label_is_untouched() {
        assert_eq!(escape_label("end"), "end");
    }
}
