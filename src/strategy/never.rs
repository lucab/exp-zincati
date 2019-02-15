use failure::Error;
use futures::future;
use futures::prelude::*;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StratNever {}

impl StratNever {
    pub(crate) fn has_green_light(self) -> Box<Future<Item = bool, Error = Error>> {
        trace!("finalizer check, strategy 'never'");

        let never = future::ok(false);
        Box::new(never)
    }

    pub(crate) fn report_steady(self) -> Box<Future<Item = bool, Error = Error>> {
        trace!("finalizer report steady, strategy 'never'");

        let never = future::ok(false);
        Box::new(never)
    }
}
