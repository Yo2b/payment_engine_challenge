//! A simple crate providing transaction features.

use serde::{Deserialize, Serialize};

mod error;
pub use error::{Error, Result};

mod process;
pub use process::Processor;

pub mod io;

/// Convenient alias for a client ID.
pub type ClientID = u16;
/// Convenient alias for a transaction ID.
pub type TransactionID = u32;
/// Convenient alias for a transaction amount.
pub type Amount = f32;

/// A transaction type.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// A transaction.
#[derive(Debug, Deserialize)]
pub struct Transaction {
    r#type: TransactionType,
    client: ClientID,
    tx: TransactionID,
    amount: Option<Amount>,
}

impl Transaction {
    /// Create a new transaction.
    #[inline]
    pub fn new(r#type: TransactionType, tx: TransactionID, amount: Option<Amount>) -> Self {
        Self {
            r#type,
            tx,
            amount,
            client: Default::default(),
        }
    }

    /// Build a transaction with its related client.
    #[inline]
    pub fn with_client(self, client: ClientID) -> Self {
        Self { client, ..self }
    }

    /// Convenient constructor for a `Deposit` transaction.
    #[inline]
    pub fn deposit(tx: TransactionID, amount: Amount) -> Self {
        Self::new(TransactionType::Deposit, tx, Some(amount))
    }

    /// Convenient constructor for a `Withdrawal` transaction.
    #[inline]
    pub fn withdrawal(tx: TransactionID, amount: Amount) -> Self {
        Self::new(TransactionType::Withdrawal, tx, Some(amount))
    }

    /// Convenient constructor for a `Dispute` transaction.
    #[inline]
    pub fn dispute(tx: TransactionID) -> Self {
        Self::new(TransactionType::Dispute, tx, None)
    }

    /// Convenient constructor for a `Resolve` transaction.
    #[inline]
    pub fn resolve(tx: TransactionID) -> Self {
        Self::new(TransactionType::Resolve, tx, None)
    }

    /// Convenient constructor for a `Chargeback` transaction.
    #[inline]
    pub fn chargeback(tx: TransactionID) -> Self {
        Self::new(TransactionType::Chargeback, tx, None)
    }
}

/// A client's account.
#[derive(Clone, Debug, Serialize)]
#[serde(into = "AccountRecord")]
pub struct Account {
    pub client: ClientID,
    /// Available funds for this account.
    pub available: Amount,
    /// Held funds for this account, ie. disputed amounts.
    pub held: Amount,
    /// An account can be locked/frozen if a transaction has been charged back.
    pub locked: bool,
}

impl Account {
    #[inline]
    fn new(client: ClientID) -> Self {
        Self {
            client,
            available: 0.0,
            held: 0.0,
            locked: false,
        }
    }

    /// Compute total funds for this account.
    #[inline]
    fn total(&self) -> Amount {
        self.available + self.held
    }
}

/// A helper to serialize a client's account record.
#[derive(Debug, Serialize)]
struct AccountRecord {
    client: ClientID,
    available: Amount,
    held: Amount,
    total: Amount,
    locked: bool,
}

impl From<Account> for AccountRecord {
    #[inline]
    fn from(account: Account) -> Self {
        Self {
            client: account.client,
            available: account.available,
            held: account.held,
            total: account.total(),
            locked: account.locked,
        }
    }
}
