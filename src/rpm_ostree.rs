//! Asynchronous rpm-ostree client.

use crate::finalize;
use crate::sync_dbus::{DbusClient, StageDeployment};
use actix::prelude::*;
use failure::{Error, Fallible};
use futures::future;
use futures::prelude::*;
use lazy_static::lazy_static;
use std::{sync, time};

lazy_static! {
    pub(crate) static ref CONFIGURED: sync::RwLock<Option<RpmOstreeClient>> =
        sync::RwLock::default();
}

/// Main actor for interacting with rpm-ostree.
#[derive(Clone, Debug)]
pub struct RpmOstreeClient {
    pending: Option<libcincinnati::Release>,
    apply_interval: time::Duration,
    dbus_client: Option<Addr<DbusClient>>,
}

pub(crate) fn configure() -> Fallible<()> {
    let client = RpmOstreeClient {
        apply_interval: time::Duration::from_secs(30),
        pending: None,
        dbus_client: None,
    };
    let mut static_cfg = CONFIGURED.try_write().unwrap();
    *static_cfg = Some(client);
    Ok(())
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

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = actix::sync::SyncArbiter::start(2, DbusClient::default);
        self.dbus_client = Some(addr);

        trace!("rpm-ostree client started");
        ctx.run_interval(self.apply_interval, |_act, ctx| ctx.notify(ApplyPending {}));
    }
}

/// Rpm-ostree request: stage a deployment.
pub(crate) struct StageUpdate {
    pub(crate) release: libcincinnati::Release,
}

impl Message for StageUpdate {
    type Result = Result<(), Error>;
}

impl Handler<StageUpdate> for RpmOstreeClient {
    type Result = Box<Future<Item = (), Error = Error>>;

    fn handle(&mut self, msg: StageUpdate, ctx: &mut Self::Context) -> Self::Result {
        if self.pending.is_none() {
            //TODO(lucab): compare and keep greatest.
            trace!(
                "enqueued new pending rpm-ostree update, version {}",
                msg.release.version()
            );
            self.pending = Some(msg.release);
        }

        ctx.notify(ApplyPending {});
        Box::new(future::ok(()))
    }
}

/// Rpm-ostree request: apply pending deployment.
pub(crate) struct ApplyPending {}

impl Message for ApplyPending {
    type Result = Result<(), Error>;
}

impl Handler<ApplyPending> for RpmOstreeClient {
    type Result = Box<Future<Item = (), Error = Error>>;

    fn handle(&mut self, _msg: ApplyPending, _ctx: &mut Self::Context) -> Self::Result {
        match self.pending.clone() {
            Some(r) => {
                debug!("pending update '{}'", r.version());
                let addr = self.dbus_client.clone().unwrap();
                let deploy = stage_update(addr, r).and_then(finalizer_trigger);
                Box::new(deploy)
            }
            None => {
                trace!("no pending update");
                Box::new(future::ok(()))
            }
        }
    }
}

pub(crate) struct FinalizeDeployment {
    pub(crate) release: libcincinnati::Release,
}

impl Message for FinalizeDeployment {
    type Result = Result<(), Error>;
}

impl Handler<FinalizeDeployment> for RpmOstreeClient {
    type Result = Box<Future<Item = (), Error = Error>>;

    fn handle(&mut self, msg: FinalizeDeployment, _ctx: &mut Self::Context) -> Self::Result {
        let finalize = finalize_deployment(msg.release);
        Box::new(finalize)
    }
}

fn stage_update(
    addr: Addr<DbusClient>,
    release: libcincinnati::Release,
) -> impl Future<Item = libcincinnati::Release, Error = Error> {
    debug!(
        "rpm-ostree, requesting to stage update '{}'",
        release.version()
    );

    future::ok(release)
        .and_then(move |release| {
            let req = StageDeployment { release };
            addr.send(req).from_err()
        })
        .and_then(|release| release)
        .inspect(|release| info!("rpm-ostree, staged update '{}'", release.version()))
}

fn finalize_deployment(release: libcincinnati::Release) -> impl Future<Item = (), Error = Error> {
    debug!(
        "rpm-ostree dbus, requesting to finalize deployment '{}'",
        release.version()
    );

    // TODO(lucab): finalize update
    // https://github.com/projectatomic/rpm-ostree/issues/1748
    future::ok(release)
        .inspect(|release| info!("rpm-ostree-dbus, finalized update '{}'", release.version()))
        .map(|_| ())
}

fn finalizer_trigger(release: libcincinnati::Release) -> impl Future<Item = (), Error = Error> {
    let addr = System::current().registry().get::<finalize::Finalizer>();
    let req = finalize::NewPending { payload: release };
    addr.send(req).map(|_| ()).from_err()
}
