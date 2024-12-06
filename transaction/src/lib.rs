//! A simple crate providing transaction features.

use serde::{Deserialize, Serialize};

mod error;
pub use error::{Error, Result};

mod process;
pub use process::Processor;

pub mod io;
pub mod num;

/// Decimal precision used for transaction amounts.
const PREC: u8 = 4;

/// Convenient alias for a client ID.
pub type ClientID = u16;
/// Convenient alias for a transaction ID.
pub type TransactionID = u32;
/// Convenient alias for a transaction amount.
pub type Amount = num::Decimal<PREC>;

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

/// A client's account status.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct AccountStatus {
    /// Available funds for this account.
    pub available: Amount,
    /// Held funds for this account, ie. disputed amounts.
    pub held: Amount,
    /// An account can be locked/frozen if a transaction has been charged back.
    pub locked: bool,
}

impl AccountStatus {
    /// Set held funds for this account status.
    #[inline]
    pub fn held(self, held: Amount) -> Self {
        Self { held, ..self }
    }
    /// Set this account status as locked.
    #[inline]
    pub fn locked(self) -> Self {
        Self { locked: true, ..self }
    }
    /// Compute total funds for this account status.
    #[inline]
    pub fn total(&self) -> Amount {
        self.available + self.held
    }
}

impl From<Amount> for AccountStatus {
    /// Create a new account status with available funds.
    #[inline]
    fn from(available: Amount) -> Self {
        Self {
            available,
            ..Default::default()
        }
    }
}

/// A client's account.
#[derive(Clone, Debug, Serialize)]
#[serde(into = "AccountRecord")]
pub struct Account {
    client: ClientID,
    status: AccountStatus,
}

impl From<(ClientID, AccountStatus)> for Account {
    #[inline]
    fn from((client, status): (ClientID, AccountStatus)) -> Self {
        Self { client, status }
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
            available: account.status.available,
            held: account.status.held,
            total: account.status.total(),
            locked: account.status.locked,
        }
    }
}
