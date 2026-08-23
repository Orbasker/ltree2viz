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
    pub fn parse(text: &str) -> Result<Self, PathError> {
        if text.is_empty() {
            return Err(PathError::Empty);
        }

        let mut labels = Vec::new();
        for (position, label) in text.split('.').enumerate() {
            if label.is_empty() {
                return Err(PathError::EmptyLabel(position));
            }
            labels.push(label.to_owned());
        }

        Ok(Self { labels })
    }

    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    #[cfg(test)]
    pub(crate) fn from_labels<I, S>(labels: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
        }
    }

    pub fn nlevel(&self) -> usize {
        self.labels.len()
    }

    pub fn last_label(&self) -> &str {
        self.labels
            .last()
            .expect("LtreePath is never empty by construction")
    }

    pub fn parent(&self) -> Option<Self> {
        if self.labels.len() <= 1 {
            return None;
        }
        Some(Self {
            labels: self.labels[..self.labels.len() - 1].to_vec(),
        })
    }

    /// Every proper ancestor, shallowest first.
    pub fn ancestors(&self) -> std::vec::IntoIter<Self> {
        (1..self.labels.len())
            .map(|len| Self {
                labels: self.labels[..len].to_vec(),
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn is_ancestor_of(&self, other: &Self) -> bool {
        self.labels.len() < other.labels.len()
            && other.labels[..self.labels.len()] == self.labels[..]
    }
}

impl fmt::Display for LtreePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.labels.join("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> LtreePath {
        LtreePath::parse(text).expect("valid path")
    }

    #[test]
    fn parses_single_label() {
        let path = parse("root");
        assert_eq!(path.labels(), ["root"]);
        assert_eq!(path.nlevel(), 1);
        assert_eq!(path.last_label(), "root");
    }

    #[test]
    fn parses_deep_path() {
        let path = parse("a.b.c.d.e");
        assert_eq!(path.labels(), ["a", "b", "c", "d", "e"]);
        assert_eq!(path.nlevel(), 5);
        assert_eq!(path.last_label(), "e");
    }

    #[test]
    fn parses_unicode_and_hyphenated_labels() {
        let path = parse("café.日本語.a-b.node_1");
        assert_eq!(
            path.labels(),
            ["café", "日本語", "a-b", "node_1"]
                .map(String::from)
                .as_slice()
        );
    }

    #[test]
    fn round_trips_through_display() {
        assert_eq!(parse("a.b.c").to_string(), "a.b.c");
    }

    #[test]
    fn rejects_empty_path() {
        assert_eq!(LtreePath::parse(""), Err(PathError::Empty));
    }

    #[test]
    fn rejects_empty_interior_label() {
        assert_eq!(LtreePath::parse("a..b"), Err(PathError::EmptyLabel(1)));
    }

    #[test]
    fn rejects_leading_dot() {
        assert_eq!(LtreePath::parse(".a"), Err(PathError::EmptyLabel(0)));
    }

    #[test]
    fn rejects_trailing_dot() {
        assert_eq!(LtreePath::parse("a.b."), Err(PathError::EmptyLabel(2)));
    }

    #[test]
    fn parent_drops_last_label() {
        assert_eq!(parse("a.b.c").parent(), Some(parse("a.b")));
        assert_eq!(parse("a.b").parent(), Some(parse("a")));
    }

    #[test]
    fn single_label_has_no_parent() {
        assert_eq!(parse("root").parent(), None);
    }

    #[test]
    fn ancestors_are_shallowest_first_and_proper() {
        let ancestors: Vec<_> = parse("a.b.c.d").ancestors().collect();
        assert_eq!(ancestors, vec![parse("a"), parse("a.b"), parse("a.b.c")]);
    }

    #[test]
    fn single_label_has_no_ancestors() {
        assert_eq!(parse("root").ancestors().count(), 0);
    }

    #[test]
    fn is_ancestor_of_is_proper_and_prefix_based() {
        let a = parse("a");
        let ab = parse("a.b");
        let abc = parse("a.b.c");
        let other = parse("a.x");

        assert!(a.is_ancestor_of(&ab));
        assert!(a.is_ancestor_of(&abc));
        assert!(ab.is_ancestor_of(&abc));

        assert!(!abc.is_ancestor_of(&a));
        assert!(!a.is_ancestor_of(&a));
        assert!(!ab.is_ancestor_of(&other));
    }

    #[test]
    fn shared_label_prefix_is_not_a_path_prefix() {
        assert!(!parse("a.b").is_ancestor_of(&parse("a.bc")));
    }
}
