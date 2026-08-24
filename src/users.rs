//! Which board user `kf` acts as, and turning user IDs into names for output.
//!
//! Issue, comment and attachment payloads carry only user IDs. `GET /users` maps
//! them to names, but it costs a request out of the board's 1000/hour budget and
//! only human output needs it — so the fetch happens lazily, at most once per
//! invocation, and a failed fetch degrades to printing the raw IDs.

use std::collections::HashMap;

use anyhow::Context as _;

use crate::api::models::User;
use crate::api::Client;
use crate::boards;
use crate::config::Config;
use crate::prompt::{ask, require_terminal};

pub const USER_ID_ENV_VAR: &str = "KANBANFLOW_USER_ID";

/// Who `kf` acts as on this repo's board, or `None` when nothing has recorded
/// it. Identity is per person and per machine, so it is looked up outside the
/// committed config: the environment variable first, so an agent or CI run can
/// state it without touching any file, then the user-level board registry.
///
pub fn my_user_id(config: &Config) -> Option<String> {
    let registry = boards::load();
    select_identity(from_env(), registry.user_id(&config.board_id))
}

fn select_identity(
    environment_identity: Option<String>,
    registry_identity: Option<&str>,
) -> Option<String> {
    environment_identity.or_else(|| registry_identity.map(str::to_string))
}

/// The identity from `KANBANFLOW_USER_ID`, if it is set and non-empty.
fn from_env() -> Option<String> {
    std::env::var(USER_ID_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// What every command that needs to know "me" fails with when nothing does.
/// A distinct type so it exits 4 alongside the token failures: the reaction is
/// the same, `kf auth login`.
#[derive(Debug, thiserror::Error)]
#[error(
    "`kf` does not know which user you are on board `{board_id}`, so it cannot tell your issues \
     from a teammate's. Run `kf auth login --board {board_id} --user <you>` to record it, or set \
     {USER_ID_ENV_VAR}."
)]
pub struct UnknownUser {
    pub board_id: String,
}

/// The board user to act as: `needle` when given, otherwise the board's only
/// member, otherwise a question.
///
/// The API exposes no "who am I" endpoint — `GET /users` lists the board's
/// members without marking the one the token acts as, and a KanbanFlow token
/// belongs to a board rather than to a person. So the answer is a choice, not
/// something derivable, which is exactly why it cannot be shared through a
/// committed file.
pub fn resolve(needle: Option<&str>, users: &[User]) -> anyhow::Result<String> {
    if let Some(needle) = needle {
        return match find(users, needle) {
            Some(user) => Ok(user.id.clone()),
            None => anyhow::bail!(
                "no board user matches `{needle}`. Known users: {}",
                describe(users)
            ),
        };
    }
    match users {
        [] => anyhow::bail!("the board reports no users; cannot determine who the token acts as"),
        [only] => Ok(only.id.clone()),
        many => {
            require_terminal("--user <userId>")?;
            println!("Which user are you on this board?");
            for (index, user) in many.iter().enumerate() {
                println!(
                    "  {}. {} ({})",
                    index + 1,
                    user.full_name,
                    user.email.as_deref().unwrap_or(&user.id)
                );
            }
            loop {
                let answer = ask(&format!("User [1-{}]: ", many.len()))?;
                if let Some(user) = find(many, &answer) {
                    return Ok(user.id.clone());
                }
                eprintln!("Not a valid choice; enter an index, a full name, an email or an ID.");
            }
        }
    }
}

/// Who this machine acts as on `board_id`, settling it if it is not settled
/// yet. Consulting the registry first is what keeps a second repo — or a second
/// `kf auth login` — from repeating the question and spending a `GET /users`
/// request out of the board's hourly budget.
pub fn resolve_for_board(
    needle: Option<&str>,
    client: &Client,
    board_id: &str,
) -> anyhow::Result<String> {
    if needle.is_none() {
        if let Some(user_id) = boards::load().user_id(board_id) {
            return Ok(user_id.to_string());
        }
    }
    let users: Vec<User> = client
        .get_json("users", &[])
        .context("listing the board's users (GET /users)")?;
    resolve(needle, &users)
}

/// Match a user by ID, 1-based index, full name or email (case-insensitive).
pub fn find<'a>(users: &'a [User], needle: &str) -> Option<&'a User> {
    let needle = needle.trim();
    if needle.is_empty() {
        return None;
    }
    users
        .iter()
        .find(|user| user.id == needle)
        .or_else(|| {
            users
                .iter()
                .find(|user| user.full_name.eq_ignore_ascii_case(needle))
        })
        .or_else(|| {
            users.iter().find(|user| {
                user.email
                    .as_deref()
                    .is_some_and(|email| email.eq_ignore_ascii_case(needle))
            })
        })
        .or_else(|| {
            needle
                .parse::<usize>()
                .ok()
                .and_then(|index| users.get(index.checked_sub(1)?))
        })
}

/// `Name (ID), Name (ID)` — for error messages that list the alternatives.
pub fn describe(users: &[User]) -> String {
    users
        .iter()
        .map(|user| format!("{} ({})", user.full_name, user.id))
        .collect::<Vec<_>>()
        .join(", ")
}

pub struct UserNames<'a> {
    client: &'a Client,
    /// The user we act as, rendered as `me`. `None` where that would be
    /// misleading, such as a comment listing that shows real authors, and where
    /// this machine has no identity for the board at all.
    my_user_id: Option<&'a str>,
    names: Option<HashMap<String, String>>,
}

impl<'a> UserNames<'a> {
    /// Render `my_user_id` as `me`; pass `None` to name every user instead.
    pub fn new(client: &'a Client, my_user_id: Option<&'a str>) -> Self {
        Self {
            client,
            my_user_id,
            names: None,
        }
    }

    /// The user's name, or the raw ID when the board does not know it.
    pub fn display(&mut self, user_id: &str) -> String {
        if self.my_user_id == Some(user_id) {
            return "me".to_string();
        }
        self.names()
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| user_id.to_string())
    }

    fn names(&mut self) -> &HashMap<String, String> {
        self.names.get_or_insert_with(|| {
            // A failed lookup is not worth failing the whole read over; the raw
            // user IDs are still printed.
            self.client
                .get_json::<Vec<User>>("users", &[])
                .map(|users| {
                    users
                        .into_iter()
                        .map(|user| (user.id, user.full_name))
                        .collect()
                })
                .unwrap_or_default()
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use super::*;

    static ENVIRONMENT_MUTEX: Mutex<()> = Mutex::new(());
    const IDENTITY_ENVIRONMENT_VARIABLES: [&str; 4] =
        ["APPDATA", "XDG_CONFIG_HOME", "HOME", USER_ID_ENV_VAR];

    struct EnvironmentGuard {
        saved: Vec<(&'static str, Option<OsString>)>,
        test_root: PathBuf,
    }

    impl EnvironmentGuard {
        fn isolated() -> Self {
            let test_root = std::env::temp_dir().join(format!(
                "kf-user-legacy-identity-environment-{}",
                std::process::id()
            ));
            std::fs::remove_dir_all(&test_root).ok();
            let app_data = test_root.join("appdata");
            std::fs::create_dir_all(&app_data).unwrap();
            let saved = IDENTITY_ENVIRONMENT_VARIABLES
                .into_iter()
                .map(|name| (name, std::env::var_os(name)))
                .collect();
            std::env::set_var("APPDATA", &app_data);
            std::env::remove_var(USER_ID_ENV_VAR);
            Self { saved, test_root }
        }

        fn config_path(&self) -> PathBuf {
            self.test_root.join("legacy-config.json")
        }
    }

    impl Drop for EnvironmentGuard {
        fn drop(&mut self) {
            for (name, value) in &self.saved {
                if let Some(value) = value {
                    std::env::set_var(name, value);
                } else {
                    std::env::remove_var(name);
                }
            }
            std::fs::remove_dir_all(&self.test_root).ok();
        }
    }

    fn users() -> Vec<User> {
        vec![
            User {
                id: "U1".to_string(),
                full_name: "John Smith".to_string(),
                email: Some("john@example.com".to_string()),
            },
            User {
                id: "U2".to_string(),
                full_name: "Jane Doe".to_string(),
                email: None,
            },
        ]
    }

    #[test]
    fn legacy_config_identity_is_not_an_identity_source() {
        let _environment_lock = ENVIRONMENT_MUTEX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let environment = EnvironmentGuard::isolated();
        let path = environment.config_path();
        std::fs::write(
            &path,
            r#"{"schemaVersion":1,"issues":{"provider":"kanbanflow","boardId":"B1","userId":"ULEGACY","states":{"todo":"C0","wip":"C1","review":"C2","done":"C3"}}}"#,
        )
        .unwrap();
        let config =
            Config::load_from(&path).expect("current config loads with unknown legacy field");

        assert_eq!(config.board_id, "B1");
        assert_eq!(my_user_id(&config), None);
    }

    #[test]
    fn identity_sources_prefer_the_environment_then_the_registry() {
        assert_eq!(
            select_identity(Some("UENV".to_string()), Some("UREGISTRY")),
            Some("UENV".to_string())
        );
        assert_eq!(
            select_identity(None, Some("UREGISTRY")),
            Some("UREGISTRY".to_string())
        );
    }

    #[test]
    fn find_matches_id_name_email_and_index() {
        let users = users();
        assert_eq!(find(&users, "U2").map(|user| user.id.as_str()), Some("U2"));
        assert_eq!(
            find(&users, "jane doe").map(|user| user.id.as_str()),
            Some("U2")
        );
        assert_eq!(
            find(&users, "JOHN@EXAMPLE.COM").map(|user| user.id.as_str()),
            Some("U1")
        );
        assert_eq!(find(&users, "1").map(|user| user.id.as_str()), Some("U1"));
        assert!(find(&users, "").is_none());
        assert!(find(&users, "9").is_none());
    }

    /// A board with one member needs no question, and an unmatched `--user` has
    /// to name the alternatives rather than pick one.
    #[test]
    fn resolve_takes_a_sole_member_and_refuses_an_unknown_needle() {
        assert_eq!(resolve(None, &users()[1..]).expect("sole member"), "U2");
        let error = resolve(Some("nobody"), &users()).expect_err("no match");
        assert!(error.to_string().contains("Jane Doe (U2)"), "{error}");
        assert!(resolve(None, &[]).is_err(), "an empty board is unusable");
    }
}
