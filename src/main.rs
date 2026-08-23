use anyhow::Result;
use clap::Parser;

use ltree2mmd::cli::{Args, Command};
use ltree2mmd::db::{connect, introspect};

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Command::Tables) => run_tables(&args),
        None => todo!(),
    }
}

fn run_tables(args: &Args) -> Result<()> {
    let mut client = connect::connect(args.dsn.as_deref())?;
    let columns = introspect::list_ltree_columns(&mut client)?;

    if columns.is_empty() {
        eprintln!("No columns of type ltree found.");
        return Ok(());
    }

    for column in columns {
        println!("{column}");
    }
    Ok(())
}
