use std::collections::{BTreeMap, HashSet};

use crate::core::path::LtreePath;

/// A hierarchy row as it arrives from any source: a database query, or a plain
/// list of paths on stdin.
#[derive(Debug, Clone)]
pub struct Row {
    pub path: LtreePath,
    pub label: Option<String>,
}

/// How to treat a row whose parent path has no row of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MissingAncestors {
    /// Create the intermediate node and mark it, so the renderer can show that
    /// it was inferred rather than read.
    #[default]
    Synthesize,
    /// Discard rows that cannot be attached to a real parent.
    Drop,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub path: LtreePath,
    pub label: String,
    /// No row carried this path; it was inferred from a descendant.
    pub synthesized: bool,
    pub children: Vec<Node>,
}

/// Multiple roots are normal, not an error condition.
#[derive(Debug, Default)]
pub struct Tree {
    pub roots: Vec<Node>,
    pub warnings: Vec<String>,
}

/// A node before its children are attached.
struct Entry {
    label: String,
    synthesized: bool,
}

/// Child order is stable across runs so that rendered output can be snapshotted
/// and diffed.
pub fn build(rows: Vec<Row>, missing: MissingAncestors) -> Tree {
    let mut tree = Tree::default();

    let (rows, present) = dedupe(rows, &mut tree.warnings);

    // Keyed by path, so iteration is in label order and every parent precedes
    // its descendants.
    let mut entries: BTreeMap<LtreePath, Entry> = BTreeMap::new();

    for row in rows {
        match missing {
            MissingAncestors::Drop => {
                if let Some(orphan) = row.path.ancestors().find(|a| !present.contains(a)) {
                    tree.warnings.push(format!(
                        "dropped {}: no row for its ancestor {orphan}",
                        row.path
                    ));
                    continue;
                }
            }
            MissingAncestors::Synthesize => {
                for ancestor in row.path.ancestors() {
                    if present.contains(&ancestor) {
                        continue;
                    }
                    let label = ancestor.last_label().to_owned();
                    entries.entry(ancestor).or_insert(Entry {
                        label,
                        synthesized: true,
                    });
                }
            }
        }

        let label = row
            .label
            .unwrap_or_else(|| row.path.last_label().to_owned());
        entries.insert(
            row.path,
            Entry {
                label,
                synthesized: false,
            },
        );
    }

    let (roots, children) = link(&entries);
    tree.roots = roots
        .into_iter()
        .map(|path| assemble(path, &entries, &children))
        .collect();
    tree
}

/// Keeps the first row for each path and reports the rest, so that a table with
/// duplicate paths still produces one node per path.
fn dedupe(rows: Vec<Row>, warnings: &mut Vec<String>) -> (Vec<Row>, HashSet<LtreePath>) {
    let mut kept = Vec::with_capacity(rows.len());
    let mut present = HashSet::with_capacity(rows.len());
    let mut reported = HashSet::new();

    for row in rows {
        if present.insert(row.path.clone()) {
            kept.push(row);
        } else if reported.insert(row.path.clone()) {
            warnings.push(format!(
                "duplicate path {}: kept the first row, ignored the others",
                row.path
            ));
        }
    }

    (kept, present)
}

/// Splits the entries into roots and parent-to-children edges. Both come out in
/// path order because `entries` is sorted.
fn link(
    entries: &BTreeMap<LtreePath, Entry>,
) -> (Vec<LtreePath>, BTreeMap<LtreePath, Vec<LtreePath>>) {
    let mut roots = Vec::new();
    let mut children: BTreeMap<LtreePath, Vec<LtreePath>> = BTreeMap::new();

    for path in entries.keys() {
        match path.parent() {
            Some(parent) if entries.contains_key(&parent) => {
                children.entry(parent).or_default().push(path.clone());
            }
            // A node whose parent was dropped stands on its own rather than
            // disappearing with it.
            _ => roots.push(path.clone()),
        }
    }

    (roots, children)
}

fn assemble(
    path: LtreePath,
    entries: &BTreeMap<LtreePath, Entry>,
    children: &BTreeMap<LtreePath, Vec<LtreePath>>,
) -> Node {
    let entry = &entries[&path];
    let kids = children
        .get(&path)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|child| assemble(child.clone(), entries, children))
        .collect();

    Node {
        path,
        label: entry.label.clone(),
        synthesized: entry.synthesized,
        children: kids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(text: &str) -> LtreePath {
        LtreePath::parse(text).expect("valid path")
    }

    fn rows(paths: &[&str]) -> Vec<Row> {
        paths
            .iter()
            .map(|text| Row {
                path: path(text),
                label: None,
            })
            .collect()
    }

    /// `path -> label` for every node, depth first, in child order.
    fn flatten(tree: &Tree) -> Vec<(String, String, bool)> {
        fn walk(node: &Node, out: &mut Vec<(String, String, bool)>) {
            out.push((node.path.to_string(), node.label.clone(), node.synthesized));
            for child in &node.children {
                walk(child, out);
            }
        }

        let mut out = Vec::new();
        for root in &tree.roots {
            walk(root, &mut out);
        }
        out
    }

    fn paths_of(tree: &Tree) -> Vec<String> {
        flatten(tree).into_iter().map(|(p, _, _)| p).collect()
    }

    #[test]
    fn nests_children_under_their_parent() {
        let tree = build(rows(&["a", "a.b", "a.b.c"]), MissingAncestors::Synthesize);

        assert_eq!(tree.warnings, Vec::<String>::new());
        assert_eq!(paths_of(&tree), ["a", "a.b", "a.b.c"]);
        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0].children[0].children.len(), 1);
    }

    #[test]
    fn label_defaults_to_the_last_path_label() {
        let tree = build(rows(&["a.b"]), MissingAncestors::Synthesize);
        assert_eq!(flatten(&tree)[1].1, "b");
    }

    #[test]
    fn explicit_label_overrides_the_path_label() {
        let tree = build(
            vec![Row {
                path: path("a"),
                label: Some("Alpha".to_owned()),
            }],
            MissingAncestors::Synthesize,
        );
        assert_eq!(flatten(&tree)[0].1, "Alpha");
    }

    #[test]
    fn synthesizes_and_flags_missing_ancestors() {
        let tree = build(
            rows(&["a.b.c", "a.b.d", "x.y.z"]),
            MissingAncestors::Synthesize,
        );

        assert_eq!(
            flatten(&tree),
            [
                ("a".to_owned(), "a".to_owned(), true),
                ("a.b".to_owned(), "b".to_owned(), true),
                ("a.b.c".to_owned(), "c".to_owned(), false),
                ("a.b.d".to_owned(), "d".to_owned(), false),
                ("x".to_owned(), "x".to_owned(), true),
                ("x.y".to_owned(), "y".to_owned(), true),
                ("x.y.z".to_owned(), "z".to_owned(), false),
            ]
        );
    }

    #[test]
    fn a_row_arriving_after_its_synthesized_placeholder_is_no_longer_flagged() {
        let tree = build(rows(&["a.b", "a"]), MissingAncestors::Synthesize);

        assert_eq!(
            flatten(&tree),
            [
                ("a".to_owned(), "a".to_owned(), false),
                ("a.b".to_owned(), "b".to_owned(), false),
            ]
        );
    }

    #[test]
    fn drop_mode_discards_rows_with_a_missing_ancestor_and_warns() {
        let tree = build(rows(&["a", "a.b.c", "x.y.z"]), MissingAncestors::Drop);

        assert_eq!(paths_of(&tree), ["a"]);
        assert_eq!(
            tree.warnings,
            [
                "dropped a.b.c: no row for its ancestor a.b",
                "dropped x.y.z: no row for its ancestor x",
            ]
        );
    }

    #[test]
    fn drop_mode_keeps_a_row_whose_ancestors_all_have_rows() {
        let tree = build(rows(&["a", "a.b", "a.b.c"]), MissingAncestors::Drop);

        assert_eq!(paths_of(&tree), ["a", "a.b", "a.b.c"]);
        assert!(tree.warnings.is_empty());
    }

    #[test]
    fn drop_mode_does_not_rescue_a_row_via_a_dropped_ancestor() {
        // `a.b` is dropped, so `a.b.c` must go too rather than attaching to it.
        let tree = build(rows(&["a.b", "a.b.c"]), MissingAncestors::Drop);

        assert!(tree.roots.is_empty());
        assert_eq!(tree.warnings.len(), 2);
    }

    #[test]
    fn multiple_roots_are_a_forest_not_an_error() {
        let tree = build(
            rows(&[
                "Fruits.Apple",
                "Fruits.Banana",
                "Vegetables.Carrot",
                "Grains.Rice.Basmati",
            ]),
            MissingAncestors::Synthesize,
        );

        let roots: Vec<_> = tree.roots.iter().map(|n| n.path.to_string()).collect();
        assert_eq!(roots, ["Fruits", "Grains", "Vegetables"]);
        assert_eq!(tree.roots[0].children.len(), 2);
    }

    #[test]
    fn ordering_is_independent_of_input_order() {
        let forward = build(
            rows(&["b", "a", "a.z", "a.b", "a.b.c", "c"]),
            MissingAncestors::Synthesize,
        );
        let reversed = build(
            rows(&["c", "a.b.c", "a.b", "a.z", "a", "b"]),
            MissingAncestors::Synthesize,
        );

        assert_eq!(paths_of(&forward), ["a", "a.b", "a.b.c", "a.z", "b", "c"]);
        assert_eq!(paths_of(&forward), paths_of(&reversed));
    }

    #[test]
    fn siblings_sort_by_label_not_by_arrival() {
        let tree = build(rows(&["r.c", "r.a", "r.b"]), MissingAncestors::Synthesize);

        let children: Vec<_> = tree.roots[0]
            .children
            .iter()
            .map(|n| n.label.clone())
            .collect();
        assert_eq!(children, ["a", "b", "c"]);
    }

    #[test]
    fn duplicate_paths_keep_the_first_row_and_warn_once() {
        let tree = build(
            vec![
                Row {
                    path: path("a"),
                    label: Some("first".to_owned()),
                },
                Row {
                    path: path("a"),
                    label: Some("second".to_owned()),
                },
                Row {
                    path: path("a"),
                    label: Some("third".to_owned()),
                },
            ],
            MissingAncestors::Synthesize,
        );

        assert_eq!(paths_of(&tree), ["a"]);
        assert_eq!(tree.roots[0].label, "first");
        assert_eq!(
            tree.warnings,
            ["duplicate path a: kept the first row, ignored the others"]
        );
    }

    #[test]
    fn a_duplicate_does_not_resurrect_a_dropped_row() {
        let tree = build(rows(&["a.b", "a.b"]), MissingAncestors::Drop);

        assert!(tree.roots.is_empty());
        assert_eq!(tree.warnings.len(), 2);
    }

    #[test]
    fn empty_input_is_an_empty_forest() {
        let tree = build(Vec::new(), MissingAncestors::Synthesize);

        assert!(tree.roots.is_empty());
        assert!(tree.warnings.is_empty());
    }

    #[test]
    fn a_label_prefix_is_not_a_path_prefix() {
        let tree = build(rows(&["a", "ab", "a.b"]), MissingAncestors::Synthesize);

        let roots: Vec<_> = tree.roots.iter().map(|n| n.path.to_string()).collect();
        assert_eq!(roots, ["a", "ab"]);
        assert_eq!(tree.roots[0].children.len(), 1);
        assert!(tree.roots[1].children.is_empty());
    }
}
