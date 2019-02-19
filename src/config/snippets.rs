/// Top-level configuration stanza.
#[derive(Debug, Deserialize)]
pub(crate) struct ConfigSnippet {
    /// Agent configuration.
    pub(crate) agent: Option<AgentSnippet>,
    /// Cincinnati client configuration.
    pub(crate) cincinnati: Option<CincinnatiSnippet>,
    /// Update strategy configuration.
    pub(crate) updates: Option<UpdateSnippet>,
    /// Agent identity.
    pub(crate) identity: Option<IdentitySnippet>,
}

/// General agent configuration.
#[derive(Debug, Deserialize)]
pub(crate) struct AgentSnippet {
//    /// Whether to print input configuration, for debug.
//    pub(crate) debug_input_config: Option<bool>,
//    /// Whether to print validated runtime configuration, for debug.
//    pub(crate) debug_runtime_config: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IdentitySnippet {
    /// Update group for this agent (default: 'default')
    pub(crate) group: Option<String>,
    pub(crate) node_uuid: Option<String>,
    /// Throttle bucket for this agent (default: dynamically computed)
    pub(crate) throttle_permille: Option<String>,
}


/// Config snippet for Cincinnati client.
#[derive(Debug, Deserialize)]
pub(crate) struct CincinnatiSnippet {
    /// Base URL to upstream cincinnati server.
    pub(crate) base_url: Option<String>,
}

/// Config snippet for update logic.
#[derive(Debug, Deserialize)]
pub(crate) struct UpdateSnippet {
    /// Update strategy (default: immediate)
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

/// Config snippet for `periodic` update strategy.
#[derive(Debug, Deserialize)]
pub(crate) struct StratPeriodicSnippet {}
