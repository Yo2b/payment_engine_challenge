//! A module providing transaction processing features.

use std::collections::HashMap;

use futures::{stream, Stream, StreamExt, TryFutureExt, TryStreamExt};
use thiserror::Error;

use crate::{Account, AccountStatus, Amount, ClientID, Result, Transaction, TransactionID, TransactionType};

const DEFAULT_TRANSACTION_CAPACITY: usize = 10_000;
const MAX_TRANSACTION_CAPACITY: usize = 1_000_000;
const ROLLOUT_TRANSACTION_THRESHOLD: usize = 1_000;

/// A transaction process error.
#[derive(Debug, Error)]
pub enum Error {
    #[error("missing amount in transaction '{0}'")]
    MissingAmount(TransactionID),
    #[error("transaction '{0}' already exists")]
    TransactionAlreadyExists(TransactionID),
    #[error("transaction '{0}' does not exist")]
    TransactionNotFound(TransactionID),
    #[error("operation not supported in transaction '{0}' ({1:?} -> {2:?})")]
    OperationNotSupported(TransactionID, Option<TransactionType>, TransactionType),
    #[error("too much funds to operate transaction '{0}' for client '{1}'")]
    TooManyFunds(TransactionID, ClientID),
    #[error("not enough funds to operate transaction '{0}' for client '{1}'")]
    NotEnoughFunds(TransactionID, ClientID),
    #[error("account locked, cannot operate transaction '{0}' for client '{1}'")]
    AccountLocked(TransactionID, ClientID),
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
#[derive(Debug)]
pub struct Processor {
    accounts: HashMap<ClientID, AccountStatus>,
    transactions: HashMap<TransactionID, TransactionStatus>,
}

impl Default for Processor {
    fn default() -> Self {
        Self {
            accounts: HashMap::default(),
            transactions: HashMap::with_capacity(DEFAULT_TRANSACTION_CAPACITY),
        }
    }
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
            return Err(Error::AccountLocked(transaction.tx, transaction.client));
        }

        match transaction.r#type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                Self::register_transaction(&mut self.transactions, transaction, account_status)?;
            }
            t => Self::dispute_transaction(&mut self.transactions, transaction.tx, t, account_status)?,
        }

        Ok(())
    }

    /// Manage a new transaction.
    fn register_transaction(
        transactions: &mut HashMap<TransactionID, TransactionStatus>,
        transaction: Transaction,
        account_status: &mut AccountStatus,
    ) -> Result<(), Error> {
        if transactions.contains_key(&transaction.tx) {
            return Err(Error::TransactionAlreadyExists(transaction.tx));
        }

        let (t, amount) = match transaction.r#type {
            t @ TransactionType::Deposit => {
                let amount = transaction.amount.ok_or(Error::MissingAmount(transaction.tx))?;
                if Amount::MAX - account_status.available < amount {
                    return Err(Error::TooManyFunds(transaction.tx, transaction.client));
                }

                account_status.available += amount;

                (t, amount)
            }
            t @ TransactionType::Withdrawal => {
                let amount = transaction.amount.ok_or(Error::MissingAmount(transaction.tx))?;
                if account_status.available < amount {
                    return Err(Error::NotEnoughFunds(transaction.tx, transaction.client));
                }

                account_status.available -= amount;

                (t, amount)
            }
            t => return Err(Error::OperationNotSupported(transaction.tx, None, t)),
        };

        Self::rollout_transactions(transactions, ROLLOUT_TRANSACTION_THRESHOLD, MAX_TRANSACTION_CAPACITY);

        transactions.insert(transaction.tx, TransactionStatus(t, amount));

        Ok(())
    }

    /// Manage a transaction dispute.
    fn dispute_transaction(
        transactions: &mut HashMap<TransactionID, TransactionStatus>,
        transaction_id: TransactionID,
        transaction_type: TransactionType,
        account_status: &mut AccountStatus,
    ) -> Result<(), Error> {
        let (t, amount) = match transactions.get_mut(&transaction_id) {
            Some(transaction_status) => transaction_status.as_mut(),
            None => return Err(Error::TransactionNotFound(transaction_id)),
        };

        match transaction_type {
            TransactionType::Dispute if matches!(t, TransactionType::Withdrawal) => account_status.hold(amount),
            TransactionType::Resolve if matches!(t, TransactionType::Dispute) => account_status.release(amount),
            TransactionType::Chargeback if matches!(t, TransactionType::Dispute) => account_status.lock(amount),
            _ => return Err(Error::OperationNotSupported(transaction_id, Some(*t), transaction_type)),
        }

        *t = transaction_type;

        Ok(())
    }

    /// Make room for incoming transactions, rolling out old transactions.
    ///
    /// It is guaranteed that room has been made for at least one future transaction wrt. expected `max_capacity`.
    ///
    /// # Panics
    /// This function will panic when called with a `max_capacity` equal to `0`.
    fn rollout_transactions(transactions: &mut HashMap<TransactionID, TransactionStatus>, rollout_threshold: usize, max_capacity: usize) {
        assert!(max_capacity > 0);

        if transactions.len() >= rollout_threshold {
            // ideal case: roll out all ended disputes
            transactions
                .retain(|_, TransactionStatus(status, _)| !matches!(status, TransactionType::Resolve | TransactionType::Chargeback));
        }
        while transactions.len() >= max_capacity {
            // worst case: got no ended dispute, make room for only one entry, presuming arbitrarily the min. transaction ID could be old enough
            let tx = *transactions.keys().min().unwrap();
            let transaction_status = transactions.remove(&tx).unwrap();

            tracing::warn!("Transaction dropped: '{tx}' ({transaction_status:?}).");
        }
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

        println!(
            "Default reserved min. size: {} bytes",
            DEFAULT_TRANSACTION_CAPACITY * (size_of::<TransactionID>() + size_of::<TransactionStatus>())
        );

        println!(
            "Max. size: {} bytes",
            MAX_TRANSACTION_CAPACITY * (size_of::<TransactionID>() + size_of::<TransactionStatus>())
        );
    }

    const DEPOSIT: Amount = Amount::raw(50000);
    const WITHDRAWAL: Amount = Amount::raw(20000);

    #[test]
    fn test_rollout_transactions() {
        let mut transactions = HashMap::from_iter([
            (1, TransactionStatus(TransactionType::Deposit, Amount::MIN)),
            (2, TransactionStatus(TransactionType::Withdrawal, Amount::MIN)),
            (3, TransactionStatus(TransactionType::Dispute, Amount::MIN)),
            (4, TransactionStatus(TransactionType::Resolve, Amount::MIN)),
            (5, TransactionStatus(TransactionType::Chargeback, Amount::MIN)),
        ]);

        Processor::rollout_transactions(&mut transactions, 6, 6);
        assert!(transactions.len() == 5);

        Processor::rollout_transactions(&mut transactions, 5, 6);
        assert!(transactions.len() == 3 && [1, 2, 3].iter().all(|id| transactions.contains_key(id)));

        Processor::rollout_transactions(&mut transactions, 0, 6);
        assert!(transactions.len() == 3);

        Processor::rollout_transactions(&mut transactions, 0, 3);
        assert!(transactions.len() == 2 && !transactions.contains_key(&1));

        Processor::rollout_transactions(&mut transactions, 0, 1);
        assert!(transactions.is_empty());
    }

    #[test]
    fn test_register_transaction() {
        let mut transactions = HashMap::default();
        let mut account_status = AccountStatus::default();

        let transaction = Transaction::deposit(1, DEPOSIT);
        Processor::register_transaction(&mut transactions, transaction, &mut account_status).unwrap();
        assert_eq!(account_status, AccountStatus::from(DEPOSIT));

        let transaction = Transaction::withdrawal(2, WITHDRAWAL);
        Processor::register_transaction(&mut transactions, transaction, &mut account_status).unwrap();
        assert_eq!(account_status, AccountStatus::from(DEPOSIT - WITHDRAWAL));

        let ref_account_status = account_status.clone();

        // Test: existing transaction
        let transaction = Transaction::deposit(2, Default::default());
        let err = Processor::register_transaction(&mut transactions, transaction, &mut account_status).unwrap_err();
        assert_matches!(err, Error::TransactionAlreadyExists(2));
        assert_eq!(account_status, ref_account_status);

        // Test: register anything else than `Deposit` or `Withdrawal`
        for transaction_type in [TransactionType::Dispute, TransactionType::Resolve, TransactionType::Chargeback] {
            let transaction = Transaction::new(transaction_type, 3, Default::default());
            let err = Processor::register_transaction(&mut transactions, transaction, &mut account_status).unwrap_err();
            assert_matches!(err, Error::OperationNotSupported(3, None, t) if t == transaction_type);
            assert_eq!(account_status, ref_account_status);
        }
    }

    fn assert_dispute_not_supported(
        transaction_id: TransactionID,
        transaction_types: &[TransactionType],
        transactions: &mut HashMap<TransactionID, TransactionStatus>,
        account_status: &mut AccountStatus,
    ) {
        let not_supported = [TransactionType::Deposit, TransactionType::Withdrawal];

        let ref_account_status = account_status.clone();

        for transaction_type in not_supported.iter().chain(transaction_types) {
            let err = Processor::dispute_transaction(transactions, transaction_id, *transaction_type, account_status).unwrap_err();
            assert_matches!(err, Error::OperationNotSupported(id, Some(_), t) if id == transaction_id && t == *transaction_type);
            assert_eq!(*account_status, ref_account_status);
        }
    }

    #[test]
    fn test_dispute_transaction_failure() {
        let mut transactions = HashMap::from_iter([
            (1, TransactionStatus(TransactionType::Deposit, DEPOSIT)),
            (2, TransactionStatus(TransactionType::Withdrawal, WITHDRAWAL)),
            (3, TransactionStatus(TransactionType::Dispute, WITHDRAWAL)),
        ]);
        let mut account_status = AccountStatus::from(DEPOSIT - WITHDRAWAL);

        // Test: dispute a `Deposit`
        assert_dispute_not_supported(
            1,
            &[TransactionType::Dispute, TransactionType::Resolve, TransactionType::Chargeback],
            &mut transactions,
            &mut account_status,
        );

        // Test: dispute a `Withdrawal`
        assert_dispute_not_supported(
            2,
            &[TransactionType::Resolve, TransactionType::Chargeback],
            &mut transactions,
            &mut account_status,
        );

        // Test: dispute a `Dispute`
        assert_dispute_not_supported(3, &[TransactionType::Dispute], &mut transactions, &mut account_status);

        // Test: not existing transaction
        let err = Processor::dispute_transaction(&mut transactions, 42, TransactionType::Deposit, &mut account_status).unwrap_err();
        assert_matches!(err, Error::TransactionNotFound(42));
        assert_eq!(account_status, AccountStatus::from(DEPOSIT - WITHDRAWAL));
    }

    #[test]
    fn test_dispute_transaction_resolve() {
        let mut transactions = HashMap::from_iter([
            (1, TransactionStatus(TransactionType::Deposit, DEPOSIT)),
            (2, TransactionStatus(TransactionType::Withdrawal, WITHDRAWAL)),
        ]);
        let mut account_status = AccountStatus::from(DEPOSIT - WITHDRAWAL);

        Processor::dispute_transaction(&mut transactions, 2, TransactionType::Dispute, &mut account_status).unwrap();
        assert_eq!(account_status, AccountStatus::from(DEPOSIT - WITHDRAWAL).held(WITHDRAWAL));

        Processor::dispute_transaction(&mut transactions, 2, TransactionType::Resolve, &mut account_status).unwrap();
        assert_eq!(account_status, AccountStatus::from(DEPOSIT));

        assert_dispute_not_supported(
            2,
            &[TransactionType::Dispute, TransactionType::Chargeback],
            &mut transactions,
            &mut account_status,
        );
    }

    #[test]
    fn test_dispute_transaction_chargeback() {
        let mut transactions = HashMap::from_iter([
            (1, TransactionStatus(TransactionType::Deposit, DEPOSIT)),
            (2, TransactionStatus(TransactionType::Withdrawal, WITHDRAWAL)),
        ]);
        let mut account_status = AccountStatus::from(DEPOSIT - WITHDRAWAL);

        Processor::dispute_transaction(&mut transactions, 2, TransactionType::Dispute, &mut account_status).unwrap();
        assert_eq!(account_status, AccountStatus::from(DEPOSIT - WITHDRAWAL).held(WITHDRAWAL));

        Processor::dispute_transaction(&mut transactions, 2, TransactionType::Chargeback, &mut account_status).unwrap();
        assert_eq!(account_status, AccountStatus::from(DEPOSIT - WITHDRAWAL).locked());

        assert_dispute_not_supported(
            2,
            &[TransactionType::Dispute, TransactionType::Resolve, TransactionType::Chargeback],
            &mut transactions,
            &mut account_status,
        );
    }

    #[test]
    fn test_process_transaction() {
        let mut processor = Processor::default();

        processor.process_transaction(Transaction::deposit(1, DEPOSIT)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus::from(DEPOSIT));

        assert_matches!(
            processor.process_transaction(Transaction::deposit(1, DEPOSIT)),
            Err(Error::TransactionAlreadyExists(1))
        );

        assert_matches!(
            processor.process_transaction(Transaction::dispute(42)),
            Err(Error::TransactionNotFound(42))
        );

        processor.process_transaction(Transaction::withdrawal(2, WITHDRAWAL)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus::from(DEPOSIT - WITHDRAWAL));

        processor.process_transaction(Transaction::dispute(2)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus::from(DEPOSIT - WITHDRAWAL).held(WITHDRAWAL));

        processor.process_transaction(Transaction::resolve(2)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus::from(DEPOSIT));

        processor.process_transaction(Transaction::withdrawal(3, WITHDRAWAL)).unwrap();

        processor.process_transaction(Transaction::dispute(3)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus::from(DEPOSIT - WITHDRAWAL).held(WITHDRAWAL));

        processor.process_transaction(Transaction::chargeback(3)).unwrap();
        assert_eq!(processor.accounts[&0], AccountStatus::from(DEPOSIT - WITHDRAWAL).locked());

        for t in [
            TransactionType::Deposit,
            TransactionType::Withdrawal,
            TransactionType::Dispute,
            TransactionType::Resolve,
            TransactionType::Chargeback,
        ] {
            assert_matches!(
                processor.process_transaction(Transaction::new(t, 4, None)),
                Err(Error::AccountLocked(4, 0))
            );
        }
    }
}
