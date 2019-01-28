//! Asynchronous Cincinnati client/scanner.
//!
//! This module contains `CincinnatiScanner`, which is the main
//! entity interacting with the Cincinnati upstream server.
//! It periodically tries to fetch a graph of updates, pick the
//! greatest one available and ask rpm-ostree daemon to fetch
//! and stage it.

use crate::identity::Identity;
use crate::rpm_ostree::{RpmOstreeClient, StageUpdate};
use actix::prelude::*;
use failure::{Error, Fallible};
use futures::future;
use futures::prelude::*;
use lazy_static::lazy_static;
use reqwest::r#async as asynchro;
use reqwest::Url;
use std::{sync, time};

/// Cincinnati graph API path endpoint (v1).
static V1_GRAPH_PATH: &str = "v1/graph";

lazy_static! {
    pub(crate) static ref CONFIGURED: sync::RwLock<Option<CincinnatiScanner>> =
        sync::RwLock::default();
}

/// Configure Cincinnati scanner.
///
/// This overwrite the global configuration for `CincinnatiScanner`.
/// It is called at least once at initialization time.
pub(crate) fn configure(base_url: reqwest::Url, identity: Identity) -> Fallible<()> {
    let endpoint = base_url.join(V1_GRAPH_PATH)?;
    let scanner = CincinnatiScanner {
        endpoint,
        fetch_interval: time::Duration::from_secs(15),
        fetch_on_start: true,
        identity,
    };
    let mut static_cfg = CONFIGURED.try_write().unwrap();
    *static_cfg = Some(scanner);
    Ok(())
}

/// Main actor for interacting with Cincinnati server.
#[derive(Clone, Debug)]
pub struct CincinnatiScanner {
    endpoint: Url,
    fetch_interval: time::Duration,
    fetch_on_start: bool,
    identity: Identity,
}

impl Default for CincinnatiScanner {
    fn default() -> Self {
        let cfg = CONFIGURED.try_read().expect("poisoned lock");
        cfg.clone().expect("not configured")
    }
}

impl Actor for CincinnatiScanner {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        trace!("cincinnati scanner started");

        if self.fetch_on_start {
            ctx.notify(FetchGraph {});
        }

        ctx.run_interval(self.fetch_interval, |_act, ctx| ctx.notify(FetchGraph {}));
    }
}

impl Supervised for CincinnatiScanner {}
impl SystemService for CincinnatiScanner {}

/// CincinnatiScanner request: fetch a graph of updates.
pub(crate) struct FetchGraph {}

impl Message for FetchGraph {
    type Result = Result<(), Error>;
}

impl Handler<FetchGraph> for CincinnatiScanner {
    type Result = Box<Future<Item = (), Error = Error>>;

    fn handle(&mut self, _msg: FetchGraph, _ctx: &mut Self::Context) -> Self::Result {
        let endpoint = self.endpoint.clone();
        let identity = self.identity.clone();

        // Ask cincinnati for an updated release.
        let next_release = fetch_cincinnati_next(endpoint, identity.into());
        // Poke rpm-ostree to fetch and stage the update.
        let notify_update = next_release.and_then(|next| match next {
            Some(release) => future::Either::A(rpm_ostree_stage_update(release)),
            None => future::Either::B(future::ok(())),
        });

        Box::new(notify_update)
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct HttpParams {
    pub(crate) current_version: String,
    pub(crate) stream: String,
    pub(crate) arch: String,
    pub(crate) platform: String,
    pub(crate) throttle_permille: String,
}

impl From<Identity> for HttpParams {
    fn from(identity: Identity) -> Self {
        Self {
            current_version: identity.current_version,
            stream: String::from("stable"),
            arch: String::from("amd64"),
            platform: String::from("metal"),
            throttle_permille: identity.throttle_permille.unwrap_or_default().to_string(),
        }
    }
}

fn rpm_ostree_stage_update(
    release: libcincinnati::Release,
) -> impl Future<Item = (), Error = Error> {
    let addr = System::current().registry().get::<RpmOstreeClient>();
    let req = StageUpdate { release };
    addr.send(req).map(|_| ()).from_err()
}

/// Fetch next available release update from Cincinnati.
///
/// Request a graph of releases from Cincinnati server, extract all
/// available updates reachable from the current version, then pick
/// up the greatest one.
fn fetch_cincinnati_next(
    endpoint: reqwest::Url,
    params: HttpParams,
) -> impl Future<Item = Option<libcincinnati::Release>, Error = Error> {
    trace!("cincinnati client parameters: {:?}", params);
    trace!("GET to remote graph endpoint: {:?}", endpoint);

    // Request cincinnati graph with client-specific parameters.
    let req = asynchro::Client::new().get(endpoint).query(&params).send();

    // Ensure response is positive.
    let resp = req
        .and_then(|resp| resp.error_for_status())
        .map_err(|err| {
            error!("{}", err);
            err
        })
        .from_err();

    // Parse a cincinnati graph from JSON.
    let graph = resp
        .inspect(|resp| trace!("graph response: {:#?}", resp))
        .and_then(|mut resp| resp.json::<libcincinnati::Graph>())
        .from_err();

    // Extract all available updates reachable from current release.
    let current = params.current_version.clone();
    let updates = graph
        .and_then(move |graph| {
            trace!("looking for current release '{}' in graph", current);
            let release_id = graph
                .find_by_version(&current)
                .ok_or_else(|| format_err!("current version '{}' not found in graph", current))?;

            let next_releases = graph
                .next_releases(&release_id)
                .cloned()
                .collect::<Vec<_>>();
            Ok(next_releases)
        })
        .inspect(|next_rels| trace!("found {} valid release-update(s)", next_rels.len()));

    // Pick up the greatest next release available, if any.
    updates
        .and_then(|ups| {
            // TODO(lucab): add stable order, then pick up the greatest.
            Ok(ups.first().cloned())
        })
        .inspect(|release| match release {
            Some(r) => info!("selected release '{}' for next update", r.version()),
            None => trace!("no next release"),
        })
}
