//! Reading the board's tasks completely.
//!
//! `GET /tasks` caps every cell of a date-grouped column at 20 tasks and flags
//! the truncation with `tasksLimited` plus a `nextTaskId` continuation cursor
//! (`docs/api/get-tasks.md`). A truncated read is not merely incomplete, it is
//! actively wrong for number resolution — the work board's Done column is date
//! grouped, so a real task can sit past the cutoff and look non-existent.
//!
//! Continuation costs requests out of the 1000/hour board budget, so it happens
//! only for groups that actually report truncation, and at the API's maximum
//! page size.

use crate::api::models::TaskGroup;
use crate::api::{ApiError, Client};

/// The largest page the API accepts (`limit` parameter, `docs/api/get-tasks.md`).
const PAGE_SIZE: &str = "100";

/// Stop after this many continuation requests per group. 2000 tasks in one cell
/// is far past anything real, and the board's request budget is not worth
/// burning on a server that never stops handing out cursors.
const MAX_CONTINUATION_PAGES: usize = 20;

/// One `GET /tasks`, exactly as the API answered it (possibly truncated).
pub fn fetch_groups(client: &Client) -> Result<Vec<TaskGroup>, ApiError> {
    client.get_json("tasks", &[])
}

/// Every task on the board, following the continuation cursor of each truncated
/// group. Costs one request plus one per continuation page.
pub fn fetch_all_groups(client: &Client) -> Result<Vec<TaskGroup>, ApiError> {
    let mut groups = fetch_groups(client)?;
    for group in &mut groups {
        complete_group(client, group)?;
    }
    Ok(groups)
}

/// Page a single truncated group until the API stops reporting more tasks.
/// A group that is not truncated costs nothing.
pub fn complete_group(client: &Client, group: &mut TaskGroup) -> Result<(), ApiError> {
    for _ in 0..MAX_CONTINUATION_PAGES {
        if !group.tasks_limited {
            return Ok(());
        }
        let Some(cursor) = group.next_task_id.clone() else {
            // Truncated without a cursor: nothing more can be requested.
            return Ok(());
        };
        let mut query = vec![
            ("columnId", group.column_id.clone()),
            ("startTaskId", cursor),
            ("limit", PAGE_SIZE.to_string()),
        ];
        if let Some(swimlane_id) = &group.swimlane_id {
            query.push(("swimlaneId", swimlane_id.clone()));
        }
        let page: Vec<TaskGroup> = client.get_json("tasks", &query)?;
        if !absorb_page(group, page) {
            return Ok(());
        }
    }
    Ok(())
}

/// Merge a continuation response into the group it continues.
///
/// Returns whether the merge made progress; a page that adds no task or repeats
/// the cursor would otherwise loop forever. `startTaskId` points *at* a task
/// rather than past it, so tasks already held are matched by ID and dropped.
fn absorb_page(group: &mut TaskGroup, page: Vec<TaskGroup>) -> bool {
    let Some(continuation) = page
        .into_iter()
        .find(|candidate| candidate.column_id == group.column_id)
    else {
        return false;
    };
    let known: std::collections::HashSet<&str> =
        group.tasks.iter().map(|task| task.id.as_str()).collect();
    let fresh: Vec<_> = continuation
        .tasks
        .into_iter()
        .filter(|task| !known.contains(task.id.as_str()))
        .collect();
    let added = !fresh.is_empty();
    let cursor_moved = continuation.next_task_id != group.next_task_id;
    group.tasks.extend(fresh);
    group.tasks_limited = continuation.tasks_limited;
    group.next_task_id = continuation.next_task_id;
    added && cursor_moved
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(json: &str) -> TaskGroup {
        serde_json::from_str(json).expect("fixture parses")
    }

    fn first_page() -> TaskGroup {
        group(
            r#"{"columnId":"CDONE","columnName":"Done","tasksLimited":true,"nextTaskId":"T3",
                "tasks":[{"_id":"T1","name":"a","columnId":"CDONE"},
                         {"_id":"T2","name":"b","columnId":"CDONE"}]}"#,
        )
    }

    #[test]
    fn a_continuation_page_appends_its_tasks_and_moves_the_cursor() {
        let mut merged = first_page();
        let progressed = absorb_page(
            &mut merged,
            vec![group(
                r#"{"columnId":"CDONE","columnName":"Done","tasksLimited":true,"nextTaskId":"T5",
                    "tasks":[{"_id":"T3","name":"c","columnId":"CDONE"},
                             {"_id":"T4","name":"d","columnId":"CDONE"}]}"#,
            )],
        );
        assert!(progressed);
        assert_eq!(
            merged
                .tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            ["T1", "T2", "T3", "T4"]
        );
        assert_eq!(merged.next_task_id.as_deref(), Some("T5"));
        assert!(merged.tasks_limited);
    }

    #[test]
    fn the_last_page_clears_the_truncation_flag() {
        let mut merged = first_page();
        let progressed = absorb_page(
            &mut merged,
            vec![group(
                r#"{"columnId":"CDONE","columnName":"Done","tasksLimited":false,
                    "tasks":[{"_id":"T3","name":"c","columnId":"CDONE"}]}"#,
            )],
        );
        assert!(progressed);
        assert!(!merged.tasks_limited);
        assert_eq!(merged.next_task_id, None);
        assert_eq!(merged.tasks.len(), 3);
    }

    #[test]
    fn tasks_already_held_are_not_duplicated() {
        let mut merged = first_page();
        absorb_page(
            &mut merged,
            vec![group(
                r#"{"columnId":"CDONE","columnName":"Done","tasksLimited":false,
                    "tasks":[{"_id":"T2","name":"b","columnId":"CDONE"},
                             {"_id":"T3","name":"c","columnId":"CDONE"}]}"#,
            )],
        );
        assert_eq!(
            merged
                .tasks
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            ["T1", "T2", "T3"]
        );
    }

    #[test]
    fn a_page_that_makes_no_progress_stops_the_loop() {
        // Same cursor, nothing new: paging on would repeat this response forever.
        let mut merged = first_page();
        assert!(!absorb_page(
            &mut merged,
            vec![group(
                r#"{"columnId":"CDONE","columnName":"Done","tasksLimited":true,"nextTaskId":"T3",
                    "tasks":[{"_id":"T1","name":"a","columnId":"CDONE"}]}"#,
            )]
        ));
        assert_eq!(merged.tasks.len(), 2);
    }

    #[test]
    fn a_response_for_another_column_is_ignored() {
        let mut merged = first_page();
        assert!(!absorb_page(
            &mut merged,
            vec![group(
                r#"{"columnId":"COTHER","columnName":"To-do","tasksLimited":false,
                    "tasks":[{"_id":"T9","name":"x","columnId":"COTHER"}]}"#,
            )]
        ));
        assert_eq!(merged.tasks.len(), 2);
        assert!(merged.tasks_limited);
    }

    #[test]
    fn the_matching_group_is_picked_out_of_a_multi_group_response() {
        let mut merged = first_page();
        assert!(absorb_page(
            &mut merged,
            vec![
                group(r#"{"columnId":"COTHER","columnName":"To-do","tasks":[]}"#),
                group(
                    r#"{"columnId":"CDONE","columnName":"Done","tasksLimited":false,
                        "tasks":[{"_id":"T3","name":"c","columnId":"CDONE"}]}"#,
                ),
            ]
        ));
        assert_eq!(merged.tasks.len(), 3);
    }
}
