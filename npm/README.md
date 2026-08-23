# ltree2mmd

Turn a Postgres [`ltree`](https://www.postgresql.org/docs/current/ltree.html)
table into a [Mermaid](https://mermaid.js.org/) diagram.

```sh
npx ltree2mmd@latest --help
```

This package is a thin JS shim. Installing it pulls in exactly one prebuilt
native binary for your platform via `optionalDependencies` — nothing is
downloaded or compiled at install time, so it works under `--ignore-scripts`,
behind proxies, and in cached `npm ci` runs.

## Usage

Read `ltree` paths from stdin and render a flowchart:

```sh
printf 'a\na.b\na.b.c\n' | npx ltree2mmd -
```

Or read straight from a database:

```sh
npx ltree2mmd --table my_tree --dsn "$DATABASE_URL"
```

The diagram is written to stdout and all diagnostics to stderr, so piping stays
clean:

```sh
npx ltree2mmd - | pbcopy
```

## Supported platforms

macOS (arm64, x64), Linux (x64, arm64; static musl builds run on Alpine and
glibc distros alike), and Windows (x64).

## Other install methods

See the [project README](https://github.com/Orbasker/ltree2mmd) for the shell
installer, `cargo install`, and Docker.

## License

MIT OR Apache-2.0
