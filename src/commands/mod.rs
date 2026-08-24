//! One module per CLI noun. Each module owns its clap definitions and its
//! implementation, so no two commands are ever edited in the same file.

pub mod attach;
pub mod auth;
pub mod board;
pub mod comment;
pub mod init;
pub mod issue;
pub mod label;
pub mod subtask;
