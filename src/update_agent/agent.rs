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
    /// Update found.
    UpdateFound(libcincinnati::Release),
    /// Update transaction in progress.
    UpdateInProgress(libcincinnati::Release),
    /// Update staged.
    UpdateStaged(libcincinnati::Release),
    /// Finalizing transaction in progress.
    UpdateFinalizing(libcincinnati::Release),
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
            UpdateAgentState::Steady => self.check_for_update(msg),
            UpdateAgentState::UpdateFound(ref r) => self.try_update_deployment(msg, r.clone()),
            UpdateAgentState::UpdateInProgress(ref r) => self.check_update_success(msg, r.clone()),
            UpdateAgentState::UpdateStaged(ref r) => self.try_finalizing(msg, r.clone()),
            UpdateAgentState::UpdateFinalizing(_) => Box::new(actix::fut::ok(())),
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

    /// Check for any available update via Cincinnati.
    fn check_for_update(&mut self, _msg: RefreshTick) -> ResponseActFuture<Self, (), Error> {
        let check_update = cincinnati_check_update();

        let staged =
            actix::fut::wrap_future::<_, Self>(check_update).map(|release, actor, _ctx| {
                if let Some(r) = release {
                    actor.state = UpdateAgentState::UpdateFound(r);
                }
            });

        Box::new(staged)
    }

    /// Start deploying an update.
    fn try_update_deployment(
        &mut self,
        _msg: RefreshTick,
        release: libcincinnati::Release,
    ) -> ResponseActFuture<Self, (), Error> {
        // Start updating.
        let update = rpm_ostree_start_update(release);

        // Progress to next state.
        let updating = actix::fut::wrap_future::<_, Self>(update).map(|release, actor, _ctx| {
            if let Some(r) = release {
                actor.state = UpdateAgentState::UpdateInProgress(r);
            }
            // else { self.check_for_update(_msg) }
        });

        Box::new(updating)
    }

    /// Start deploying an update.
    fn check_update_success(
        &mut self,
        _msg: RefreshTick,
        release: libcincinnati::Release,
    ) -> ResponseActFuture<Self, (), Error> {
        // Start updating.
        let update = rpm_ostree_check_update(release);

        // Progress to next state.
        let updating = actix::fut::wrap_future::<_, Self>(update).map(|release, actor, _ctx| {
            if let Some(r) = release {
                actor.state = UpdateAgentState::UpdateInProgress(r);
            }
            // else { self.check_for_update(_msg) }
        });

        Box::new(updating)
    }

    /// Check for finalization green-flag and try to finalize the update.
    fn try_finalizing(
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
                actor.state = UpdateAgentState::UpdateFinalizing(r);
            }
            // else { self.try_stage_update(_msg) }
        });

        Box::new(finalized)
    }
}

fn rpm_ostree_start_update(
    release: libcincinnati::Release,
) -> impl Future<Item = Option<libcincinnati::Release>, Error = Error> {
    let addr = System::current()
        .registry()
        .get::<rpm_ostree::RpmOstreeClient>();
    let req = rpm_ostree::StageUpdate { release };
    addr.send(req).flatten().from_err()
}

fn rpm_ostree_check_update(
    release: libcincinnati::Release,
) -> impl Future<Item = Option<libcincinnati::Release>, Error = Error> {
    let addr = System::current()
        .registry()
        .get::<rpm_ostree::RpmOstreeClient>();
    let req = rpm_ostree::CheckUpdateTxn { release };
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
