use std::collections::{HashSet, VecDeque};

use crate::core::path::LtreePath;
use crate::core::tree::{Node, Tree};

/// Label placed on the synthetic `+N more` node, and the last label of its path.
///
/// Collision with a real sibling of the same name is theoretically possible but
/// would only share a rendered id — never a panic or a lost node.
const MORE_LABEL: &str = "__more__";

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_nodes: usize,
    pub max_children: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_nodes: 300,
            max_children: 20,
        }
    }
}

/// What the guards removed. Reported on stderr and surfaced in the diagram as
/// `+N more` nodes, so a clipped tree never passes for a complete one.
#[derive(Debug, Default)]
pub struct Truncation {
    pub nodes_dropped: usize,
    pub children_folded: usize,
}

impl Truncation {
    pub fn is_empty(&self) -> bool {
        self.nodes_dropped == 0 && self.children_folded == 0
    }

    /// A human-readable summary for stderr, or `None` when nothing was cut.
    ///
    /// The caller is expected to write this loudly: a silently clipped tree is a
    /// wrong answer that looks like a right one.
    pub fn summary(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        if self.children_folded > 0 {
            parts.push(format!(
                "folded {} sibling(s) into \"+N more\" nodes",
                self.children_folded
            ));
        }
        if self.nodes_dropped > 0 {
            parts.push(format!(
                "dropped {} node(s) past the node limit",
                self.nodes_dropped
            ));
        }
        Some(format!("truncated: {}", parts.join("; ")))
    }
}

/// Enforces the size guards on `tree` in place and reports what was removed.
///
/// Children are folded first so the tree is as narrow as it will get before the
/// global node cap decides what still fits; both limits are applied so the
/// output is always at or under `max_nodes`.
pub fn apply(tree: &mut Tree, limits: Limits) -> Truncation {
    let mut truncation = Truncation::default();
    fold_children(&mut tree.roots, None, limits.max_children, &mut truncation);
    cap_nodes(tree, limits.max_nodes, &mut truncation);
    truncation
}

/// Recursively folds each sibling list down to `max_children`, appending a
/// single `+N more` node for the remainder.
fn fold_children(
    nodes: &mut Vec<Node>,
    parent: Option<&LtreePath>,
    max_children: usize,
    truncation: &mut Truncation,
) {
    if nodes.len() > max_children {
        let folded = nodes.len() - max_children;
        nodes.truncate(max_children);
        nodes.push(more_node(parent, folded));
        truncation.children_folded += folded;
    }

    for node in nodes.iter_mut() {
        if node.fold.is_some() {
            continue;
        }
        let path = node.path.clone();
        fold_children(&mut node.children, Some(&path), max_children, truncation);
    }
}

fn more_node(parent: Option<&LtreePath>, folded: usize) -> Node {
    Node {
        path: more_path(parent),
        label: format!("+{folded} more"),
        synthesized: false,
        fold: Some(folded),
        children: Vec::new(),
    }
}

/// A distinct path for the fold node: the parent's path with a sentinel label
/// appended, or the sentinel alone when folding roots.
fn more_path(parent: Option<&LtreePath>) -> LtreePath {
    let text = match parent {
        Some(parent) => format!("{parent}.{MORE_LABEL}"),
        None => MORE_LABEL.to_owned(),
    };
    LtreePath::parse(&text).expect("sentinel path is always well-formed")
}

/// Keeps the first `max_nodes` in breadth-first order and drops the rest, so the
/// surviving diagram is a shallow view of the tree rather than one deep branch.
fn cap_nodes(tree: &mut Tree, max_nodes: usize, truncation: &mut Truncation) {
    let total = count(&tree.roots);
    if total <= max_nodes {
        return;
    }

    let mut keep: HashSet<LtreePath> = HashSet::with_capacity(max_nodes);
    let mut queue: VecDeque<&Node> = tree.roots.iter().collect();
    while let Some(node) = queue.pop_front() {
        if keep.len() == max_nodes {
            break;
        }
        keep.insert(node.path.clone());
        queue.extend(node.children.iter());
    }

    truncation.nodes_dropped += total - keep.len();
    retain(&mut tree.roots, &keep);
}

fn count(nodes: &[Node]) -> usize {
    nodes.iter().map(|n| 1 + count(&n.children)).sum()
}

fn retain(nodes: &mut Vec<Node>, keep: &HashSet<LtreePath>) {
    nodes.retain(|node| keep.contains(&node.path));
    for node in nodes {
        retain(&mut node.children, keep);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(text: &str) -> LtreePath {
        LtreePath::from_labels(text.split('.'))
    }

    fn leaf(text: &str) -> Node {
        node(text, Vec::new())
    }

    fn node(text: &str, children: Vec<Node>) -> Node {
        Node {
            path: path(text),
            label: text.rsplit('.').next().unwrap().to_owned(),
            synthesized: false,
            fold: None,
            children,
        }
    }

    fn tree(roots: Vec<Node>) -> Tree {
        Tree {
            roots,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn nothing_to_do_leaves_the_tree_untouched() {
        let mut t = tree(vec![node("a", vec![leaf("a.b"), leaf("a.c")])]);
        let report = apply(&mut t, Limits::default());
        assert!(report.is_empty());
        assert_eq!(report.summary(), None);
        assert_eq!(t.roots[0].children.len(), 2);
    }

    #[test]
    fn folds_extra_children_into_a_single_more_node() {
        let children: Vec<Node> = (0..25).map(|i| leaf(&format!("a.c{i}"))).collect();
        let mut t = tree(vec![node("a", children)]);

        let report = apply(
            &mut t,
            Limits {
                max_nodes: 300,
                max_children: 20,
            },
        );

        assert_eq!(report.children_folded, 5);
        assert_eq!(report.nodes_dropped, 0);

        let kids = &t.roots[0].children;
        assert_eq!(kids.len(), 21, "20 kept + 1 fold node");
        let more = kids.last().unwrap();
        assert_eq!(more.fold, Some(5));
        assert_eq!(more.label, "+5 more");
        assert!(more.children.is_empty());
    }

    #[test]
    fn folds_roots_too() {
        let roots: Vec<Node> = (0..30).map(|i| leaf(&format!("r{i}"))).collect();
        let mut t = tree(roots);

        let report = apply(
            &mut t,
            Limits {
                max_nodes: 300,
                max_children: 20,
            },
        );

        assert_eq!(report.children_folded, 10);
        assert_eq!(t.roots.len(), 21);
        assert_eq!(t.roots.last().unwrap().fold, Some(10));
    }

    #[test]
    fn caps_total_nodes_breadth_first() {
        // A chain of 500 nodes: no folding (one child each), only the node cap
        // applies.
        let mut current = leaf("l0");
        let mut labels = String::from("l0");
        for i in 1..500 {
            labels.push_str(&format!(".l{i}"));
            current = node(&labels, vec![current]);
        }
        let mut t = tree(vec![current]);

        let report = apply(
            &mut t,
            Limits {
                max_nodes: 300,
                max_children: 20,
            },
        );

        assert_eq!(report.children_folded, 0);
        assert_eq!(report.nodes_dropped, 200);
        assert_eq!(count(&t.roots), 300);
        assert_eq!(
            report.summary().unwrap(),
            "truncated: dropped 200 node(s) past the node limit"
        );
    }

    #[test]
    fn pathological_50k_stays_under_cap_and_reports_real_counts() {
        // One root with 50k flat children — the shape that turns Mermaid into a
        // grey blob.
        let children: Vec<Node> = (0..50_000).map(|i| leaf(&format!("root.c{i}"))).collect();
        let mut t = tree(vec![node("root", children)]);

        let report = apply(&mut t, Limits::default());

        assert!(count(&t.roots) <= 300, "output stays under the node cap");
        assert_eq!(report.children_folded, 50_000 - 20);
        assert_eq!(report.nodes_dropped, 0);
        assert!(report.summary().is_some());
    }

    #[test]
    fn summary_names_both_kinds_of_loss() {
        // 40 roots each with 40 children: folding at every level, then the cap.
        let roots: Vec<Node> = (0..40)
            .map(|r| {
                let kids = (0..40).map(|c| leaf(&format!("r{r}.c{c}"))).collect();
                node(&format!("r{r}"), kids)
            })
            .collect();
        let mut t = tree(roots);

        let report = apply(&mut t, Limits::default());

        assert!(report.children_folded > 0);
        assert!(report.nodes_dropped > 0);
        assert!(count(&t.roots) <= 300);
        let summary = report.summary().unwrap();
        assert!(summary.contains("folded"));
        assert!(summary.contains("dropped"));
    }
}
