//! A guided, prompt-driven front end so the tool is usable without memorizing
//! flags. It resolves the same choices the flags express (connection, column,
//! label, grouping, filters, format, output) and then hands off to the normal
//! render pipeline in `main`.

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use ltree2viz::cli::{Args, DirectionArg, Format};
use ltree2viz::db::fetch::{Filter, fetch};
use ltree2viz::db::{connect, introspect};

/// The wizard runs when asked for explicitly, or when the tool is launched with
/// nothing to do and a real terminal to prompt on. A non-terminal (a pipe or CI)
/// keeps the old "no input selected" error so scripts fail loudly.
pub fn should_run(args: &Args) -> bool {
    if args.interactive {
        return true;
    }
    args.command.is_none()
        && args.input.is_none()
        && args.table.is_none()
        && io::stdin().is_terminal()
        && io::stderr().is_terminal()
}

pub fn run(args: &Args) -> Result<()> {
    eprintln!("ltree2viz — interactive mode (Ctrl-C to quit)\n");

    let dsn = match &args.dsn {
        Some(dsn) => Some(dsn.clone()),
        None => {
            let entered = prompt_line(
                "Postgres connection (blank = DATABASE_URL / PG* env / local)",
                "",
            )?;
            (!entered.is_empty()).then_some(entered)
        }
    };

    let mut client = connect::connect(dsn.as_deref())
        .context("could not connect; check the connection details and that Postgres is running")?;

    let columns = introspect::list_ltree_columns(&mut client)?;
    if columns.is_empty() {
        bail!("no columns of type ltree were found in this database.");
    }
    let column = if columns.len() == 1 {
        eprintln!("Using the only ltree column: {}\n", columns[0]);
        columns[0].clone()
    } else {
        let labels: Vec<String> = columns.iter().map(ToString::to_string).collect();
        columns[select("Which ltree column holds the hierarchy?", &labels, None)?].clone()
    };

    let other_columns: Vec<String> =
        introspect::list_columns(&mut client, &column.schema, &column.table)?
            .into_iter()
            .filter(|c| *c != column.column)
            .collect();

    let label_column = optional_column(
        "Display-label column (blank = last label of each path)",
        &other_columns,
    )?;
    let group_column = optional_column(
        "Group-by column — every distinct value becomes its own root",
        &other_columns,
    )?;

    let root = {
        let value = prompt_line("Restrict to a subtree/root path (blank = whole tree)", "")?;
        (!value.is_empty()).then_some(value)
    };
    let depth = {
        let value = prompt_line("Max levels below the root (blank = unlimited)", "")?;
        match value.as_str() {
            "" => None,
            n => Some(n.parse::<u32>().context("depth must be a whole number")?),
        }
    };

    let format = match select(
        "Output format",
        &[
            "HTML — interactive, collapsible page".into(),
            "Mermaid — diagram source".into(),
            "Markdown — fenced mermaid block".into(),
        ],
        Some(0),
    )? {
        0 => Format::Html,
        1 => Format::Mermaid,
        _ => Format::Md,
    };

    // HTML rotates in the page itself, so a fixed direction only matters for the
    // Mermaid formats.
    let direction = if matches!(format, Format::Html) {
        args.direction
    } else {
        match select(
            "Flow direction",
            &[
                "TD — top-down".into(),
                "LR — left-to-right".into(),
                "BT — bottom-up".into(),
                "RL — right-to-left".into(),
            ],
            Some(0),
        )? {
            0 => DirectionArg::Td,
            1 => DirectionArg::Lr,
            2 => DirectionArg::Bt,
            _ => DirectionArg::Rl,
        }
    };

    let ext = match format {
        Format::Html => "html",
        Format::Md => "md",
        Format::Mermaid => "mmd",
    };
    let default_out = format!("{}.{ext}", sanitize_filename(&column.table));
    let entered = prompt_line(
        &format!("Write to file ('-' for stdout) [{default_out}]"),
        &default_out,
    )?;
    let output = match entered.as_str() {
        "-" => None,
        path => Some(PathBuf::from(path)),
    };

    let mut resolved = args.clone();
    resolved.direction = direction;
    resolved.format = format;
    resolved.title = args.title.clone().or_else(|| Some(column.table.clone()));
    resolved.output = output;

    let filter = Filter { root, depth };
    let rows = fetch(
        &mut client,
        &column,
        label_column.as_deref(),
        group_column.as_deref(),
        &filter,
    )?;

    crate::render_rows(rows, &resolved)?;

    if let Some(path) = &resolved.output {
        eprintln!("\n✓ Wrote {}", path.display());
    }
    Ok(())
}

/// Reads one line, returning `default` when the user just presses Enter.
fn prompt_line(message: &str, default: &str) -> Result<String> {
    eprint!("{message}: ");
    io::stderr().flush().ok();
    let mut line = String::new();
    let read = io::stdin().read_line(&mut line).context("reading input")?;
    if read == 0 {
        bail!("input closed before a value was entered");
    }
    let line = line.trim();
    Ok(if line.is_empty() {
        default.to_owned()
    } else {
        line.to_owned()
    })
}

/// A numbered menu. `default` (a 0-based index) is chosen on an empty line.
fn select(message: &str, items: &[String], default: Option<usize>) -> Result<usize> {
    loop {
        eprintln!("{message}:");
        for (i, item) in items.iter().enumerate() {
            let marker = if Some(i) == default {
                "  (default)"
            } else {
                ""
            };
            eprintln!("  [{}] {item}{marker}", i + 1);
        }
        let prompt = match default {
            Some(d) => format!("enter 1-{} [{}]", items.len(), d + 1),
            None => format!("enter 1-{}", items.len()),
        };
        eprint!("{prompt}: ");
        io::stderr().flush().ok();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).context("reading input")? == 0 {
            bail!("input closed before a choice was made");
        }
        let line = line.trim();
        if line.is_empty() {
            if let Some(d) = default {
                return Ok(d);
            }
        } else if let Ok(n) = line.parse::<usize>() {
            if (1..=items.len()).contains(&n) {
                return Ok(n - 1);
            }
        }
        eprintln!("  please enter a number between 1 and {}.\n", items.len());
    }
}

/// Picks an optional column: a menu with a leading "(none)" when candidates are
/// known, or a free-text prompt when they are not (e.g. a view we cannot list).
fn optional_column(message: &str, columns: &[String]) -> Result<Option<String>> {
    if columns.is_empty() {
        let value = prompt_line(message, "")?;
        return Ok((!value.is_empty()).then_some(value));
    }
    let mut items = Vec::with_capacity(columns.len() + 1);
    items.push("(none)".to_owned());
    items.extend(columns.iter().cloned());
    let index = select(message, &items, Some(0))?;
    Ok((index != 0).then(|| items[index].clone()))
}

/// Keeps a table name usable as a filename, turning anything exotic into `_`.
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "tree".to_owned()
    } else {
        cleaned
    }
}
