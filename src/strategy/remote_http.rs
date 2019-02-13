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
    /// This POSTs to a remote lock manager in order to check
    /// whether this node can finalize the update at this point
    /// in time.
    pub(crate) fn has_green_light(
        self,
        params: HttpParams,
    ) -> Box<Future<Item = bool, Error = Error>> {
        trace!("finalizer check, strategy 'remote_http'");
        trace!("finalizer client parameters: {:?}", params.client_params);

        let endpoint = match self.base_url.join(LOCK_V1_PRE_REBOOT_PATH) {
            Ok(url) => url,
            Err(e) => return Box::new(future::err(format_err!("{}", e))),
        };

        // Ask the remote lock manager for a finalization slot.
        trace!("POST to remote finalizer: {}", endpoint);
        let req = asynchro::Client::new().post(endpoint).json(&params).send();

        // Ensure response is positive.
        let resp = req
            .and_then(|resp| resp.error_for_status())
            .map_err(|err| {
                error!("finalize remote_http: {}", err);
                err
            })
            .from_err();

        // Ensure response status is 200. That would be the
        // definitive green-light to proceed with update
        // finalization.
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
