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
pub const SCHEMA_VERSION: u64 = 1;

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
                    &format!("`issues.states.{state}` must be a non-empty column ID"),
                ));
            }
            let Some(column_id) = column_id else {
                continue;
            };
            if column_id.trim().is_empty() {
                return Err(invalid(
                    path,
                    &format!("`issues.states.{state}` must be a non-empty column ID when present"),
                ));
            }
            if let Some(previous_state) = mapped_columns.insert(column_id, state) {
                return Err(invalid(
                    path,
                    &format!(
                        "`issues.states.{state}` and `issues.states.{previous_state}` must use distinct columns"
                    ),
                ));
            }
        }
        Ok(())
    }
}

/// Runtime view of `issues` when its provider is KanbanFlow.
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
        validate_schema_version(&document, path)?;
        let issues = document
            .get("issues")
            .ok_or_else(|| invalid(path, "missing `issues` binding"))?;
        if issues.get("provider").and_then(Value::as_str) != Some("kanbanflow") {
            return Err(invalid(path, "`issues.provider` must be `kanbanflow`"));
        }
        let config: Self = serde_json::from_value(issues.clone())
            .map_err(|source| invalid(path, &format!("invalid issues binding: {source}")))?;
        config.validate(path)?;
        Ok(config)
    }
    fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        if self.board_id.trim().is_empty() {
            return Err(invalid(path, "`issues.boardId` must not be empty"));
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
        validate_project_document(&document, path)
    }
    pub fn merge_into_file(&self, path: &Path) -> Result<(), ConfigError> {
        let mut document = read_document(path)?;
        validate_project_document(&document, path)?;
        self.validate(path)?;
        let root = document
            .as_object_mut()
            .ok_or_else(|| invalid(path, "root must be an object"))?;
        let issues = root
            .entry("issues")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if !issues.is_object() {
            *issues = Value::Object(serde_json::Map::new());
        }
        let issues = issues.as_object_mut().expect("issues object");
        issues.insert("provider".into(), Value::String("kanbanflow".into()));
        issues.insert("boardId".into(), Value::String(self.board_id.clone()));
        if let Some(board_name) = &self.board_name {
            issues.insert("boardName".into(), Value::String(board_name.clone()));
        } else {
            issues.remove("boardName");
        }
        issues.remove("token");
        issues.remove("userId");

        let states = issues
            .entry("states")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if !states.is_object() {
            *states = Value::Object(serde_json::Map::new());
        }
        let states = states.as_object_mut().expect("states object");
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
fn validate_schema_version(document: &Value, path: &Path) -> Result<(), ConfigError> {
    if document.get("schemaVersion").and_then(Value::as_u64) != Some(SCHEMA_VERSION) {
        return Err(invalid(path, "`schemaVersion` must be 1"));
    }
    if !document.is_object() {
        return Err(invalid(path, "root must be an object"));
    }
    Ok(())
}
fn validate_project_document(document: &Value, path: &Path) -> Result<(), ConfigError> {
    validate_schema_version(document, path)?;
    let project_id = document
        .pointer("/project/id")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(path, "`project.id` must be a string"))?;
    validate_project_id(project_id, path)?;

    let repository_provider = document
        .pointer("/repository/provider")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(path, "`repository.provider` must be a string"))?;
    if !matches!(
        repository_provider,
        "github" | "gitlab" | "gerrit" | "generic"
    ) {
        return Err(invalid(
            path,
            "`repository.provider` must be one of github, gitlab, gerrit, generic",
        ));
    }

    if document
        .pointer("/repository/remote")
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

fn validate_project_id(project_id: &str, path: &Path) -> Result<(), ConfigError> {
    if project_id.len() > 129 {
        return Err(invalid(path, "`project.id` must be at most 129 characters"));
    }
    let segments: Vec<_> = project_id.split('/').collect();
    if segments.len() != 2 || segments.iter().any(|segment| segment.is_empty()) {
        return Err(invalid(
            path,
            "`project.id` must contain exactly two non-empty slash-separated segments",
        ));
    }
    for segment in segments {
        if segment.len() > 64 {
            return Err(invalid(
                path,
                "each `project.id` segment must be at most 64 characters",
            ));
        }
        if !segment
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
            || !segment
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
            || !segment
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(invalid(
                path,
                "`project.id` segments must start and end with an ASCII alphanumeric character and contain only ASCII alphanumeric characters, `.`, `_`, or `-`",
            ));
        }
    }
    Ok(())
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
        let nested = root.join("a/b");
        std::fs::create_dir_all(root.join(".mpx")).unwrap();
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
    fn loads_valid_kanbanflow_issues_binding() {
        let path = temp("load.json");
        std::fs::write(&path, r#"{"schemaVersion":1,"issues":{"provider":"kanbanflow","boardId":"B1","states":{"todo":"C0","wip":"C1","review":"C2","done":"C3"}}}"#).unwrap();
        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.board_id, "B1");
        assert_eq!(config.board_name, None);
        std::fs::remove_file(path).ok();
    }
    #[test]
    fn rejects_wrong_schema_provider_and_unusable_mapping() {
        for json in [
            r#"{"schemaVersion":2,"issues":{"provider":"kanbanflow"}}"#,
            r#"{"schemaVersion":1,"issues":{"provider":"github"}}"#,
            r#"{"schemaVersion":1,"issues":{"provider":"kanbanflow","boardId":"B","states":{"todo":"C"}}}"#,
            r#"{"schemaVersion":1,"issues":{"provider":"kanbanflow","boardId":"B","states":{"todo":"C0","wip":"C1","review":"C1","done":"C3"}}}"#,
        ] {
            let path = temp("invalid.json");
            std::fs::write(&path, json).unwrap();
            assert!(Config::load_from(&path).is_err());
            std::fs::remove_file(path).ok();
        }
    }
    #[test]
    fn invalid_issues_binding_types_are_reported_as_invalid_config() {
        let path = temp("invalid-binding-type.json");
        std::fs::write(
            &path,
            r#"{"schemaVersion":1,"issues":{"provider":"kanbanflow","boardId":42,"states":{}}}"#,
        )
        .unwrap();

        let error = Config::load_from(&path).expect_err("wrong field type must fail");
        match error {
            ConfigError::Invalid { message, .. } => {
                assert!(message.starts_with("invalid issues binding: "), "{message}");
            }
            other => panic!("expected invalid config error, got {other}"),
        }
        std::fs::remove_file(path).ok();
    }
    #[test]
    fn init_merge_rejects_an_incomplete_mpx_project_document_without_changing_it() {
        let path = temp("incomplete.json");
        let original = br#"{"schemaVersion":1}"#;
        std::fs::write(&path, original).unwrap();

        assert!(sample().merge_into_file(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn project_validation_enforces_known_mpx_v1_constraints() {
        let valid = serde_json::json!({
            "schemaVersion": 1,
            "project": {"id": "group/project", "futureProjectValue": true},
            "repository": {
                "provider": "gitlab",
                "remote": "ssh://example.test/group/project.git",
                "futureRepositoryValue": [1, 2]
            },
            "futureRootValue": {"preserved": true}
        });
        let invalid_documents = [
            ("one-segment", serde_json::json!("x"), None, None),
            (
                "bad-character",
                serde_json::json!("group/pro ject"),
                None,
                None,
            ),
            (
                "overlong-segment",
                serde_json::json!(format!("group/{}", "x".repeat(65))),
                None,
                None,
            ),
            (
                "bogus-provider",
                serde_json::json!("group/project"),
                Some("bogus"),
                None,
            ),
            (
                "blank-remote",
                serde_json::json!("group/project"),
                None,
                Some(" \t "),
            ),
        ];

        let valid_path = temp("valid-project-document.json");
        std::fs::write(&valid_path, serde_json::to_vec(&valid).unwrap()).unwrap();
        Config::validate_project_file(&valid_path).expect("known and unknown fields are valid");
        std::fs::remove_file(valid_path).ok();

        for (name, project_id, provider, remote) in invalid_documents {
            let mut document = valid.clone();
            document["project"]["id"] = project_id;
            if let Some(provider) = provider {
                document["repository"]["provider"] = serde_json::json!(provider);
            }
            if let Some(remote) = remote {
                document["repository"]["remote"] = serde_json::json!(remote);
            }
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
    fn merge_preserves_unrelated_and_unknown_root_fields_and_is_stable() {
        let path = temp("merge.json");
        let original = serde_json::json!({
            "schemaVersion": 1,
            "project": {
                "id": "group/project",
                "name": "Project Name",
                "future": {"owners": ["alpha", "beta"], "locked": true}
            },
            "repository": {
                "provider": "gitlab",
                "remote": "ssh://git@example.test/group/project.git",
                "defaultBranch": "main",
                "future": {"mirror": {"enabled": false}}
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
                "states": {
                    "archive": "COLDARCHIVE",
                    "futureStateValue": {"nested": "keep"}
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
            "boardId": "F2QMK1B",
            "boardName": "My first board",
            "futureIssueValue": {"replace": true},
            "states": {
                "todo": "C0",
                "wip": "C1",
                "review": "C2",
                "done": "C3",
                "futureStateValue": {"nested": "keep"}
            }
        });
        assert_eq!(value, expected);
        assert!(value["issues"].get("userId").is_none());
        assert!(value["issues"].get("token").is_none());
        std::fs::remove_file(path).ok();
    }
    #[test]
    fn merge_replaces_non_object_states_and_removes_absent_optional_known_fields() {
        let path = temp("merge-non-object-states.json");
        std::fs::write(
            &path,
            r#"{"schemaVersion":1,"project":{"id":"a/b"},"repository":{"provider":"generic","remote":"x"},"issues":{"provider":"github","boardName":"Old name","future":true,"states":"invalid"}}"#,
        )
        .unwrap();
        let mut config = sample();
        config.board_name = None;

        config.merge_into_file(&path).unwrap();
        let document: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(document["issues"].get("boardName").is_none());
        assert_eq!(document["issues"]["future"], true);
        assert_eq!(
            document["issues"]["states"],
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
        std::fs::write(&path, r#"{"schemaVersion":1,"project":{"id":"a/b"},"repository":{"provider":"gitlab","remote":"x"},"issues":{"provider":"kanbanflow","boardId":"B1","userId":"U1","states":{"todo":"C0","wip":"C1","review":"C2","done":"C3"}}}"#).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        let runtime_value = serde_json::to_value(&loaded).unwrap();
        assert!(runtime_value.get("userId").is_none());

        loaded.merge_into_file(&path).unwrap();
        let document: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(document["issues"].get("userId").is_none());
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
