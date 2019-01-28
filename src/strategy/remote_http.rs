use crate::config::StratHttpConfig;
use crate::identity::Identity;
use failure::{Error, Fallible};
use futures::prelude::*;
use reqwest::r#async as asynchro;

static DEFAULT_REMOTE_HTTP_BASE: &str = "http://localhost:2000";

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StratRemoteHTTP {
    #[serde(with = "url_serde")]
    pub(crate) base_url: reqwest::Url,
}

impl StratRemoteHTTP {
    pub(crate) fn parse(cfg: StratHttpConfig) -> Fallible<Self> {
        let base_url = if cfg.base_url.is_empty() {
            String::from(DEFAULT_REMOTE_HTTP_BASE)
        } else {
            cfg.base_url
        };

        let http = Self {
            base_url: reqwest::Url::parse(&base_url)?,
        };

        Ok(http)
    }

    pub(crate) fn finalize(
        self,
        params: HttpParams,
    ) -> Box<Future<Item = bool, Error = Error>> {
        trace!("finalizer check, strategy 'remote_http'");
        trace!("finalizer client parameters: {:?}", params);

        let endpoint = format!("{}/{}", self.base_url, "v1/pre-reboot");
        trace!("POST to remote finalizer: {:?}", endpoint);

        let req = asynchro::Client::new()
            .post(&endpoint)
            .json(&params)
            .send();

        // Ensure response is positive.
        let resp = req
            .and_then(|resp| resp.error_for_status())
            .map_err(|err| {
                error!("finalize remote_http: {}", err);
                err
            })
            .from_err();

        // Ensure response status is 200.
        let is_ok = resp.map(|_| true);

        Box::new(is_ok)
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct HttpParams {
    pub(crate) current_version: String,
    pub(crate) node_uuid: String,
    pub(crate) group: String,
}

impl From<Identity> for HttpParams {
    fn from(identity: Identity) -> Self {
        Self {
            current_version: identity.current_version,
            group: identity.group,
            node_uuid: identity.node_uuid.to_string(),
        }
    }
}
