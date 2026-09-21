//! KanbanFlow's project binding in the repository's root `mpxconfig.json`.
//!
//! Project configuration contains only shared board facts. Tokens and acting-user
//! identity remain in the credential store and user-level board registry.

use std::ffi::OsStr;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CONFIG_RELATIVE_PATH: &str = "mpxconfig.json";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("No `{CONFIG_RELATIVE_PATH}` found in this directory or any parent. Create a valid MPX project config first.")]
    NotFound,
    #[error("Could not read `{path}`: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Could not write `{path}`: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("`{path}` is not valid JSON: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("`{path}` is not a usable mpxconfig.json: {message}")]
    Invalid { path: PathBuf, message: String },
    #[error("No column is mapped to the `{state}` state in `{CONFIG_RELATIVE_PATH}`. Re-run `kf init` to map it.")]
    UnmappedState { state: CanonicalState },
    #[error("`--open` needs to know where work ends, but `{CONFIG_RELATIVE_PATH}` maps no column to `done` or `archive`. Re-run `kf init` to map one.")]
    NoClosedState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lowercase")]
pub enum CanonicalState {
    Todo,
    Wip,
    Review,
    Done,
    Archive,
}

impl CanonicalState {
    pub const ALL: [CanonicalState; 5] = [
        Self::Todo,
        Self::Wip,
        Self::Review,
        Self::Done,
        Self::Archive,
    ];
    pub const CLOSED: [CanonicalState; 2] = [Self::Done, Self::Archive];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Wip => "wip",
            Self::Review => "review",
            Self::Done => "done",
            Self::Archive => "archive",
        }
    }
}
impl std::fmt::Display for CanonicalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl std::str::FromStr for CanonicalState {
    type Err = String;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim().to_ascii_lowercase().as_str() {
            "todo" | "to-do" | "to_do" => Ok(Self::Todo),
            "wip" | "in-progress" | "inprogress" => Ok(Self::Wip),
            "review" => Ok(Self::Review),
            "done" => Ok(Self::Done),
            "archive" | "archived" => Ok(Self::Archive),
            other => Err(format!(
                "unknown state `{other}`; expected one of todo, wip, review, done, archive"
            )),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateColumns {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub todo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<String>,
}
impl StateColumns {
    pub fn get(&self, state: CanonicalState) -> Option<&str> {
        match state {
            CanonicalState::Todo => &self.todo,
            CanonicalState::Wip => &self.wip,
            CanonicalState::Review => &self.review,
            CanonicalState::Done => &self.done,
            CanonicalState::Archive => &self.archive,
        }
        .as_deref()
    }
    pub fn set(&mut self, state: CanonicalState, column_id: Option<String>) {
        *match state {
            CanonicalState::Todo => &mut self.todo,
            CanonicalState::Wip => &mut self.wip,
            CanonicalState::Review => &mut self.review,
            CanonicalState::Done => &mut self.done,
            CanonicalState::Archive => &mut self.archive,
        } = column_id;
    }
    pub(crate) fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        let mut mapped_columns = std::collections::HashMap::new();
        for state in CanonicalState::ALL {
            let column_id = self.get(state);
            if state != CanonicalState::Archive && column_id.is_none_or(|id| id.trim().is_empty()) {
                return Err(invalid(
                    path,
                    &format!("`issues.metadata.states.{state}` must be a non-empty column ID"),
                ));
            }
            let Some(column_id) = column_id else {
                continue;
            };
            if column_id.trim().is_empty() {
                return Err(invalid(
                    path,
                    &format!("`issues.metadata.states.{state}` must be a non-empty column ID when present"),
                ));
            }
            if let Some(previous_state) = mapped_columns.insert(column_id, state) {
                return Err(invalid(
                    path,
                    &format!(
                        "`issues.metadata.states.{state}` and `issues.metadata.states.{previous_state}` must use distinct columns"
                    ),
                ));
            }
        }
        Ok(())
    }
}

/// Runtime view of `issues.metadata` when its provider is KanbanFlow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "boardId")]
    pub board_id: String,
    #[serde(rename = "boardName", default, skip_serializing_if = "Option::is_none")]
    pub board_name: Option<String>,
    pub states: StateColumns,
}

impl Config {
    pub fn find_path(start: &Path) -> Option<PathBuf> {
        for directory in start.ancestors() {
            let candidate = directory.join(CONFIG_RELATIVE_PATH);
            if candidate.is_file() {
                return Some(candidate);
            }
            if directory.join(".git").exists() {
                return None;
            }
        }
        None
    }
    pub fn load() -> Result<Self, ConfigError> {
        let cwd = std::env::current_dir().map_err(|source| ConfigError::Read {
            path: PathBuf::from("."),
            source,
        })?;
        let path = Self::find_path(&cwd).ok_or(ConfigError::NotFound)?;
        Self::load_from(&path)
    }
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let document = read_document(path)?;
        validate_project_document(&document, path)?;
        let issues = document
            .get("issues")
            .ok_or_else(|| invalid(path, "missing `issues` binding"))?;
        if issues.get("provider").and_then(Value::as_str) != Some("kanbanflow") {
            return Err(invalid(path, "`issues.provider` must be `kanbanflow`"));
        }
        deserialize_kanbanflow_metadata(issues, path)
    }
    fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        if self.board_id.trim().is_empty() {
            return Err(invalid(path, "`issues.metadata.boardId` must not be empty"));
        }
        self.states.validate(path)
    }
    pub fn default_path() -> Result<PathBuf, ConfigError> {
        let cwd = std::env::current_dir().map_err(|source| ConfigError::Read {
            path: PathBuf::from("."),
            source,
        })?;
        Ok(cwd.join(CONFIG_RELATIVE_PATH))
    }
    pub fn validate_project_file(path: &Path) -> Result<(), ConfigError> {
        let document = read_document(path)?;
        validate_project_document(&document, path)?;
        if let Some(issues) = document
            .get("issues")
            .filter(|issues| issues.get("provider").and_then(Value::as_str) == Some("kanbanflow"))
        {
            deserialize_kanbanflow_metadata(issues, path)?;
        }
        Ok(())
    }
    pub fn merge_into_file(&self, path: &Path) -> Result<(), ConfigError> {
        let mut document = read_document(path)?;
        validate_project_document(&document, path)?;
        self.validate(path)?;
        let root = document
            .as_object_mut()
            .ok_or_else(|| invalid(path, "root must be an object"))?;
        let issues = object_field(root, "issues");
        issues.insert("provider".into(), Value::String("kanbanflow".into()));
        issues.remove("boardId");
        issues.remove("boardName");
        issues.remove("states");
        issues.remove("token");
        issues.remove("userId");

        let metadata = object_field(issues, "metadata");
        metadata.insert("boardId".into(), Value::String(self.board_id.clone()));
        if let Some(board_name) = &self.board_name {
            metadata.insert("boardName".into(), Value::String(board_name.clone()));
        } else {
            metadata.remove("boardName");
        }
        metadata.remove("token");
        metadata.remove("userId");

        let states = object_field(metadata, "states");
        for state in CanonicalState::ALL {
            if let Some(column_id) = self.states.get(state) {
                states.insert(state.as_str().into(), Value::String(column_id.to_string()));
            } else {
                states.remove(state.as_str());
            }
        }
        let mut json = serde_json::to_string_pretty(&document).expect("document serializes");
        json.push('\n');
        write_atomically(path, json.as_bytes())
    }
    pub fn column_id(&self, state: CanonicalState) -> Result<&str, ConfigError> {
        self.states
            .get(state)
            .ok_or(ConfigError::UnmappedState { state })
    }
    pub fn closed_column_ids(&self) -> Result<Vec<&str>, ConfigError> {
        let columns: Vec<_> = CanonicalState::CLOSED
            .into_iter()
            .filter_map(|s| self.states.get(s))
            .collect();
        if columns.is_empty() {
            Err(ConfigError::NoClosedState)
        } else {
            Ok(columns)
        }
    }
    pub fn state_for_column(&self, column_id: &str) -> Option<CanonicalState> {
        CanonicalState::ALL
            .into_iter()
            .find(|state| self.states.get(*state) == Some(column_id))
    }
}

static TEMPORARY_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn write_atomically(path: &Path, contents: &[u8]) -> Result<(), ConfigError> {
    let temporary_path = loop {
        let counter = TEMPORARY_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = path
            .file_name()
            .unwrap_or_else(|| OsStr::new(CONFIG_RELATIVE_PATH))
            .to_os_string();
        temporary_name.push(format!(".{}.{}.tmp", std::process::id(), counter));
        let candidate = path.with_file_name(temporary_name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                let result = file.write_all(contents).and_then(|()| file.sync_all());
                drop(file);
                if let Err(source) = result {
                    std::fs::remove_file(&candidate).ok();
                    return Err(write_error(path, source));
                }
                break candidate;
            }
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(write_error(path, source)),
        }
    };

    if let Err(source) = std::fs::rename(&temporary_path, path) {
        std::fs::remove_file(&temporary_path).ok();
        return Err(write_error(path, source));
    }
    Ok(())
}

fn write_error(path: &Path, source: std::io::Error) -> ConfigError {
    ConfigError::Write {
        path: path.to_path_buf(),
        source,
    }
}

fn read_document(path: &Path) -> Result<Value, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}
fn deserialize_kanbanflow_metadata(issues: &Value, path: &Path) -> Result<Config, ConfigError> {
    let metadata = issues
        .get("metadata")
        .ok_or_else(|| invalid(path, "missing `issues.metadata`"))?;
    let config = Config::deserialize(metadata)
        .map_err(|source| invalid(path, &format!("invalid issues metadata: {source}")))?;
    config.validate(path)?;
    Ok(config)
}

fn validate_project_document(document: &Value, path: &Path) -> Result<(), ConfigError> {
    let root = document
        .as_object()
        .ok_or_else(|| invalid(path, "root must be an object"))?;
    if root
        .get("projectId")
        .and_then(Value::as_str)
        .is_none_or(|project_id| project_id.trim().is_empty())
    {
        return Err(invalid(path, "`projectId` must be a non-empty string"));
    }

    let Some(repository) = root.get("repository") else {
        return Ok(());
    };
    let repository = repository
        .as_object()
        .ok_or_else(|| invalid(path, "`repository` must be an object when present"))?;
    if let Some(unsupported_key) = repository
        .keys()
        .find(|key| !matches!(key.as_str(), "provider" | "remote"))
    {
        return Err(invalid(
            path,
            &format!("unsupported repository key `repository.{unsupported_key}`"),
        ));
    }
    let repository_provider = repository
        .get("provider")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(path, "`repository.provider` must be a string"))?;
    if !matches!(repository_provider, "github" | "gitlab" | "gerrit") {
        return Err(invalid(
            path,
            "`repository.provider` must be one of github, gitlab, gerrit",
        ));
    }
    if repository
        .get("remote")
        .and_then(Value::as_str)
        .is_none_or(|remote| remote.trim().is_empty())
    {
        return Err(invalid(
            path,
            "`repository.remote` must be a non-empty string",
        ));
    }
    Ok(())
}

fn object_field<'a>(
    object: &'a mut serde_json::Map<String, Value>,
    field: &str,
) -> &'a mut serde_json::Map<String, Value> {
    let value = object
        .entry(field)
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !value.is_object() {
        *value = Value::Object(serde_json::Map::new());
    }
    value.as_object_mut().expect("object field")
}

fn invalid(path: &Path, message: &str) -> ConfigError {
    ConfigError::Invalid {
        path: path.to_path_buf(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Config {
        Config {
            board_id: "F2QMK1B".into(),
            board_name: Some("My first board".into()),
            states: StateColumns {
                todo: Some("C0".into()),
                wip: Some("C1".into()),
                review: Some("C2".into()),
                done: Some("C3".into()),
                archive: None,
            },
        }
    }
    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("kf-{name}-{}", std::process::id()))
    }

    #[test]
    fn does_not_discover_a_valid_looking_legacy_config() {
        let root = temp("search");
        std::fs::remove_dir_all(&root).ok();
        let nested = root.join("a/b");
        std::fs::create_dir_all(root.join(".mpx")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            root.join(".mpx").join("kanbanflow.json"),
            r#"{"boardId":"B1","userId":"U1","states":{"todo":"C0","wip":"C1","review":"C2","done":"C3"}}"#,
        )
        .unwrap();
        assert_eq!(Config::find_path(&nested), None);
        std::fs::write(root.join("mpxconfig.json"), "{}").unwrap();
        assert_eq!(
            Config::find_path(&nested),
            Some(root.join("mpxconfig.json"))
        );
        std::fs::remove_dir_all(root).ok();
    }
    #[test]
    fn discovery_stops_at_the_nearest_repository_boundary() {
        let outer = temp("repository-boundary");
        std::fs::remove_dir_all(&outer).ok();
        let nested_repository = outer.join("nested-repository");
        let nested_working_directory = nested_repository.join("src/commands");
        let linked_worktree = outer.join("linked-worktree");
        let linked_working_directory = linked_worktree.join("src");
        std::fs::create_dir_all(&nested_working_directory).unwrap();
        std::fs::create_dir_all(nested_repository.join(".git")).unwrap();
        std::fs::create_dir_all(&linked_working_directory).unwrap();
        std::fs::write(linked_worktree.join(".git"), "gitdir: elsewhere").unwrap();
        std::fs::write(outer.join(CONFIG_RELATIVE_PATH), "{}").unwrap();

        assert_eq!(Config::find_path(&nested_working_directory), None);
        assert_eq!(Config::find_path(&linked_working_directory), None);

        std::fs::write(nested_repository.join(CONFIG_RELATIVE_PATH), "{}").unwrap();
        assert_eq!(
            Config::find_path(&nested_working_directory),
            Some(nested_repository.join(CONFIG_RELATIVE_PATH))
        );
        std::fs::remove_dir_all(outer).ok();
    }
    #[test]
    fn loads_current_binding_without_repository_and_with_arbitrary_project_id() {
        let path = temp("load.json");
        std::fs::write(&path, r#"{"projectId":" project with spaces / and symbols ! ","issues":{"provider":"kanbanflow","metadata":{"boardId":"B1","states":{"todo":"C0","wip":"C1","review":"C2","done":"C3"}}}}"#).unwrap();
        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.board_id, "B1");
        assert_eq!(config.board_name, None);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn rejects_malformed_current_fields_and_unusable_mapping() {
        for json in [
            r#"[]"#,
            r#"{"projectId":" "}"#,
            r#"{"projectId":42}"#,
            r#"{"projectId":"project","repository":null}"#,
            r#"{"projectId":"project","repository":{"provider":"generic","remote":"origin"}}"#,
            r#"{"projectId":"project","repository":{"provider":"github","remote":" "}}"#,
            r#"{"projectId":"project","issues":{"provider":"github","metadata":{}}}"#,
            r#"{"projectId":"project","issues":{"provider":"kanbanflow","metadata":{"boardId":"B","states":{"todo":"C"}}}}"#,
            r#"{"projectId":"project","issues":{"provider":"kanbanflow","metadata":{"boardId":"B","states":{"todo":"C0","wip":"C1","review":"C1","done":"C3"}}}}"#,
        ] {
            let path = temp("invalid.json");
            std::fs::write(&path, json).unwrap();
            assert!(Config::load_from(&path).is_err(), "accepted {json}");
            std::fs::remove_file(path).ok();
        }
    }

    #[test]
    fn rejects_legacy_only_issues_binding() {
        let path = temp("legacy-only-binding.json");
        std::fs::write(&path, r#"{"projectId":"project","issues":{"provider":"kanbanflow","boardId":"B1","states":{"todo":"C0","wip":"C1","review":"C2","done":"C3"}}}"#).unwrap();

        assert!(Config::load_from(&path).is_err());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn invalid_issues_metadata_types_are_reported_as_invalid_config() {
        let path = temp("invalid-binding-type.json");
        std::fs::write(
            &path,
            r#"{"projectId":"project","issues":{"provider":"kanbanflow","metadata":{"boardId":42,"states":{}}}}"#,
        )
        .unwrap();

        let error = Config::load_from(&path).expect_err("wrong field type must fail");
        match error {
            ConfigError::Invalid { message, .. } => {
                assert!(
                    message.starts_with("invalid issues metadata: "),
                    "{message}"
                );
            }
            other => panic!("expected invalid config error, got {other}"),
        }
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn init_merge_rejects_an_incomplete_mpx_project_document_without_changing_it() {
        let path = temp("incomplete.json");
        let original = br#"{"repository":{"provider":"github","remote":"origin"}}"#;
        std::fs::write(&path, original).unwrap();

        assert!(sample().merge_into_file(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn project_validation_accepts_current_schema_and_checks_optional_repository() {
        let valid_documents = [
            serde_json::json!({
                "projectId": "a project ID with no path restriction",
                "futureRootValue": {"preserved": true}
            }),
            serde_json::json!({
                "projectId": "project",
                "repository": {
                    "provider": "gitlab",
                    "remote": "ssh://example.test/group/project.git"
                }
            }),
        ];
        let invalid_documents = [
            ("non-object-root", serde_json::json!([])),
            ("missing-project-id", serde_json::json!({})),
            ("non-string-project-id", serde_json::json!({"projectId": 2})),
            ("blank-project-id", serde_json::json!({"projectId": " \t "})),
            (
                "non-object-repository",
                serde_json::json!({"projectId": "project", "repository": "origin"}),
            ),
            (
                "bogus-provider",
                serde_json::json!({"projectId": "project", "repository": {"provider": "generic", "remote": "origin"}}),
            ),
            (
                "blank-remote",
                serde_json::json!({"projectId": "project", "repository": {"provider": "github", "remote": " "}}),
            ),
            (
                "unsupported-repository-key",
                serde_json::json!({
                    "projectId": "project",
                    "repository": {
                        "provider": "github",
                        "remote": "origin",
                        "futureRepositoryValue": [1, 2]
                    }
                }),
            ),
        ];

        for (index, document) in valid_documents.into_iter().enumerate() {
            let path = temp(&format!("valid-project-document-{index}.json"));
            std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
            Config::validate_project_file(&path).expect("current project document is valid");
            std::fs::remove_file(path).ok();
        }

        for (name, document) in invalid_documents {
            let path = temp(&format!("invalid-project-{name}.json"));
            std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
            assert!(
                matches!(
                    Config::validate_project_file(&path),
                    Err(ConfigError::Invalid { .. })
                ),
                "{name} should be rejected"
            );
            std::fs::remove_file(path).ok();
        }
    }

    #[test]
    fn project_validation_requires_valid_metadata_for_kanbanflow_bindings() {
        let valid_documents = [
            serde_json::json!({"projectId": "project"}),
            serde_json::json!({
                "projectId": "project",
                "issues": {
                    "provider": "github",
                    "boardId": "legacy-value-is-ignored",
                    "states": "also ignored"
                }
            }),
            serde_json::json!({
                "projectId": "project",
                "issues": {
                    "provider": "kanbanflow",
                    "futureIssueValue": true,
                    "metadata": {
                        "boardId": "B1",
                        "futureMetadataValue": true,
                        "states": {
                            "todo": "C0",
                            "wip": "C1",
                            "review": "C2",
                            "done": "C3",
                            "backlog": "CBACKLOG"
                        }
                    }
                }
            }),
        ];
        let invalid_documents = [
            serde_json::json!({
                "projectId": "project",
                "issues": {
                    "provider": "kanbanflow",
                    "boardId": "B1",
                    "states": {"todo": "C0", "wip": "C1", "review": "C2", "done": "C3"}
                }
            }),
            serde_json::json!({
                "projectId": "project",
                "issues": {"provider": "kanbanflow", "metadata": "invalid"}
            }),
            serde_json::json!({
                "projectId": "project",
                "issues": {
                    "provider": "kanbanflow",
                    "metadata": {
                        "boardId": "B1",
                        "states": {"todo": "C0", "wip": "C1", "review": "C2"}
                    }
                }
            }),
        ];

        for (index, document) in valid_documents.into_iter().enumerate() {
            let path = temp(&format!("valid-issues-binding-{index}.json"));
            std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
            Config::validate_project_file(&path).expect("supported issues binding is valid");
            std::fs::remove_file(path).ok();
        }

        for (index, document) in invalid_documents.into_iter().enumerate() {
            let path = temp(&format!("invalid-issues-binding-{index}.json"));
            std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
            assert!(
                matches!(
                    Config::validate_project_file(&path),
                    Err(ConfigError::Invalid { .. })
                ),
                "malformed KanbanFlow binding should be rejected: {document}"
            );
            std::fs::remove_file(path).ok();
        }
    }

    #[test]
    fn merge_preserves_unrelated_and_unknown_root_fields_and_is_stable() {
        let path = temp("merge.json");
        let original = serde_json::json!({
            "projectId": "project with arbitrary identifier characters / !",
            "repository": {
                "provider": "gitlab",
                "remote": "ssh://git@example.test/group/project.git"
            },
            "tooling": {
                "language": "rust",
                "commands": {"format": ["cargo", "fmt"], "check": "cargo clippy"}
            },
            "future": {
                "nested": {
                    "enabled": true,
                    "values": ["keep", 42, {"mode": "unchanged"}]
                }
            },
            "unknownRoot": {"preserve": [1, 2, 3]},
            "issues": {
                "provider": "github",
                "userId": "legacy-user",
                "token": "legacy-token",
                "futureIssueValue": {"replace": true},
                "metadata": {
                    "userId": "metadata-user",
                    "token": "metadata-token",
                    "futureMetadataValue": {"preserve": true},
                    "states": {
                        "archive": "COLDARCHIVE",
                        "backlog": "CBACKLOG",
                        "futureStateValue": {"nested": "keep"}
                    }
                }
            }
        });
        std::fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
        sample().merge_into_file(&path).unwrap();
        let once = std::fs::read_to_string(&path).unwrap();
        sample().merge_into_file(&path).unwrap();
        let twice = std::fs::read_to_string(&path).unwrap();
        assert_eq!(once, twice);

        let value: Value = serde_json::from_str(&twice).unwrap();
        let mut expected = original;
        expected["issues"] = serde_json::json!({
            "provider": "kanbanflow",
            "futureIssueValue": {"replace": true},
            "metadata": {
                "boardId": "F2QMK1B",
                "boardName": "My first board",
                "futureMetadataValue": {"preserve": true},
                "states": {
                    "todo": "C0",
                    "wip": "C1",
                    "review": "C2",
                    "done": "C3",
                    "backlog": "CBACKLOG",
                    "futureStateValue": {"nested": "keep"}
                }
            }
        });
        assert_eq!(value, expected);
        assert!(value["issues"].get("userId").is_none());
        assert!(value["issues"].get("token").is_none());
        assert!(value["issues"]["metadata"].get("userId").is_none());
        assert!(value["issues"]["metadata"].get("token").is_none());
        std::fs::remove_file(path).ok();
    }
    #[test]
    fn merge_replaces_non_object_states_and_removes_absent_optional_known_fields() {
        let path = temp("merge-non-object-states.json");
        std::fs::write(
            &path,
            r#"{"projectId":"project","issues":{"provider":"github","future":true,"metadata":{"boardName":"Old name","futureMetadata":true,"states":"invalid"}}}"#,
        )
        .unwrap();
        let mut config = sample();
        config.board_name = None;

        config.merge_into_file(&path).unwrap();
        let document: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(document["issues"]["metadata"].get("boardName").is_none());
        assert_eq!(document["issues"]["future"], true);
        assert_eq!(document["issues"]["metadata"]["futureMetadata"], true);
        assert_eq!(
            document["issues"]["metadata"]["states"],
            serde_json::json!({"todo": "C0", "wip": "C1", "review": "C2", "done": "C3"})
        );
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn atomic_write_failure_preserves_a_directory_destination_and_cleans_up_temp_file() {
        let root = temp("atomic-directory-destination");
        std::fs::remove_dir_all(&root).ok();
        let destination = root.join("existing");
        let marker = destination.join("marker.txt");
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(&marker, b"keep").unwrap();

        assert!(matches!(
            write_atomically(&destination, b"replacement"),
            Err(ConfigError::Write { .. })
        ));
        assert!(destination.is_dir());
        assert_eq!(std::fs::read(&marker).unwrap(), b"keep");
        let temporary_prefix = format!("{}.", destination.file_name().unwrap().to_string_lossy());
        let temporary_files: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(&temporary_prefix) && name.ends_with(".tmp"))
            .collect();
        assert!(temporary_files.is_empty(), "{temporary_files:?}");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn legacy_user_id_is_not_part_of_config_identity_and_is_removed_by_init_merge() {
        let path = temp("legacy-identity.json");
        std::fs::write(&path, r#"{"projectId":"project","issues":{"provider":"kanbanflow","userId":"U1","metadata":{"boardId":"B1","userId":"UMETADATA","states":{"todo":"C0","wip":"C1","review":"C2","done":"C3"}}}}"#).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        let runtime_value = serde_json::to_value(&loaded).unwrap();
        assert!(runtime_value.get("userId").is_none());

        loaded.merge_into_file(&path).unwrap();
        let document: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(document["issues"].get("userId").is_none());
        assert!(document["issues"]["metadata"].get("userId").is_none());
        std::fs::remove_file(path).ok();
    }
    #[test]
    fn complete_distinct_state_columns_validate() {
        assert!(sample()
            .states
            .validate(Path::new(CONFIG_RELATIVE_PATH))
            .is_ok());
    }
    #[test]
    fn incomplete_and_duplicate_state_columns_are_rejected() {
        let mut states = sample().states;
        states.review = None;
        assert!(states.validate(Path::new(CONFIG_RELATIVE_PATH)).is_err());

        let mut states = sample().states;
        states.review = states.wip.clone();
        assert!(states.validate(Path::new(CONFIG_RELATIVE_PATH)).is_err());
    }
    #[test]
    fn canonical_state_parses_aliases_and_rejects_junk() {
        assert_eq!("TODO".parse(), Ok(CanonicalState::Todo));
        assert_eq!(" in-progress ".parse(), Ok(CanonicalState::Wip));
        assert!("backlog".parse::<CanonicalState>().is_err());
    }
    #[test]
    fn column_lookup_is_bidirectional() {
        let config = sample();
        assert_eq!(config.column_id(CanonicalState::Wip).unwrap(), "C1");
        assert_eq!(config.state_for_column("C3"), Some(CanonicalState::Done));
        assert_eq!(config.state_for_column("x"), None);
    }
    #[test]
    fn closed_columns_include_done_and_optional_archive() {
        let mut config = sample();
        assert_eq!(config.closed_column_ids().unwrap(), vec!["C3"]);
        config.states.archive = Some("C4".into());
        assert_eq!(config.closed_column_ids().unwrap(), vec!["C3", "C4"]);
    }
}
