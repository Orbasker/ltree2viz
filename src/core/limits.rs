use crate::core::tree::Tree;

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
}

pub fn apply(_tree: &mut Tree, _limits: Limits) -> Truncation {
    todo!()
}
