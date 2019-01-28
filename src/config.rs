//! Runtime configuration.
//!
//! This module contains logic and data structures to read
//! multiple file snippets into a single configuration structure.
//! No input validation is done here, only config sourcing.
//! Semantic parsing is responsibility of `RunState` instead.

use failure::{Fallible, ResultExt};

/// Runtime configuration holding environmental input.
#[derive(Debug, Serialize)]
pub(crate) struct RunConfig {
    pub(crate) cincinnati: CincinnatiConfig,
    pub(crate) finalize: FinalizeConfig,
    pub(crate) identity: IdentityConfig,
}

impl RunConfig {
    /// Read config snippets and merge them into a single config.
    pub(crate) fn read_config(_dirs: Vec<&str>) -> Fallible<Self> {
        use std::io::Read;
        let path = "/etc/zincati/conf.d/00-config-sample.toml";
        trace!("reading config snippets from {:?}", path);

        let fp = std::fs::File::open(path).context(format!("failed to open file '{}'", path))?;
        let mut bufrd = std::io::BufReader::new(fp);
        let mut content = vec![];
        bufrd
            .read_to_end(&mut content)
            .context("failed to read file content")?;
        let snippet: SingleSnippet = toml::from_slice(&content).context("failed to parse TOML")?;

        let snips = vec![snippet];
        let cfg = Self::merge_snippets(snips);
        info!(
            "TOML runtime config:\n{}",
            toml::to_string_pretty(&cfg).unwrap()
        );

        Ok(cfg)
    }

    /// Merge multiple snippets into a single configuration.
    fn merge_snippets(snippets: Vec<SingleSnippet>) -> Self {
        let mut cincinnatis = vec![];
        let mut finalizes = vec![];
        let mut identities = vec![];

        for snip in snippets {
            if let Some(c) = snip.cincinnati {
                cincinnatis.push(c);
            }
            if let Some(f) = snip.finalize {
                finalizes.push(f);
            }
            if let Some(i) = snip.identity {
                identities.push(i);
            }
        }

        Self {
            cincinnati: CincinnatiConfig::from_snippets(cincinnatis),
            finalize: FinalizeConfig::from_snippets(finalizes),
            identity: IdentityConfig::from_snippets(identities),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CincinnatiConfig {
    pub(crate) base_url: String,
}

impl CincinnatiConfig {
    fn from_snippets(snippets: Vec<CincinnatiSnippet>) -> Self {
        let mut cfg = Self {
            base_url: String::new(),
        };

        for snip in snippets {
            if let Some(u) = snip.base_url {
                cfg.base_url = u;
            }
        }

        cfg
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct SingleSnippet {
    pub(crate) agent: Option<AgentSnippet>,
    pub(crate) cincinnati: Option<CincinnatiSnippet>,
    pub(crate) finalize: Option<FinalizeSnippet>,
    pub(crate) identity: Option<IdentitySnippet>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AgentSnippet {}

#[derive(Debug, Deserialize)]
pub(crate) struct IdentitySnippet {
    pub(crate) group: Option<String>,
    pub(crate) node_uuid: Option<String>,
    pub(crate) throttle_permille: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct IdentityConfig {
    pub(crate) group: String,
    pub(crate) node_uuid: String,
    pub(crate) throttle_permille: String,
}

impl IdentityConfig {
    fn from_snippets(snippets: Vec<IdentitySnippet>) -> Self {
        let mut cfg = Self {
            group: String::new(),
            node_uuid: String::new(),
            throttle_permille: String::new(),
        };

        for snip in snippets {
            if let Some(g) = snip.group {
                cfg.group = g;
            }
            if let Some(nu) = snip.node_uuid {
                cfg.node_uuid = nu;
            }
            if let Some(tp) = snip.throttle_permille {
                cfg.throttle_permille = tp;
            }
        }

        cfg
    }
}

/// Config snippet for Cincinnati client.
#[derive(Debug, Deserialize)]
pub(crate) struct CincinnatiSnippet {
    pub(crate) base_url: Option<String>,
}

/// Config for finalizer.
#[derive(Debug, Serialize)]
pub(crate) struct FinalizeConfig {
    pub(crate) strategy: String,
    /// `remote_http` strategy config.
    pub(crate) remote_http: StratHttpConfig,
    /// `periodic` strategy config.
    pub(crate) periodic: StratPeriodicConfig,
}

impl FinalizeConfig {
    fn from_snippets(snippets: Vec<FinalizeSnippet>) -> Self {
        let mut strategy = String::new();
        let mut remote_http = StratHttpConfig {
            base_url: String::new(),
        };
        let periodic = StratPeriodicConfig {};

        for snip in snippets {
            if let Some(s) = snip.strategy {
                strategy = s;
            }
            if let Some(remote) = snip.remote_http {
                if let Some(b) = remote.base_url {
                    remote_http.base_url = b;
                }
            }
        }

        Self {
            strategy,
            remote_http,
            periodic,
        }
    }
}

/// Config snippet for `remote_http` finalizer strategy.
#[derive(Debug, Serialize)]
pub(crate) struct StratHttpConfig {
    /// Base URL for the remote semaphore manager.
    pub(crate) base_url: String,
}

/// Config snippet for `periodic` finalizer strategy.
#[derive(Debug, Serialize)]
pub(crate) struct StratPeriodicConfig {}

/// Config snippet for finalizer.
#[derive(Debug, Deserialize)]
pub(crate) struct FinalizeSnippet {
    pub(crate) strategy: Option<String>,
    /// `remote_http` strategy config.
    pub(crate) remote_http: Option<StratHttpSnippet>,
    /// `periodic` strategy config.
    pub(crate) periodic: Option<StratPeriodicSnippet>,
}

/// Config snippet for `remote_http` finalizer strategy.
#[derive(Debug, Deserialize)]
pub(crate) struct StratHttpSnippet {
    /// Base URL for the remote semaphore manager.
    pub(crate) base_url: Option<String>,
}

/// Config snippet for `periodic` finalizer strategy.
#[derive(Debug, Deserialize)]
pub(crate) struct StratPeriodicSnippet {}
