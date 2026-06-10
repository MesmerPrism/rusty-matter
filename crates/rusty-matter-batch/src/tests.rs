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
    serial_config(batch_size)
}

fn serial_config(batch_size: usize) -> BatchConfig {
    BatchConfig {
        backend: BatchBackendKind::Serial,
        batch_size: NonZeroUsize::new(batch_size).expect("test batch size is non-zero"),
        max_threads: None,
    }
}

#[cfg(feature = "rayon")]
fn rayon_config(batch_size: usize, max_threads: Option<usize>) -> BatchConfig {
    BatchConfig {
        backend: BatchBackendKind::Rayon,
        batch_size: NonZeroUsize::new(batch_size).expect("test batch size is non-zero"),
        max_threads,
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
fn slice_chunks_cover_each_index_once_for_varied_lengths() {
    for len in [0, 1, 2, 3, 7, 31, 32, 33, 255, 256, 257, 1024] {
        for batch_size in [1, 2, 3, 7, 16, 64, 256] {
            let executor = BatchExecutor::new(serial_config(batch_size)).expect("executor builds");
            let mut output = vec![usize::MAX; len];
            let report = executor.run_slice_chunks(&mut output, |chunk, values| {
                for (offset, value) in values.iter_mut().enumerate() {
                    *value = chunk.range.start + offset;
                }
                OrderDiagnostics {
                    visited: vec![chunk.index],
                    sum: values.iter().sum(),
                }
            });

            assert_eq!(output, (0..len).collect::<Vec<_>>());
            assert_eq!(report.diagnostics.sum, (0..len).sum());
            assert_eq!(report.chunk_count, len.div_ceil(batch_size));
        }
    }
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

#[cfg(feature = "rayon")]
#[test]
fn rayon_executor_matches_serial_diagnostics() {
    let serial = BatchExecutor::new(serial_config(3)).expect("serial executor builds");
    let rayon = BatchExecutor::new(rayon_config(3, Some(2))).expect("rayon executor builds");

    let serial_report = serial.run_chunks(37, |chunk| OrderDiagnostics {
        visited: vec![chunk.index],
        sum: chunk.range.sum(),
    });
    let rayon_report = rayon.run_chunks(37, |chunk| OrderDiagnostics {
        visited: vec![chunk.index],
        sum: chunk.range.sum(),
    });

    assert_eq!(rayon_report.backend, BatchBackendKind::Rayon);
    assert_eq!(rayon_report.worker_count, 2);
    assert_eq!(rayon_report.chunk_count, serial_report.chunk_count);
    assert_eq!(rayon_report.diagnostics, serial_report.diagnostics);
}

#[cfg(feature = "rayon")]
#[test]
fn rayon_slice_chunks_match_serial_output() {
    let serial = BatchExecutor::new(serial_config(4)).expect("serial executor builds");
    let rayon = BatchExecutor::new(rayon_config(4, Some(2))).expect("rayon executor builds");
    let mut serial_output = vec![0usize; 31];
    let mut rayon_output = vec![0usize; 31];

    let serial_report = serial.run_slice_chunks(&mut serial_output, |chunk, values| {
        for (offset, value) in values.iter_mut().enumerate() {
            *value = (chunk.range.start + offset) * 3;
        }
        OrderDiagnostics {
            visited: vec![chunk.index],
            sum: values.iter().sum(),
        }
    });
    let rayon_report = rayon.run_slice_chunks(&mut rayon_output, |chunk, values| {
        for (offset, value) in values.iter_mut().enumerate() {
            *value = (chunk.range.start + offset) * 3;
        }
        OrderDiagnostics {
            visited: vec![chunk.index],
            sum: values.iter().sum(),
        }
    });

    assert_eq!(rayon_output, serial_output);
    assert_eq!(rayon_report.chunk_count, serial_report.chunk_count);
    assert_eq!(rayon_report.diagnostics, serial_report.diagnostics);
}

#[cfg(feature = "rayon")]
#[test]
fn rayon_slice_chunks_cover_each_index_once_for_varied_lengths() {
    for len in [0, 1, 2, 3, 7, 31, 32, 33, 255, 256, 257, 1024] {
        for batch_size in [1, 2, 3, 7, 16, 64, 256] {
            let serial =
                BatchExecutor::new(serial_config(batch_size)).expect("serial executor builds");
            let rayon = BatchExecutor::new(rayon_config(batch_size, Some(2)))
                .expect("rayon executor builds");
            let mut serial_output = vec![usize::MAX; len];
            let mut rayon_output = vec![usize::MAX; len];

            let serial_report = serial.run_slice_chunks(&mut serial_output, |chunk, values| {
                for (offset, value) in values.iter_mut().enumerate() {
                    *value = chunk.range.start + offset;
                }
                OrderDiagnostics {
                    visited: vec![chunk.index],
                    sum: values.iter().sum(),
                }
            });
            let rayon_report = rayon.run_slice_chunks(&mut rayon_output, |chunk, values| {
                for (offset, value) in values.iter_mut().enumerate() {
                    *value = chunk.range.start + offset;
                }
                OrderDiagnostics {
                    visited: vec![chunk.index],
                    sum: values.iter().sum(),
                }
            });

            assert_eq!(rayon_output, serial_output);
            assert_eq!(rayon_output, (0..len).collect::<Vec<_>>());
            assert_eq!(rayon_report.chunk_count, serial_report.chunk_count);
            assert_eq!(rayon_report.diagnostics, serial_report.diagnostics);
        }
    }
}

#[cfg(feature = "rayon")]
#[test]
fn rayon_integer_reduction_is_batch_size_invariant() {
    let expected_sum = (0..129).sum::<usize>();
    for batch_size in [1, 2, 7, 32, 129] {
        let executor =
            BatchExecutor::new(rayon_config(batch_size, Some(2))).expect("rayon executor builds");
        let report = executor.run_chunks(129, |chunk| OrderDiagnostics {
            visited: Vec::new(),
            sum: chunk.range.sum(),
        });

        assert_eq!(report.diagnostics.sum, expected_sum);
    }
}
