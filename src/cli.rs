use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::core::render::flowchart::Direction;

#[derive(Debug, Clone, Copy, ValueEnum, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Mermaid,
    Md,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "UPPER")]
pub enum DirectionArg {
    Td,
    Lr,
    Bt,
    Rl,
}

impl From<DirectionArg> for Direction {
    fn from(value: DirectionArg) -> Self {
        match value {
            DirectionArg::Td => Direction::TD,
            DirectionArg::Lr => Direction::LR,
            DirectionArg::Bt => Direction::BT,
            DirectionArg::Rl => Direction::RL,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "ltree2mmd",
    version,
    about = "Turn a Postgres ltree table into a Mermaid diagram",
    long_about = "Turn a Postgres ltree hierarchy into a Mermaid flowchart.\n\n\
        Three ways to run it:\n\n  \
        ltree2mmd --table catalog        render a table from the database\n  \
        ltree2mmd tables                 list the ltree columns to choose from\n  \
        ltree2mmd -                      render newline-delimited paths from stdin\n\n\
        The diagram goes to stdout; every diagnostic (warnings, truncation\n\
        notices, errors) goes to stderr, so `ltree2mmd --table t | pbcopy` and\n\
        `-o out.mmd` stay clean.\n\n\
        Connection details are resolved in order: --dsn, then DATABASE_URL, then\n\
        the libpq PG* variables (PGHOST, PGPORT, PGUSER, PGPASSWORD, PGDATABASE).",
    after_help = "EXAMPLES:\n  \
        ltree2mmd --table catalog\n  \
        ltree2mmd --table store.catalog --root Electronics --depth 2\n  \
        ltree2mmd --table catalog --direction LR --format md -o tree.md\n  \
        ltree2mmd tables\n  \
        printf 'a\\na.b\\na.b.c\\n' | ltree2mmd -"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Read newline-delimited paths from stdin instead of a database (pass `-`)
    #[arg(value_name = "-")]
    pub input: Option<String>,

    /// Postgres connection string
    #[arg(long, env = "DATABASE_URL")]
    pub dsn: Option<String>,

    /// Table holding the hierarchy, optionally schema-qualified
    #[arg(short, long)]
    pub table: Option<String>,

    /// Column of type ltree; auto-detected when the table has exactly one
    #[arg(short = 'c', long)]
    pub path_column: Option<String>,

    /// Column to display; defaults to the last label of each path
    #[arg(short, long)]
    pub label_column: Option<String>,

    /// Restrict output to this subtree
    #[arg(short, long)]
    pub root: Option<String>,

    /// Levels to include below the root
    #[arg(long)]
    pub depth: Option<u32>,

    /// Flow direction of the rendered graph
    #[arg(long, value_enum, default_value_t = DirectionArg::Td)]
    pub direction: DirectionArg,

    /// Cap on total nodes; the rest are dropped and reported
    #[arg(long, default_value_t = 300)]
    pub max_nodes: usize,

    /// Siblings beyond this fold into a single "+N more" node
    #[arg(long, default_value_t = 20)]
    pub max_children: usize,

    /// Drop rows with missing ancestors instead of inferring them
    #[arg(long)]
    pub no_synthesize: bool,

    /// Title shown above the diagram
    #[arg(long)]
    pub title: Option<String>,

    /// Write to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Output format: raw mermaid, or a fenced markdown block
    #[arg(short, long, value_enum, default_value_t = Format::Mermaid)]
    pub format: Format,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List columns of type ltree in the database
    Tables,
}
