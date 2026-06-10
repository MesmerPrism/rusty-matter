use std::time::Instant;

use crate::{build_chunks, BatchBackendKind, BatchChunk, BatchConfig, BatchError, BatchReport};

/// Deterministic diagnostic reduction for per-chunk batch output.
pub trait BatchReduce: Default + Send {
    /// Merges another chunk diagnostic record into this accumulator.
    fn reduce(&mut self, other: Self);
}

/// Reusable Matter batch executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchExecutor {
    config: BatchConfig,
}

impl BatchExecutor {
    /// Creates a batch executor.
    ///
    /// # Errors
    ///
    /// Returns [`BatchError`] if the config is invalid.
    pub fn new(config: BatchConfig) -> Result<Self, BatchError> {
        if matches!(config.max_threads, Some(0)) {
            return Err(BatchError::InvalidMaxThreads);
        }
        Ok(Self { config })
    }

    /// Returns the executor config.
    #[must_use]
    pub const fn config(&self) -> &BatchConfig {
        &self.config
    }

    /// Runs one kernel per logical chunk and reduces diagnostics by chunk
    /// index.
    pub fn run_chunks<D, F>(&self, len: usize, kernel: F) -> BatchReport<D>
    where
        D: BatchReduce,
        F: Fn(BatchChunk) -> D + Sync + Send,
    {
        let started_at = Instant::now();
        let chunks = build_chunks(len, &self.config);
        let mut diagnostics = D::default();

        match self.config.backend {
            BatchBackendKind::Serial => {
                for chunk in chunks.iter().cloned() {
                    diagnostics.reduce(kernel(chunk));
                }
            }
        }

        BatchReport {
            backend: self.config.backend,
            len,
            batch_size: self.config.batch_size.get(),
            chunk_count: chunks.len(),
            worker_count: self.worker_count(),
            elapsed: started_at.elapsed(),
            diagnostics,
        }
    }

    /// Runs one kernel per mutable output chunk and reduces diagnostics by
    /// chunk index.
    pub fn run_slice_chunks<T, D, F>(&self, output: &mut [T], kernel: F) -> BatchReport<D>
    where
        T: Send,
        D: BatchReduce,
        F: Fn(BatchChunk, &mut [T]) -> D + Sync + Send,
    {
        let started_at = Instant::now();
        let len = output.len();
        let batch_size = self.config.batch_size.get();
        let mut diagnostics = D::default();
        let mut chunk_count = 0usize;

        match self.config.backend {
            BatchBackendKind::Serial => {
                for (index, output_chunk) in output.chunks_mut(batch_size).enumerate() {
                    let start = index * batch_size;
                    let end = start + output_chunk.len();
                    let chunk = BatchChunk {
                        index,
                        range: start..end,
                    };
                    diagnostics.reduce(kernel(chunk, output_chunk));
                    chunk_count += 1;
                }
            }
        }

        BatchReport {
            backend: self.config.backend,
            len,
            batch_size,
            chunk_count,
            worker_count: self.worker_count(),
            elapsed: started_at.elapsed(),
            diagnostics,
        }
    }

    fn worker_count(&self) -> usize {
        match self.config.backend {
            BatchBackendKind::Serial => 1,
        }
    }
}

impl Default for BatchExecutor {
    fn default() -> Self {
        Self::new(BatchConfig::default()).expect("default batch config is valid")
    }
}
