mod client;
mod blocking;

pub(crate) use client::RpmOstreeClient;
pub(crate) use client::{FinalizeUpdate, StageUpdate};

pub(crate) fn configure() -> failure::Fallible<()> {
    let client = RpmOstreeClient {
        pending: None,
        dbus_client: None,
    };
    let mut static_cfg = client::CONFIGURED.try_write().unwrap();
    *static_cfg = Some(client);
    Ok(())
}
