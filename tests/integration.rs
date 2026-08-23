//! Integration tests. Path fixtures live in `tests/fixtures/`; accepted `insta`
//! snapshots are written to `tests/snapshots/`.

use std::fs;

const FIXTURES: [&str; 3] = ["simple.txt", "forest.txt", "missing_ancestors.txt"];

#[test]
fn fixtures_are_readable() {
    for name in FIXTURES {
        let path = format!("tests/fixtures/{name}");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        assert!(!text.trim().is_empty(), "{name} is empty");
    }
}
