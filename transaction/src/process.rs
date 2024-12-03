//! A module providing transaction processing features.

use std::collections::HashMap;

use futures::{stream, Stream, StreamExt, TryFutureExt, TryStreamExt};
use thiserror::Error;

use crate::{Account, AccountStatus, Amount, ClientID, Result, Transaction, TransactionType};

/// A transaction process error.
#[derive(Debug, Error)]
pub enum Error {
    #[error("missing amount")]
    MissingAmount,
    #[error("too much funds")]
    TooManyFunds,
    #[error("not enough funds")]
    NotEnoughFunds,
}

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
                // processor.process_transaction(transaction)?;
                if let Err(err) = processor.process_transaction(transaction) {
                    tracing::error!("Transaction ignored: {err}.")
                }

                Ok(processor)
            })
            .map_ok(|processor| stream::iter(processor.accounts).map(Into::into).map(Ok))
            .try_flatten_stream()
    }

    /// Process a single transaction.
    pub fn process_transaction(&mut self, transaction: Transaction) -> Result<(), Error> {
        tracing::debug!("{transaction:?}");

        let account_status = self.accounts.entry(transaction.client).or_default();

        match transaction.r#type {
            TransactionType::Deposit => {
                let amount = transaction.amount.ok_or(Error::MissingAmount)?;
                if Amount::MAX - account_status.available < amount {
                    return Err(Error::TooManyFunds);
                }

                account_status.available += amount;
            }
            TransactionType::Withdrawal => {
                let amount = transaction.amount.ok_or(Error::MissingAmount)?;
                if account_status.available < amount {
                    return Err(Error::NotEnoughFunds);
                }

                account_status.available -= amount;
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not an actual test"]
    fn test_size_of() {
        fn print_size_of<T>() {
            println!("{}: {}", std::any::type_name::<T>(), size_of::<T>());
        }

        print_size_of::<Error>();
        print_size_of::<AccountStatus>();
        print_size_of::<TransactionType>();
    }

    #[test]
    #[rustfmt::skip]
    fn test_process_transaction() {
        let mut processor = Processor::default();

        let deposit = 5.0;
        let withdrawal = 2.0;

        processor.process_transaction(Transaction::deposit(1, deposit)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus { available: deposit, held: 0.0, locked: false });

        processor.process_transaction(Transaction::withdrawal(2, withdrawal)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus { available: deposit - withdrawal, held: 0.0, locked: false });
    }
}
