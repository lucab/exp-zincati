//! Update agent.

use super::identity::Identity;
use crate::cincinnati;
use crate::rpm_ostree;
use crate::strategy;
use actix::prelude::*;
use failure::Error;
use futures::future;
use futures::prelude::*;
use lazy_static::lazy_static;
use std::sync;
use std::time;

lazy_static! {
    pub(crate) static ref CONFIGURED: sync::RwLock<Option<UpdateAgent>> = sync::RwLock::default();
}

#[derive(Clone, Debug)]
pub(crate) struct UpdateAgent {
    pub(crate) identity: Identity,
    pub(crate) refresh_period: time::Duration,
    pub(crate) strategy: strategy::UpStrategy,
    pub(crate) state: UpdateAgentState,
}

#[derive(Clone, Debug)]
pub(crate) enum UpdateAgentState {
    /// Initial state upon actor start.
    StartState,
    /// Actor has been successfully initialized.
    Initialization,
    /// Actor is checking and waiting for updates.
    Steady,
    /// Update found and staged.
    UpdateStaged(libcincinnati::Release),
    /// Update finalized.
    UpdateFinalized(libcincinnati::Release),
}

impl Default for UpdateAgent {
    fn default() -> Self {
        let cfg = CONFIGURED.try_read().expect("poisoned lock");
        cfg.clone().expect("not configured")
    }
}

impl Actor for UpdateAgent {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        trace!("update agent started");

        // Schedule periodical refresh.
        ctx.notify(RefreshTick {});
        ctx.run_interval(self.refresh_period, |_act, ctx| ctx.notify(RefreshTick {}));
    }
}

impl Supervised for UpdateAgent {}
impl SystemService for UpdateAgent {}

pub(crate) struct RefreshTick {}

impl Message for RefreshTick {
    type Result = Result<(), Error>;
}

impl Handler<RefreshTick> for UpdateAgent {
    type Result = ResponseActFuture<Self, (), Error>;

    fn handle(&mut self, msg: RefreshTick, _ctx: &mut Self::Context) -> Self::Result {
        trace!("update agent tick, current state: {:?}", self.state);

        match self.state {
            UpdateAgentState::StartState => self.try_initialize(msg),
            UpdateAgentState::Initialization => self.try_steady(msg),
            UpdateAgentState::Steady => self.try_stage_update(msg),
            UpdateAgentState::UpdateStaged(ref r) => self.try_finalize_update(msg, r.clone()),
            UpdateAgentState::UpdateFinalized(_) => Box::new(actix::fut::ok(())),
        }
    }
}
impl UpdateAgent {
    /// Try to initialize the update agent.
    fn try_initialize(&mut self, _msg: RefreshTick) -> ResponseActFuture<Self, (), Error> {
        // TODO(lucab): double-check if initialization needs more crash-recovery logic.
        // If not, maybe get rid of `StartState`.
        let empty = future::ok(());
        let initialization = actix::fut::wrap_future::<_, Self>(empty).map(|_r, actor, _ctx| {
            actor.state = UpdateAgentState::Initialization;
        });

        Box::new(initialization)
    }

    /// Try to report agent readiness and move to steady state.
    fn try_steady(&mut self, _msg: RefreshTick) -> ResponseActFuture<Self, (), Error> {
        let report_steady = self.strategy.clone().report_steady(self.identity.clone());

        let steady_state =
            actix::fut::wrap_future::<_, Self>(report_steady).map(|is_ok, actor, _ctx| {
                if is_ok {
                    info!("steady state confirmed");
                    actor.state = UpdateAgentState::Steady;
                }
            });

        Box::new(steady_state)
    }

    /// Check for any available update and try to stage it.
    fn try_stage_update(&mut self, _msg: RefreshTick) -> ResponseActFuture<Self, (), Error> {
        let stage_update = cincinnati_check_update().and_then(|next| match next {
            Some(release) => future::Either::A(rpm_ostree_stage(release)),
            None => future::Either::B(future::ok(None)),
        });

        let staged =
            actix::fut::wrap_future::<_, Self>(stage_update).map(|release, actor, _ctx| {
                if let Some(r) = release {
                    actor.state = UpdateAgentState::UpdateStaged(r);
                }
            });

        Box::new(staged)
    }

    /// Check for finalization green-flag and try to finalize the update.
    fn try_finalize_update(
        &mut self,
        _msg: RefreshTick,
        release: libcincinnati::Release,
    ) -> ResponseActFuture<Self, (), Error> {
        // Check if finalization is allowed at this time.
        let green_light = self.strategy.clone().has_green_light(self.identity.clone());

        // Try to finalize.
        let finalize = green_light.and_then(move |ok| {
            if ok {
                info!("green-light for finalization");
                future::Either::A(rpm_ostree_finalize(release))
            } else {
                trace!("finalization not allowed now");
                future::Either::B(future::ok(None))
            }
        });

        // Progress to next state.
        let finalized = actix::fut::wrap_future::<_, Self>(finalize).map(|release, actor, _ctx| {
            if let Some(r) = release {
                actor.state = UpdateAgentState::UpdateFinalized(r);
            }
            // else { self.try_stage_update(_msg) }
        });

        Box::new(finalized)
    }
}

fn rpm_ostree_stage(
    release: libcincinnati::Release,
) -> impl Future<Item = Option<libcincinnati::Release>, Error = Error> {
    let addr = System::current()
        .registry()
        .get::<rpm_ostree::RpmOstreeClient>();
    let req = rpm_ostree::StageUpdate { release };
    addr.send(req).flatten().from_err()
}

fn rpm_ostree_finalize(
    release: libcincinnati::Release,
) -> impl Future<Item = Option<libcincinnati::Release>, Error = Error> {
    let addr = System::current()
        .registry()
        .get::<rpm_ostree::RpmOstreeClient>();
    let req = rpm_ostree::FinalizeUpdate { release };
    addr.send(req).flatten().from_err()
}

fn cincinnati_check_update() -> impl Future<Item = Option<libcincinnati::Release>, Error = Error> {
    let addr = System::current()
        .registry()
        .get::<cincinnati::CincinnatiClient>();
    let req = cincinnati::FetchGraph {};
    addr.send(req).flatten().from_err()
}
