//! Serde models mirroring the KanbanFlow API v1 payloads documented in `docs/api/`.
//!
//! Response models keep every documented property so `--json` output is lossless.
//! Request payloads use `skip_serializing_if = "Option::is_none"` because the API
//! treats an absent property as "leave unchanged" and an explicit `null` as "clear".

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------

/// `GET /board` — the structure of the board the token belongs to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub columns: Vec<Column>,
    /// Present only on boards that use swimlanes. The work board has none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub swimlanes: Vec<Swimlane>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<BoardColor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    #[serde(rename = "uniqueId")]
    pub unique_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Swimlane {
    pub name: String,
    #[serde(rename = "uniqueId")]
    pub unique_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A named card color configured on the board (e.g. `red` = "Bug").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardColor {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

/// `GET /users` — a board member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "fullName")]
    pub full_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

/// The human-facing task reference, e.g. `E613` = `{ prefix: "E", value: 613 }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskNumber {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    pub value: u64,
}

impl std::fmt::Display for TaskNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.prefix.as_deref().unwrap_or(""), self.value)
    }
}

/// A label on a task. Labels are never created implicitly by this CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

/// A checklist item on a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub name: String,
    #[serde(default)]
    pub finished: bool,
    #[serde(rename = "userId", default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(
        rename = "dueDateTimestamp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub due_date_timestamp: Option<String>,
    #[serde(
        rename = "dueDateTimestampLocal",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub due_date_timestamp_local: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    #[serde(rename = "userId")]
    pub user_id: String,
}

/// A due date that moves the task to `target_column_id` when it fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDate {
    #[serde(rename = "dueTimestamp")]
    pub due_timestamp: String,
    #[serde(
        rename = "targetColumnId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub target_column_id: Option<String>,
}

/// Start/end dates shown as a bar on the board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

/// A custom field value on a task. Out of v1 scope, kept so `--json` stays lossless.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFieldValue {
    #[serde(rename = "customFieldId")]
    pub custom_field_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

/// `GET /tasks/<id>`. Subtasks, labels and custom fields come inline; comments
/// and attachments never do (a full task read costs 3 API calls).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    #[serde(rename = "columnId")]
    pub column_id: String,
    #[serde(
        rename = "swimlaneId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub swimlane_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<TaskNumber>,
    #[serde(
        rename = "responsibleUserId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub responsible_user_id: Option<String>,
    #[serde(
        rename = "totalSecondsSpent",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub total_seconds_spent: Option<i64>,
    #[serde(
        rename = "totalSecondsEstimate",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub total_seconds_estimate: Option<i64>,
    #[serde(
        rename = "pointsEstimate",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub points_estimate: Option<f64>,
    /// `YYYY-MM-DD`, present only when the task sits in a date-grouped column.
    #[serde(
        rename = "groupingDate",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub grouping_date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dates: Vec<TaskDate>,
    #[serde(rename = "subTasks", default, skip_serializing_if = "Vec::is_empty")]
    pub sub_tasks: Vec<SubTask>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<Label>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collaborators: Vec<Collaborator>,
    #[serde(
        rename = "customFields",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub custom_fields: Vec<CustomFieldValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<Timeline>,
}

impl Task {
    /// `E613` when the board numbers tasks, otherwise the raw task ID.
    pub fn reference(&self) -> String {
        match &self.number {
            Some(number) => number.to_string(),
            None => self.id.clone(),
        }
    }
}

/// One entry of the `GET /tasks` response: the tasks of a single column
/// (or column/swimlane cell), possibly truncated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGroup {
    #[serde(rename = "columnId")]
    pub column_id: String,
    #[serde(rename = "columnName", default)]
    pub column_name: String,
    #[serde(
        rename = "swimlaneId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub swimlane_id: Option<String>,
    #[serde(
        rename = "swimlaneName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub swimlane_name: Option<String>,
    /// True when the (date-grouped) column has more tasks than were returned.
    #[serde(rename = "tasksLimited", default)]
    pub tasks_limited: bool,
    /// Pass as `startTaskId` to page through a truncated date-grouped column.
    #[serde(
        rename = "nextTaskId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub next_task_id: Option<String>,
    #[serde(default)]
    pub tasks: Vec<Task>,
}

// ---------------------------------------------------------------------------
// Comments and attachments
// ---------------------------------------------------------------------------

/// `GET /tasks/<id>/comments`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    #[serde(rename = "_id")]
    pub id: String,
    pub text: String,
    #[serde(rename = "createdTimestamp", default)]
    pub created_timestamp: Option<String>,
    #[serde(
        rename = "authorUserId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub author_user_id: Option<String>,
}

/// `GET /tasks/<id>/attachments`. `link` expires at `link_expires_timestamp`
/// (~24h) for KanbanFlow-hosted files — download immediately, never persist it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    #[serde(rename = "_id")]
    pub id: String,
    pub provider: String,
    pub name: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(rename = "mimeType", default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub link: String,
    #[serde(
        rename = "linkExpiresTimestamp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub link_expires_timestamp: Option<String>,
    #[serde(
        rename = "createdTimestamp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub created_timestamp: Option<String>,
    #[serde(
        rename = "createdByFullName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub created_by_full_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Request payloads
// ---------------------------------------------------------------------------

/// `POST /tasks`. Only `name` and a column are required.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateTask {
    pub name: String,
    #[serde(rename = "columnId")]
    pub column_id: String,
    #[serde(rename = "swimlaneId", skip_serializing_if = "Option::is_none")]
    pub swimlane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(rename = "responsibleUserId", skip_serializing_if = "Option::is_none")]
    pub responsible_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    /// Required when the target column is date grouped; the API otherwise
    /// defaults it to today's UTC date.
    #[serde(rename = "groupingDate", skip_serializing_if = "Option::is_none")]
    pub grouping_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,
    #[serde(rename = "subTasks", skip_serializing_if = "Option::is_none")]
    pub sub_tasks: Option<Vec<SubTask>>,
    #[serde(
        rename = "totalSecondsEstimate",
        skip_serializing_if = "Option::is_none"
    )]
    pub total_seconds_estimate: Option<i64>,
}

/// `POST /tasks/<id>`. Every field is optional: absent means "leave unchanged".
/// Use `Some(Value::Null)` on the nullable fields to clear them server-side.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateTask {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "columnId", skip_serializing_if = "Option::is_none")]
    pub column_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// `null` clears the responsible user, a string sets it.
    #[serde(rename = "responsibleUserId", skip_serializing_if = "Option::is_none")]
    pub responsible_user_id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(rename = "groupingDate", skip_serializing_if = "Option::is_none")]
    pub grouping_date: Option<String>,
    /// Replaces the whole label set; read-modify-write to add or remove one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,
    /// Replaces the whole subtask list.
    #[serde(rename = "subTasks", skip_serializing_if = "Option::is_none")]
    pub sub_tasks: Option<Vec<SubTask>>,
    #[serde(
        rename = "totalSecondsEstimate",
        skip_serializing_if = "Option::is_none"
    )]
    pub total_seconds_estimate: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateComment {
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateComment {
    pub text: String,
}

/// `POST /tasks/<id>/subtasks` and the by-index/by-name update payload.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SubTaskPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished: Option<bool>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Write responses
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskResponse {
    #[serde(rename = "taskId")]
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<TaskNumber>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentResponse {
    #[serde(rename = "taskCommentId")]
    pub task_comment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAttachmentResponse {
    #[serde(rename = "taskAttachmentId")]
    pub task_attachment_id: String,
}

// Public MPX vocabulary aliases. The underlying names remain because these
// structs directly mirror KanbanFlow's `/tasks` wire contract.
pub type Issue = Task;
pub type IssueGroup = TaskGroup;
pub type CreateIssue = CreateTask;
pub type UpdateIssue = UpdateTask;
pub type CreateIssueResponse = CreateTaskResponse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_number_renders_prefix_and_value() {
        let number = TaskNumber {
            prefix: Some("E".to_string()),
            value: 613,
        };
        assert_eq!(number.to_string(), "E613");
        assert_eq!(
            TaskNumber {
                prefix: None,
                value: 5
            }
            .to_string(),
            "5"
        );
    }

    #[test]
    fn task_deserializes_minimal_documented_response() {
        let task: Task = serde_json::from_str(
            r#"{"_id":"T3s6UGyzY","name":"Write report","description":"","color":"red","columnId":"C9LIn5sEEpqT"}"#,
        )
        .expect("minimal task response should deserialize");
        assert_eq!(task.id, "T3s6UGyzY");
        assert!(task.sub_tasks.is_empty());
        assert_eq!(task.reference(), "T3s6UGyzY");
    }

    #[test]
    fn update_task_omits_untouched_fields() {
        let update = UpdateTask {
            color: Some("green".to_string()),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&update).expect("serializable"),
            r#"{"color":"green"}"#
        );
    }

    #[test]
    fn update_task_can_clear_responsible_user() {
        let update = UpdateTask {
            responsible_user_id: Some(serde_json::Value::Null),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&update).expect("serializable"),
            r#"{"responsibleUserId":null}"#
        );
    }
}
