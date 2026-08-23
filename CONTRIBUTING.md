# Contributing to ltree2mmd

Thanks for taking the time. Bug reports, feature ideas, and pull requests are all welcome.

## Toolchain

Stable Rust 1.85 or newer (the crate is edition 2024). No other tooling is required to build and
run the test suite.

## The checks

Run these before pushing:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

A bare `cargo test` passes without a database — the Postgres-backed targets detect the missing
`TEST_DATABASE_URL` and skip with a note on stderr.

`.github/workflows/ci.yml` runs the same three on Linux, macOS, and Windows, plus two jobs you
can't easily reproduce with one command: an `integration` job that repeats `cargo test` against a
`postgres:16` service with `TEST_DATABASE_URL` set, and a `mermaid` job that renders every
`tests/fixtures/*.txt` and feeds the result through `mmdc` to prove the generated diagrams actually
parse.

## Running the database tests

Most of the suite is pure logic, but `tests/cli.rs`, `tests/fetch.rs`, and `tests/introspect.rs`
exercise real SQL. Point `TEST_DATABASE_URL` at a Postgres you don't mind being scribbled on:

```sh
docker run --rm -d --name ltree2mmd-pg \
  -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres:17

export TEST_DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres
cargo test
```

The role needs `CREATE SCHEMA`, `CREATE EXTENSION ltree`, and `CREATE DATABASE` — the last one only
for a single test that installs `ltree` outside `search_path`; it skips on its own if the role can't
create databases. Each test creates a throwaway schema and drops it on the way out, so runs are
isolated and parallel-safe.

## Snapshot tests

`tests/corpus.rs` uses [`insta`](https://insta.rs/) to pin the rendered output of a corpus of
hierarchies. `tests/snapshots/` is committed. When output changes on purpose:

```sh
cargo install cargo-insta   # once
cargo insta review          # accept or reject each diff
```

`*.snap.new` files are gitignored, so an unreviewed snapshot never lands accidentally.

## Code conventions

- `rustfmt` and `clippy` settings live in `rustfmt.toml` and `clippy.toml`; don't override them
  per-file.
- `unsafe_code` is `forbid`ed crate-wide.
- Comments explain non-obvious behaviour — a subtle invariant, an ordering constraint, a
  workaround. They do not narrate what changed, reference tickets, or restate the code. The diff
  and the PR description are where "why it changed" belongs.
- Errors surface through `anyhow` with `.context(...)` describing the operation that failed.
  Diagnostics go to stderr; stdout is reserved for the rendered document.

## Pull requests

- Branch off `main`.
- Keep the change focused; a PR that does one thing is far easier to review.
- Commit and PR titles follow the existing history: a short imperative sentence, then the issue
  key. For example: `Add snapshot test corpus (ANI-56)`.
- New behaviour comes with a test. Rendering changes come with a corpus case.

## Releasing

Releases are automated by [`dist`](https://opensource.axo.dev/cargo-dist/). Pushing a version tag
builds the binaries for all five supported platforms, creates the GitHub release, and publishes to
crates.io.

1. Bump `version` in `Cargo.toml`.
2. `cargo build` to refresh `Cargo.lock`, then commit and merge to `main`.
3. Tag and push:

   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```

To preview what a tag would produce without pushing anything, install `dist` and run `dist plan`.
Release builds enable the `vendored-tls` feature so the Linux binaries statically link OpenSSL and
run on older distributions; the feature is off by default and does not affect normal development.

## Licence

By contributing, you agree that your contributions are dual-licensed under the MIT and Apache-2.0
licences, matching the rest of the project. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).
