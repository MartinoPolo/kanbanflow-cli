//! `kf auth` — the API token in the OS credential store.
//!
//! KanbanFlow tokens are per-board, so the board a token belongs to is
//! discovered by calling `GET /board` with it; that call doubles as validation.
//! This module also owns the token-input helpers `kf init` reuses.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use clap::{Args, Subcommand};

use crate::api::models::Board;
use crate::api::Client;
use crate::config::Config;
use crate::output;
use crate::token;

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Store an API token for the board it belongs to.
    Login(LoginArgs),
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
        AuthCommand::Status(args) => status(args),
    }
}

fn login(args: LoginArgs) -> anyhow::Result<()> {
    let api_token = read_token(
        args.token,
        args.token_stdin,
        args.with_token_file.as_deref(),
    )?;
    let (_client, board) = verify_token(&api_token)?;
    token::store(&board.id, &api_token)
        .with_context(|| format!("storing the token for board `{}`", board.id))?;

    println!("Stored the token for {} ({}).", board.name, board.id);
    if token::from_env().is_some() {
        println!(
            "Note: {} is set and takes precedence over the stored token.",
            token::TOKEN_ENV_VAR
        );
    }
    Ok(())
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

    if args.json {
        return output::print_json(&serde_json::json!({
            "source": source,
            "environmentVariableSet": environment_token.is_some(),
            "storedInCredentialStore": stored,
            "boardId": config.as_ref().map(|config| config.board_id.as_str()),
            "boardName": config.as_ref().map(|config| config.board_name.as_str()),
        }))
        .map_err(Into::into);
    }

    println!("Token source: {source}");
    match &config {
        Some(config) => println!("Board:        {} ({})", config.board_name, config.board_id),
        None => println!(
            "Board:        unknown (no `{}`)",
            crate::config::CONFIG_RELATIVE_PATH
        ),
    }
    if source == "none" {
        println!("Run `kf auth login` or set {}.", token::TOKEN_ENV_VAR);
    }
    Ok(())
}

/// True when the credential store holds a token for this board. The keyring is
/// queried directly, because `token::resolve` would answer with the environment
/// variable instead. A store error is reported as "absent": status is
/// diagnostic and must never fail hard.
fn stored_token_present(board_id: &str) -> bool {
    keyring::Entry::new(token::KEYRING_SERVICE, board_id)
        .and_then(|entry| entry.get_password())
        .is_ok()
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
pub(crate) fn read_token(
    flag: Option<String>,
    from_stdin: bool,
    file: Option<&Path>,
) -> anyhow::Result<String> {
    let raw = match (flag, from_stdin, file) {
        (Some(value), _, _) => value,
        (None, true, _) => {
            read_line_from_stdin().context("reading the API token from stdin (--token-stdin)")?
        }
        (None, false, Some(path)) => std::fs::read_to_string(path)
            .with_context(|| format!("reading the API token from `{}`", path.display()))?,
        (None, false, None) => prompt_for_token()?,
    };
    let api_token = raw.trim().to_string();
    if api_token.is_empty() {
        anyhow::bail!("the API token is empty");
    }
    Ok(api_token)
}

/// Ask for the token on the terminal. Input is echoed — hiding it would need a
/// new dependency (`rpassword`), so the prompt points at the piping alternative.
pub(crate) fn prompt_for_token() -> anyhow::Result<String> {
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "no API token given and stdin is not a terminal; pass `--token <token>`, \
             `--token-stdin`, or set {}",
            token::TOKEN_ENV_VAR
        );
    }
    eprintln!(
        "Paste your KanbanFlow API token (it will be visible; \
         `printf %s \"$TOKEN\" | kf auth login --token-stdin` keeps it out of shell history)."
    );
    eprint!("Token: ");
    std::io::stderr().flush().ok();
    read_line_from_stdin()
}

/// One line from stdin, without its trailing newline.
pub(crate) fn read_line_from_stdin() -> anyhow::Result<String> {
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .context("reading from stdin")?;
    Ok(line)
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

    #[test]
    fn read_token_reads_and_trims_a_token_file() {
        let path = std::env::temp_dir().join("kf-auth-token-file-test.txt");
        std::fs::write(&path, "tokenFromFile\n").expect("writable temp file");
        let api_token = read_token(None, false, Some(&path)).expect("file token");
        std::fs::remove_file(&path).ok();
        assert_eq!(api_token, "tokenFromFile");
    }
}
