//! Asynchronous rpm-ostree client.

use super::blocking::{DbusClient, StageDeployment};
use actix::prelude::*;
use failure::Error;
use futures::future;
use futures::prelude::*;
use lazy_static::lazy_static;
use std::sync;

lazy_static! {
    pub(crate) static ref CONFIGURED: sync::RwLock<Option<RpmOstreeClient>> =
        sync::RwLock::default();
}

/// Main actor for interacting with rpm-ostree.
#[derive(Clone, Debug)]
pub struct RpmOstreeClient {
    pub(crate) pending: Option<libcincinnati::Release>,
    pub(crate) dbus_client: Option<Addr<DbusClient>>,
}

/// Main actor for interacting with Cincinnati server.
impl Default for RpmOstreeClient {
    fn default() -> Self {
        let cfg = CONFIGURED.try_read().expect("poisoned lock");
        cfg.clone().expect("not configured")
    }
}

impl Supervised for RpmOstreeClient {}
impl SystemService for RpmOstreeClient {}
impl Actor for RpmOstreeClient {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let addr = actix::sync::SyncArbiter::start(2, DbusClient::default);
        self.dbus_client = Some(addr);

        trace!("rpm-ostree client started");
    }
}

/// Rpm-ostree request: stage a deployment.
pub(crate) struct StageUpdate {
    pub(crate) release: libcincinnati::Release,
}

impl Message for StageUpdate {
    type Result = Result<Option<libcincinnati::Release>, Error>;
}

impl Handler<StageUpdate> for RpmOstreeClient {
    type Result = Box<Future<Item = Option<libcincinnati::Release>, Error = Error>>;

    fn handle(&mut self, msg: StageUpdate, _ctx: &mut Self::Context) -> Self::Result {
        let stage = stage_update(self.dbus_client.clone().unwrap(), msg.release);
        Box::new(stage)
    }
}

pub(crate) struct FinalizeUpdate {
    pub(crate) release: libcincinnati::Release,
}

impl Message for FinalizeUpdate {
    type Result = Result<Option<libcincinnati::Release>, Error>;
}

impl Handler<FinalizeUpdate> for RpmOstreeClient {
    type Result = Box<Future<Item = Option<libcincinnati::Release>, Error = Error>>;

    fn handle(&mut self, msg: FinalizeUpdate, _ctx: &mut Self::Context) -> Self::Result {
        let finalize = finalize_update(self.dbus_client.clone().unwrap(), msg.release);
        Box::new(finalize)
    }
}

fn stage_update(
    addr: Addr<DbusClient>,
    release: libcincinnati::Release,
) -> impl Future<Item = Option<libcincinnati::Release>, Error = Error> {
    debug!(
        "rpm-ostree, requesting to stage update '{}'",
        release.version()
    );

    // TODO(lucab): stage update
    // https://github.com/projectatomic/rpm-ostree/issues/1748
    future::ok::<_, Error>(release)
        .and_then(move |release| {
            let req = StageDeployment { release };
            addr.send(req).from_err()
        })
        .flatten()
        .inspect(|release| info!("rpm-ostree, staged update '{}'", release.version()))
        .map(|release| (Some(release)))
}

fn finalize_update(
    addr: Addr<DbusClient>,
    release: libcincinnati::Release,
) -> impl Future<Item = Option<libcincinnati::Release>, Error = Error> {
    debug!(
        "rpm-ostree dbus, requesting to finalize deployment '{}'",
        release.version()
    );

    // TODO(lucab): finalize update
    // https://github.com/projectatomic/rpm-ostree/issues/1748
    future::ok::<_, Error>(release)
        .and_then(move |release| {
            let req = StageDeployment { release };
            addr.send(req).from_err()
        })
        .flatten()
        .inspect(|release| info!("rpm-ostree-dbus, finalized update '{}'", release.version()))
        .map(|release| (Some(release)))
}
