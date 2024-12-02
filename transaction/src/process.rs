//! A module providing transaction processing features.

use futures::stream::{Stream, TryStreamExt};

use crate::{ClientID, Result, Transaction};

/// A transaction processor.
#[derive(Debug)]
pub struct Processor {}

impl Processor {
    /// Process a stream of transactions on-the-fly.
    pub fn process(transactions: impl Stream<Item = Result<Transaction>>) -> impl Stream<Item = Result<ClientID>> {
        transactions.map_ok(|record| {
            tracing::debug!("{record:?}");
            record.client
        })
    }
}
