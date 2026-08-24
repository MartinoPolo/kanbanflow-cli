//! `kf` — KanbanFlow from the command line.
//!
//! The binary is a thin wrapper: everything lives here so the shared
//! infrastructure (`api`, `boards`, `config`, `token`, `output`, `resolve`, `guard`,
//! `files`, `labels`, `issues`, `users`, `prompt`)
//! is reachable from integration tests as well as from the commands.

pub mod api;
pub mod boards;
pub mod cli;
pub mod commands;
pub mod config;
pub mod context;
pub mod exit;
pub mod files;
pub mod guard;
pub mod issues;
pub mod labels;
pub mod output;
pub mod prompt;
pub mod resolve;
pub mod token;
pub mod users;
