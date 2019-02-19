use crate::config::snippets;
use failure::{Fallible, ResultExt};

/// Runtime configuration holding environmental inputs.
#[derive(Debug, Serialize)]
pub(crate) struct ConfigInput {
    pub(crate) cincinnati: CincinnatiInput,
    pub(crate) updates: UpdateConfig,
    pub(crate) identity: IdentityInput,
}

impl ConfigInput {
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
        let snippet: snippets::ConfigSnippet =
            toml::from_slice(&content).context("failed to parse TOML")?;

        let snips = vec![snippet];
        let cfg = Self::merge_snippets(snips);
        debug!(
            "Configuration input:\n{}",
            toml::to_string_pretty(&cfg).unwrap()
        );

        Ok(cfg)
    }

    /// Merge multiple snippets into a single configuration.
    fn merge_snippets(snippets: Vec<snippets::ConfigSnippet>) -> Self {
        let mut cincinnatis = vec![];
        let mut updates = vec![];
        let mut identities = vec![];

        for snip in snippets {
            if let Some(c) = snip.cincinnati {
                cincinnatis.push(c);
            }
            if let Some(f) = snip.updates {
                updates.push(f);
            }
            if let Some(i) = snip.identity {
                identities.push(i);
            }
        }

        Self {
            cincinnati: CincinnatiInput::from_snippets(cincinnatis),
            updates: UpdateConfig::from_snippets(updates),
            identity: IdentityInput::from_snippets(identities),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CincinnatiInput {
    pub(crate) base_url: String,
}

impl CincinnatiInput {
    fn from_snippets(snippets: Vec<snippets::CincinnatiSnippet>) -> Self {
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

#[derive(Debug, Serialize)]
pub(crate) struct IdentityInput {
    pub(crate) group: String,
    pub(crate) node_uuid: String,
    pub(crate) throttle_permille: String,
}

impl IdentityInput {
    fn from_snippets(snippets: Vec<snippets::IdentitySnippet>) -> Self {
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

/// Config for finalizer.
#[derive(Debug, Serialize)]
pub(crate) struct UpdateConfig {
    pub(crate) strategy: String,
    /// `remote_http` strategy config.
    pub(crate) remote_http: StratHttpInput,
    /// `periodic` strategy config.
    pub(crate) periodic: StratPeriodicConfig,
}

impl UpdateConfig {
    fn from_snippets(snippets: Vec<snippets::UpdateSnippet>) -> Self {
        let mut strategy = String::new();
        let mut remote_http = StratHttpInput {
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
pub(crate) struct StratHttpInput {
    /// Base URL for the remote semaphore manager.
    pub(crate) base_url: String,
}

/// Config snippet for `periodic` finalizer strategy.
#[derive(Debug, Serialize)]
pub(crate) struct StratPeriodicConfig {}
