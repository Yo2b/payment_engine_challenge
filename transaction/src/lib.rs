//! A simple crate providing transaction features.

use serde::Deserialize;

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
