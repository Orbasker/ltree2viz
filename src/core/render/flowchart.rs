use crate::core::limits::Truncation;
use crate::core::render::ids::{IdMap, escape_label};
use crate::core::tree::{Node, Tree};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    TD,
    LR,
    BT,
    RL,
}

impl Direction {
    fn as_str(self) -> &'static str {
        match self {
            Direction::TD => "TD",
            Direction::LR => "LR",
            Direction::BT => "BT",
            Direction::RL => "RL",
        }
    }
}

#[derive(Debug, Default)]
pub struct Options {
    pub direction: Direction,
    pub title: Option<String>,
}

/// Styling for nodes the tool invented rather than read: synthesized ancestors
/// and the `+N more` collapse nodes the limits pass leaves behind. Both are
/// carried on the tree as `synthesized` nodes, so a single class covers them.
const INFERRED_CLASS_DEF: &str =
    "classDef inferred stroke-dasharray:5 5,stroke:#999,color:#666,fill:#f4f4f4;";

/// Renders the tree as a Mermaid `flowchart`.
///
/// Output is byte-stable for a given input, which snapshot tests depend on and
/// which keeps committed diagrams diffable. Stability comes from the tree
/// already being in path order and from a single pre-order walk that assigns
/// ids, declarations, and edges in lockstep.
///
/// `_truncation` is part of the render contract but the collapse nodes it counts
/// are already spliced into `tree` by the limits pass; the stderr summary is the
/// caller's job.
pub fn render(tree: &Tree, _truncation: &Truncation, options: &Options) -> String {
    let mut out = String::new();

    if let Some(title) = &options.title {
        out.push_str("---\n");
        out.push_str(&format!("title: {}\n", yaml_quote(title)));
        out.push_str("---\n");
    }

    out.push_str(&format!("flowchart {}\n", options.direction.as_str()));

    let mut ids = IdMap::default();
    let mut nodes = String::new();
    let mut edges = String::new();
    let mut inferred: Vec<String> = Vec::new();

    for root in &tree.roots {
        walk(root, None, &mut ids, &mut nodes, &mut edges, &mut inferred);
    }

    out.push_str(&nodes);
    out.push_str(&edges);

    if !inferred.is_empty() {
        out.push_str(&format!("{INFERRED_CLASS_DEF}\n"));
        out.push_str(&format!("class {} inferred\n", inferred.join(",")));
    }

    out
}

fn walk(
    node: &Node,
    parent_id: Option<&str>,
    ids: &mut IdMap,
    nodes: &mut String,
    edges: &mut String,
    inferred: &mut Vec<String>,
) {
    let id = ids.id_for(&node.path).to_owned();

    nodes.push_str(&format!("    {id}[\"{}\"]\n", escape_label(&node.label)));
    if node.synthesized {
        inferred.push(id.clone());
    }
    if let Some(parent_id) = parent_id {
        edges.push_str(&format!("    {parent_id} --> {id}\n"));
    }

    for child in &node.children {
        walk(child, Some(&id), ids, nodes, edges, inferred);
    }
}

/// Quotes a title for the YAML frontmatter block. Always quoting keeps output
/// stable regardless of the characters the title happens to contain.
fn yaml_quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::path::LtreePath;

    fn path(text: &str) -> LtreePath {
        LtreePath::parse(text).expect("valid path")
    }

    fn leaf(p: &str, label: &str) -> Node {
        Node {
            path: path(p),
            label: label.to_owned(),
            synthesized: false,
            children: Vec::new(),
        }
    }

    fn render_default(tree: &Tree) -> String {
        render(tree, &Truncation::default(), &Options::default())
    }

    #[test]
    fn empty_tree_is_just_the_header() {
        let tree = Tree::default();
        assert_eq!(render_default(&tree), "flowchart TD\n");
    }

    #[test]
    fn single_node_has_a_declaration_and_no_edges() {
        let tree = Tree {
            roots: vec![leaf("a", "Alpha")],
            warnings: Vec::new(),
        };
        assert_eq!(render_default(&tree), "flowchart TD\n    n0[\"Alpha\"]\n");
    }

    #[test]
    fn direction_is_configurable() {
        let tree = Tree {
            roots: vec![leaf("a", "a")],
            warnings: Vec::new(),
        };
        let out = render(
            &tree,
            &Truncation::default(),
            &Options {
                direction: Direction::LR,
                title: None,
            },
        );
        assert!(out.starts_with("flowchart LR\n"), "{out}");
    }

    #[test]
    fn title_is_emitted_as_frontmatter() {
        let tree = Tree {
            roots: vec![leaf("a", "a")],
            warnings: Vec::new(),
        };
        let out = render(
            &tree,
            &Truncation::default(),
            &Options {
                direction: Direction::TD,
                title: Some("My Tree".to_owned()),
            },
        );
        assert_eq!(
            out,
            "---\ntitle: \"My Tree\"\n---\nflowchart TD\n    n0[\"a\"]\n"
        );
    }

    #[test]
    fn title_with_quotes_is_escaped() {
        let tree = Tree {
            roots: vec![leaf("a", "a")],
            warnings: Vec::new(),
        };
        let out = render(
            &tree,
            &Truncation::default(),
            &Options {
                direction: Direction::TD,
                title: Some("a \"b\" c".to_owned()),
            },
        );
        assert!(out.starts_with("---\ntitle: \"a \\\"b\\\" c\"\n---\n"), "{out}");
    }

    #[test]
    fn emits_one_edge_per_parent_child_pair() {
        let tree = Tree {
            roots: vec![Node {
                path: path("a"),
                label: "a".to_owned(),
                synthesized: false,
                children: vec![leaf("a.b", "b"), leaf("a.c", "c")],
            }],
            warnings: Vec::new(),
        };
        assert_eq!(
            render_default(&tree),
            "flowchart TD\n    \
                 n0[\"a\"]\n    \
                 n1[\"b\"]\n    \
                 n2[\"c\"]\n    \
                 n0 --> n1\n    \
                 n0 --> n2\n"
        );
    }

    #[test]
    fn synthesized_nodes_get_the_inferred_class() {
        let tree = Tree {
            roots: vec![Node {
                path: path("a"),
                label: "a".to_owned(),
                synthesized: true,
                children: vec![leaf("a.b", "b")],
            }],
            warnings: Vec::new(),
        };
        let out = render_default(&tree);
        assert!(out.contains(&format!("{INFERRED_CLASS_DEF}\n")), "{out}");
        assert!(out.contains("class n0 inferred\n"), "{out}");
    }

    #[test]
    fn a_collapse_node_renders_dashed_like_any_synthesized_node() {
        // The limits pass leaves `+N more` in the tree as a synthesized child.
        let tree = Tree {
            roots: vec![Node {
                path: path("a"),
                label: "a".to_owned(),
                synthesized: false,
                children: vec![
                    leaf("a.b", "b"),
                    Node {
                        path: path("a.__more__"),
                        label: "+3 more".to_owned(),
                        synthesized: true,
                        children: Vec::new(),
                    },
                ],
            }],
            warnings: Vec::new(),
        };
        let out = render_default(&tree);
        assert!(out.contains("    n2[\"+3 more\"]\n"), "{out}");
        assert!(out.contains("    n0 --> n2\n"), "{out}");
        assert!(out.contains("class n2 inferred\n"), "{out}");
    }

    #[test]
    fn no_class_line_when_nothing_is_inferred() {
        let tree = Tree {
            roots: vec![leaf("a", "a")],
            warnings: Vec::new(),
        };
        let out = render_default(&tree);
        assert!(!out.contains("classDef"), "{out}");
        assert!(!out.contains("class "), "{out}");
    }

    #[test]
    fn labels_are_escaped() {
        let tree = Tree {
            roots: vec![leaf("a", "<x> \"y\" #z")],
            warnings: Vec::new(),
        };
        assert_eq!(
            render_default(&tree),
            "flowchart TD\n    n0[\"#lt;x#gt; #quot;y#quot; #35;z\"]\n"
        );
    }

    #[test]
    fn multiple_roots_render_in_tree_order() {
        let tree = Tree {
            roots: vec![leaf("a", "a"), leaf("b", "b")],
            warnings: Vec::new(),
        };
        assert_eq!(
            render_default(&tree),
            "flowchart TD\n    n0[\"a\"]\n    n1[\"b\"]\n"
        );
    }
}
