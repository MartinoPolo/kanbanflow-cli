//! The board's label vocabulary.
//!
//! The API has no board-level label endpoint (`GET /tasks/<id>/labels` is
//! per-task), so the vocabulary is the union of the labels carried by the tasks
//! one `GET /tasks` returns. Labels are never created implicitly, so that union
//! is also exactly the set a command may apply.

use std::collections::BTreeMap;

use anyhow::Context as _;

use crate::api::models::{Label, TaskGroup};
use crate::api::Client;
use crate::tasks;

/// How often a label name occurs, and whether any task pins it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LabelUsage {
    pub task_count: usize,
    pub pinned: bool,
}

/// Every task on the board, with the context string every label read shares.
/// Truncated date-grouped cells are paged through, so a label used only by an
/// old Done task still counts.
pub fn fetch_groups(client: &Client) -> anyhow::Result<Vec<TaskGroup>> {
    tasks::fetch_all_groups(client).context("listing tasks to collect their labels (GET /tasks)")
}

/// Distinct label names across every returned task, sorted by name.
pub fn usage(groups: &[TaskGroup]) -> BTreeMap<String, LabelUsage> {
    let mut usage: BTreeMap<String, LabelUsage> = BTreeMap::new();
    for label in groups
        .iter()
        .flat_map(|group| &group.tasks)
        .flat_map(|task| &task.labels)
    {
        let entry = usage.entry(label.name.clone()).or_default();
        entry.task_count += 1;
        entry.pinned |= label.pinned.unwrap_or(false);
    }
    usage
}

/// The applicable label names, sorted case-insensitively and deduplicated so a
/// board that spells one label two ways still offers a single choice.
pub fn names(groups: &[TaskGroup]) -> Vec<String> {
    let mut names: Vec<String> = usage(groups).into_keys().collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    names
}

pub fn fetch_names(client: &Client) -> anyhow::Result<Vec<String>> {
    Ok(names(&fetch_groups(client)?))
}

/// The board's spelling of `requested`, matched case-insensitively.
pub fn match_name<'a>(requested: &str, vocabulary: &'a [String]) -> Option<&'a str> {
    let requested = requested.trim();
    vocabulary
        .iter()
        .find(|name| name.eq_ignore_ascii_case(requested))
        .map(String::as_str)
}

/// Map requested names onto the board's spellings, refusing anything unknown.
pub fn canonicalize(requested: &[String], vocabulary: &[String]) -> anyhow::Result<Vec<Label>> {
    requested
        .iter()
        .map(|name| match match_name(name, vocabulary) {
            Some(canonical) => Ok(Label {
                name: canonical.to_string(),
                pinned: None,
            }),
            None => anyhow::bail!(
                "No label `{}` on this board, and labels are never created implicitly. \
                 Existing labels: {}",
                name.trim(),
                if vocabulary.is_empty() {
                    "(none)".to_string()
                } else {
                    vocabulary.join(", ")
                }
            ),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<TaskGroup> {
        serde_json::from_str(
            r#"[{"columnId":"C0","columnName":"To-do","tasks":[
                {"_id":"T1","name":"a","columnId":"C0","labels":[{"name":"Priority","pinned":true},{"name":"Project X"}]},
                {"_id":"T2","name":"b","columnId":"C0","labels":[{"name":"Priority","pinned":false}]}
            ]}]"#,
        )
        .expect("sample response deserializes")
    }

    #[test]
    fn usage_counts_tasks_and_keeps_pinned_sticky() {
        let usage = usage(&groups());
        assert_eq!(usage.len(), 2);
        assert_eq!(usage["Priority"].task_count, 2);
        assert!(usage["Priority"].pinned);
        assert!(!usage["Project X"].pinned);
    }

    #[test]
    fn usage_is_empty_without_labels() {
        assert!(usage(&[]).is_empty());
    }

    #[test]
    fn names_are_sorted_case_insensitively_and_deduplicated() {
        let groups: Vec<TaskGroup> = serde_json::from_str(
            r#"[{"columnId":"C0","columnName":"To-do","tasks":[
                {"_id":"T1","name":"a","columnId":"C0","labels":[{"name":"zebra"},{"name":"Apple"},{"name":"APPLE"}]}
            ]}]"#,
        )
        .expect("sample response deserializes");
        assert_eq!(names(&groups), ["APPLE", "zebra"]);
    }

    #[test]
    fn match_name_is_case_insensitive_and_returns_board_spelling() {
        let vocabulary = vec!["Yoursafe Components".to_string()];
        assert_eq!(
            match_name(" yoursafe components ", &vocabulary),
            Some("Yoursafe Components")
        );
        assert_eq!(match_name("Unknown", &vocabulary), None);
    }

    #[test]
    fn canonicalize_refuses_unknown_names() {
        let vocabulary = vec!["Bug".to_string()];
        assert!(canonicalize(&["bug".to_string()], &vocabulary).is_ok());
        let error =
            canonicalize(&["Feature".to_string()], &vocabulary).expect_err("unknown are refused");
        assert!(error.to_string().contains("Bug"));
    }
}
