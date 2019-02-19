//! zincati: Cincinnati to rpm-ostree update agent.
//!
//! This binary provides an on-host manager for auto-updates.
//! It consists of an update-management agent which
//! bridges between a Cincinnati server and the rpm-ostree daemon,
//! implementing conditional strategies for finalization.
//!
//! It is made of three actors passing action-requests to each
//! other:
//!  * `UpdateAgent` - main agent state-machine, with support for several user-strategies.
//!  * `CincinnatiClient` - HTTP client to Cincinnati, periodic scraper.
//!  * `RpmOstreeClient` - DBus client to rpm-ostree daemon.

extern crate cincinnati as libcincinnati;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate dbus;
extern crate dbus_tokio;
extern crate serde_json;
extern crate structopt;
extern crate url_serde;
extern crate uuid;

mod cincinnati;
mod config;
mod rpm_ostree;
mod strategy;
mod update_agent;

use crate::cincinnati::CincinnatiClient;
use crate::config::AgentConfig;
use crate::rpm_ostree::RpmOstreeClient;
use crate::update_agent::UpdateAgent;
use actix::prelude::*;
use failure::Fallible;

fn main() -> Fallible<()> {
    env_logger::Builder::from_default_env().try_init()?;
    info!("starting zincati");

    // Configure whole application.
    {
        let dirs = vec!["/usr/lib", "/run", "/etc"];
        let cfg = AgentConfig::read_config(dirs)?;
        cincinnati::configure(cfg.cincinnati, cfg.identity.clone())?;
        rpm_ostree::configure()?;
        update_agent::configure(cfg.strategy, cfg.identity)?;
    }

    let sys = actix::System::new("zincati");

    // Start rpm-ostree client in its own thread and event loop.
    let dbus_arbiter = Arbiter::builder()
        .name("dbus")
        .stop_system_on_panic(true)
        .build();
    let dbus_supervisor =
        Supervisor::start_in_arbiter(&dbus_arbiter, |_| RpmOstreeClient::default());
    System::current().registry().set(dbus_supervisor);

    // Start cincinnati client in its own thread and event loop.
    let cincinnati_arbiter = Arbiter::builder()
        .name("cincinnati")
        .stop_system_on_panic(true)
        .build();
    let cincinnati_supervisor =
        Supervisor::start_in_arbiter(&cincinnati_arbiter, |_| CincinnatiClient::default());
    System::current().registry().set(cincinnati_supervisor);

    // Start update agent in its own thread and event loop.
    let agent_arbiter = Arbiter::builder()
        .name("update_agent")
        .stop_system_on_panic(true)
        .build();
    let agent_supervisor =
        Supervisor::start_in_arbiter(&agent_arbiter, |_| UpdateAgent::default());
    System::current().registry().set(agent_supervisor);

    sys.run();
    Ok(())
}
