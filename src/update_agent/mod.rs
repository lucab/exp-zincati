//! Update agent state-machine.

mod identity;
mod agent;

pub(crate) use identity::Identity;
pub(crate) use agent::UpdateAgent;

use crate::strategy;

pub(crate) fn configure(strategy: strategy::UpStrategy, identity: Identity) -> failure::Fallible<()> {
    let actor = UpdateAgent {
        identity,
        refresh_period: std::time::Duration::from_secs(3),
        state: agent::UpdateAgentState::StartState,
        strategy,
    };
    let mut static_cfg = agent::CONFIGURED.try_write().unwrap();
    *static_cfg = Some(actor);
    Ok(())
}
