//! Snapshot corpus for the pure `core/` pipeline: build a tree from a flat path
//! list, apply the size limits, and render the Mermaid flowchart. No database is
//! involved, which is the whole point — the renderer is locked byte-for-byte
//! against a set of hand-picked hierarchies that exercise its edge cases.

use ltree2viz::core::limits::{Limits, apply};
use ltree2viz::core::path::LtreePath;
use ltree2viz::core::render::flowchart::{Direction, Options, render};
use ltree2viz::core::tree::{MissingAncestors, Row, build};

/// Limits high enough to never fire, so a case snapshots the tree as-is. Cases
/// that mean to exercise folding opt into [`Limits::default`] instead.
const UNLIMITED: Limits = Limits {
    max_nodes: usize::MAX,
    max_children: usize::MAX,
};

fn rows(paths: &[&str]) -> Vec<Row> {
    paths
        .iter()
        .map(|text| Row {
            path: LtreePath::parse(text).expect("valid fixture path"),
            label: None,
        })
        .collect()
}

fn labeled_rows(pairs: &[(&str, &str)]) -> Vec<Row> {
    pairs
        .iter()
        .map(|(path, label)| Row {
            path: LtreePath::parse(path).expect("valid fixture path"),
            label: Some((*label).to_owned()),
        })
        .collect()
}

/// The full core pipeline, returning exactly what a caller writes to stdout.
fn diagram(rows: Vec<Row>, missing: MissingAncestors, limits: Limits, options: Options) -> String {
    let mut tree = build(rows, missing);
    let truncation = apply(&mut tree, limits);
    render(&tree, &truncation, &options)
}

/// A convenience for the common case: synthesize ancestors, no limits, defaults.
fn render_paths(paths: &[&str]) -> String {
    diagram(
        rows(paths),
        MissingAncestors::Synthesize,
        UNLIMITED,
        Options::default(),
    )
}

#[test]
fn deep_chain() {
    // One long path, every level a real row.
    let paths = [
        "a",
        "a.b",
        "a.b.c",
        "a.b.c.d",
        "a.b.c.d.e",
        "a.b.c.d.e.f",
        "a.b.c.d.e.f.g",
        "a.b.c.d.e.f.g.h",
        "a.b.c.d.e.f.g.h.i",
        "a.b.c.d.e.f.g.h.i.j",
    ];
    insta::assert_snapshot!(render_paths(&paths));
}

#[test]
fn wide_level() {
    // Hundreds of siblings under one root, rendered without folding so the full
    // fan-out is locked.
    let root = "team";
    let mut owned = vec![root.to_owned()];
    for i in 0..250 {
        owned.push(format!("{root}.member{i:03}"));
    }
    let paths: Vec<&str> = owned.iter().map(String::as_str).collect();
    insta::assert_snapshot!(render_paths(&paths));
}

#[test]
fn folded_children() {
    // The same wide shape under the default child limit: 20 kept, the rest
    // collapsed into a single dashed "+N more" node.
    let root = "team";
    let mut owned = vec![root.to_owned()];
    for i in 0..30 {
        owned.push(format!("{root}.member{i:03}"));
    }
    let paths: Vec<&str> = owned.iter().map(String::as_str).collect();
    let out = diagram(
        rows(&paths),
        MissingAncestors::Synthesize,
        Limits::default(),
        Options::default(),
    );
    insta::assert_snapshot!(out);
}

#[test]
fn forest_with_several_roots() {
    let paths = [
        "Fruits.Apple",
        "Fruits.Banana",
        "Vegetables.Carrot",
        "Vegetables.Potato",
        "Grains.Rice.Basmati",
        "Grains.Wheat",
    ];
    insta::assert_snapshot!(render_paths(&paths));
}

#[test]
fn missing_ancestors_synthesized() {
    // Only deep leaves arrive; the intermediate nodes are inferred and marked.
    let paths = ["a.b.c", "a.b.d", "x.y.z"];
    insta::assert_snapshot!(render_paths(&paths));
}

#[test]
fn missing_ancestors_dropped() {
    // Same input, but rows without a real parent are discarded instead.
    let out = diagram(
        rows(&["a", "a.b.c", "x.y.z"]),
        MissingAncestors::Drop,
        UNLIMITED,
        Options::default(),
    );
    insta::assert_snapshot!(out);
}

#[test]
fn unicode_labels() {
    let paths = [
        "categoría.niño",
        "categoría.日本語",
        "categoría.Ελληνικά",
        "categoría.emoji_🌳",
    ];
    insta::assert_snapshot!(render_paths(&paths));
}

#[test]
fn labels_needing_escaping() {
    // Quotes, hash, and angle brackets all become Mermaid HTML entities.
    let paths = [
        r#"root.a"quoted""#,
        "root.c#hash",
        "root.e<tag>",
        r##"root.all_<"#>"##,
    ];
    insta::assert_snapshot!(render_paths(&paths));
}

#[test]
fn reserved_word_end() {
    // `end` is a Mermaid keyword; ids are synthetic so it renders as a label.
    let paths = ["end", "end.child", "graph.end", "class.o.x"];
    insta::assert_snapshot!(render_paths(&paths));
}

#[test]
fn duplicate_paths() {
    // The first row for a path wins; later duplicates are dropped.
    let rows = labeled_rows(&[
        ("team.eng", "Engineering"),
        ("team.eng", "Eng Department"),
        ("team.eng", "E"),
        ("team.sales", "Sales"),
    ]);
    let out = diagram(
        rows,
        MissingAncestors::Synthesize,
        UNLIMITED,
        Options::default(),
    );
    insta::assert_snapshot!(out);
}

#[test]
fn single_node() {
    insta::assert_snapshot!(render_paths(&["solo"]));
}

#[test]
fn empty_input() {
    insta::assert_snapshot!(render_paths(&[]));
}

#[test]
fn title_and_direction() {
    // Options coverage: YAML frontmatter title plus a non-default direction.
    let out = diagram(
        rows(&["a.b", "a.c"]),
        MissingAncestors::Synthesize,
        UNLIMITED,
        Options {
            direction: Direction::LR,
            title: Some(r#"Org "chart""#.to_owned()),
        },
    );
    insta::assert_snapshot!(out);
}
