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
    about = "Turn a Postgres ltree table into a Mermaid diagram"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Read newline-delimited paths from stdin instead of a database
    #[arg(value_name = "-", hide = true)]
    pub stdin: Option<String>,

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

    #[arg(long, value_enum, default_value_t = DirectionArg::Td)]
    pub direction: DirectionArg,

    #[arg(long, default_value_t = 300)]
    pub max_nodes: usize,

    /// Siblings beyond this fold into a single "+N more" node
    #[arg(long, default_value_t = 20)]
    pub max_children: usize,

    /// Drop rows with missing ancestors instead of inferring them
    #[arg(long)]
    pub no_synthesize: bool,

    #[arg(long)]
    pub title: Option<String>,

    /// Write to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    #[arg(short, long, value_enum, default_value_t = Format::Mermaid)]
    pub format: Format,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List columns of type ltree in the database
    Tables,
}
