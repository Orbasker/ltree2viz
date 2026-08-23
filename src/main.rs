use anyhow::Result;
use clap::Parser;

use ltree2mmd::cli::{Args, Command};

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Command::Tables) => todo!(),
        None => todo!(),
    }
}
