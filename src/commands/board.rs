//! `kf board` — the board's columns and their canonical-state mapping.

use anyhow::Context as _;
use clap::Args;

use crate::api::models::{Board, TaskGroup};
use crate::context::Context;
use crate::output::{self, Table};
use crate::tasks;

#[derive(Debug, Args)]
pub struct BoardArgs {
    /// Print the raw `GET /board` response.
    #[arg(long)]
    pub json: bool,
    /// Also show how many tasks sit in each column (costs one extra request).
    #[arg(long)]
    pub counts: bool,
}

pub fn run(args: BoardArgs) -> anyhow::Result<()> {
    let context = Context::load()?;

    if args.json {
        // Deserialized as Value so `--json` stays lossless if the API grows fields.
        let raw: serde_json::Value = context
            .client
            .get_json("board", &[])
            .context("fetching the board (GET /board)")?;
        return output::print_json(&raw).map_err(Into::into);
    }

    let board: Board = context
        .client
        .get_json("board", &[])
        .context("fetching the board (GET /board)")?;
    let groups: Vec<TaskGroup> = if args.counts {
        tasks::fetch_all_groups(&context.client).context("counting tasks (GET /tasks)")?
    } else {
        Vec::new()
    };

    println!("{} ({})", board.name, board.id);

    let mut headers = vec!["COLUMN", "STATE", "COLUMN ID"];
    if args.counts {
        headers.push("TASKS");
    }
    let mut table = Table::new(&headers);
    let mut truncated = false;
    for column in &board.columns {
        let state = context
            .config
            .state_for_column(&column.unique_id)
            .map(|state| state.as_str().to_string())
            .unwrap_or_else(|| "-".to_string());
        let mut row = vec![column.name.clone(), state, column.unique_id.clone()];
        if args.counts {
            let matching = groups
                .iter()
                .filter(|group| group.column_id == column.unique_id);
            let mut count = 0usize;
            for group in matching {
                count += group.tasks.len();
                truncated |= group.tasks_limited;
            }
            row.push(count.to_string());
        }
        table.row(row);
    }
    table.print_or("This board has no columns.");

    // Truncated cells are paged through, so this only fires when a cell held more
    // tasks than the continuation cap allows.
    if truncated {
        eprintln!(
            "A date-grouped column has more tasks than `kf` pages through; its count is a lower bound."
        );
    }
    if !board.swimlanes.is_empty() {
        println!("Swimlanes: {}", board.swimlanes.len());
    }
    Ok(())
}
