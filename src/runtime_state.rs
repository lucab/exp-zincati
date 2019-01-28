//! Runtime configuration.
//!
//! This module contains logic and data structures to transform
//! external configuration into valid runtime state.
//! It performs semantic parsing and validation over configuration
//! inputs.

use crate::config;
use crate::identity;
use crate::strategy;
use failure::{Fallible, ResultExt};

/// Runtime state for the agent.
///
/// It holds agent parameters, validated from configs.
#[derive(Debug, Serialize)]
pub(crate) struct RunState {
    pub(crate) identity: identity::Identity,
    #[serde(with = "url_serde")]
    pub(crate) cincinnati: reqwest::Url,
    pub(crate) strategy: strategy::FinStrategy,
}

impl RunState {
    /// Validate config and return a valid agent state.
    pub(crate) fn try_from_config(cfg: config::RunConfig) -> Fallible<Self> {
        let cincinnati = if !cfg.cincinnati.base_url.is_empty() {
            reqwest::Url::parse(&cfg.cincinnati.base_url)?
        } else {
            reqwest::Url::parse("http://localhost:9876")?
        };
        let identity = identity::Identity::try_from_config(cfg.identity)
            .context("failed to build identity")?;
        let strategy = strategy::FinStrategy::try_from_config(cfg.finalize)?;

        let state = RunState {
            cincinnati,
            identity,
            strategy,
        };
        info!(
            "JSON runtime state:\n{}",
            serde_json::to_string_pretty(&state).unwrap()
        );

        Ok(state)
    }
}
