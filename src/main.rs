//! zincati: cincinnati to rpm-ostree update agent.
//!
//! This program contains an update-management agent which
//! bridges between a cincinnati server and rpm-ostree daemon,
//! implementing conditional strategies for finalization.
//!
//! It is made of three actors passing action-requests to each
//! other:
//!  * `CincinnatiScanner` - HTTP client to Cincinnati, periodic scraper.
//!  * `Finalizer` - update finalizer, with support for several user-strategies.
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
extern crate uuid;
extern crate url_serde;

mod cincinnati;
mod identity;
mod config;
mod sync_dbus;
mod finalize;
mod runtime_state;
mod rpm_ostree;
mod strategy;

use crate::cincinnati::CincinnatiScanner;
use crate::finalize::Finalizer;
use crate::rpm_ostree::RpmOstreeClient;
use actix::prelude::*;
use failure::Fallible;

fn main() -> Fallible<()> {
    env_logger::Builder::from_default_env().try_init()?;
    info!("starting zincati");

    // Configure whole application.
    {
        let dirs = vec!["/usr/lib", "/run", "/etc"];
        let input = config::RunConfig::read_config(dirs)?;
        let cfg = runtime_state::RunState::try_from_config(input)?;
        cincinnati::configure(cfg.cincinnati, cfg.identity.clone())?;
        finalize::configure(cfg.strategy, cfg.identity)?;
        rpm_ostree::configure()?;
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

    // Start finalizer in its own thread and event loop.
    let finalizer_arbiter = Arbiter::builder()
        .name("finalizer")
        .stop_system_on_panic(true)
        .build();
    let finalizer_supervisor =
        Supervisor::start_in_arbiter(&finalizer_arbiter, |_| Finalizer::default());
    System::current().registry().set(finalizer_supervisor);

    // Start cincinnati client in its own thread and event loop.
    let cincinnati_arbiter = Arbiter::builder()
        .name("cincinnati")
        .stop_system_on_panic(true)
        .build();
    let cincinnati_supervisor =
        Supervisor::start_in_arbiter(&cincinnati_arbiter, |_| CincinnatiScanner::default());
    System::current().registry().set(cincinnati_supervisor);

    sys.run();
    Ok(())
}
