/// Configuration parsing and validation.
///
/// This module contains three logical entities:
///  * Snippets: single configuration files, holding a subset of configuration entries.
///  * Inputs: configuration snippets merged, but not yet validated.
///  * AgentConfig: validated configuration for the update agent.

mod inputs;
mod snippets;

pub(crate) use crate::config::inputs::{IdentityInput, StratHttpInput, UpdateConfig};
use crate::update_agent::Identity;
use crate::strategy;
use failure::{Fallible, ResultExt};

/// Runtime configuration for the agent.
///
/// It holds validated agent configuration.
#[derive(Debug, Serialize)]
pub(crate) struct AgentConfig {
    pub(crate) identity: Identity,
    #[serde(with = "url_serde")]
    pub(crate) cincinnati: reqwest::Url,
    pub(crate) strategy: strategy::UpStrategy,
}

impl AgentConfig {
    pub(crate) fn read_config(_dirs: Vec<&str>) -> Fallible<Self> {
        let cfg = inputs::ConfigInput::read_config(_dirs)?;
        Self::try_from_input(cfg)
    }

    /// Validate inputs and return a valid agent configuration.
    fn try_from_input(cfg: inputs::ConfigInput) -> Fallible<Self> {
        let cincinnati = if !cfg.cincinnati.base_url.is_empty() {
            reqwest::Url::parse(&cfg.cincinnati.base_url)?
        } else {
            reqwest::Url::parse("http://localhost:9876")?
        };
        let identity = Identity::try_from_config(cfg.identity)
            .context("failed to build identity")?;
        let strategy = strategy::UpStrategy::try_from_config(cfg.updates)?;

        let state = AgentConfig {
            cincinnati,
            identity,
            strategy,
        };
        debug!(
            "Runtime configuration:\n{}",
            serde_json::to_string_pretty(&state).unwrap()
        );

        Ok(state)
    }
}
