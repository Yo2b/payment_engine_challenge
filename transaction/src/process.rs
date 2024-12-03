//! A module providing transaction processing features.

use std::collections::HashMap;

use futures::{stream, Stream, StreamExt, TryFutureExt, TryStreamExt};

use crate::{Account, AccountStatus, ClientID, Result, Transaction};

/// A transaction processor.
#[derive(Debug, Default)]
pub struct Processor {
    accounts: HashMap<ClientID, AccountStatus>,
}

impl Processor {
    /// Process a stream of transactions on-the-fly.
    pub fn process(transactions: impl Stream<Item = Result<Transaction>>) -> impl Stream<Item = Result<Account>> {
        transactions
            .try_fold(Self::default(), |mut processor, transaction| async move {
                processor.process_transaction(transaction)?;

                Ok(processor)
            })
            .map_ok(|processor| stream::iter(processor.accounts).map(Into::into).map(Ok))
            .try_flatten_stream()
    }

    /// Process a single transaction.
    pub fn process_transaction(&mut self, transaction: Transaction) -> Result<()> {
        tracing::debug!("{transaction:?}");

        self.accounts.entry(transaction.client).or_default();

        Ok(())
    }
}
