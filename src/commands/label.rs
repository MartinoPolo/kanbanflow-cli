//! `kf label` — read-only label metadata. Labels are never created implicitly.
//!
//! The vocabulary itself is derived in `crate::labels`, which `kf task` shares
//! for validating `--label` / `--add-label`.

use clap::{Args, Subcommand};

use crate::context::Context;
use crate::labels;
use crate::output::{self, Table};

#[derive(Debug, Subcommand)]
pub enum LabelCommand {
    /// List the labels in use on the board.
    List(ListArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

pub fn run(command: LabelCommand) -> anyhow::Result<()> {
    match command {
        LabelCommand::List(args) => list(args),
    }
}

fn list(args: ListArgs) -> anyhow::Result<()> {
    let context = Context::load()?;
    let groups = labels::fetch_groups(&context.client)?;
    let usage = labels::usage(&groups);

    if args.json {
        let payload: Vec<serde_json::Value> = usage
            .iter()
            .map(|(name, usage)| {
                serde_json::json!({
                    "name": name,
                    "pinned": usage.pinned,
                    "taskCount": usage.task_count,
                })
            })
            .collect();
        return output::print_json(&payload).map_err(Into::into);
    }

    let mut table = Table::new(&["NAME", "TASKS", "PINNED"]);
    for (name, usage) in &usage {
        table.row([
            name.clone(),
            usage.task_count.to_string(),
            if usage.pinned { "yes" } else { "no" }.to_string(),
        ]);
    }
    table.print_or("No labels are in use on this board.");

    // Truncated cells are paged through, so this only fires when a cell held more
    // tasks than the continuation cap allows.
    if groups.iter().any(|group| group.tasks_limited) {
        eprintln!(
            "A date-grouped column has more tasks than `kf` pages through; labels used solely by the oldest of them may be missing."
        );
    }
    Ok(())
}
