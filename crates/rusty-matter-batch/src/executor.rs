use std::time::Instant;

#[cfg(feature = "rayon")]
use std::sync::Arc;

use crate::{build_chunks, BatchBackendKind, BatchChunk, BatchConfig, BatchError, BatchReport};

#[cfg(feature = "rayon")]
use rayon::prelude::*;

/// Deterministic diagnostic reduction for per-chunk batch output.
pub trait BatchReduce: Default + Send {
    /// Merges another chunk diagnostic record into this accumulator.
    fn reduce(&mut self, other: Self);
}

/// Reusable Matter batch executor.
#[derive(Clone)]
pub struct BatchExecutor {
    config: BatchConfig,
    #[cfg(feature = "rayon")]
    rayon_pool: Option<Arc<rayon::ThreadPool>>,
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
        #[cfg(feature = "rayon")]
        let rayon_pool = if matches!(config.backend, BatchBackendKind::Rayon) {
            let mut builder = rayon::ThreadPoolBuilder::new();
            if let Some(max_threads) = config.max_threads {
                builder = builder.num_threads(max_threads);
            }
            Some(Arc::new(builder.build().map_err(|error| {
                BatchError::RayonPoolBuild(error.to_string())
            })?))
        } else {
            None
        };
        Ok(Self {
            config,
            #[cfg(feature = "rayon")]
            rayon_pool,
        })
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
            #[cfg(feature = "rayon")]
            BatchBackendKind::Rayon => {
                let pool = self.rayon_pool.as_ref().expect("rayon pool exists");
                let mut diagnostics_by_chunk = pool.install(|| {
                    chunks
                        .par_iter()
                        .map(|chunk| (chunk.index, kernel(chunk.clone())))
                        .collect::<Vec<_>>()
                });
                diagnostics_by_chunk.sort_by_key(|(chunk_index, _)| *chunk_index);
                for (_, chunk_diagnostics) in diagnostics_by_chunk {
                    diagnostics.reduce(chunk_diagnostics);
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
            #[cfg(feature = "rayon")]
            BatchBackendKind::Rayon => {
                let pool = self.rayon_pool.as_ref().expect("rayon pool exists");
                let mut diagnostics_by_chunk = pool.install(|| {
                    output
                        .par_chunks_mut(batch_size)
                        .enumerate()
                        .map(|(index, output_chunk)| {
                            let start = index * batch_size;
                            let end = start + output_chunk.len();
                            let chunk = BatchChunk {
                                index,
                                range: start..end,
                            };
                            (index, kernel(chunk, output_chunk))
                        })
                        .collect::<Vec<_>>()
                });
                chunk_count = diagnostics_by_chunk.len();
                diagnostics_by_chunk.sort_by_key(|(chunk_index, _)| *chunk_index);
                for (_, chunk_diagnostics) in diagnostics_by_chunk {
                    diagnostics.reduce(chunk_diagnostics);
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
            #[cfg(feature = "rayon")]
            BatchBackendKind::Rayon => self
                .rayon_pool
                .as_ref()
                .map_or(1, |pool| pool.current_num_threads()),
        }
    }
}

impl std::fmt::Debug for BatchExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BatchExecutor")
            .field("config", &self.config)
            .field("worker_count", &self.worker_count())
            .finish()
    }
}

impl Default for BatchExecutor {
    fn default() -> Self {
        Self::new(BatchConfig::default()).expect("default batch config is valid")
    }
}
