use std::{error::Error, fmt};

/// Errors returned while creating a batch executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchError {
    /// The configured worker cap is zero.
    InvalidMaxThreads,
}

impl fmt::Display for BatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMaxThreads => {
                formatter.write_str("batch max_threads must be absent or positive")
            }
        }
    }
}

impl Error for BatchError {}
