use crate::config;
use failure::{Error, Fallible};
use futures::prelude::*;

mod immediate;
pub(crate) use immediate::StratImmediate;

mod never;
pub(crate) use never::StratNever;

mod periodic;
pub(crate) use periodic::StratPeriodic;

mod remote_http;
pub(crate) use remote_http::{HttpParams, StratRemoteHTTP};

#[derive(Clone, Debug, Serialize)]
pub(crate) enum FinStrategy {
    Http(StratRemoteHTTP),
    Immediate(StratImmediate),
    Never(StratNever),
    Periodic(StratPeriodic),
}

impl FinStrategy {
    pub(crate) fn finalize(self, params: HttpParams) -> Box<Future<Item = bool, Error = Error>> {
        match self {
            FinStrategy::Http(h) => h.finalize(params),
            FinStrategy::Immediate(i) => i.finalize(),
            FinStrategy::Never(n) => n.finalize(),
            FinStrategy::Periodic(p) => p.finalize(),
        }
    }

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
