use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::Parser;

use ltree2viz::cli::{Args, Command, Format};
use ltree2viz::core::limits::{Limits, apply};
use ltree2viz::core::path::LtreePath;
use ltree2viz::core::render::flowchart::{Options, render};
use ltree2viz::core::render::html;
use ltree2viz::core::tree::{MissingAncestors, Row, build};
use ltree2viz::db::fetch::{Filter, fetch};
use ltree2viz::db::{connect, introspect};

mod interactive;

const STDIN_ARG: &str = "-";

fn main() -> ExitCode {
    let args = Args::parse();

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<()> {
    if interactive::should_run(args) {
        return interactive::run(args);
    }
    match &args.command {
        Some(Command::Tables) => run_tables(args),
        None => match args.input.as_deref() {
            Some(STDIN_ARG) => run_stdin(args),
            Some(other) => bail!(
                "unexpected argument {other:?}; pass `-` to read paths from stdin, \
                 or --table <TABLE> to read from a database"
            ),
            None => run_database(args),
        },
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

/// Reads newline-delimited paths from stdin and renders them with no database.
fn run_stdin(args: &Args) -> Result<()> {
    let mut text = String::new();
    io::stdin()
        .read_to_string(&mut text)
        .context("reading paths from stdin")?;

    let rows = parse_paths(&text)?;
    render_rows(rows, args)
}

/// Parses one path per non-blank line, reporting the line that fails.
fn parse_paths(text: &str) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let path = LtreePath::parse(line)
            .with_context(|| format!("line {}: invalid path {line:?}", number + 1))?;
        rows.push(Row { path, label: None });
    }
    Ok(rows)
}

fn run_database(args: &Args) -> Result<()> {
    let Some(table) = args.table.as_deref() else {
        bail!(
            "no input selected: pass --table <TABLE> to read from a database, \
             `-` to read paths from stdin, or run `ltree2viz tables` to discover \
             ltree columns. See --help for details."
        );
    };

    let mut client = connect::connect(args.dsn.as_deref())?;
    let column = introspect::resolve_column(&mut client, table, args.path_column.as_deref())?;
    let filter = Filter {
        root: args.root.clone(),
        depth: args.depth,
    };
    let rows = fetch(
        &mut client,
        &column,
        args.label_column.as_deref(),
        args.group_by.as_deref(),
        &filter,
    )?;

    render_rows(rows, args)
}

/// The shared core pipeline: build the tree, apply limits, render, and write.
///
/// Every diagnostic goes to stderr so the rendered diagram on stdout (or in
/// `--output`) stays clean.
fn render_rows(rows: Vec<Row>, args: &Args) -> Result<()> {
    let missing = if args.no_synthesize {
        MissingAncestors::Drop
    } else {
        MissingAncestors::Synthesize
    };

    let mut tree = build(rows, missing);
    for warning in &tree.warnings {
        eprintln!("warning: {warning}");
    }

    // HTML collapses interactively, so it renders the full tree rather than a
    // pre-truncated one.
    let document = if let Format::Html = args.format {
        html::render(
            &tree,
            &html::Options {
                title: args.title.clone(),
            },
        )
    } else {
        let limits = Limits {
            max_nodes: args.max_nodes,
            max_children: args.max_children,
        };
        let truncation = apply(&mut tree, limits);
        if let Some(summary) = truncation.summary() {
            eprintln!("{summary}");
        }

        let options = Options {
            direction: args.direction.into(),
            title: args.title.clone(),
        };
        let diagram = render(&tree, &truncation, &options);
        wrap(diagram, args.format)
    };

    write_output(&document, args.output.as_deref())
}

/// Wraps the flowchart in a fenced ```` ```mermaid ```` block for `--format md`.
fn wrap(diagram: String, format: Format) -> String {
    match format {
        Format::Md => format!("```mermaid\n{diagram}```\n"),
        _ => diagram,
    }
}

fn write_output(document: &str, output: Option<&Path>) -> Result<()> {
    match output {
        Some(path) => {
            std::fs::write(path, document)
                .with_context(|| format!("writing output to {}", path.display()))?;
        }
        None => {
            io::stdout()
                .write_all(document.as_bytes())
                .context("writing output to stdout")?;
        }
    }
    Ok(())
}
