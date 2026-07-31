//! API token resolution and storage.
//!
//! Resolution order: `KANBANFLOW_TOKEN` env var, then the OS credential store
//! (Windows Credential Manager via `keyring`). Tokens never touch config files.

use crate::config::Config;

pub const KEYRING_SERVICE: &str = "kanbanflow-cli";
pub const TOKEN_ENV_VAR: &str = "KANBANFLOW_TOKEN";

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error(
        "No API token for board `{board_id}`. Run `kf auth login` to store one, \
         or set the {TOKEN_ENV_VAR} environment variable."
    )]
    Missing { board_id: String },
    #[error("Could not access the OS credential store for board `{board_id}`: {source}")]
    Keyring {
        board_id: String,
        #[source]
        source: keyring::Error,
    },
}

fn entry(board_id: &str) -> Result<keyring::Entry, TokenError> {
    keyring::Entry::new(KEYRING_SERVICE, board_id).map_err(|source| TokenError::Keyring {
        board_id: board_id.to_string(),
        source,
    })
}

/// The token from `KANBANFLOW_TOKEN`, if it is set and non-empty.
pub fn from_env() -> Option<String> {
    std::env::var(TOKEN_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Resolve the token for one board: env var wins, then the credential store.
pub fn resolve(board_id: &str) -> Result<String, TokenError> {
    if let Some(token) = from_env() {
        return Ok(token);
    }
    match entry(board_id)?.get_password() {
        Ok(token) => Ok(token),
        Err(keyring::Error::NoEntry) => Err(TokenError::Missing {
            board_id: board_id.to_string(),
        }),
        Err(source) => Err(TokenError::Keyring {
            board_id: board_id.to_string(),
            source,
        }),
    }
}

/// Convenience for the common "config is already loaded" path.
pub fn resolve_for_config(config: &Config) -> Result<String, TokenError> {
    resolve(&config.board_id)
}

/// Persist a token for a board in the OS credential store.
pub fn store(board_id: &str, token: &str) -> Result<(), TokenError> {
    entry(board_id)?
        .set_password(token)
        .map_err(|source| TokenError::Keyring {
            board_id: board_id.to_string(),
            source,
        })
}

/// Remove a stored token. Missing entries are treated as already deleted.
pub fn delete(board_id: &str) -> Result<(), TokenError> {
    match entry(board_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(source) => Err(TokenError::Keyring {
            board_id: board_id.to_string(),
            source,
        }),
    }
}
