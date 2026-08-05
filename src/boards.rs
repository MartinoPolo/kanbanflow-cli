//! User-level registry of the boards a token has been stored for, and of who
//! this machine's user is on each of them.
//!
//! Holds no secrets: the tokens stay in the OS credential store, which cannot
//! be enumerated portably. This file remembers the board IDs to look them up
//! by, so authenticating once is enough — `kf init` in a fresh repo reuses the
//! stored token instead of asking for it a second time.
//!
//! The acting user lives here rather than in the repo's `kanbanflow.json`
//! because that file is committed and shared: a teammate who cloned a config
//! carrying someone else's `userId` would have `--mine` list that person's
//! tasks and the guardrail protect the wrong ones.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Directory name under the platform's per-user configuration root.
pub const APPLICATION_DIRECTORY: &str = "kanbanflow-cli";
pub const REGISTRY_FILE_NAME: &str = "boards.json";

/// A board `kf auth login` has stored a token for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownBoard {
    pub id: String,
    pub name: String,
    /// The board user this machine acts as. Absent for an entry written before
    /// the identity moved out of the repo config, and for a board reached only
    /// through `KANBANFLOW_TOKEN`.
    #[serde(rename = "userId", default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl std::fmt::Display for KnownBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

/// Contents of `boards.json`, in the order the boards were first seen.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub boards: Vec<KnownBoard>,
}

impl Registry {
    /// Add a board, or refresh the name and user of one already listed. Returns
    /// whether anything changed, so an unchanged registry is not rewritten.
    ///
    /// A `user_id` of `None` leaves a recorded identity in place: a command that
    /// merely re-verified the token must not make the machine forget who it is.
    pub fn upsert(&mut self, id: &str, name: &str, user_id: Option<&str>) -> bool {
        match self.boards.iter_mut().find(|board| board.id == id) {
            Some(board) => {
                let renamed = board.name != name;
                let reidentified =
                    user_id.is_some_and(|user_id| board.user_id.as_deref() != Some(user_id));
                if renamed {
                    board.name = name.to_string();
                }
                if reidentified {
                    board.user_id = user_id.map(str::to_string);
                }
                renamed || reidentified
            }
            None => {
                self.boards.push(KnownBoard {
                    id: id.to_string(),
                    name: name.to_string(),
                    user_id: user_id.map(str::to_string),
                });
                true
            }
        }
    }

    /// The board user this machine acts as, if it has been recorded.
    pub fn user_id(&self, board_id: &str) -> Option<&str> {
        self.boards
            .iter()
            .find(|board| board.id == board_id)?
            .user_id
            .as_deref()
    }

    /// Drop a board. Returns whether it was listed, so a caller can tell
    /// "forgotten" from "was never there".
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.boards.len();
        self.boards.retain(|board| board.id != id);
        before != self.boards.len()
    }

    /// Match a board by ID, name (case-insensitive) or 1-based index — the same
    /// references `--map` and `--user` accept.
    pub fn find(&self, needle: &str) -> Option<&KnownBoard> {
        find(&self.boards, needle)
    }

    /// `Name (ID), Name (ID)` — for error messages that list the alternatives.
    pub fn describe(&self) -> String {
        self.boards
            .iter()
            .map(KnownBoard::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut json = serde_json::to_string_pretty(self).expect("registry is always serializable");
        json.push('\n');
        std::fs::write(path, json)
    }
}

/// Match a board by ID, name (case-insensitive) or 1-based index. Free-standing
/// so a bare slice of candidates resolves the same way the whole registry does.
pub fn find<'a>(boards: &'a [KnownBoard], needle: &str) -> Option<&'a KnownBoard> {
    let needle = needle.trim();
    if needle.is_empty() {
        return None;
    }
    boards
        .iter()
        .find(|board| board.id == needle)
        .or_else(|| {
            boards
                .iter()
                .find(|board| board.name.eq_ignore_ascii_case(needle))
        })
        .or_else(|| {
            needle
                .parse::<usize>()
                .ok()
                .and_then(|index| boards.get(index.checked_sub(1)?))
        })
}

/// `%APPDATA%\kanbanflow-cli\boards.json` on Windows, `$XDG_CONFIG_HOME` or
/// `~/.config` elsewhere. `None` when the environment names no home at all.
pub fn path() -> Option<PathBuf> {
    let directory = if let Some(app_data) = std::env::var_os("APPDATA") {
        PathBuf::from(app_data).join(APPLICATION_DIRECTORY)
    } else if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(config_home).join(APPLICATION_DIRECTORY)
    } else {
        PathBuf::from(std::env::var_os("HOME")?)
            .join(".config")
            .join(APPLICATION_DIRECTORY)
    };
    Some(directory.join(REGISTRY_FILE_NAME))
}

/// The registry, or an empty one. A missing, unreadable or corrupt file is not
/// an error: it is a cache of board IDs, and every command must still run
/// without it.
pub fn load() -> Registry {
    path().and_then(|path| load_from(&path)).unwrap_or_default()
}

/// `None` when the file is absent, unreadable or not valid registry JSON.
pub fn load_from(path: &Path) -> Option<Registry> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// The registry path, as an error rather than an `Option`, for the two
/// functions that have to write it.
fn writable_path() -> std::io::Result<PathBuf> {
    path().ok_or_else(|| {
        std::io::Error::other(
            "no per-user configuration directory (APPDATA, XDG_CONFIG_HOME or HOME)",
        )
    })
}

/// Record a board and, when known, the user this machine acts as on it.
/// Failing to write is reported to the caller, which warns — the token is
/// already stored, so the command itself has succeeded.
pub fn remember(id: &str, name: &str, user_id: Option<&str>) -> std::io::Result<()> {
    let path = writable_path()?;
    let mut registry = load_from(&path).unwrap_or_default();
    if registry.upsert(id, name, user_id) {
        registry.save_to(&path)?;
    }
    Ok(())
}

/// Drop a board from the registry, after its token has been deleted. Returns
/// whether the board was listed at all.
pub fn forget(id: &str) -> std::io::Result<bool> {
    let path = writable_path()?;
    let Some(mut registry) = load_from(&path) else {
        return Ok(false);
    };
    if !registry.remove(id) {
        return Ok(false);
    }
    registry.save_to(&path)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        Registry {
            boards: vec![
                KnownBoard {
                    id: "jZqpF4H".to_string(),
                    name: "Team E".to_string(),
                    user_id: Some("VrvR3H".to_string()),
                },
                KnownBoard {
                    id: "F2QMK1B".to_string(),
                    name: "My first board".to_string(),
                    user_id: None,
                },
            ],
        }
    }

    #[test]
    fn upsert_adds_a_board_once_and_refreshes_a_renamed_one() {
        let mut registry = registry();
        assert!(
            !registry.upsert("jZqpF4H", "Team E", Some("VrvR3H")),
            "no change to save"
        );
        assert_eq!(registry.boards.len(), 2);

        assert!(registry.upsert("jZqpF4H", "Team Europe", None), "rename");
        assert_eq!(registry.boards[0].name, "Team Europe");

        assert!(registry.upsert("B3new", "Third", None), "new board");
        assert_eq!(registry.boards.len(), 3);
    }

    /// Re-verifying a token says nothing about who we are, so it must not erase
    /// an identity that `kf auth login` or `kf init` already recorded.
    #[test]
    fn upsert_records_a_user_and_never_clears_one() {
        let mut registry = registry();
        assert!(!registry.upsert("jZqpF4H", "Team E", None), "no change");
        assert_eq!(registry.user_id("jZqpF4H"), Some("VrvR3H"));

        assert!(
            registry.upsert("jZqpF4H", "Team E", Some("Uother")),
            "switch"
        );
        assert_eq!(registry.user_id("jZqpF4H"), Some("Uother"));

        assert!(registry.upsert("F2QMK1B", "My first board", Some("UHJ9JgtA")));
        assert_eq!(registry.user_id("F2QMK1B"), Some("UHJ9JgtA"));
        assert_eq!(registry.user_id("never-seen"), None);
    }

    #[test]
    fn remove_drops_a_listed_board_and_reports_a_miss() {
        let mut registry = registry();
        assert!(registry.remove("jZqpF4H"));
        assert_eq!(registry.boards.len(), 1);
        assert_eq!(registry.boards[0].id, "F2QMK1B");
        assert!(!registry.remove("jZqpF4H"), "already gone");
    }

    #[test]
    fn find_matches_id_name_and_index() {
        let registry = registry();
        assert_eq!(
            registry.find("F2QMK1B").map(|b| b.id.as_str()),
            Some("F2QMK1B")
        );
        assert_eq!(
            registry.find("team e").map(|b| b.id.as_str()),
            Some("jZqpF4H")
        );
        assert_eq!(registry.find("2").map(|b| b.id.as_str()), Some("F2QMK1B"));
        assert!(registry.find("").is_none());
        assert!(registry.find("9").is_none());
        assert!(registry.find("nope").is_none());
    }

    #[test]
    fn describe_lists_every_board() {
        assert_eq!(
            registry().describe(),
            "Team E (jZqpF4H), My first board (F2QMK1B)"
        );
    }

    #[test]
    fn registry_round_trips_through_a_file() {
        let path = std::env::temp_dir()
            .join("kf-boards-round-trip-test")
            .join(REGISTRY_FILE_NAME);
        registry().save_to(&path).expect("writable temp path");
        let loaded = load_from(&path).expect("just written");
        std::fs::remove_file(&path).ok();
        assert_eq!(loaded, registry());
    }

    /// A hand-edited or truncated file must not break every later command.
    #[test]
    fn a_corrupt_or_missing_file_loads_as_empty() {
        let path = std::env::temp_dir().join("kf-boards-corrupt-test.json");
        std::fs::write(&path, "{ not json").expect("writable temp file");
        assert!(load_from(&path).is_none());
        std::fs::remove_file(&path).ok();
        assert!(load_from(&std::env::temp_dir().join("kf-boards-absent.json")).is_none());
    }
}
