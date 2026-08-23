# ltree2mmd

Turn a Postgres [`ltree`](https://www.postgresql.org/docs/current/ltree.html)
hierarchy into a [Mermaid](https://mermaid.js.org/) diagram.

Point it at a table and it renders the tree to stdout. GitHub renders Mermaid
natively, so a `--format md` diagram pasted into a README *is* the screenshot:

```mermaid
flowchart TD
    n0["Electronics"]
    n1["Computers"]
    n2["Desktops"]
    n3["Laptops"]
    n4["Phones"]
    n5["Android"]
    n6["iOS"]
    n7["Home"]
    n8["Garden"]
    n9["Tools"]
    n10["Kitchen"]
    n11["Cookware"]
    n0 --> n1
    n1 --> n2
    n1 --> n3
    n0 --> n4
    n4 --> n5
    n4 --> n6
    n7 --> n8
    n8 --> n9
    n7 --> n10
    n10 --> n11
```

That diagram is the exact output of the [demo](#30-second-demo) below.

## Try it in 10 seconds — no database

Stdin mode reads newline-delimited paths, so you can see what the tool does
before pointing it at anything:

```console
$ printf 'a\na.b\na.b.c\n' | cargo run -- -
flowchart TD
    n0["a"]
    n1["b"]
    n2["c"]
    n0 --> n1
    n1 --> n2
```

Or feed it a file:

```console
$ cargo run -- - < demo/paths.txt
```

## 30-second demo

`git clone` → rendered diagram, with a seeded Postgres from Docker:

```console
$ docker compose -f demo/docker-compose.yml up -d   # postgres:16 + seed.sql
$ export DATABASE_URL='postgres://demo:demo@localhost:5432/demo'
$ cargo run -- tables                               # discover ltree columns
public.catalog.path
$ cargo run -- --table catalog --format md > catalog.md
```

`catalog.md` is a fenced ```` ```mermaid ```` block — paste it into any GitHub
file and it renders as the diagram at the top of this README.

Tear down with `docker compose -f demo/docker-compose.yml down -v`.

## Usage

```
ltree2mmd --table catalog        render a table from the database
ltree2mmd tables                 list the ltree columns to choose from
ltree2mmd -                      render newline-delimited paths from stdin
```

The diagram goes to **stdout**; every diagnostic — warnings, truncation
notices, errors — goes to **stderr**. So `ltree2mmd --table t | pbcopy` and
`-o out.mmd` stay clean.

Common flags (`--help` has the full list):

| Flag | Meaning |
| --- | --- |
| `--table <T>` | Table holding the hierarchy, optionally `schema.table` |
| `-c, --path-column <C>` | The `ltree` column; auto-detected when the table has exactly one |
| `-l, --label-column <C>` | Column to display; defaults to the last label of each path |
| `-r, --root <PATH>` | Restrict output to this subtree |
| `--depth <N>` | Levels to include below the root |
| `--direction <TD\|LR\|BT\|RL>` | Flow direction (default `TD`) |
| `--format <mermaid\|md\|html>` | Raw Mermaid, a fenced markdown block, or a self-contained interactive HTML page |
| `-o, --output <FILE>` | Write to a file instead of stdout |
| `--max-nodes <N>` | Cap on total nodes (default 300) |
| `--max-children <N>` | Siblings beyond this fold into `+N more` (default 20) |
| `--no-synthesize` | Drop rows with missing ancestors instead of inferring them |
| `--title <T>` | Title shown above the diagram |

## Connecting

Connection details are resolved in this order:

1. `--dsn <URL>` (also reads from the `DATABASE_URL` env var)
2. `DATABASE_URL`
3. The libpq `PG*` variables — `PGHOST`, `PGPORT`, `PGUSER`, `PGPASSWORD`, `PGDATABASE`

The first source that yields a connection string wins. The session is opened
**read-only** with a 30-second statement timeout; the crate has no write path at
all. TLS is negotiated automatically (so managed providers like Neon, Supabase,
and RDS work out of the box) and falls back to plaintext for a local database
that does not offer it.

## Size guards and truncation

Big trees turn Mermaid into an unreadable grey blob, so two guards keep the
output legible — and both announce themselves on stderr rather than clipping
silently:

- **`--max-children`** folds each sibling list down to the limit, replacing the
  remainder with a single `+N more` node.
- **`--max-nodes`** caps the total node count, keeping a breadth-first slice so
  the surviving diagram is a shallow view of the whole tree rather than one deep
  branch.

```console
$ printf 'r.a\nr.b\nr.c\nr.d\nr.e\n' | ltree2mmd - --max-children 2
truncated: folded 3 sibling(s) into "+N more" nodes     # ← stderr
```
```mermaid
flowchart TD
    n0["r"]
    n1["a"]
    n2["b"]
    n3["+3 more"]
    n0 --> n1
    n0 --> n2
    n0 --> n3
classDef inferred stroke-dasharray:5 5,stroke:#999,color:#666,fill:#f4f4f4;
class n0,n3 inferred
```

The `+N more` node is dashed, for the same reason as the next section.

## Synthesized ancestors (the dashed nodes)

`ltree` rows can name a deep path whose intermediate ancestors have no row of
their own. By default ltree2mmd **synthesizes** those ancestors so the tree
stays connected, and styles them dashed to mark that they were inferred rather
than read from the table:

```console
$ printf 'Fruits.Apple\nFruits.Banana\nVegetables.Carrot\nGrains.Rice.Basmati\n' | ltree2mmd -
```
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

`Fruits`, `Grains`, `Rice`, and `Vegetables` are dashed: no row carried them.
Pass `--no-synthesize` to drop such rows instead (each is reported on stderr).

## Install

```console
$ cargo install --path .
```

## License

MIT OR Apache-2.0
