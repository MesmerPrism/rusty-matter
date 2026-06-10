use std::num::NonZeroUsize;

use crate::{build_chunks, BatchBackendKind, BatchConfig, BatchError, BatchExecutor, BatchReduce};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct OrderDiagnostics {
    visited: Vec<usize>,
    sum: usize,
}

impl BatchReduce for OrderDiagnostics {
    fn reduce(&mut self, other: Self) {
        self.visited.extend(other.visited);
        self.sum += other.sum;
    }
}

fn config(batch_size: usize) -> BatchConfig {
    BatchConfig {
        backend: BatchBackendKind::Serial,
        batch_size: NonZeroUsize::new(batch_size).expect("test batch size is non-zero"),
        max_threads: None,
    }
}

#[test]
fn chunk_builder_handles_empty_length() {
    assert_eq!(build_chunks(0, &config(4)), Vec::new());
}

#[test]
fn chunk_builder_handles_short_length() {
    assert_eq!(
        build_chunks(3, &config(4)),
        vec![crate::BatchChunk {
            index: 0,
            range: 0..3
        }]
    );
}

#[test]
fn chunk_builder_handles_exact_batch() {
    assert_eq!(
        build_chunks(4, &config(4)),
        vec![crate::BatchChunk {
            index: 0,
            range: 0..4
        }]
    );
}

#[test]
fn chunk_builder_handles_non_divisible_lengths() {
    assert_eq!(
        build_chunks(10, &config(4)),
        vec![
            crate::BatchChunk {
                index: 0,
                range: 0..4
            },
            crate::BatchChunk {
                index: 1,
                range: 4..8
            },
            crate::BatchChunk {
                index: 2,
                range: 8..10
            },
        ]
    );
}

#[test]
fn chunk_builder_handles_unit_batch_size() {
    assert_eq!(
        build_chunks(3, &config(1)),
        vec![
            crate::BatchChunk {
                index: 0,
                range: 0..1
            },
            crate::BatchChunk {
                index: 1,
                range: 1..2
            },
            crate::BatchChunk {
                index: 2,
                range: 2..3
            },
        ]
    );
}

#[test]
fn chunk_builder_handles_batch_size_equal_to_len() {
    assert_eq!(
        build_chunks(5, &config(5)),
        vec![crate::BatchChunk {
            index: 0,
            range: 0..5
        }]
    );
}

#[test]
fn serial_executor_reduces_in_chunk_order() {
    let executor = BatchExecutor::new(config(3)).expect("executor builds");
    let report = executor.run_chunks(10, |chunk| OrderDiagnostics {
        visited: vec![chunk.index],
        sum: chunk.range.sum(),
    });

    assert_eq!(report.backend, BatchBackendKind::Serial);
    assert_eq!(report.len, 10);
    assert_eq!(report.batch_size, 3);
    assert_eq!(report.chunk_count, 4);
    assert_eq!(report.worker_count, 1);
    assert_eq!(report.diagnostics.visited, vec![0, 1, 2, 3]);
    assert_eq!(report.diagnostics.sum, (0..10).sum());
}

#[test]
fn serial_executor_writes_mutable_slice_chunks() {
    let executor = BatchExecutor::new(config(2)).expect("executor builds");
    let mut output = vec![0usize; 5];
    let report = executor.run_slice_chunks(&mut output, |chunk, values| {
        for (offset, value) in values.iter_mut().enumerate() {
            *value = chunk.range.start + offset;
        }
        OrderDiagnostics {
            visited: vec![chunk.index],
            sum: values.iter().sum(),
        }
    });

    assert_eq!(output, vec![0, 1, 2, 3, 4]);
    assert_eq!(report.chunk_count, 3);
    assert_eq!(report.diagnostics.visited, vec![0, 1, 2]);
    assert_eq!(report.diagnostics.sum, (0..5).sum());
}

#[test]
fn executor_rejects_zero_worker_cap() {
    let error = BatchExecutor::new(BatchConfig {
        backend: BatchBackendKind::Serial,
        batch_size: NonZeroUsize::new(8).expect("batch size is non-zero"),
        max_threads: Some(0),
    })
    .unwrap_err();

    assert_eq!(error, BatchError::InvalidMaxThreads);
}
