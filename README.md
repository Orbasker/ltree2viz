<a id="readme-top"></a>

<div align="center">

# ltree2viz

**Visualize a PostgreSQL [`ltree`](https://www.postgresql.org/docs/current/ltree.html) hierarchy as a [Mermaid](https://mermaid.js.org/) diagram or an interactive HTML tree — straight from the database, from a query, or from a plain list of paths on stdin.**

[![Crates.io][crates-badge]][crates-url]
[![Downloads][downloads-badge]][crates-url]
[![npm][npm-badge]][npm-url]
[![CI][ci-badge]][ci-url]
[![License][license-badge]][license-url]
[![MSRV][msrv-badge]][msrv-url]

[Try it](#try-it-in-30-seconds) · [Install](#install) · [Usage](#usage) · [Report a bug][issues-url] · [Request a feature][issues-url]

</div>

<details>
  <summary>Table of contents</summary>

- [Try it in 30 seconds](#try-it-in-30-seconds)
- [Install](#install)
- [Usage](#usage)
- [Connecting to a database](#connecting-to-a-database)
- [Size guards and truncation](#size-guards-and-truncation)
- [Synthesized ancestors (the dashed nodes)](#synthesized-ancestors-the-dashed-nodes)
- [Requirements](#requirements)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)

</details>

## About

Point ltree2viz at a Postgres table with an `ltree` column and it renders the
hierarchy — no query writing, no export step. Read-only by construction (the
crate `forbid`s `unsafe` and has no write path), it fits equally in a shell
pipeline, a `docker compose up`, or a CI job that keeps a diagram in your docs
in sync with the data.

```mermaid
---
title: "catalog"
---
flowchart TD
    n0["Top"]
    n1["Collections"]
    n2["Pictures"]
    n3["Astronomy"]
    n4["Hobbies"]
    n5["Amateur Astronomy"]
    n6["Science"]
    n7["Astronomy"]
    n8["Astrophysics"]
    n9["Cosmology"]
    n0 --> n1
    n1 --> n2
    n2 --> n3
    n0 --> n4
    n4 --> n5
    n0 --> n6
    n6 --> n7
    n7 --> n8
    n7 --> n9
```

That diagram is the tool's actual output, rendered by GitHub from a
` ```mermaid ` block. `--format md` wraps the output in exactly this block, so
your README doubles as a screenshot that never drifts from the data.

## Try it in 30 seconds

No database, no build — pipe newline-delimited paths through stdin:

```sh
printf 'a\na.b\na.b.c\na.b.d\n' | ltree2viz -
```

Or run the full database demo. It starts a seeded Postgres and prints a diagram,
going from `git clone` to rendered output in one command:

```sh
git clone https://github.com/Orbasker/ltree2viz
cd ltree2viz
docker compose up
```

The `db` service loads [`demo/seed.sql`](demo/seed.sql) — a `catalog` table with
a `path ltree` column — and the `ltree2viz` service renders it, printing the
` ```mermaid ` block above to the log. Paste it into any Markdown file on GitHub
to see the picture.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Install

With Node already on your machine, no toolchain needed:

```sh
npx ltree2viz@latest --help
```

`npx` pulls exactly one prebuilt binary for your platform via
`optionalDependencies` — nothing compiles at install time. To put it on your
`PATH`: `npm i -g ltree2viz`.

Or with Cargo:

```sh
cargo install ltree2viz
```

Prebuilt binaries for macOS (arm64, x64), Linux (x64, arm64), and Windows (x64)
are attached to every
[release](https://github.com/Orbasker/ltree2viz/releases), so you do not need a
Rust toolchain:

```sh
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Orbasker/ltree2viz/releases/latest/download/ltree2viz-installer.sh | sh
```

```powershell
# Windows
powershell -c "irm https://github.com/Orbasker/ltree2viz/releases/latest/download/ltree2viz-installer.ps1 | iex"
```

On macOS or Linuxbrew, install the prebuilt binary through Homebrew (no Rust
toolchain needed):

```sh
brew tap Orbasker/tap
brew install ltree2viz
```

Or from a clone: `cargo install --path .`

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Usage

Three ways to feed it a hierarchy:

```sh
ltree2viz --table catalog        # render a table from the database
ltree2viz tables                 # list the ltree columns to choose from
ltree2viz -                      # render newline-delimited paths from stdin
```

The diagram goes to **stdout**; every diagnostic — warnings, truncation
notices, errors — goes to **stderr**. So `ltree2viz --table t | pbcopy` and
`ltree2viz --table t -o out.mmd` both stay clean.

Common options:

| Flag | Meaning |
| --- | --- |
| `-t, --table <TABLE>` | Table holding the hierarchy, optionally schema-qualified |
| `-c, --path-column <COL>` | The `ltree` column; auto-detected when the table has exactly one |
| `-l, --label-column <COL>` | Column to display; defaults to the last label of each path |
| `-r, --root <PATH>` | Restrict output to this subtree |
| `--depth <N>` | Levels to include below the root |
| `--direction <TD\|LR\|BT\|RL>` | Flow direction (default `TD`) |
| `--max-nodes <N>` | Cap on total nodes (default `300`) |
| `--max-children <N>` | Siblings beyond this fold into a `+N more` node (default `20`) |
| `--no-synthesize` | Drop rows with missing ancestors instead of inferring them |
| `--title <TEXT>` | Title shown above the diagram |
| `-f, --format <mermaid\|md\|html>` | Output format (default `mermaid`) |
| `-o, --output <FILE>` | Write to a file instead of stdout |

`--format md` wraps the flowchart in a fenced ` ```mermaid ` block for pasting
into Markdown. `--format html` emits a self-contained interactive page — one
file, no network — with a collapsible tree, node search, expand/collapse to a
depth, orientation control, and shift-click path highlighting:

<div align="center">
  <img src="demo/assets/html-tree.png" alt="ltree2viz --format html: an interactive collapsible tree with a toolbar for expand/collapse, depth, orientation, search, and path highlighting" width="80%">
</div>

That is the actual `--format html` output for the same `catalog` hierarchy shown
above — generated with the [demo](#try-it-in-30-seconds):

```sh
printf 'Fruits.Apple\nFruits.Banana\nVegetables.Carrot\nGrains.Rice.Basmati\n' \
  | ltree2viz - --format html --title catalog -o catalog.html
```

Open `catalog.html` in any browser — there is no build step and nothing to
serve.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Connecting to a database

Connection details are resolved in this order — the first one that is set wins:

1. **`--dsn <URL>`** on the command line
2. **`DATABASE_URL`** in the environment
3. the standard libpq **`PG*`** variables (`PGHOST`, `PGPORT`, `PGUSER`,
   `PGPASSWORD`, `PGDATABASE`)

```sh
ltree2viz --dsn 'postgres://user:pass@host/db' --table catalog
DATABASE_URL='postgres://user:pass@host/db' ltree2viz --table catalog
PGHOST=host PGUSER=user PGDATABASE=db ltree2viz --table catalog
```

The session is opened `READ ONLY` with a 30-second statement timeout, and the
crate forbids `unsafe` and contains no write path at all. Managed providers
(Neon, Supabase, RDS, …) are reached over TLS using the platform trust store;
for a local plaintext server, add `?sslmode=disable` to the URL.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Size guards and truncation

Mermaid turns into an unreadable grey blob past a few hundred nodes, so two
guards keep the diagram legible. **Both are reported loudly on stderr** — a
clipped tree never silently passes for a complete one:

- **`--max-children`** (default 20): once a node has more than this many
  siblings, the extras collapse into a single dashed `+N more` node.
- **`--max-nodes`** (default 300): children are folded first, then the tree is
  cut breadth-first to this many nodes, keeping a shallow overview rather than
  one deep branch.

```
$ ltree2viz --table big_catalog --max-nodes 100
truncated: folded 4120 sibling(s) into "+N more" nodes; dropped 380 node(s) past the node limit
```

Raise the limits when you want the whole thing: `--max-nodes 100000
--max-children 100000`.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Synthesized ancestors (the dashed nodes)

`ltree` stores a full path per row, but the intermediate rows need not exist. If
`Fruits.Apple` is present but `Fruits` is not, ltree2viz **synthesizes** the
missing `Fruits` node so the tree still connects — and marks it dashed, so an
inferred node is never mistaken for one that was actually read:

```mermaid
flowchart TD
    n0["Fruits"]
    n1["Apple"]
    n2["Banana"]
    n3["Grains"]
    n4["Rice"]
    n5["Basmati"]
    n6["Vegetables"]
    n7["Carrot"]
    n0 --> n1
    n0 --> n2
    n3 --> n4
    n4 --> n5
    n6 --> n7
classDef inferred stroke-dasharray:5 5,stroke:#999,color:#666,fill:#f4f4f4;
class n0,n3,n4,n6 inferred
```

That was produced by:

```sh
printf 'Fruits.Apple\nFruits.Banana\nVegetables.Carrot\nGrains.Rice.Basmati\n' | ltree2viz -
```

`Fruits`, `Vegetables`, `Grains`, and `Rice` are all dashed — no row carried
them. The `+N more` collapse nodes from the size guards are drawn the same way,
for the same reason. Pass `--no-synthesize` to drop rows with missing ancestors
(and get a warning for each) instead.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Requirements

- Postgres with the `ltree` extension installed. It does not need to be on your
  `search_path` — the query casts the path column to `text`, so any schema
  works.
- Rust 1.85 or newer to build from source. Prebuilt binaries need nothing.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the dev setup, how to run the
Postgres-backed tests, and the release procedure.

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 licence, shall
be dual-licensed as above, without any additional terms or conditions.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Acknowledgments

- [Mermaid](https://mermaid.js.org/) — the diagram syntax GitHub renders inline.
- PostgreSQL [`ltree`](https://www.postgresql.org/docs/current/ltree.html) — the
  hierarchical label-path type this tool reads.
- [Best-README-Template](https://github.com/othneildrew/Best-README-Template) —
  the layout this README borrows from.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

[crates-badge]: https://img.shields.io/crates/v/ltree2viz.svg?logo=rust
[crates-url]: https://crates.io/crates/ltree2viz
[downloads-badge]: https://img.shields.io/crates/d/ltree2viz.svg
[npm-badge]: https://img.shields.io/npm/v/ltree2viz.svg?logo=npm
[npm-url]: https://www.npmjs.com/package/ltree2viz
[ci-badge]: https://github.com/Orbasker/ltree2viz/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/Orbasker/ltree2viz/actions/workflows/ci.yml
[license-badge]: https://img.shields.io/crates/l/ltree2viz.svg
[license-url]: #license
[msrv-badge]: https://img.shields.io/badge/MSRV-1.85-blue.svg
[msrv-url]: #requirements
[issues-url]: https://github.com/Orbasker/ltree2viz/issues
