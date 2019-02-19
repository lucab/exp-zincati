use failure::Error;
use futures::future;
use futures::prelude::*;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StratImmediate {}

impl StratImmediate {
    pub(crate) fn has_green_light(self) -> Box<Future<Item = bool, Error = Error>> {
        trace!("green_light check, strategy 'immediate'");

        let immediate = future::ok(true);
        Box::new(immediate)
    }

    pub(crate) fn report_steady(self) -> Box<Future<Item = bool, Error = Error>> {
        trace!("finalizer report steady, strategy 'immediate'");

        let immediate = future::ok(true);
        Box::new(immediate)
    }
}
