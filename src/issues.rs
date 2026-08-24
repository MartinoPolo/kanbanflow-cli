//! Reading the board's issues completely.
//!
//! `GET /tasks` caps every cell of a date-grouped column at 20 issues and flags
//! the truncation with `tasksLimited` plus a `nextTaskId` continuation cursor
//! (`docs/api/get-tasks.md`). A truncated read is not merely incomplete, it is
//! actively wrong for number resolution — the work board's Done column is date
//! grouped, so a real issue can sit past the cutoff and look non-existent.
//!
//! Continuation costs requests out of the 1000/hour board budget, so it happens
//! only for groups that actually report truncation, and at the API's maximum
//! page size.

use crate::api::models::IssueGroup;
use crate::api::{ApiError, Client};

/// The largest page the API accepts (`limit` parameter, `docs/api/get-tasks.md`).
const PAGE_SIZE: &str = "100";

/// Stop after this many continuation requests per group. 2000 issues in one cell
/// is far past anything real, and the board's request budget is not worth
/// burning on a server that never stops handing out cursors.
const MAX_CONTINUATION_PAGES: usize = 20;

/// One `GET /tasks`, exactly as the API answered it (possibly truncated).
pub fn fetch_groups(client: &Client) -> Result<Vec<IssueGroup>, ApiError> {
    client.get_json("tasks", &[])
}

/// Every issue on the board, following the continuation cursor of each truncated
/// group. Costs one request plus one per continuation page.
pub fn fetch_all_groups(client: &Client) -> Result<Vec<IssueGroup>, ApiError> {
    let mut groups = fetch_groups(client)?;
    for group in &mut groups {
        complete_group(client, group)?;
    }
    Ok(groups)
}

/// Page a single truncated group until the API stops reporting more issues.
/// A group that is not truncated costs nothing.
pub fn complete_group(client: &Client, group: &mut IssueGroup) -> Result<(), ApiError> {
    for _ in 0..MAX_CONTINUATION_PAGES {
        if !group.tasks_limited {
            return Ok(());
        }
        let Some(cursor) = group.next_task_id.clone() else {
            // Truncated without a cursor: nothing more can be requested.
            return Ok(());
        };
        let query = continuation_query(group, cursor);
        let page: Vec<IssueGroup> = client.get_json("tasks", &query)?;
        if !absorb_page(group, page) {
            return Ok(());
        }
    }
    Ok(())
}

fn continuation_query(group: &IssueGroup, cursor: String) -> Vec<(&'static str, String)> {
    let mut query = vec![
        ("columnId", group.column_id.clone()),
        ("startTaskId", cursor),
        ("limit", PAGE_SIZE.to_string()),
    ];
    if let Some(swimlane_id) = &group.swimlane_id {
        query.push(("swimlaneId", swimlane_id.clone()));
    }
    query
}

/// Merge a continuation response into the group it continues.
///
/// Returns whether the merge made progress; a page that adds no issue or repeats
/// the cursor would otherwise loop forever. `startTaskId` points *at* an issue
/// rather than past it, so issues already held are matched by ID and dropped.
fn absorb_page(group: &mut IssueGroup, page: Vec<IssueGroup>) -> bool {
    let Some(continuation) = page
        .into_iter()
        .find(|candidate| candidate.column_id == group.column_id)
    else {
        return false;
    };
    let known: std::collections::HashSet<&str> =
        group.tasks.iter().map(|issue| issue.id.as_str()).collect();
    let fresh: Vec<_> = continuation
        .tasks
        .into_iter()
        .filter(|issue| !known.contains(issue.id.as_str()))
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
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::time::Duration;

    use super::*;

    fn group(json: &str) -> IssueGroup {
        serde_json::from_str(json).expect("fixture parses")
    }

    fn first_page() -> IssueGroup {
        group(
            r#"{"columnId":"CDONE","columnName":"Done","tasksLimited":true,"nextTaskId":"T3",
                "tasks":[{"_id":"T1","name":"a","columnId":"CDONE"},
                         {"_id":"T2","name":"b","columnId":"CDONE"}]}"#,
        )
    }

    #[test]
    fn continuation_requests_follow_each_wire_cursor_and_merge_every_page() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let server = std::thread::spawn(move || -> std::io::Result<Vec<String>> {
            let response_bodies = [
                r#"[{"columnId":"C/one+two","tasksLimited":true,"nextTaskId":"T5","tasks":[{"_id":"T3","name":"c","columnId":"C/one+two"},{"_id":"T4","name":"d","columnId":"C/one+two"}]}]"#,
                r#"[{"columnId":"C/one+two","tasksLimited":false,"tasks":[{"_id":"T5","name":"e","columnId":"C/one+two"}]}]"#,
            ];
            let mut requests = Vec::new();
            for body in response_bodies {
                let (mut stream, _) = listener.accept()?;
                stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let bytes_read = stream.read(&mut buffer)?;
                    if bytes_read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..bytes_read]);
                }

                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )?;
                stream.flush()?;
                requests.push(String::from_utf8(request).map_err(|error| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
                })?);
            }
            Ok(requests)
        });

        let client = Client::with_base_url("test-token".to_string(), format!("http://{address}"))
            .expect("build test client");
        let mut issue_group = group(
            r#"{"columnId":"C/one+two","columnName":"Done","swimlaneId":"S/five+six",
                "tasksLimited":true,"nextTaskId":"T3",
                "tasks":[{"_id":"T1","name":"a","columnId":"C/one+two"},
                         {"_id":"T2","name":"b","columnId":"C/one+two"}]}"#,
        );
        let completion = complete_group(&client, &mut issue_group);
        let requests = server
            .join()
            .expect("test server thread did not panic")
            .expect("test server handled both requests");

        completion.expect("continuation requests succeed");
        let request_lines: Vec<_> = requests
            .iter()
            .map(|request| request.lines().next().expect("request line"))
            .collect();
        assert_eq!(
            request_lines,
            [
                "GET /tasks?columnId=C%2Fone%2Btwo&startTaskId=T3&limit=100&swimlaneId=S%2Ffive%2Bsix HTTP/1.1",
                "GET /tasks?columnId=C%2Fone%2Btwo&startTaskId=T5&limit=100&swimlaneId=S%2Ffive%2Bsix HTTP/1.1"
            ]
        );
        assert!(requests.iter().all(|request| request
            .lines()
            .any(|line| line.eq_ignore_ascii_case("authorization: Bearer test-token"))));
        assert!(requests
            .iter()
            .all(|request| !request.contains("startIssueId")));
        assert_eq!(
            issue_group
                .tasks
                .iter()
                .map(|issue| issue.id.as_str())
                .collect::<Vec<_>>(),
            ["T1", "T2", "T3", "T4", "T5"]
        );
        assert!(!issue_group.tasks_limited);
        assert_eq!(issue_group.next_task_id, None);
    }

    #[test]
    fn a_continuation_page_appends_its_issues_and_moves_the_cursor() {
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
                .map(|issue| issue.id.as_str())
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
    fn issues_already_held_are_not_duplicated() {
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
                .map(|issue| issue.id.as_str())
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
