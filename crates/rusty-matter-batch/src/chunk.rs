use std::ops::Range;

use crate::BatchConfig;

/// One deterministic logical chunk of a batch execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchChunk {
    /// Zero-based logical chunk index.
    pub index: usize,
    /// Half-open element range processed by this chunk.
    pub range: Range<usize>,
}

/// Builds deterministic logical chunks for a batch length and config.
#[must_use]
pub fn build_chunks(len: usize, config: &BatchConfig) -> Vec<BatchChunk> {
    let batch_size = config.batch_size.get();
    let chunk_count = len.div_ceil(batch_size);
    let mut chunks = Vec::with_capacity(chunk_count);
    let mut start = 0usize;
    while start < len {
        let end = start.saturating_add(batch_size).min(len);
        chunks.push(BatchChunk {
            index: chunks.len(),
            range: start..end,
        });
        start = end;
    }
    chunks
}
