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
    #[error("the API token is empty")]
    Empty,
    #[error(
        "the API token contains {count} character(s) that an HTTP header cannot carry ({details}). \
         A KanbanFlow token is printable ASCII only — copy the whole token again from \
         Menu → Settings → API & Webhooks, or pass it with `--token-stdin` to keep the \
         terminal out of the way."
    )]
    Unusable { count: usize, details: String },
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

/// A pasted token with the invisible characters removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedToken {
    pub token: String,
    /// The invisible characters that were dropped, in the order they appeared.
    pub removed: Vec<char>,
}

impl SanitizedToken {
    /// A one-line explanation for stderr when the paste was not clean, so the
    /// user learns why. `None` when the token arrived intact.
    pub fn note(&self) -> Option<String> {
        let plural = match self.removed.len() {
            0 => return None,
            1 => "character",
            _ => "characters",
        };
        Some(format!(
            "note: removed {} invisible {plural} from the token ({}).",
            self.removed.len(),
            describe(&self.removed)
        ))
    }
}

/// Anything a terminal or a web page can smuggle into a copied token: spaces,
/// newlines, control codes (a stray DEL or ESC from a paste), and the
/// zero-width and format marks that survive a plain `trim()`.
fn is_invisible(character: char) -> bool {
    character.is_whitespace()
        || character.is_control()
        || matches!(character,
            '\u{00ad}' | '\u{061c}' | '\u{180e}' | '\u{200b}'..='\u{200f}'
            | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{2064}' | '\u{feff}')
}

/// Drop the invisible characters, keeping everything else — including anything
/// unusable, so a broken token still fails loudly instead of being ignored.
///
/// Surrounding whitespace is trimmed rather than reported: a trailing newline
/// from stdin is normal, while an interior control code is worth a note.
fn strip_invisible(raw: &str) -> SanitizedToken {
    let raw = raw.trim();
    let mut token = String::with_capacity(raw.len());
    let mut removed = Vec::new();
    for character in raw.chars() {
        if is_invisible(character) {
            removed.push(character);
        } else {
            token.push(character);
        }
    }
    SanitizedToken { token, removed }
}

/// Clean a token that came from a human: strip the invisible characters, then
/// reject anything an `Authorization` header cannot carry.
///
/// Without this, a single stray control character reaches `reqwest` and fails
/// as `builder error: failed to parse header value` — which reads like a bug in
/// the CLI rather than a bad paste.
pub fn sanitize(raw: &str) -> Result<SanitizedToken, TokenError> {
    let sanitized = strip_invisible(raw);
    let rejected: Vec<char> = sanitized
        .token
        .chars()
        .filter(|character| !character.is_ascii_graphic())
        .collect();
    if !rejected.is_empty() {
        return Err(TokenError::Unusable {
            count: rejected.len(),
            details: describe(&rejected),
        });
    }
    if sanitized.token.is_empty() {
        return Err(TokenError::Empty);
    }
    Ok(sanitized)
}

/// Name characters by code point, so a message can point at an invisible
/// character without echoing the token itself. At most three, then a count.
fn describe(characters: &[char]) -> String {
    let mut seen: Vec<char> = Vec::new();
    for character in characters {
        if !seen.contains(character) {
            seen.push(*character);
        }
    }
    let listed = seen
        .iter()
        .take(3)
        .map(|character| format!("U+{:04X}", *character as u32))
        .collect::<Vec<_>>()
        .join(", ");
    match seen.len().checked_sub(3) {
        Some(rest) if rest > 0 => format!("{listed} and {rest} more"),
        _ => listed,
    }
}

/// The token from `KANBANFLOW_TOKEN`, if it is set and non-empty.
pub fn from_env() -> Option<String> {
    std::env::var(TOKEN_ENV_VAR)
        .ok()
        .map(|value| strip_invisible(&value).token)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_a_clean_token_untouched() {
        let sanitized = sanitize("6FsUJdDqjyQj9Gfk29dLcgAJSM").expect("a clean token is usable");
        assert_eq!(sanitized.token, "6FsUJdDqjyQj9Gfk29dLcgAJSM");
        assert!(sanitized.removed.is_empty());
        assert_eq!(sanitized.note(), None);
    }

    /// Surrounding whitespace is the normal case and stays silent.
    #[test]
    fn sanitize_trims_surrounding_whitespace_without_a_note() {
        let sanitized = sanitize("  6FsUJdDqjy\r\n").expect("usable");
        assert_eq!(sanitized.token, "6FsUJdDqjy");
        assert_eq!(sanitized.note(), None);
    }

    /// The bad paste that made `kf init` fail with an opaque `reqwest` builder
    /// error: a control character `trim()` leaves in place.
    #[test]
    fn sanitize_strips_interior_control_characters_and_reports_them() {
        let sanitized =
            sanitize("  6FsUJd\u{7f}Dqjy\r\n").expect("stripping leaves a usable token");
        assert_eq!(sanitized.token, "6FsUJdDqjy");
        assert_eq!(sanitized.removed, vec!['\u{7f}']);
        assert_eq!(
            sanitized.note().expect("a note explains the removal"),
            "note: removed 1 invisible character from the token (U+007F)."
        );
    }

    /// Zero-width marks and a BOM are valid header bytes, so they used to reach
    /// the API and come back as a bare 401 with no hint as to why.
    #[test]
    fn sanitize_strips_zero_width_marks_and_a_byte_order_mark() {
        let sanitized = sanitize("\u{feff}6FsUJd\u{200b}Dqjy\u{00a0}").expect("usable");
        assert_eq!(sanitized.token, "6FsUJdDqjy");
        assert_eq!(sanitized.removed, vec!['\u{feff}', '\u{200b}']);
    }

    #[test]
    fn sanitize_rejects_visible_non_ascii_and_empty_input() {
        assert!(matches!(
            sanitize("6FsUJdDqé"),
            Err(TokenError::Unusable { count: 1, .. })
        ));
        assert!(matches!(sanitize("   \r\n"), Err(TokenError::Empty)));
        assert!(matches!(sanitize(""), Err(TokenError::Empty)));
    }

    #[test]
    fn describe_lists_distinct_code_points_then_counts_the_rest() {
        assert_eq!(describe(&['\u{7f}', '\u{7f}']), "U+007F");
        assert_eq!(
            describe(&['\u{1}', '\u{2}', '\u{3}', '\u{4}', '\u{5}']),
            "U+0001, U+0002, U+0003 and 2 more"
        );
    }
}
