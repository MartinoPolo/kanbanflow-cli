//! Per-repo board configuration stored at `.mpx/kanbanflow.json`.
//!
//! The file is committed: column IDs are not secrets, and the canonical-state
//! mapping is what keeps `kf-` skills board-agnostic. It therefore holds board
//! facts only — everything here is identical for every member of the team. The
//! token never lives here, and neither does the acting user, which is per person
//! and lives in the user-level board registry instead (`crate::boards`).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Path of the config file relative to a repo root.
pub const CONFIG_RELATIVE_PATH: &str = ".mpx/kanbanflow.json";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error(
        "No `{CONFIG_RELATIVE_PATH}` found in this directory or any parent. Run `kf init` first."
    )]
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
    #[error("`{path}` is not valid kanbanflow config JSON: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("No column is mapped to the `{state}` state in `{CONFIG_RELATIVE_PATH}`. Re-run `kf init` to map it.")]
    UnmappedState { state: CanonicalState },
    #[error(
        "`--open` needs to know where work ends, but `{CONFIG_RELATIVE_PATH}` maps no column to \
         `done` or `archive`. Re-run `kf init` to map one."
    )]
    NoClosedState,
}

/// Board-agnostic workflow states; `kf init` maps each to a real column ID.
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
        CanonicalState::Todo,
        CanonicalState::Wip,
        CanonicalState::Review,
        CanonicalState::Done,
        CanonicalState::Archive,
    ];

    /// The states where work has ended. Everything else — including a column with
    /// no canonical state at all, like a board's own "Do today" — is still open.
    pub const CLOSED: [CanonicalState; 2] = [CanonicalState::Done, CanonicalState::Archive];

    pub fn as_str(self) -> &'static str {
        match self {
            CanonicalState::Todo => "todo",
            CanonicalState::Wip => "wip",
            CanonicalState::Review => "review",
            CanonicalState::Done => "done",
            CanonicalState::Archive => "archive",
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
            "todo" | "to-do" | "to_do" => Ok(CanonicalState::Todo),
            "wip" | "in-progress" | "inprogress" => Ok(CanonicalState::Wip),
            "review" => Ok(CanonicalState::Review),
            "done" => Ok(CanonicalState::Done),
            "archive" | "archived" => Ok(CanonicalState::Archive),
            other => Err(format!(
                "unknown state `{other}`; expected one of todo, wip, review, done, archive"
            )),
        }
    }
}

/// Canonical state → column ID. A state may be unmapped when the board has no
/// matching column (e.g. a board without a Review lane).
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
        let column = match state {
            CanonicalState::Todo => &self.todo,
            CanonicalState::Wip => &self.wip,
            CanonicalState::Review => &self.review,
            CanonicalState::Done => &self.done,
            CanonicalState::Archive => &self.archive,
        };
        column.as_deref()
    }

    pub fn set(&mut self, state: CanonicalState, column_id: Option<String>) {
        let slot = match state {
            CanonicalState::Todo => &mut self.todo,
            CanonicalState::Wip => &mut self.wip,
            CanonicalState::Review => &mut self.review,
            CanonicalState::Done => &mut self.done,
            CanonicalState::Archive => &mut self.archive,
        };
        *slot = column_id;
    }
}

/// Contents of `.mpx/kanbanflow.json`. Field order is the on-disk key order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "boardId")]
    pub board_id: String,
    #[serde(rename = "boardName")]
    pub board_name: String,
    /// Where the acting user used to be recorded, before it moved to the
    /// user-level registry. Still read as a last resort so repos configured by
    /// an older `kf init` keep working; never written again.
    #[serde(rename = "userId", default, skip_serializing_if = "Option::is_none")]
    pub legacy_user_id: Option<String>,
    /// Which VCS hosts this repo's merge requests (`"gitlab"` when absent).
    /// The CLI never interprets it — it exists for the `board-sync` skill and
    /// is carried here only so `kf init --overwrite` does not drop it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vcs: Option<String>,
    pub states: StateColumns,
}

impl Config {
    /// Walk up from `start` looking for `.mpx/kanbanflow.json`.
    pub fn find_path(start: &Path) -> Option<PathBuf> {
        start.ancestors().find_map(|directory| {
            let candidate = directory.join(".mpx").join("kanbanflow.json");
            candidate.is_file().then_some(candidate)
        })
    }

    /// Load the config found by searching upward from the current directory.
    pub fn load() -> Result<Self, ConfigError> {
        let cwd = std::env::current_dir().map_err(|source| ConfigError::Read {
            path: PathBuf::from("."),
            source,
        })?;
        let path = Config::find_path(&cwd).ok_or(ConfigError::NotFound)?;
        Config::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Write pretty-printed JSON, creating `.mpx/` if needed.
    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
                    path: path.to_path_buf(),
                    source,
                })?;
            }
        }
        let mut json = serde_json::to_string_pretty(self).expect("config is always serializable");
        json.push('\n');
        std::fs::write(path, json).map_err(|source| ConfigError::Write {
            path: path.to_path_buf(),
            source,
        })
    }

    /// `<cwd>/.mpx/kanbanflow.json` — where `kf init` writes a fresh config.
    pub fn default_path() -> Result<PathBuf, ConfigError> {
        let cwd = std::env::current_dir().map_err(|source| ConfigError::Read {
            path: PathBuf::from("."),
            source,
        })?;
        Ok(cwd.join(".mpx").join("kanbanflow.json"))
    }

    pub fn column_id(&self, state: CanonicalState) -> Result<&str, ConfigError> {
        self.states
            .get(state)
            .ok_or(ConfigError::UnmappedState { state })
    }

    /// Columns where work has ended, so `--open` can filter by exclusion: every
    /// other column counts as open, which keeps board-specific lanes that map to
    /// no canonical state (a "Do today") in the answer instead of dropping them.
    pub fn closed_column_ids(&self) -> Result<Vec<&str>, ConfigError> {
        let columns: Vec<&str> = CanonicalState::CLOSED
            .into_iter()
            .filter_map(|state| self.states.get(state))
            .collect();
        if columns.is_empty() {
            return Err(ConfigError::NoClosedState);
        }
        Ok(columns)
    }

    /// Reverse lookup used when printing a task's state instead of its column.
    pub fn state_for_column(&self, column_id: &str) -> Option<CanonicalState> {
        CanonicalState::ALL
            .into_iter()
            .find(|state| self.states.get(*state) == Some(column_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Config {
        Config {
            board_id: "F2QMK1B".to_string(),
            board_name: "My first board".to_string(),
            legacy_user_id: None,
            vcs: None,
            states: StateColumns {
                todo: Some("C9LIn5sEEpqT".to_string()),
                wip: Some("CBO1VNGqDc4K".to_string()),
                review: None,
                done: Some("COxkPjd0wra4".to_string()),
                archive: None,
            },
        }
    }

    #[test]
    fn config_round_trips_through_json() {
        let config = sample();
        let json = serde_json::to_string_pretty(&config).expect("serializable");
        let parsed: Config = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(config, parsed);
    }

    /// The written file is what the team shares, so it must carry board facts
    /// and nothing personal.
    #[test]
    fn config_json_keys_are_stable_and_hold_no_user() {
        let json = serde_json::to_string(&sample()).expect("serializable");
        assert_eq!(
            json,
            r#"{"boardId":"F2QMK1B","boardName":"My first board","states":{"todo":"C9LIn5sEEpqT","wip":"CBO1VNGqDc4K","done":"COxkPjd0wra4"}}"#
        );
    }

    /// The `vcs` key belongs to the skills, not the CLI: it must survive a
    /// serialize round-trip so `kf init --overwrite` cannot drop it.
    #[test]
    fn the_vcs_key_round_trips() {
        let mut config = sample();
        config.vcs = Some("gitlab".to_string());
        let json = serde_json::to_string(&config).expect("serializable");
        assert!(json.contains(r#""vcs":"gitlab""#));
        let parsed: Config = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(parsed.vcs.as_deref(), Some("gitlab"));
    }

    /// A config written by an older `kf init` still parses, and its `userId`
    /// stays available as the last-resort identity.
    #[test]
    fn a_config_with_a_legacy_user_id_still_parses() {
        let config: Config = serde_json::from_str(
            r#"{"boardId":"F2QMK1B","boardName":"My first board","userId":"UHJ9JgtA","states":{}}"#,
        )
        .expect("deserializable");
        assert_eq!(config.legacy_user_id.as_deref(), Some("UHJ9JgtA"));
    }

    #[test]
    fn canonical_state_parses_aliases_and_rejects_junk() {
        assert_eq!("TODO".parse::<CanonicalState>(), Ok(CanonicalState::Todo));
        assert_eq!(
            " in-progress ".parse::<CanonicalState>(),
            Ok(CanonicalState::Wip)
        );
        assert_eq!(
            "archived".parse::<CanonicalState>(),
            Ok(CanonicalState::Archive)
        );
        assert!("backlog".parse::<CanonicalState>().is_err());
    }

    #[test]
    fn column_lookup_is_bidirectional() {
        let config = sample();
        assert_eq!(
            config.column_id(CanonicalState::Wip).expect("mapped"),
            "CBO1VNGqDc4K"
        );
        assert!(config.column_id(CanonicalState::Review).is_err());
        assert_eq!(
            config.state_for_column("COxkPjd0wra4"),
            Some(CanonicalState::Done)
        );
        assert_eq!(config.state_for_column("Cunknown"), None);
    }

    #[test]
    fn closed_columns_skip_unmapped_states_and_fail_when_there_are_none() {
        let mut config = sample();
        assert_eq!(
            config.closed_column_ids().expect("done is mapped"),
            vec!["COxkPjd0wra4"]
        );
        config
            .states
            .set(CanonicalState::Archive, Some("CarchiveXX".to_string()));
        assert_eq!(
            config.closed_column_ids().expect("both are mapped"),
            vec!["COxkPjd0wra4", "CarchiveXX"]
        );
        config.states.set(CanonicalState::Done, None);
        config.states.set(CanonicalState::Archive, None);
        assert!(matches!(
            config.closed_column_ids(),
            Err(ConfigError::NoClosedState)
        ));
    }
}
