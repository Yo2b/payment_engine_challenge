//! A module providing transaction processing features.

use futures::stream::{Stream, TryStreamExt};

use crate::Result;

/// A transaction processor.
#[derive(Debug)]
pub struct Processor {}

impl Processor {
    /// Process a stream of transactions on-the-fly.
    pub fn process(transactions: impl Stream<Item = Result<Vec<String>>>) -> impl Stream<Item = Result<Vec<String>>> {
        transactions.map_ok(|record| {
            tracing::debug!("{record:?}");
            record
        })
    }
}
