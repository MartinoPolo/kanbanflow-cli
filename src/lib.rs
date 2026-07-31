//! `kf` — KanbanFlow from the command line.
//!
//! The binary is a thin wrapper: everything lives here so the shared
//! infrastructure (`api`, `config`, `token`, `output`, `resolve`, `guard`,
//! `files`, `labels`, `tasks`, `users`, `prompt`)
//! is reachable from integration tests as well as from the commands.

pub mod api;
pub mod cli;
pub mod commands;
pub mod config;
pub mod context;
pub mod exit;
pub mod files;
pub mod guard;
pub mod labels;
pub mod output;
pub mod prompt;
pub mod resolve;
pub mod tasks;
pub mod token;
pub mod users;
