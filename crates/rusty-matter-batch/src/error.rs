use std::{error::Error, fmt};

/// Errors returned while creating a batch executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchError {
    /// The configured worker cap is zero.
    InvalidMaxThreads,
    /// The Rayon thread pool could not be created.
    #[cfg(feature = "rayon")]
    RayonPoolBuild(String),
}

impl fmt::Display for BatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMaxThreads => {
                formatter.write_str("batch max_threads must be absent or positive")
            }
            #[cfg(feature = "rayon")]
            Self::RayonPoolBuild(message) => {
                write!(
                    formatter,
                    "rayon batch thread pool could not be built: {message}"
                )
            }
        }
    }
}

impl Error for BatchError {}
