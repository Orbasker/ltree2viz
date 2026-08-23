//! Integration tests. Path fixtures live in `tests/fixtures/`; accepted `insta`
//! snapshots are written to `tests/snapshots/`.

use std::fs;

use ltree2mmd::core::path::LtreePath;
use ltree2mmd::core::tree::{MissingAncestors, Node, Row, Tree, build};

const FIXTURES: [&str; 3] = ["simple.txt", "forest.txt", "missing_ancestors.txt"];

#[test]
fn fixtures_are_readable() {
    for name in FIXTURES {
        let path = format!("tests/fixtures/{name}");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        assert!(!text.trim().is_empty(), "{name} is empty");
    }
}

fn rows(fixture: &str) -> Vec<Row> {
    let text = fs::read_to_string(format!("tests/fixtures/{fixture}")).expect("read fixture");
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| Row {
            path: LtreePath::parse(line).expect("fixture holds valid paths"),
            label: None,
        })
        .collect()
}

fn outline(tree: &Tree) -> String {
    fn walk(node: &Node, depth: usize, out: &mut String) {
        let mark = if node.synthesized {
            " (synthesized)"
        } else {
            ""
        };
        out.push_str(&format!("{}{}{}\n", "  ".repeat(depth), node.label, mark));
        for child in &node.children {
            walk(child, depth + 1, out);
        }
    }

    let mut out = String::new();
    for root in &tree.roots {
        walk(root, 0, &mut out);
    }
    out
}

#[test]
fn simple_fixture_builds_a_single_rooted_tree() {
    let tree = build(rows("simple.txt"), MissingAncestors::Synthesize);

    assert!(tree.warnings.is_empty());
    assert_eq!(
        outline(&tree),
        "Top\n  \
           Collections\n    \
             Pictures\n      \
               Astronomy\n  \
           Hobbies\n    \
             Amateurs_Astronomy\n  \
           Science\n    \
             Astronomy\n      \
               Astrophysics\n      \
               Cosmology\n"
    );
}

#[test]
fn forest_fixture_builds_several_roots_with_synthesized_tops() {
    let tree = build(rows("forest.txt"), MissingAncestors::Synthesize);

    assert_eq!(
        outline(&tree),
        "Fruits (synthesized)\n  \
           Apple\n  \
           Banana\n\
         Grains (synthesized)\n  \
           Rice (synthesized)\n    \
             Basmati\n\
         Vegetables (synthesized)\n  \
           Carrot\n"
    );
}

#[test]
fn missing_ancestors_fixture_drops_every_row_without_synthesis() {
    let tree = build(rows("missing_ancestors.txt"), MissingAncestors::Drop);

    assert!(tree.roots.is_empty());
    assert_eq!(tree.warnings.len(), 3);
}
