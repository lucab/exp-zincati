use failure::Error;
use futures::future;
use futures::prelude::*;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StratNever {}

impl StratNever {
    pub(crate) fn finalize(self) -> Box<Future<Item = bool, Error = Error>> {
        trace!("finalizer check, strategy 'never'");

        let immediate = future::ok(false);
        Box::new(immediate)
    }
}
