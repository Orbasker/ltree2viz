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

/// Child order is stable across runs so that rendered output can be snapshotted
/// and diffed.
pub fn build(_rows: Vec<Row>, _missing: MissingAncestors) -> Tree {
    todo!()
}
