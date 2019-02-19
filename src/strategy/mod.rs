//! Update and reboot strategies.

use crate::config;
use crate::update_agent::Identity;
use failure::{Error, Fallible};
use futures::prelude::*;

mod immediate;
pub(crate) use immediate::StratImmediate;

mod never;
pub(crate) use never::StratNever;

mod periodic;
pub(crate) use periodic::StratPeriodic;

mod remote_http;
pub(crate) use remote_http::StratRemoteHTTP;

#[derive(Clone, Debug, Serialize)]
pub(crate) enum UpStrategy {
    Http(StratRemoteHTTP),
    Immediate(StratImmediate),
    Never(StratNever),
    Periodic(StratPeriodic),
}

impl UpStrategy {
    /// Try to parse config inputs into a valid strategy.
    pub(crate) fn try_from_config(cfg: config::UpdateConfig) -> Fallible<Self> {
        let strategy = match cfg.strategy.as_ref() {
            "immediate" => UpStrategy::Immediate(StratImmediate {}),
            "never" => UpStrategy::Never(StratNever {}),
            "periodic" => UpStrategy::try_periodic()?,
            "remote_http" => UpStrategy::try_remote_http(cfg.remote_http)?,
            "" => UpStrategy::default(),
            x => bail!("unsupported strategy '{}'", x),
        };
        Ok(strategy)
    }

    /// Check if finalization is allowed at this time.
    pub(crate) fn has_green_light(
        self,
        identity: Identity,
    ) -> Box<Future<Item = bool, Error = Error>> {
        match self {
            UpStrategy::Http(h) => h.has_green_light(identity.into()),
            UpStrategy::Immediate(i) => i.has_green_light(),
            UpStrategy::Never(n) => n.has_green_light(),
            UpStrategy::Periodic(p) => p.finalize(),
        }
    }

    /// Check if this agent is allowed to check for updates at this time.
    pub(crate) fn report_steady(
        self,
        identity: Identity,
    ) -> Box<Future<Item = bool, Error = Error>> {
        match self {
            UpStrategy::Http(h) => h.report_steady(identity.into()),
            UpStrategy::Immediate(i) => i.report_steady(),
            UpStrategy::Never(n) => n.report_steady(),
            UpStrategy::Periodic(p) => p.finalize(),
        }
    }

    fn try_periodic() -> Fallible<Self> {
        let periodic = StratPeriodic {};
        Ok(UpStrategy::Periodic(periodic))
    }

    fn try_remote_http(cfg: config::StratHttpInput) -> Fallible<Self> {
        let remote_http = StratRemoteHTTP::parse(cfg)?;
        Ok(UpStrategy::Http(remote_http))
    }
}

impl Default for UpStrategy {
    fn default() -> Self {
        let immediate = StratImmediate {};
        UpStrategy::Immediate(immediate)
    }
}
