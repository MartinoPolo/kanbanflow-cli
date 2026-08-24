//! Exit codes are part of the agent contract; keep them stable.

use crate::api::ApiError;
use crate::config::ConfigError;
use crate::guard::GuardError;
use crate::resolve::ResolveError;
use crate::token::TokenError;
use crate::users::UnknownUser;

/// Anything that is not one of the specific cases below.
pub const FAILURE: i32 = 1;
/// clap usage error (clap exits with this itself).
pub const USAGE: i32 = 2;
/// The shared-board guardrail refused a mutation; `--force` overrides.
pub const GUARDRAIL: i32 = 3;
/// No usable API token, the API rejected it, or `kf` does not know which board
/// user it acts as. All three are fixed by `kf auth login`.
pub const AUTH: i32 = 4;
/// The board, issue, comment or attachment does not exist.
pub const NOT_FOUND: i32 = 5;
/// Rate limit hit; back off before retrying.
pub const RATE_LIMITED: i32 = 6;
/// No `mpxconfig.json`, or it is unusable. Run `kf init`.
pub const NO_CONFIG: i32 = 7;

/// Map an error chain onto the documented exit codes.
pub fn classify(error: &anyhow::Error) -> i32 {
    if error.downcast_ref::<GuardError>().is_some() {
        return GUARDRAIL;
    }
    if error.downcast_ref::<TokenError>().is_some() || error.downcast_ref::<UnknownUser>().is_some()
    {
        return AUTH;
    }
    if error.downcast_ref::<ConfigError>().is_some() {
        return NO_CONFIG;
    }
    // `ResolveError` wraps `ApiError`, which makes it the root of the chain and
    // hides the inner variant from the `ApiError` downcast below.
    match error.downcast_ref::<ResolveError>() {
        Some(ResolveError::NumberNotFound { .. }) => return NOT_FOUND,
        Some(ResolveError::Api(inner)) => return for_api_error(inner),
        None => {}
    }
    match error.downcast_ref::<ApiError>() {
        Some(api_error) => for_api_error(api_error),
        None => FAILURE,
    }
}

/// The single place an `ApiError` variant becomes an exit code.
fn for_api_error(error: &ApiError) -> i32 {
    match error {
        ApiError::Unauthorized | ApiError::UnusableToken => AUTH,
        ApiError::NotFound { .. } => NOT_FOUND,
        ApiError::RateLimited { .. } => RATE_LIMITED,
        _ => FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guardrail_refusal_has_its_own_exit_code() {
        let error = anyhow::Error::new(GuardError::Unassigned {
            issue: "E613".to_string(),
        })
        .context("moving issue E613");
        assert_eq!(classify(&error), GUARDRAIL);
    }

    #[test]
    fn api_errors_map_to_specific_codes() {
        assert_eq!(classify(&anyhow::Error::new(ApiError::Unauthorized)), AUTH);
        assert_eq!(
            classify(&anyhow::Error::new(ApiError::NotFound {
                message: "no such issue".to_string()
            })),
            NOT_FOUND
        );
        assert_eq!(
            classify(&anyhow::Error::new(ApiError::RateLimited {
                advice: "wait".to_string()
            })),
            RATE_LIMITED
        );
    }

    /// `resolve_issue` returns `ResolveError::Api`, so the wrapped variant — not
    /// the wrapper — has to decide the exit code, through any `.context()` layers.
    #[test]
    fn api_errors_wrapped_by_resolve_keep_their_codes() {
        let wrapped = |api_error: ApiError| {
            anyhow::Error::new(ResolveError::Api(api_error)).context("looking up issue E613")
        };
        assert_eq!(
            classify(&wrapped(ApiError::NotFound {
                message: "no such issue".to_string()
            })),
            NOT_FOUND
        );
        assert_eq!(
            classify(&wrapped(ApiError::RateLimited {
                advice: "wait".to_string()
            })),
            RATE_LIMITED
        );
        assert_eq!(classify(&wrapped(ApiError::Unauthorized)), AUTH);
        assert_eq!(
            classify(&wrapped(ApiError::Forbidden {
                message: "nope".to_string()
            })),
            FAILURE
        );
    }

    #[test]
    fn resolve_number_not_found_is_a_not_found() {
        let error = anyhow::Error::new(ResolveError::NumberNotFound {
            reference: "E613".to_string(),
        })
        .context("looking up issue E613");
        assert_eq!(classify(&error), NOT_FOUND);
    }

    /// Not knowing who we are is an authentication gap, and an agent's reaction
    /// to it is the same as to a missing token.
    #[test]
    fn an_unknown_board_user_exits_as_an_auth_failure() {
        let error = anyhow::Error::new(UnknownUser {
            board_id: "F2QMK1B".to_string(),
        })
        .context("listing my issues");
        assert_eq!(classify(&error), AUTH);
    }

    #[test]
    fn unclassified_errors_are_generic_failures() {
        assert_eq!(classify(&anyhow::anyhow!("something else")), FAILURE);
        assert_eq!(USAGE, 2);
    }
}
