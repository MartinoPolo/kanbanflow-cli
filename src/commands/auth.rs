//! `kf auth` — the API token in the OS credential store, and the board user it
//! is used as.
//!
//! KanbanFlow tokens are per-board, so the board a token belongs to is
//! discovered by calling `GET /board` with it; that call doubles as validation.
//! A token belongs to a board rather than to a person, so logging in also
//! settles which member of that board this machine acts as.
//! This module also owns the token-input helpers `kf init` reuses.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use clap::{Args, Subcommand};

use crate::api::models::Board;
use crate::api::Client;
use crate::boards;
use crate::config::{Config, CONFIG_RELATIVE_PATH};
use crate::output;
use crate::prompt::{confirm, read_line};
use crate::token;
use crate::users;

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Store an API token for the board it belongs to.
    Login(LoginArgs),
    /// Delete a board's stored token.
    Logout(LogoutArgs),
    /// Show which token source this directory would use.
    Status(StatusArgs),
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// The API token. Visible in shell history — prefer `--token-stdin`.
    #[arg(long, value_name = "TOKEN")]
    pub token: Option<String>,
    /// Read the token as a single line from stdin (keeps it out of shell history).
    #[arg(long, conflicts_with = "token")]
    pub token_stdin: bool,
    /// Read the token from this file instead of prompting.
    #[arg(long, value_name = "PATH", conflicts_with_all = ["token", "token_stdin"])]
    pub with_token_file: Option<PathBuf>,
    /// Re-use the token already stored for this board (ID, name, or 1-based
    /// index from `kf auth status`) instead of asking for one. Pair it with
    /// `--user` to record who you are on a board you logged in to earlier.
    #[arg(long, value_name = "BOARD", conflicts_with_all = ["token", "token_stdin", "with_token_file"])]
    pub board: Option<String>,
    /// Which board member you are: a user ID, full name or email. Asked for on
    /// a multi-member board when it is not already known.
    #[arg(long, value_name = "USER")]
    pub user: Option<String>,
}

#[derive(Debug, Args)]
pub struct LogoutArgs {
    /// The board to forget: an ID, a name, or a 1-based index from
    /// `kf auth status`. Required when several boards are logged in.
    #[arg(long, value_name = "BOARD")]
    pub board: Option<String>,
    /// Forget every logged-in board.
    #[arg(long, conflicts_with = "board")]
    pub all: bool,
    /// Skip the confirmation prompt.
    #[arg(long)]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Print JSON instead of the human-readable output.
    #[arg(long)]
    pub json: bool,
}

pub fn run(command: AuthCommand) -> anyhow::Result<()> {
    match command {
        AuthCommand::Login(args) => login(args),
        AuthCommand::Logout(args) => logout(args),
        AuthCommand::Status(args) => status(args),
    }
}

fn login(args: LoginArgs) -> anyhow::Result<()> {
    let reused = args.board.is_some();
    let api_token = match &args.board {
        Some(needle) => stored_token_of(needle)?,
        None => read_token(
            args.token,
            args.token_stdin,
            args.with_token_file.as_deref(),
        )?,
    };
    let (client, board) = verify_token(&api_token)?;
    if !reused {
        token::store(&board.id, &api_token)
            .with_context(|| format!("storing the token for board `{}`", board.id))?;
    }

    let user_id = users::resolve_for_board(args.user.as_deref(), &client, &board.id)?;
    remember_board(&board.id, &board.name, Some(&user_id));

    match reused {
        true => println!("Kept the stored token for {} ({}).", board.name, board.id),
        false => println!("Stored the token for {} ({}).", board.name, board.id),
    }
    println!("Acting as user {user_id} on this board.");
    println!("`kf init` in any repo can now use it without asking for the token again.");
    if token::from_env().is_some() {
        println!(
            "Note: {} is set and takes precedence over the stored token.",
            token::TOKEN_ENV_VAR
        );
    }
    Ok(())
}

/// The token already in the credential store for `needle`, so `--user` can be
/// recorded for a board logged in before identities were kept — without a trip
/// to the KanbanFlow settings page for a token that is already on this machine.
///
/// A board that predates the registry is listed nowhere, so an unmatched needle
/// is taken as a raw board ID, the same fallback `kf auth logout` allows.
fn stored_token_of(needle: &str) -> anyhow::Result<String> {
    let registry = boards::load();
    let board_id = match registry.find(needle) {
        Some(board) => board.id.clone(),
        None if stored_token_present(needle) => needle.to_string(),
        None => anyhow::bail!(
            "no logged-in board and no stored token matches `{needle}`.{}",
            match registry.boards.is_empty() {
                true => " Run `kf auth login` without --board to paste one.".to_string(),
                false => format!(" Logged in to: {}", registry.describe()),
            }
        ),
    };
    Ok(token::stored(&board_id)?)
}

/// A board `kf auth logout` will forget. The name is absent for a credential
/// stored before the registry existed, which `--board <id>` can still target.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LogoutTarget {
    id: String,
    name: Option<String>,
}

impl LogoutTarget {
    fn known(board: &boards::KnownBoard) -> Self {
        Self {
            id: board.id.clone(),
            name: Some(board.name.clone()),
        }
    }
}

impl std::fmt::Display for LogoutTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{name} ({})", self.id),
            None => write!(f, "{}", self.id),
        }
    }
}

fn logout(args: LogoutArgs) -> anyhow::Result<()> {
    let registry = boards::load();
    let targets = logout_targets(&args, &registry, stored_token_present)?;

    let question = match targets.as_slice() {
        [only] => format!("Remove the stored token for {only}?"),
        many => format!(
            "Remove the stored tokens for all {} logged-in boards ({})?",
            many.len(),
            describe_targets(many)
        ),
    };
    if !args.yes && !confirm(&question)? {
        println!("Cancelled.");
        return Ok(());
    }

    for target in &targets {
        let had_token = stored_token_present(&target.id);
        token::delete(&target.id)
            .with_context(|| format!("deleting the stored token for board `{}`", target.id))?;
        // The registry is only a lookup aid, so failing to update it must not
        // mask the fact that the credential itself is gone.
        if let Err(error) = boards::forget(&target.id) {
            eprintln!("warning: could not update the board registry: {error}");
        }
        if had_token {
            println!("Removed the stored token for {target}.");
        } else {
            println!("No stored token for {target}; forgot the board anyway.");
        }
    }

    println!("`kf auth login` stores a token again; `{CONFIG_RELATIVE_PATH}` is left alone.");
    if token::from_env().is_some() {
        println!(
            "Note: {} is still set, so commands keep authenticating with it.",
            token::TOKEN_ENV_VAR
        );
    }
    Ok(())
}

/// Which boards to forget. Ambiguity is refused rather than guessed: deleting
/// the wrong credential costs a trip to the KanbanFlow settings page.
///
/// `credential_exists` is injected so the rules can be tested without touching
/// the real credential store.
fn logout_targets(
    args: &LogoutArgs,
    registry: &boards::Registry,
    credential_exists: impl Fn(&str) -> bool,
) -> anyhow::Result<Vec<LogoutTarget>> {
    if args.all {
        if registry.boards.is_empty() {
            anyhow::bail!("no board is logged in, so there is nothing to forget");
        }
        return Ok(registry.boards.iter().map(LogoutTarget::known).collect());
    }
    match (args.board.as_deref(), registry.boards.as_slice()) {
        (Some(needle), _) => {
            if let Some(board) = registry.find(needle) {
                return Ok(vec![LogoutTarget::known(board)]);
            }
            // A token stored before the registry existed is not listed anywhere,
            // so an unlisted `--board` is taken as a raw board ID.
            if credential_exists(needle) {
                return Ok(vec![LogoutTarget {
                    id: needle.to_string(),
                    name: None,
                }]);
            }
            anyhow::bail!(
                "no logged-in board and no stored token matches `{needle}`.{}",
                match registry.boards.is_empty() {
                    true => String::new(),
                    false => format!(" Logged in to: {}", registry.describe()),
                }
            )
        }
        (None, []) => anyhow::bail!(
            "no board is logged in. Pass `--board <boardId>` to remove a token stored \
             before the board registry existed."
        ),
        (None, [only]) => Ok(vec![LogoutTarget::known(only)]),
        (None, many) => anyhow::bail!(
            "{} boards are logged in ({}). Pass `--board <id|name>` or `--all`.",
            many.len(),
            registry.describe()
        ),
    }
}

fn describe_targets(targets: &[LogoutTarget]) -> String {
    targets
        .iter()
        .map(LogoutTarget::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn status(args: StatusArgs) -> anyhow::Result<()> {
    // Deliberately makes no HTTP request: `kf auth status` must stay free of
    // the board's 1000 requests/hour budget.
    let config = Config::load().ok();
    let environment_token = token::from_env();
    let stored = config
        .as_ref()
        .map(|config| stored_token_present(&config.board_id));

    let source = if environment_token.is_some() {
        format!("{} environment variable", token::TOKEN_ENV_VAR)
    } else if stored == Some(true) {
        "OS credential store".to_string()
    } else {
        "none".to_string()
    };

    let registry = boards::load();
    let user_id = config.as_ref().and_then(users::my_user_id);

    if args.json {
        return output::print_json(&serde_json::json!({
            "source": source,
            "environmentVariableSet": environment_token.is_some(),
            "storedInCredentialStore": stored,
            "boardId": config.as_ref().map(|config| config.board_id.as_str()),
            "boardName": config.as_ref().map(|config| config.board_name.as_str()),
            "userId": user_id,
            "knownBoards": registry.boards,
        }))
        .map_err(Into::into);
    }

    println!("Token source: {source}");
    match &config {
        Some(config) => println!("Board:        {} ({})", config.board_name, config.board_id),
        None => println!("Board:        unknown (no `{CONFIG_RELATIVE_PATH}`)"),
    }
    match &user_id {
        Some(user_id) => println!("Acting as:    {user_id}"),
        None if config.is_some() => println!(
            "Acting as:    unknown — run `kf auth login` or set {}",
            users::USER_ID_ENV_VAR
        ),
        None => {}
    }
    if registry.boards.is_empty() {
        println!("Logged in to: no board yet");
    } else {
        println!("Logged in to: {}", registry.describe());
    }
    if source == "none" && config.is_none() && !registry.boards.is_empty() {
        println!("Run `kf init` here to wire this repo to one of them.");
    } else if source == "none" {
        println!("Run `kf auth login` or set {}.", token::TOKEN_ENV_VAR);
    }
    Ok(())
}

/// Record the board, and who we are on it, in the user-level registry so a
/// later `kf init` can find its token and skip the identity question. A failure
/// here is a warning: the token is stored either way, and the registry is only
/// a lookup aid.
pub(crate) fn remember_board(board_id: &str, board_name: &str, user_id: Option<&str>) {
    if let Err(error) = boards::remember(board_id, board_name, user_id) {
        eprintln!("warning: could not record the board in the registry: {error}");
    }
}

/// True when the credential store holds a token for this board — `token::stored`
/// rather than `token::resolve`, which would answer with the environment
/// variable instead. A store error is reported as "absent": status is
/// diagnostic and must never fail hard.
fn stored_token_present(board_id: &str) -> bool {
    token::stored(board_id).is_ok()
}

/// `GET /board` with a candidate token: proves the token works and reveals
/// which board it belongs to (tokens carry no board ID of their own).
pub(crate) fn verify_token(api_token: &str) -> anyhow::Result<(Client, Board)> {
    // The client owns its token, and the caller still needs the string to store it.
    let client = Client::new(api_token.to_string())
        .context("could not build the HTTP client for the KanbanFlow API")?;
    let board: Board = client.get_json("board", &[]).context(
        "the API token was rejected or unreachable; copy the whole token from \
         Menu → Settings → API & Webhooks on a premium board",
    )?;
    Ok((client, board))
}

/// Token from, in order: the `--token` flag, stdin, a file, an interactive prompt.
///
/// Whatever the source, the value is a paste from a browser, so it is sanitized
/// here — the single funnel for human-supplied tokens.
pub(crate) fn read_token(
    flag: Option<String>,
    from_stdin: bool,
    file: Option<&Path>,
) -> anyhow::Result<String> {
    let raw = match (flag, from_stdin, file) {
        (Some(value), _, _) => value,
        (None, true, _) => {
            read_line().context("reading the API token from stdin (--token-stdin)")?
        }
        (None, false, Some(path)) => std::fs::read_to_string(path)
            .with_context(|| format!("reading the API token from `{}`", path.display()))?,
        (None, false, None) => prompt_for_token()?,
    };
    let sanitized = token::sanitize(&raw)?;
    if let Some(note) = sanitized.note() {
        eprintln!("{note}");
    }
    Ok(sanitized.token)
}

/// Ask for the token on the terminal, with the input hidden.
///
/// The return value is raw: `read_token` is the only place that sanitizes, so
/// every source is cleaned by the same rules.
pub(crate) fn prompt_for_token() -> anyhow::Result<String> {
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "no API token given and stdin is not a terminal; pass `--token <token>`, \
             `--token-stdin`, or set {}",
            token::TOKEN_ENV_VAR
        );
    }
    eprintln!("Paste your KanbanFlow API token (the input stays hidden).");
    match rpassword::prompt_password("Token: ") {
        Ok(token) => Ok(token),
        // Terminals that cannot switch off echo (some emulators, redirected
        // consoles) would otherwise leave no way to type a token at all.
        Err(error) => {
            eprintln!(
                "warning: could not hide the input ({error}); it will be visible. \
                 `printf %s \"$TOKEN\" | kf auth login --token-stdin` avoids the terminal."
            );
            eprint!("Token: ");
            std::io::stderr().flush().ok();
            read_line()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_token_prefers_the_flag_and_trims_it() {
        let api_token = read_token(Some("  abc123 \n".to_string()), true, None)
            .expect("the flag wins over every other source");
        assert_eq!(api_token, "abc123");
    }

    #[test]
    fn read_token_rejects_a_blank_flag_value() {
        assert!(read_token(Some("   ".to_string()), false, None).is_err());
    }

    fn logout_args(board: Option<&str>, all: bool) -> LogoutArgs {
        LogoutArgs {
            board: board.map(str::to_string),
            all,
            yes: true,
        }
    }

    fn registry(boards: &[(&str, &str)]) -> boards::Registry {
        boards::Registry {
            boards: boards
                .iter()
                .map(|(id, name)| boards::KnownBoard {
                    id: (*id).to_string(),
                    name: (*name).to_string(),
                    user_id: None,
                })
                .collect(),
        }
    }

    fn no_credentials(_board_id: &str) -> bool {
        false
    }

    #[test]
    fn logout_takes_the_only_logged_in_board_without_asking() {
        let targets = logout_targets(
            &logout_args(None, false),
            &registry(&[("jZqpF4H", "Team E")]),
            no_credentials,
        )
        .expect("one board is unambiguous");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].to_string(), "Team E (jZqpF4H)");
    }

    /// Deleting the wrong credential means a trip to the KanbanFlow settings
    /// page, so several boards and no `--board` is refused rather than guessed.
    #[test]
    fn logout_refuses_to_choose_between_several_boards() {
        let registry = registry(&[("jZqpF4H", "Team E"), ("F2QMK1B", "My first board")]);
        let error = logout_targets(&logout_args(None, false), &registry, no_credentials)
            .expect_err("ambiguous");
        assert!(error.to_string().contains("--all"), "{error}");

        let targets = logout_targets(&logout_args(None, true), &registry, no_credentials)
            .expect("--all covers both");
        assert_eq!(targets.len(), 2);

        let targets = logout_targets(
            &logout_args(Some("team e"), false),
            &registry,
            no_credentials,
        )
        .expect("matched by name");
        assert_eq!(targets[0].id, "jZqpF4H");
    }

    /// Tokens stored before the registry existed are listed nowhere, so a raw
    /// board ID has to reach the credential store.
    #[test]
    fn logout_accepts_a_board_id_that_is_not_in_the_registry() {
        let targets = logout_targets(
            &logout_args(Some("F2QMK1B"), false),
            &registry(&[]),
            |board_id| board_id == "F2QMK1B",
        )
        .expect("the credential store has it");
        assert_eq!(targets[0].name, None);
        assert_eq!(targets[0].to_string(), "F2QMK1B");

        assert!(logout_targets(
            &logout_args(Some("F2QMK1B"), false),
            &registry(&[]),
            no_credentials
        )
        .is_err());
        assert!(logout_targets(&logout_args(None, true), &registry(&[]), no_credentials).is_err());
    }

    #[test]
    fn read_token_reads_and_trims_a_token_file() {
        let path = std::env::temp_dir().join("kf-auth-token-file-test.txt");
        std::fs::write(&path, "tokenFromFile\n").expect("writable temp file");
        let api_token = read_token(None, false, Some(&path)).expect("file token");
        std::fs::remove_file(&path).ok();
        assert_eq!(api_token, "tokenFromFile");
    }
}
