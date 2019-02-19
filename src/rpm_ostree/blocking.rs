//! Blocking DBus client for rpm-ostree.

use actix::prelude::*;
use dbus::arg::Array;
use dbus::{BusType, Connection};
use failure::Fallible;

/// DBus client, blocking implementation.
#[derive(Debug, Default)]
pub struct DbusClient {
    conn: Option<Connection>,
}

impl Actor for DbusClient {
    type Context = SyncContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        match Connection::get_private(BusType::Session) {
            Ok(c) => {
                self.conn = Some(c);
            }
            Err(e) => {
                error!("failed to connect to DBus: {}", e);
                ctx.terminate();
            }
        };
        trace!("dbus client started");
    }
}

/// DBus request: stage an rpm-ostree deployment.
pub(crate) struct StageDeployment {
    pub(crate) release: libcincinnati::Release,
}

impl Message for StageDeployment {
    type Result = Fallible<libcincinnati::Release>;
}

impl Handler<StageDeployment> for DbusClient {
    type Result = Fallible<libcincinnati::Release>;

    fn handle(&mut self, msg: StageDeployment, _ctx: &mut Self::Context) -> Self::Result {
        // TODO(lucab): implement real call to rpm-ostree.
        // https://github.com/projectatomic/rpm-ostree/issues/1748
        let call = dbus::Message::new_method_call(
            "org.freedesktop.DBus",
            "/",
            "org.freedesktop.DBus",
            "ListNames",
        )
        .map_err(|e| format_err!("{}", e))?;
        let r = self
            .conn
            .as_ref()
            .unwrap()
            .send_with_reply_and_block(call, 2000)
            .unwrap();
        let arr: Array<&str, _> = r.get1().unwrap();
        for name in arr {
            if name.starts_with("org.") {
                debug!("dbus result: {}", name);
                break;
            }
        }

        warn!("rpm-ostree stage: stubbed");
        Ok(msg.release)
    }
}

/// DBus request: stage an rpm-ostree deployment.
pub(crate) struct FinalizeDeployment {
    pub(crate) release: libcincinnati::Release,
}

impl Message for FinalizeDeployment {
    type Result = Fallible<libcincinnati::Release>;
}

impl Handler<FinalizeDeployment> for DbusClient {
    type Result = Fallible<libcincinnati::Release>;

    fn handle(&mut self, msg: FinalizeDeployment, _ctx: &mut Self::Context) -> Self::Result {
        // TODO(lucab): implement real call to rpm-ostree.
        // https://github.com/projectatomic/rpm-ostree/issues/1748
        let call = dbus::Message::new_method_call(
            "org.freedesktop.DBus",
            "/",
            "org.freedesktop.DBus",
            "ListNames",
        )
        .map_err(|e| format_err!("{}", e))?;
        let r = self
            .conn
            .as_ref()
            .unwrap()
            .send_with_reply_and_block(call, 2000)
            .unwrap();
        let arr: Array<&str, _> = r.get1().unwrap();
        for name in arr {
            if name.starts_with("com.") {
                debug!("dbus result: {}", name);
                break;
            }
        }

        warn!("rpm-ostree finalize: stubbed");
        Ok(msg.release)
    }
}
