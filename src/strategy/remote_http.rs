use crate::config::StratHttpConfig;
use crate::identity::Identity;
use failure::{Error, Fallible};
use futures::future;
use futures::prelude::*;
use reqwest::r#async as asynchro;

/// Default base URL to the lock manager.
static DEFAULT_REMOTE_HTTP_BASE: &str = "http://localhost:9999";

/// Lock Manager pre-reboot endpoint (v1).
static LOCK_V1_PRE_REBOOT_PATH: &str = "v1/pre-reboot";

/// Lock Manager steady-state endpoint (v1).
static LOCK_V1_STEADY_STATE_PATH: &str = "v1/steady-state";

/// Strategy: remote HTTP lock manager.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct StratRemoteHTTP {
    /// Base URL to the lock manager.
    #[serde(with = "url_serde")]
    pub(crate) base_url: reqwest::Url,
}

impl StratRemoteHTTP {
    // Try to parse strategy configuration.
    pub(crate) fn parse(cfg: StratHttpConfig) -> Fallible<Self> {
        let base_url = if cfg.base_url.is_empty() {
            String::from(DEFAULT_REMOTE_HTTP_BASE)
        } else {
            cfg.base_url
        };

        let remote = Self {
            base_url: reqwest::Url::parse(&base_url)?,
        };

        Ok(remote)
    }

    /// Check if finalization is allowed.
    ///
    /// This POSTs to a remote reboot manager in order to check
    /// whether this node can finalize the update at this point
    /// in time.
    pub(crate) fn has_green_light(
        self,
        params: HttpParams,
    ) -> Box<Future<Item = bool, Error = Error>> {
        trace!("finalizer check, strategy 'remote_http'");
        trace!("finalizer client parameters: {:?}", params.client_params);

        // A positive response (status: 200) from the remote manager
        // is the definitive green-light to proceed with update finalization.
        let green_light = self.post_to_manager(LOCK_V1_PRE_REBOOT_PATH, params);

        Box::new(green_light)
    }

    /// Report steady state.
    ///
    /// This POSTs to a remote reboot manager in order to report
    /// that this node reached a steady state, unlocking any reboot
    /// semaphore it was previously holding.
    pub(crate) fn report_steady(
        self,
        params: HttpParams,
    ) -> Box<Future<Item = bool, Error = Error>> {
        trace!("report steady state, strategy 'remote_http'");
        trace!("steady state client parameters: {:?}", params.client_params);

        // A positive response (status: 200) from the remote manager
        // is the definitive confirmation this node reached steady state.
        let steady = self.post_to_manager(LOCK_V1_STEADY_STATE_PATH, params);

        Box::new(steady)
    }

    /// POST to a remote manager endpoint.
    fn post_to_manager(
        self,
        path: &'static str,
        params: HttpParams,
    ) -> Box<Future<Item = bool, Error = Error>> {
        // POST to remote manager endpoint.
        let endpoint = match self.base_url.join(path) {
            Ok(url) => url,
            Err(e) => return Box::new(future::err(format_err!("{}", e))),
        };
        trace!("POST to remote manager: {}", endpoint);
        let req = asynchro::Client::new().post(endpoint).json(&params).send();

        // Ensure response is positive.
        let resp = req
            .and_then(|resp| resp.error_for_status())
            .map_err(|err| {
                error!("remote_http: {}", err);
                err
            })
            .from_err();

        // Ensure response status is 200.
        let is_ok = resp.map(|r| r.status() == reqwest::StatusCode::OK);

        Box::new(is_ok)
    }
}

/// Client parameters for requests to the lock manager.
#[derive(Clone, Debug, Serialize)]
struct ClientParams {
    /// Current OS version.
    current_version: String,
    /// Unique node identifier.
    node_uuid: String,
    /// Reboot group.
    group: String,
}

/// Content for requests to the lock manager.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct HttpParams {
    client_params: ClientParams,
}

impl From<Identity> for HttpParams {
    fn from(identity: Identity) -> Self {
        let client_params = ClientParams {
            current_version: identity.current_version,
            group: identity.group,
            node_uuid: identity.node_uuid.to_string(),
        };
        Self { client_params }
    }
}
