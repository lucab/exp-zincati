use crate::config;
use crate::identity;
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
pub(crate) enum FinStrategy {
    Http(StratRemoteHTTP),
    Immediate(StratImmediate),
    Never(StratNever),
    Periodic(StratPeriodic),
}

impl FinStrategy {
    pub(crate) fn try_from_config(cfg: config::FinalizeConfig) -> Fallible<Self> {
        let strategy = match cfg.strategy.as_ref() {
            "immediate" => FinStrategy::Immediate(StratImmediate {}),
            "never" => FinStrategy::Never(StratNever {}),
            "periodic" => FinStrategy::try_periodic()?,
            "remote_http" => FinStrategy::try_remote_http(cfg.remote_http)?,
            "" => FinStrategy::default(),
            x => bail!("unsupported strategy '{}'", x),
        };
        Ok(strategy)
    }

    /// Check if finalization is allowed at this time.
    pub(crate) fn has_green_light(
        self,
        identity: identity::Identity,
    ) -> Box<Future<Item = bool, Error = Error>> {
        match self {
            FinStrategy::Http(h) => h.has_green_light(identity.into()),
            FinStrategy::Immediate(i) => i.finalize(),
            FinStrategy::Never(n) => n.has_green_light(),
            FinStrategy::Periodic(p) => p.finalize(),
        }
    }

    /// Check if finalization is allowed at this time.
    pub(crate) fn report_steady(
        self,
        identity: identity::Identity,
    ) -> Box<Future<Item = bool, Error = Error>> {
        match self {
            FinStrategy::Http(h) => h.report_steady(identity.into()),
            FinStrategy::Immediate(i) => i.finalize(),
            FinStrategy::Never(n) => n.report_steady(),
            FinStrategy::Periodic(p) => p.finalize(),
        }
    }

    fn try_periodic() -> Fallible<Self> {
        let periodic = StratPeriodic {};
        Ok(FinStrategy::Periodic(periodic))
    }

    fn try_remote_http(cfg: config::StratHttpConfig) -> Fallible<Self> {
        let remote_http = StratRemoteHTTP::parse(cfg)?;
        Ok(FinStrategy::Http(remote_http))
    }
}

impl Default for FinStrategy {
    fn default() -> Self {
        let immediate = StratImmediate {};
        FinStrategy::Immediate(immediate)
    }
}
