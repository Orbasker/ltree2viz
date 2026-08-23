use crate::core::limits::Truncation;
use crate::core::tree::Tree;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    TD,
    LR,
    BT,
    RL,
}

#[derive(Debug, Default)]
pub struct Options {
    pub direction: Direction,
    pub title: Option<String>,
}

/// Renders the tree as a Mermaid `flowchart`.
///
/// Output is byte-stable for a given input, which snapshot tests depend on and
/// which keeps committed diagrams diffable.
pub fn render(_tree: &Tree, _truncation: &Truncation, _options: &Options) -> String {
    todo!()
}
