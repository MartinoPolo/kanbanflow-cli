//! The per-invocation essentials every board-touching command needs:
//! the repo's config and an authenticated API client.

use anyhow::Context as _;

use crate::api::Client;
use crate::config::Config;
use crate::token;

pub struct Context {
    pub config: Config,
    pub client: Client,
}

impl Context {
    /// Load `.mpx/kanbanflow.json` (searching upward) and resolve the token.
    /// No HTTP request is made here.
    pub fn load() -> anyhow::Result<Self> {
        let config = Config::load()?;
        let api_token = token::resolve_for_config(&config)?;
        let client = Client::new(api_token)
            .context("could not build the HTTP client for the KanbanFlow API")?;
        Ok(Self { config, client })
    }

    /// The user ID the token acts as; the guardrail's notion of "me".
    pub fn my_user_id(&self) -> &str {
        &self.config.user_id
    }
}
