//! Finalizer.

use crate::identity::Identity;
use crate::rpm_ostree;
use crate::strategy;
use actix::prelude::*;
use failure::{Error, Fallible};
use futures::future;
use futures::prelude::*;
use lazy_static::lazy_static;
use std::sync;
use std::time;

lazy_static! {
    pub(crate) static ref CONFIGURED: sync::RwLock<Option<Finalizer>> = sync::RwLock::default();
}

pub(crate) fn configure(strategy: strategy::FinStrategy, identity: Identity) -> Fallible<()> {
    let finalizer = Finalizer {
        pending: None,
        identity,
        strategy,
    };
    let mut static_cfg = CONFIGURED.try_write().unwrap();
    *static_cfg = Some(finalizer);
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Finalizer {
    pending: Option<libcincinnati::Release>,
    identity: Identity,
    strategy: strategy::FinStrategy,
}

impl Default for Finalizer {
    fn default() -> Self {
        let cfg = CONFIGURED.try_read().expect("poisoned lock");
        cfg.clone().expect("not configured")
    }
}

impl Actor for Finalizer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if let strategy::FinStrategy::Http(_) = self.strategy {
            // TODO(lucab): run_interval to report steady state.
        }

        ctx.run_interval(time::Duration::from_secs(20), |_act, ctx| {
            ctx.notify(TriggerFinalize {})
        });
    }
}

impl Supervised for Finalizer {}
impl SystemService for Finalizer {}

pub(crate) struct NewPending {
    pub(crate) payload: libcincinnati::Release,
}

impl Message for NewPending {
    type Result = Result<(), Error>;
}

impl Handler<NewPending> for Finalizer {
    type Result = Box<Future<Item = (), Error = Error>>;

    fn handle(&mut self, msg: NewPending, ctx: &mut Self::Context) -> Self::Result {
        self.pending = Some(msg.payload);

        ctx.notify(TriggerFinalize {});
        Box::new(future::ok(()))
    }
}

pub(crate) struct TriggerFinalize {}

impl Message for TriggerFinalize {
    type Result = Result<(), Error>;
}

impl Handler<TriggerFinalize> for Finalizer {
    type Result = Box<Future<Item = (), Error = Error>>;

    fn handle(&mut self, _msg: TriggerFinalize, _ctx: &mut Self::Context) -> Self::Result {
        let release = match self.pending.clone() {
            Some(r) => r,
            None => return Box::new(future::ok(())),
        };

        // Check if finalization is allowed at this time.
        let green_light = self.strategy.clone().has_green_light(self.identity.clone());

        // Try to finalize.
        let finalize = green_light.and_then(move |ok| {
            if ok {
                debug!("green-light for finalization");
                future::Either::A(rpm_ostree_finalize(release))
            } else {
                trace!("finalization not allowed now");
                future::Either::B(future::ok(()))
            }
        });

        Box::new(finalize)
    }
}

fn rpm_ostree_finalize(release: libcincinnati::Release) -> impl Future<Item = (), Error = Error> {
    let addr = System::current()
        .registry()
        .get::<rpm_ostree::RpmOstreeClient>();
    let req = rpm_ostree::FinalizeDeployment { release };
    addr.send(req).map(|_| ()).from_err()
}
