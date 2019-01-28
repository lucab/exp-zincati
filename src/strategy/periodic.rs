use failure::Error;
use futures::future;
use futures::prelude::*;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StratPeriodic {}

impl StratPeriodic {
    pub(crate) fn finalize(self) -> Box<Future<Item = bool, Error = Error>> {
        trace!("finalizer check, strategy 'immediate'");

        let immediate = future::ok(true);
        Box::new(immediate)
    }
}
