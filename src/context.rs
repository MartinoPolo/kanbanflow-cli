//! The per-invocation essentials every board-touching command needs:
//! the repo's config, an authenticated API client, and who we are on the board.

use anyhow::Context as _;

use crate::api::Client;
use crate::config::Config;
use crate::token;
use crate::users;

pub struct Context {
    pub config: Config,
    pub client: Client,
    /// Resolved once at load: reading it per call would re-read the registry.
    /// `None` when nothing on this machine says who we are, which only the
    /// commands that need an identity may complain about.
    user_id: Option<String>,
}

impl Context {
    /// Load `mpxconfig.json` (searching upward), resolve the token and
    /// the acting user. No HTTP request is made here.
    pub fn load() -> anyhow::Result<Self> {
        let config = Config::load()?;
        let api_token = token::resolve_for_config(&config)?;
        let client = Client::new(api_token)
            .context("could not build the HTTP client for the KanbanFlow API")?;
        let user_id = users::my_user_id(&config);
        Ok(Self {
            config,
            client,
            user_id,
        })
    }

    /// The user we act as; the guardrail's notion of "me". An error when this
    /// machine has no identity for the board, because guessing would either
    /// block the user from their own issues or let them walk over a teammate's.
    pub fn my_user_id(&self) -> anyhow::Result<&str> {
        self.user_id.as_deref().ok_or_else(|| {
            anyhow::Error::new(users::UnknownUser {
                board_id: self.config.board_id.clone(),
            })
        })
    }

    /// The user we act as, or `None` — for output that renders "me" as a
    /// courtesy and reads fine without it.
    pub fn my_user_id_optional(&self) -> Option<&str> {
        self.user_id.as_deref()
    }
}
