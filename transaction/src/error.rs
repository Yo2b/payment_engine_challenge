//! A module providing transaction error features.

use thiserror::Error;

/// A crate error.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Csv(#[from] csv_async::Error),
    // #[error(transparent)]
    // Process(#[from] crate::process::Error),
}

/// Convenient alias for a crate result.
pub type Result<T, E = Error> = std::result::Result<T, E>;
