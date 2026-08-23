use std::fmt;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathError {
    #[error("path is empty")]
    Empty,
    #[error("empty label at position {0}")]
    EmptyLabel(usize),
}

/// A parsed `ltree` path: an ordered, non-empty list of labels.
///
/// Parsing is permissive about which characters a label may contain and strict
/// only about structure. The set of legal label characters differs between
/// Postgres versions, so pinning it here would reject valid data.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LtreePath {
    labels: Vec<String>,
}

impl LtreePath {
    pub fn parse(_text: &str) -> Result<Self, PathError> {
        todo!()
    }

    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    pub fn nlevel(&self) -> usize {
        self.labels.len()
    }

    pub fn last_label(&self) -> &str {
        todo!()
    }

    pub fn parent(&self) -> Option<Self> {
        todo!()
    }

    /// Every proper ancestor, shallowest first.
    pub fn ancestors(&self) -> std::vec::IntoIter<Self> {
        todo!()
    }

    pub fn is_ancestor_of(&self, _other: &Self) -> bool {
        todo!()
    }
}

impl fmt::Display for LtreePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.labels.join("."))
    }
}
