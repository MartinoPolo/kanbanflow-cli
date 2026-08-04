//! `kf init` — verify the token, fetch the board, map columns to canonical
//! states and write `.mpx/kanbanflow.json`.
//!
//! Interactive by default; fully scriptable with `--map` and `--user` so agents
//! and CI can set a repo up without a terminal.

use std::io::Write;
use std::path::Path;

use anyhow::Context as _;
use clap::Args;

use crate::api::models::{Board, Column, User};
use crate::boards;
use crate::commands::auth;
use crate::config::{CanonicalState, Config, StateColumns, CONFIG_RELATIVE_PATH};
use crate::output::{self, Table};
use crate::prompt::require_terminal;
use crate::token;

#[derive(Debug, Args)]
pub struct InitArgs {
    /// The API token. Visible in shell history — prefer `--token-stdin`.
    #[arg(long, value_name = "TOKEN")]
    pub token: Option<String>,
    /// Read the token as a single line from stdin.
    #[arg(long, conflicts_with = "token")]
    pub token_stdin: bool,
    /// Do not save the token in the OS credential store.
    #[arg(long)]
    pub no_store: bool,
    /// Map a canonical state to a column, e.g. `--map wip="In progress"`.
    /// Repeatable. Any use switches the mapping to non-interactive mode; states
    /// left out stay unmapped. The column may be a name, a `uniqueId`, or a
    /// 1-based index.
    #[arg(long = "map", value_name = "STATE=COLUMN")]
    pub maps: Vec<String>,
    /// Use the stored token of this board (ID, name, or 1-based index from
    /// `kf auth status`). Skips the question when several boards are logged in.
    #[arg(long, value_name = "BOARD")]
    pub board: Option<String>,
    /// The user ID (or full name / email) the token acts as.
    #[arg(long, value_name = "USER")]
    pub user: Option<String>,
    /// Overwrite an existing `.mpx/kanbanflow.json`.
    #[arg(long, visible_alias = "force")]
    pub overwrite: bool,
    /// Print the written configuration as JSON.
    #[arg(long)]
    pub json: bool,
}

/// Where a usable token came from; decides whether it is worth storing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenSource {
    Environment,
    Argument,
    CredentialStore,
    Prompt,
}

pub fn run(args: InitArgs) -> anyhow::Result<()> {
    let target_path = Config::default_path()?;
    if target_path.exists() && !args.overwrite {
        anyhow::bail!(
            "`{}` already exists. Pass --overwrite to replace it.",
            target_path.display()
        );
    }
    let existing = load_existing_config(&target_path);

    let (api_token, source) = resolve_token(&args, existing.as_ref())?;
    let (client, board) = auth::verify_token(&api_token)?;
    if !args.no_store && matches!(source, TokenSource::Argument | TokenSource::Prompt) {
        token::store(&board.id, &api_token)
            .with_context(|| format!("storing the token for board `{}`", board.id))?;
        // Registering it here is what lets the next repo skip the token entirely.
        auth::remember_board(&board.id, &board.name);
    }

    let users: Vec<User> = client
        .get_json("users", &[])
        .context("listing the board's users (GET /users)")?;
    let user_id = resolve_user(&args, &users)?;
    let states = resolve_states(&args, &board)?;

    let config = Config {
        board_id: board.id.clone(),
        board_name: board.name.clone(),
        user_id,
        states,
    };
    config.save_to(&target_path)?;

    if args.json {
        return output::print_json(&config).map_err(Into::into);
    }
    print_summary(&config, &board, &target_path, source, args.no_store);
    Ok(())
}

/// The config already in place, if any — its board ID unlocks the stored token
/// when re-initialising a repo for a board that is already set up.
fn load_existing_config(target_path: &Path) -> Option<Config> {
    let path = if target_path.exists() {
        target_path.to_path_buf()
    } else {
        Config::find_path(&std::env::current_dir().ok()?)?
    };
    Config::load_from(&path).ok()
}

/// Order: environment variable, explicit argument, the existing board's stored
/// token, then an interactive prompt.
///
/// Every human-supplied token goes through `auth::read_token`, which sanitizes
/// it. Calling `auth::prompt_for_token` directly here used to skip that, so the
/// prompt handed the trailing newline straight to the `Authorization` header.
fn resolve_token(
    args: &InitArgs,
    existing: Option<&Config>,
) -> anyhow::Result<(String, TokenSource)> {
    if let Some(api_token) = token::from_env() {
        return Ok((api_token, TokenSource::Environment));
    }
    if args.token.is_some() || args.token_stdin {
        let api_token = auth::read_token(args.token.clone(), args.token_stdin, None)?;
        return Ok((api_token, TokenSource::Argument));
    }
    if let Some(config) = existing {
        if let Ok(api_token) = token::resolve(&config.board_id) {
            return Ok((api_token, TokenSource::CredentialStore));
        }
    }
    if let Some(api_token) = token_from_registry(args.board.as_deref())? {
        return Ok((api_token, TokenSource::CredentialStore));
    }
    let api_token = auth::read_token(None, false, None)?;
    Ok((api_token, TokenSource::Prompt))
}

/// The stored token of a board `kf auth login` already handled, so a token is
/// pasted once per board rather than once per repo.
///
/// `None` means "nothing usable here, go on and ask": an empty registry, or a
/// board whose credential has since been deleted.
fn token_from_registry(requested: Option<&str>) -> anyhow::Result<Option<String>> {
    let registry = boards::load();
    let board = match (requested, registry.boards.as_slice()) {
        (Some(needle), []) => anyhow::bail!(
            "`--board {needle}` was given, but no board is logged in yet. \
             Run `kf auth login` first."
        ),
        (Some(needle), _) => registry.find(needle).ok_or_else(|| {
            anyhow::anyhow!(
                "no logged-in board matches `{needle}`. Logged in to: {}",
                registry.describe()
            )
        })?,
        (None, []) => return Ok(None),
        (None, [only]) => only,
        (None, many) => match choose_board(many)? {
            Some(board) => board,
            // The user asked to paste a token for a board that is not listed.
            None => return Ok(None),
        },
    };

    match token::resolve(&board.id) {
        Ok(api_token) => {
            eprintln!("Using the stored token for {board}.");
            Ok(Some(api_token))
        }
        // The registry outlived the credential: say so and fall back to asking,
        // rather than failing a command the user can still complete.
        Err(error) => {
            eprintln!("warning: {error}");
            Ok(None)
        }
    }
}

/// Ask which logged-in board this repo belongs to. `None` when the user wants
/// to paste a token for a board that is not in the registry yet.
fn choose_board(candidates: &[boards::KnownBoard]) -> anyhow::Result<Option<&boards::KnownBoard>> {
    require_terminal("--board <id|name>")?;
    println!("Tokens are stored for these boards:");
    for (index, board) in candidates.iter().enumerate() {
        println!("  {}. {}", index + 1, board);
    }
    loop {
        let answer = prompt(&format!(
            "Board [1-{}, or `n` to paste a new token]: ",
            candidates.len()
        ))?;
        let answer = answer.trim();
        if answer.eq_ignore_ascii_case("n") {
            return Ok(None);
        }
        if let Some(board) = boards::find(candidates, answer) {
            return Ok(Some(board));
        }
        eprintln!("Not a valid choice; enter an index, a board name, an ID, or `n`.");
    }
}

/// The API exposes no "who am I" endpoint: `GET /users` lists the board's
/// members without marking the one the token acts as, so the user is either
/// given with `--user` or picked from the list.
fn resolve_user(args: &InitArgs, users: &[User]) -> anyhow::Result<String> {
    if let Some(needle) = &args.user {
        return match find_user(users, needle) {
            Some(user) => Ok(user.id.clone()),
            None => anyhow::bail!(
                "no board user matches `{needle}`. Known users: {}",
                describe_users(users)
            ),
        };
    }
    match users {
        [] => anyhow::bail!("the board reports no users; cannot determine who the token acts as"),
        [only] => Ok(only.id.clone()),
        many => {
            require_terminal("--user <userId>")?;
            println!("Which user does this API token act as?");
            for (index, user) in many.iter().enumerate() {
                println!(
                    "  {}. {} ({})",
                    index + 1,
                    user.full_name,
                    user.email.as_deref().unwrap_or(&user.id)
                );
            }
            loop {
                let answer = prompt(&format!("User [1-{}]: ", many.len()))?;
                if let Some(user) = find_user(many, &answer) {
                    return Ok(user.id.clone());
                }
                eprintln!("Not a valid choice; enter an index, a full name, an email or an ID.");
            }
        }
    }
}

/// Match a user by ID, 1-based index, full name or email (case-insensitive).
fn find_user<'a>(users: &'a [User], needle: &str) -> Option<&'a User> {
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

fn describe_users(users: &[User]) -> String {
    users
        .iter()
        .map(|user| format!("{} ({})", user.full_name, user.id))
        .collect::<Vec<_>>()
        .join(", ")
}

fn resolve_states(args: &InitArgs, board: &Board) -> anyhow::Result<StateColumns> {
    if !args.maps.is_empty() {
        return build_state_columns(&args.maps, &board.columns);
    }
    require_terminal("--map <state>=<column>")?;
    map_states_interactively(&board.columns)
}

/// Parse the repeatable `--map state=column` flag into a state mapping.
fn build_state_columns(entries: &[String], columns: &[Column]) -> anyhow::Result<StateColumns> {
    let mut states = StateColumns::default();
    for entry in entries {
        let (state, column_reference) = parse_map_entry(entry)?;
        if states.get(state).is_some() {
            anyhow::bail!("--map sets the `{state}` state twice");
        }
        let column = find_column(columns, &column_reference).ok_or_else(|| {
            anyhow::anyhow!(
                "no column matches `{column_reference}`. Board columns: {}",
                describe_columns(columns)
            )
        })?;
        states.set(state, Some(column.unique_id.clone()));
    }
    Ok(states)
}

fn parse_map_entry(entry: &str) -> anyhow::Result<(CanonicalState, String)> {
    let (raw_state, raw_column) = entry
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("`--map {entry}` is not in `state=column` form"))?;
    let state: CanonicalState = raw_state
        .parse()
        .map_err(|message: String| anyhow::anyhow!("`--map {entry}`: {message}"))?;
    let column = raw_column.trim();
    if column.is_empty() {
        anyhow::bail!("`--map {entry}` names no column");
    }
    Ok((state, column.to_string()))
}

/// Match a column by `uniqueId`, name (case-insensitive) or 1-based index.
fn find_column<'a>(columns: &'a [Column], needle: &str) -> Option<&'a Column> {
    let needle = needle.trim();
    columns
        .iter()
        .find(|column| column.unique_id == needle)
        .or_else(|| {
            columns
                .iter()
                .find(|column| column.name.eq_ignore_ascii_case(needle))
        })
        .or_else(|| {
            needle
                .parse::<usize>()
                .ok()
                .and_then(|index| columns.get(index.checked_sub(1)?))
        })
}

fn describe_columns(columns: &[Column]) -> String {
    columns
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn map_states_interactively(columns: &[Column]) -> anyhow::Result<StateColumns> {
    println!("Board columns:");
    for (index, column) in columns.iter().enumerate() {
        println!("  {}. {}  ({})", index + 1, column.name, column.unique_id);
    }
    println!("Map each state to a column. Enter accepts the suggestion, `-` skips the state.");

    let mut states = StateColumns::default();
    for state in CanonicalState::ALL {
        let suggestion = suggest_column(state, columns);
        let hint = match suggestion {
            Some(column) => format!(" [{}]", column.name),
            None => " [skip]".to_string(),
        };
        loop {
            let answer = prompt(&format!("{state}{hint}: "))?;
            let answer = answer.trim();
            if answer == "-" {
                break;
            }
            if answer.is_empty() {
                if let Some(column) = suggestion {
                    states.set(state, Some(column.unique_id.clone()));
                }
                break;
            }
            match find_column(columns, answer) {
                Some(column) => {
                    states.set(state, Some(column.unique_id.clone()));
                    break;
                }
                None => eprintln!("No column matches `{answer}`; enter an index, a name or an ID."),
            }
        }
    }
    Ok(states)
}

/// A first guess per state from the column's name, so the common board layout
/// maps with five presses of Enter.
fn suggest_column(state: CanonicalState, columns: &[Column]) -> Option<&Column> {
    let keywords: &[&str] = match state {
        CanonicalState::Todo => &["to-do", "todo", "to do", "backlog", "inbox"],
        CanonicalState::Wip => &["in progress", "in-progress", "wip", "doing"],
        CanonicalState::Review => &["review", "in review", "qa", "testing"],
        CanonicalState::Done => &["done", "completed", "finished"],
        CanonicalState::Archive => &["archive", "archived"],
    };
    let names: Vec<String> = columns
        .iter()
        .map(|column| column.name.to_ascii_lowercase())
        .collect();
    let exact = names
        .iter()
        .position(|name| keywords.contains(&name.as_str()));
    let matched = exact.or_else(|| {
        names
            .iter()
            .position(|name| keywords.iter().any(|keyword| name.contains(keyword)))
    })?;
    columns.get(matched)
}

fn prompt(question: &str) -> anyhow::Result<String> {
    print!("{question}");
    std::io::stdout().flush().ok();
    auth::read_line_from_stdin()
}

fn print_summary(config: &Config, board: &Board, path: &Path, source: TokenSource, no_store: bool) {
    println!("Wrote {}", path.display());
    println!("Board: {} ({})", config.board_name, config.board_id);
    println!("User:  {}", config.user_id);
    match source {
        TokenSource::Environment => println!(
            "Token: {} (not stored in the credential store)",
            token::TOKEN_ENV_VAR
        ),
        TokenSource::CredentialStore => println!("Token: OS credential store"),
        TokenSource::Argument | TokenSource::Prompt if no_store => {
            println!("Token: used once, not stored (--no-store)")
        }
        TokenSource::Argument | TokenSource::Prompt => {
            println!("Token: stored in the OS credential store")
        }
    }

    let mut table = Table::new(&["STATE", "COLUMN", "COLUMN ID"]);
    for state in CanonicalState::ALL {
        match config.states.get(state) {
            Some(column_id) => {
                let name = board
                    .columns
                    .iter()
                    .find(|column| column.unique_id == column_id)
                    .map(|column| column.name.as_str())
                    .unwrap_or("(unknown column)");
                table.row([state.as_str(), name, column_id]);
            }
            None => table.row([state.as_str(), "-", "-"]),
        }
    }
    table.print();
    println!("Commit `{CONFIG_RELATIVE_PATH}`: column IDs are not secrets.");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns() -> Vec<Column> {
        ["To-do", "Do today", "In progress", "Done"]
            .iter()
            .enumerate()
            .map(|(index, name)| Column {
                name: (*name).to_string(),
                unique_id: format!("C{index}"),
                description: None,
            })
            .collect()
    }

    #[test]
    fn parse_map_entry_accepts_state_aliases_and_quoted_names() {
        let (state, column) = parse_map_entry("in-progress=In progress").expect("valid entry");
        assert_eq!(state, CanonicalState::Wip);
        assert_eq!(column, "In progress");
    }

    #[test]
    fn parse_map_entry_rejects_malformed_input() {
        assert!(parse_map_entry("wip").is_err());
        assert!(parse_map_entry("nonsense=Done").is_err());
        assert!(parse_map_entry("done=  ").is_err());
    }

    #[test]
    fn build_state_columns_resolves_by_name_id_and_index() {
        let states = build_state_columns(
            &[
                "todo=To-do".to_string(),
                "wip=C2".to_string(),
                "done=4".to_string(),
            ],
            &columns(),
        )
        .expect("all three references resolve");
        assert_eq!(states.get(CanonicalState::Todo), Some("C0"));
        assert_eq!(states.get(CanonicalState::Wip), Some("C2"));
        assert_eq!(states.get(CanonicalState::Done), Some("C3"));
        assert_eq!(states.get(CanonicalState::Review), None);
    }

    #[test]
    fn build_state_columns_rejects_duplicates_and_unknown_columns() {
        let columns = columns();
        assert!(build_state_columns(
            &["todo=To-do".to_string(), "todo=Done".to_string()],
            &columns
        )
        .is_err());
        assert!(build_state_columns(&["todo=Nowhere".to_string()], &columns).is_err());
    }

    #[test]
    fn suggest_column_guesses_the_usual_layout() {
        let columns = columns();
        assert_eq!(
            suggest_column(CanonicalState::Todo, &columns).map(|column| column.name.as_str()),
            Some("To-do")
        );
        assert_eq!(
            suggest_column(CanonicalState::Wip, &columns).map(|column| column.name.as_str()),
            Some("In progress")
        );
        assert_eq!(
            suggest_column(CanonicalState::Done, &columns).map(|column| column.name.as_str()),
            Some("Done")
        );
        assert!(suggest_column(CanonicalState::Archive, &columns).is_none());
    }

    #[test]
    fn find_user_matches_id_name_email_and_index() {
        let users = vec![
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
        ];
        assert_eq!(
            find_user(&users, "U2").map(|user| user.id.as_str()),
            Some("U2")
        );
        assert_eq!(
            find_user(&users, "jane doe").map(|user| user.id.as_str()),
            Some("U2")
        );
        assert_eq!(
            find_user(&users, "JOHN@EXAMPLE.COM").map(|user| user.id.as_str()),
            Some("U1")
        );
        assert_eq!(
            find_user(&users, "1").map(|user| user.id.as_str()),
            Some("U1")
        );
        assert!(find_user(&users, "").is_none());
        assert!(find_user(&users, "9").is_none());
    }
}
