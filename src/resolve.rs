//! Turn a user-supplied task reference (`E613`, `BUG-5`, `T3s6UGyzY`) into a task.
//!
//! Task IDs go straight to `GET /tasks/<id>` (1 request). Numbers require the
//! board-wide `GET /tasks` scan (1 request) because the API has no number lookup.

use anyhow::Context as _;

use crate::api::models::{Task, TaskGroup};
use crate::api::{ApiError, Client};
use crate::tasks;

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("No task numbered {reference} on this board.")]
    NumberNotFound { reference: String },
    #[error(transparent)]
    Api(#[from] ApiError),
}

/// How a reference string was interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRef {
    /// An internal task ID such as `T3s6UGyzY`.
    Id(String),
    /// A board number such as `E613` = prefix `E`, value `613`.
    Number { prefix: String, value: u64 },
}

/// Classify a reference. A reference is a number only when it ends in digits and
/// anything before them is a short, digit-free prefix — task IDs mix cases and
/// digits throughout, so they never match.
pub fn classify(input: &str) -> TaskRef {
    let trimmed = input.trim();
    let digits_start = trimmed
        .char_indices()
        .rev()
        .take_while(|(_, character)| character.is_ascii_digit())
        .map(|(index, _)| index)
        .last();

    if let Some(start) = digits_start {
        let (prefix, digits) = trimmed.split_at(start);
        let prefix_is_plausible = prefix.len() <= 5
            && prefix
                .chars()
                .all(|character| character.is_ascii_alphabetic() || character == '-');
        if prefix_is_plausible {
            if let Ok(value) = digits.parse::<u64>() {
                return TaskRef::Number {
                    prefix: prefix.to_string(),
                    value,
                };
            }
        }
    }
    TaskRef::Id(trimmed.to_string())
}

/// Does this task carry the given board number? Prefixes match case-insensitively.
fn has_number(task: &Task, prefix: &str, value: u64) -> bool {
    match &task.number {
        Some(number) => {
            number.value == value
                && number
                    .prefix
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(prefix)
        }
        None => false,
    }
}

/// Fetch the task a reference points at.
///
/// A number is looked up by scanning `GET /tasks`, which truncates date-grouped
/// cells. Reporting `NumberNotFound` for a task that merely sits past the cutoff
/// would be a lie, so a miss pages through the truncated groups before giving
/// up — and only then, keeping the common case at one request.
pub fn resolve_task(client: &Client, reference: &str) -> Result<Task, ResolveError> {
    match classify(reference) {
        TaskRef::Id(id) => Ok(client.get_json(&format!("tasks/{id}"), &[])?),
        TaskRef::Number { prefix, value } => {
            let mut groups: Vec<TaskGroup> = tasks::fetch_groups(client)?;
            if let Some(found) = groups
                .iter()
                .flat_map(|group| &group.tasks)
                .find(|task| has_number(task, &prefix, value))
            {
                return Ok(found.clone());
            }
            for group in groups.iter_mut().filter(|group| group.tasks_limited) {
                let already_seen = group.tasks.len();
                tasks::complete_group(client, group)?;
                if let Some(found) = group.tasks[already_seen..]
                    .iter()
                    .find(|task| has_number(task, &prefix, value))
                {
                    return Ok(found.clone());
                }
            }
            Err(ResolveError::NumberNotFound {
                reference: reference.trim().to_string(),
            })
        }
    }
}

/// Resolve to just an ID. Costs no request when the reference is already an ID.
pub fn resolve_task_id(client: &Client, reference: &str) -> Result<String, ResolveError> {
    match classify(reference) {
        TaskRef::Id(id) => Ok(id),
        TaskRef::Number { .. } => Ok(resolve_task(client, reference)?.id),
    }
}

/// `resolve_task` with the context line every call site owes the user. The typed
/// error stays intact underneath, so `exit::classify` still sees its variant.
pub fn resolve_task_named(client: &Client, reference: &str) -> anyhow::Result<Task> {
    resolve_task(client, reference).with_context(|| format!("looking up task {reference}"))
}

/// `resolve_task_id` with the same context line.
pub fn resolve_task_id_named(client: &Client, reference: &str) -> anyhow::Result<String> {
    resolve_task_id(client, reference).with_context(|| format!("looking up task {reference}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixed_numbers_are_recognised() {
        assert_eq!(
            classify("E613"),
            TaskRef::Number {
                prefix: "E".to_string(),
                value: 613
            }
        );
        assert_eq!(
            classify("BUG-5"),
            TaskRef::Number {
                prefix: "BUG-".to_string(),
                value: 5
            }
        );
        assert_eq!(
            classify(" 42 "),
            TaskRef::Number {
                prefix: String::new(),
                value: 42
            }
        );
    }

    #[test]
    fn numbers_match_case_insensitively_on_the_prefix_only() {
        let task: Task = serde_json::from_str(
            r#"{"_id":"T1","name":"n","columnId":"C1","number":{"prefix":"E","value":613}}"#,
        )
        .expect("fixture parses");
        assert!(has_number(&task, "E", 613));
        assert!(has_number(&task, "e", 613));
        assert!(!has_number(&task, "E", 614));
        assert!(!has_number(&task, "BUG-", 613));
    }

    #[test]
    fn task_ids_are_passed_through() {
        assert_eq!(classify("T3s6UGyzY"), TaskRef::Id("T3s6UGyzY".to_string()));
        // Ends in digits, but the head mixes digits and case: still an ID.
        assert_eq!(classify("T3s6UGy12"), TaskRef::Id("T3s6UGy12".to_string()));
        assert_eq!(classify("Aks4VwyRB"), TaskRef::Id("Aks4VwyRB".to_string()));
    }
}
