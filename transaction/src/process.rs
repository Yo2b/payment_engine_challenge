//! A module providing transaction processing features.

use std::collections::{hash_map::Entry, HashMap};

use futures::{stream, Stream, StreamExt, TryFutureExt, TryStreamExt};
use thiserror::Error;

use crate::{Account, AccountStatus, Amount, ClientID, Result, Transaction, TransactionID, TransactionType};

/// A transaction process error.
#[derive(Debug, Error)]
pub enum Error {
    #[error("missing amount")]
    MissingAmount,
    #[error("transaction already exists")]
    TransactionAlreadyExists,
    #[error("transaction does not exist")]
    TransactionNotFound,
    #[error("operation not supported")]
    OperationNotSupported,
    #[error("too much funds")]
    TooManyFunds,
    #[error("not enough funds")]
    NotEnoughFunds,
    #[error("account locked")]
    AccountLocked,
}

/// A transaction process status.
#[derive(Debug)]
struct TransactionStatus(TransactionType, Amount);

impl TransactionStatus {
    fn as_mut(&mut self) -> (&mut TransactionType, Amount) {
        (&mut self.0, self.1)
    }
}

/// A transaction processor.
#[derive(Debug, Default)]
pub struct Processor {
    accounts: HashMap<ClientID, AccountStatus>,
    transactions: HashMap<TransactionID, TransactionStatus>,
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

        if account_status.locked {
            return Err(Error::AccountLocked);
        }

        match transaction.r#type {
            t @ TransactionType::Deposit => {
                let amount = transaction.amount.ok_or(Error::MissingAmount)?;
                if Amount::MAX - account_status.available < amount {
                    return Err(Error::TooManyFunds);
                }

                match self.transactions.entry(transaction.tx) {
                    Entry::Occupied(..) => return Err(Error::TransactionAlreadyExists),
                    vacant => {
                        vacant.insert_entry(TransactionStatus(t, amount));
                    }
                }

                account_status.available += amount;
            }
            t @ TransactionType::Withdrawal => {
                let amount = transaction.amount.ok_or(Error::MissingAmount)?;
                if account_status.available < amount {
                    return Err(Error::NotEnoughFunds);
                }

                match self.transactions.entry(transaction.tx) {
                    Entry::Occupied(..) => return Err(Error::TransactionAlreadyExists),
                    vacant => {
                        vacant.insert_entry(TransactionStatus(t, amount));
                    }
                }

                account_status.available -= amount;
            }
            TransactionType::Dispute => match self.transactions.get_mut(&transaction.tx).map(TransactionStatus::as_mut) {
                Some((t, amount)) if matches!(t, TransactionType::Withdrawal) => {
                    // account_status.available -= amount; // challenge wording error
                    account_status.held += amount;

                    *t = TransactionType::Dispute;
                }
                Some(_) => return Err(Error::OperationNotSupported),
                None => return Err(Error::TransactionNotFound),
            },
            TransactionType::Resolve => match self.transactions.get_mut(&transaction.tx).map(TransactionStatus::as_mut) {
                Some((t, amount)) if matches!(t, TransactionType::Dispute) => {
                    account_status.available += amount;
                    account_status.held -= amount;

                    *t = TransactionType::Resolve;
                }
                Some(_) => return Err(Error::OperationNotSupported),
                None => return Err(Error::TransactionNotFound),
            },
            TransactionType::Chargeback => match self.transactions.get_mut(&transaction.tx).map(TransactionStatus::as_mut) {
                Some((t, amount)) if matches!(t, TransactionType::Dispute) => {
                    // account_status.available -= amount;
                    account_status.held -= amount;
                    account_status.locked = true;

                    *t = TransactionType::Chargeback;
                }
                Some(_) => return Err(Error::OperationNotSupported),
                None => return Err(Error::TransactionNotFound),
            },
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use assert_matches::assert_matches;

    #[test]
    #[ignore = "not an actual test"]
    fn test_size_of() {
        fn print_size_of<T>() {
            println!("{}: {}", std::any::type_name::<T>(), size_of::<T>());
        }

        print_size_of::<Error>();
        print_size_of::<AccountStatus>();
        print_size_of::<TransactionStatus>();
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

        let err = processor.process_transaction(Transaction::dispute(2)).unwrap_err();
        assert_matches!(err, Error::TransactionNotFound);

        processor.process_transaction(Transaction::withdrawal(2, withdrawal)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus { available: deposit - withdrawal, held: 0.0, locked: false });

        processor.process_transaction(Transaction::dispute(2)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus { available: deposit - withdrawal, held: withdrawal, locked: false });

        processor.process_transaction(Transaction::resolve(2)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus { available: deposit, held: 0.0, locked: false });

        processor.process_transaction(Transaction::withdrawal(3, withdrawal)).unwrap();

        processor.process_transaction(Transaction::dispute(3)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus { available: deposit - withdrawal, held: withdrawal, locked: false });

        processor.process_transaction(Transaction::chargeback(3)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus { available: deposit - withdrawal, held: 0.0, locked: true });

        for t in [
            TransactionType::Deposit,
            TransactionType::Withdrawal,
            TransactionType::Dispute,
            TransactionType::Resolve,
            TransactionType::Chargeback,
        ] {
            let err = processor.process_transaction(Transaction::new(t, 4, None)).unwrap_err();
            assert_matches!(err, Error::AccountLocked);
        }
    }
}
