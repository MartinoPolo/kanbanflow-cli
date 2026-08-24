//! `kf subtask` — an issue's checklist items.
//!
//! Subtasks come inline with the issue read, so every verb here resolves the issue
//! once and then addresses the item through the API's `by-index` endpoint —
//! indexes are unambiguous where names may repeat or need URL escaping.

use anyhow::Context as _;
use clap::{Args, Subcommand};

use crate::api::models::{Issue, SubTask, SubTaskPayload};
use crate::context::Context;
use crate::guard;
use crate::output::{self, Table};
use crate::resolve;

#[derive(Debug, Subcommand)]
pub enum SubtaskCommand {
    /// Append a checklist item.
    Add(AddArgs),
    /// List an issue's checklist items.
    List(ListArgs),
    /// Mark a checklist item finished.
    Check(CheckArgs),
    /// Mark a checklist item unfinished.
    Uncheck(CheckArgs),
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Checklist item name.
    pub name: String,
    /// Add even when the issue belongs to someone else.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Issue number (`E613`) or issue ID.
    pub issue: String,
    /// Exact checklist item name, or its 1-based position in the list.
    pub subtask: String,
    /// Change even when the issue belongs to someone else.
    #[arg(long)]
    pub force: bool,
}

/// Why a `name-or-index` selector matched nothing usable.
#[derive(Debug, PartialEq, Eq)]
enum SelectError {
    NotFound,
    /// Several items share the name; the position must be used instead.
    AmbiguousName(usize),
}

pub fn run(command: SubtaskCommand) -> anyhow::Result<()> {
    let context = Context::load()?;
    match command {
        SubtaskCommand::Add(args) => add(&context, args),
        SubtaskCommand::List(args) => list(&context, args),
        SubtaskCommand::Check(args) => set_finished(&context, args, true),
        SubtaskCommand::Uncheck(args) => set_finished(&context, args, false),
    }
}

fn add(context: &Context, args: AddArgs) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    guard::ensure_can_mutate(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("adding a subtask to issue {}", issue.reference()))?;

    let payload = SubTaskPayload {
        name: Some(args.name.clone()),
        ..Default::default()
    };
    context
        .client
        .post_json_discard(&format!("tasks/{}/subtasks", issue.id), &payload)
        .with_context(|| {
            format!(
                "adding subtask `{}` to issue {}",
                args.name,
                issue.reference()
            )
        })?;
    println!("[ ] {}", args.name);
    output::print_affected_issue(&issue);
    Ok(())
}

fn list(context: &Context, args: ListArgs) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;

    if args.json {
        output::print_json(&issue.sub_tasks)?;
        return Ok(());
    }
    let mut table = Table::new(&["#", "DONE", "NAME"]);
    for (index, subtask) in issue.sub_tasks.iter().enumerate() {
        table.row([
            format!("{}", index + 1),
            checkbox(subtask.finished).to_string(),
            subtask.name.clone(),
        ]);
    }
    table.print_or("No subtasks on this issue.");
    Ok(())
}

fn set_finished(context: &Context, args: CheckArgs, finished: bool) -> anyhow::Result<()> {
    let issue = resolve::resolve_issue_named(&context.client, &args.issue)?;
    guard::ensure_can_mutate(&issue, context.my_user_id()?, args.force)
        .with_context(|| format!("changing a subtask of issue {}", issue.reference()))?;

    let index = find_subtask(&issue.sub_tasks, &args.subtask)
        .map_err(|error| selector_error(error, &issue, &args.subtask))?;
    let subtask = &issue.sub_tasks[index];

    if subtask.finished == finished {
        println!(
            "{} {} (already {})",
            checkbox(finished),
            subtask.name,
            if finished { "checked" } else { "unchecked" }
        );
        return Ok(());
    }

    let payload = SubTaskPayload {
        finished: Some(finished),
        ..Default::default()
    };
    context
        .client
        .post_json_discard(
            &format!("tasks/{}/subtasks/by-index/{index}", issue.id),
            &payload,
        )
        .with_context(|| {
            format!(
                "updating subtask `{}` of issue {}",
                subtask.name,
                issue.reference()
            )
        })?;
    println!("{} {}", checkbox(finished), subtask.name);
    output::print_affected_issue(&issue);
    Ok(())
}

fn checkbox(finished: bool) -> &'static str {
    if finished {
        "[x]"
    } else {
        "[ ]"
    }
}

/// Resolve a `name-or-index` selector to a 0-based subtask index.
///
/// An exact (case-sensitive, as the API matches) name wins over a positional
/// reading, so a checklist item literally called "2" stays addressable.
fn find_subtask(subtasks: &[SubTask], selector: &str) -> Result<usize, SelectError> {
    let by_name: Vec<usize> = subtasks
        .iter()
        .enumerate()
        .filter(|(_, subtask)| subtask.name == selector)
        .map(|(index, _)| index)
        .collect();
    match by_name.as_slice() {
        [single] => return Ok(*single),
        [] => {}
        several => return Err(SelectError::AmbiguousName(several.len())),
    }
    match selector.trim().parse::<usize>() {
        Ok(position) if position >= 1 && position <= subtasks.len() => Ok(position - 1),
        _ => Err(SelectError::NotFound),
    }
}

/// Turn a selector failure into a message that shows the caller what exists.
fn selector_error(error: SelectError, issue: &Issue, selector: &str) -> anyhow::Error {
    let listing = issue
        .sub_tasks
        .iter()
        .enumerate()
        .map(|(index, subtask)| {
            format!(
                "\n  {}. {} {}",
                index + 1,
                checkbox(subtask.finished),
                subtask.name
            )
        })
        .collect::<String>();
    match error {
        SelectError::AmbiguousName(count) => anyhow::anyhow!(
            "Issue {} has {count} subtasks named `{selector}`; use its position instead:{listing}",
            issue.reference()
        ),
        SelectError::NotFound if issue.sub_tasks.is_empty() => {
            anyhow::anyhow!("Issue {} has no subtasks.", issue.reference())
        }
        SelectError::NotFound => anyhow::anyhow!(
            "No subtask named `{selector}` on issue {}, and it is not a position between 1 and {}:{listing}",
            issue.reference(),
            issue.sub_tasks.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subtasks(names: &[&str]) -> Vec<SubTask> {
        names
            .iter()
            .map(|name| SubTask {
                name: (*name).to_string(),
                finished: false,
                user_id: None,
                due_date_timestamp: None,
                due_date_timestamp_local: None,
            })
            .collect()
    }

    #[test]
    fn exact_name_matches_first() {
        let items = subtasks(&["Write", "Proofread"]);
        assert_eq!(find_subtask(&items, "Proofread"), Ok(1));
        // Case-sensitive, like the API's by-name endpoint.
        assert_eq!(
            find_subtask(&items, "proofread"),
            Err(SelectError::NotFound)
        );
    }

    #[test]
    fn positions_are_one_based_and_bounded() {
        let items = subtasks(&["Write", "Proofread"]);
        assert_eq!(find_subtask(&items, "1"), Ok(0));
        assert_eq!(find_subtask(&items, " 2 "), Ok(1));
        assert_eq!(find_subtask(&items, "0"), Err(SelectError::NotFound));
        assert_eq!(find_subtask(&items, "3"), Err(SelectError::NotFound));
    }

    #[test]
    fn a_numeric_name_beats_the_position_reading() {
        let items = subtasks(&["Write", "1"]);
        assert_eq!(find_subtask(&items, "1"), Ok(1));
    }

    #[test]
    fn repeated_names_are_ambiguous() {
        let items = subtasks(&["Write", "Write"]);
        assert_eq!(
            find_subtask(&items, "Write"),
            Err(SelectError::AmbiguousName(2))
        );
    }

    #[test]
    fn empty_checklist_finds_nothing() {
        assert_eq!(find_subtask(&[], "Write"), Err(SelectError::NotFound));
    }
}
