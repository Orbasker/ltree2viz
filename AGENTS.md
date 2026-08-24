# Agent & contributor guide

## Versioning and releases

Releases are **tag-driven**: pushing a `vX.Y.Z` tag runs the `Release` workflow
(cargo-dist), which builds artifacts and publishes to GitHub Releases, crates.io,
npm, and the Homebrew tap. `Cargo.toml`'s `version` is the **single source of
truth** — the tag must match it exactly.

### Every PR must bump `Cargo.toml`

CI (`version-bump` job in `ci.yml`) fails any PR that does not raise
`Cargo.toml`'s `version` above the base branch. Pick the level by the nature of
the change, following [SemVer](https://semver.org):

- **major** (`X`+1.0.0) — a breaking change: removed/renamed CLI flags or
  subcommands, changed default output format, changed exit codes, or any change
  that would break an existing user's invocation or downstream parsing.
- **minor** (`x.Y`+1.0) — backwards-compatible new capability: a new flag,
  subcommand, output mode, or rendering feature that doesn't change existing
  behavior.
- **patch** (`x.y.Z`+1) — backwards-compatible fix or internal-only change: bug
  fixes, docs, CI, refactors, dependency bumps with no user-visible effect.

When in doubt between two levels, choose the higher one. Never reuse a version
that already has a tag — that collides with a published release.

### Releasing

Do **not** push tags by hand. On merge to `main`, the `Auto-tag release`
workflow (`auto-tag.yml`) reads `Cargo.toml`'s version and pushes the matching
`vX.Y.Z` tag if it doesn't already exist, which triggers `Release`. So the whole
release is decided by the version you put in `Cargo.toml` in the PR.

If a release must be re-cut, bump `Cargo.toml` again in a new PR rather than
re-tagging.
