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
    pub fn id_for(&mut self, _path: &LtreePath) -> &str {
        todo!()
    }
}

/// Escapes a label for use inside a quoted Mermaid node.
pub fn escape_label(_label: &str) -> String {
    todo!()
}
