# AGENTS.md

Guidance for contributors (human and AI) working in this repository.

## Versioning

`Cargo.toml` is the single source of truth for the release version. Releases
are tag-driven: pushing a `vX.Y.Z` tag triggers the `Release` workflow
(cargo-dist). Tagging is automated — see [Release process](#release-process).

CI enforces a version bump on every PR (the `version-bump` gate in `ci.yml`):
the `Cargo.toml` version must be strictly higher than the base branch. This
document defines *which* level to bump.

### SemVer bump rules

Given `MAJOR.MINOR.PATCH`, pick the level by the largest applicable change:

- **MAJOR** — a breaking change to the user-facing contract: CLI flags/args
  removed or changed in meaning, output format changed incompatibly, or exit
  codes changed.
- **MINOR** — a backwards-compatible new capability: a new flag, a new output
  option, or additive behavior that doesn't break existing invocations.
- **PATCH** — everything else: bug fixes, documentation, CI/build changes,
  refactors, and dependency bumps with no user-facing effect.

Bump `Cargo.toml` (and keep `Cargo.lock` in sync) in the same PR as the change.

## Release process

1. Merge a PR to `main` with the appropriate `Cargo.toml` version bump.
2. On merge, `.github/workflows/auto-tag.yml` reads the `Cargo.toml` version
   and pushes the matching `vX.Y.Z` tag if it doesn't already exist.
3. The tag push triggers the `Release` workflow, which builds and publishes
   the GitHub Release, crate, and npm artifacts.

Do not push release tags by hand — a tag that doesn't match `Cargo.toml`
produces a failed `Release` run ("This workspace doesn't have anything for
dist to Release!").

### Required setup

- `RELEASE_PAT` repo secret (a PAT with contents read/write). Tags pushed with
  the default `GITHUB_TOKEN` do not trigger other workflows, so auto-tag uses a
  PAT to push tags that can fire `Release`.
