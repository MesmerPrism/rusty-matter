use std::time::Duration;

use crate::BatchBackendKind;

/// Summary of one batch execution.
#[derive(Clone, Debug, PartialEq)]
pub struct BatchReport<D> {
    /// Backend used for execution.
    pub backend: BatchBackendKind,
    /// Total number of elements covered by the batch.
    pub len: usize,
    /// Configured logical chunk size.
    pub batch_size: usize,
    /// Number of logical chunks executed.
    pub chunk_count: usize,
    /// Number of workers used by the backend.
    pub worker_count: usize,
    /// Wall-clock time spent inside the executor call.
    pub elapsed: Duration,
    /// Diagnostics reduced in deterministic chunk-index order.
    pub diagnostics: D,
}
