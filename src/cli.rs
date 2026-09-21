//! Top-level command grammar: `kf <noun> <verb>`.
//!
//! Each noun's arguments live in its own module under `commands/`, so a command
//! and its clap definition are always edited in a single file.

use clap::{Parser, Subcommand};

use crate::commands;

#[derive(Debug, Parser)]
#[command(
    name = "kf",
    version,
    about = "KanbanFlow from the command line",
    propagate_version = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Set up this repo: verify the token, map columns to canonical states.
    Init(commands::init::InitArgs),
    /// Manage the stored API token.
    Auth {
        #[command(subcommand)]
        command: commands::auth::AuthCommand,
    },
    /// Create, read and change issues.
    Issue {
        #[command(subcommand)]
        command: commands::issue::IssueCommand,
    },
    /// Attach files to issues and download them again.
    Attach {
        #[command(subcommand)]
        command: commands::attach::AttachCommand,
    },
    /// Read and write issue comments.
    Comment {
        #[command(subcommand)]
        command: commands::comment::CommentCommand,
    },
    /// Manage an issue's checklist items.
    Subtask {
        #[command(subcommand)]
        command: commands::subtask::SubtaskCommand,
    },
    /// Inspect the board's labels.
    Label {
        #[command(subcommand)]
        command: commands::label::LabelCommand,
    },
    /// Show the board's columns and canonical-state mapping.
    Board(commands::board::BoardArgs),
}

impl Cli {
    /// Dispatch to the owning command module.
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Init(args) => commands::init::run(args),
            Commands::Auth { command } => commands::auth::run(command),
            Commands::Issue { command } => commands::issue::run(command),
            Commands::Attach { command } => commands::attach::run(command),
            Commands::Comment { command } => commands::comment::run(command),
            Commands::Subtask { command } => commands::subtask::run(command),
            Commands::Label { command } => commands::label::run(command),
            Commands::Board(args) => commands::board::run(args),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::Cli;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn public_noun_is_issue_without_legacy_alias() {
        assert!(Cli::try_parse_from(["kf", "issue", "list"]).is_ok());
        assert!(Cli::try_parse_from(["kf", "task", "list"]).is_err());
    }

    #[test]
    fn issue_move_requires_exactly_one_target() {
        assert!(Cli::try_parse_from(["kf", "issue", "move", "E613"]).is_err());
        assert!(Cli::try_parse_from([
            "kf", "issue", "move", "E613", "--to", "done", "--column", "Done"
        ])
        .is_err());
        assert!(Cli::try_parse_from(["kf", "issue", "move", "E613", "--to", "backlog"]).is_ok());
        assert!(
            Cli::try_parse_from(["kf", "issue", "move", "E613", "--column", "Do today"]).is_ok()
        );
    }
}
