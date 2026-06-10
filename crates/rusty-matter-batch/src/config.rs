use std::num::NonZeroUsize;

/// Execution backend selected for a batch executor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BatchBackendKind {
    /// Deterministic single-threaded execution.
    Serial,
    /// Rayon-backed execution with deterministic logical chunk reduction.
    #[cfg(feature = "rayon")]
    Rayon,
}

/// Batch execution configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchConfig {
    /// Selected backend.
    pub backend: BatchBackendKind,
    /// Maximum element count in one logical chunk.
    pub batch_size: NonZeroUsize,
    /// Optional worker cap for future parallel backends.
    pub max_threads: Option<usize>,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            backend: BatchBackendKind::Serial,
            batch_size: NonZeroUsize::new(256).expect("default batch size is non-zero"),
            max_threads: None,
        }
    }
}
