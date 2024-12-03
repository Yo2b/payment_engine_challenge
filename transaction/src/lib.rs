//! A simple crate providing transaction features.

mod error;
pub use error::{Error, Result};

mod process;
pub use process::Processor;

pub mod io;
