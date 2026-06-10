//! Deterministic batch execution helpers for Matter CPU reference kernels.
//!
//! The serial backend is the default and dependency-free. Parallel backends
//! can be added behind explicit features without changing the logical chunk
//! contract or reduction order.

mod chunk;
mod config;
mod error;
mod executor;
mod report;

pub use chunk::{build_chunks, BatchChunk};
pub use config::{BatchBackendKind, BatchConfig};
pub use error::BatchError;
pub use executor::{BatchExecutor, BatchReduce};
pub use report::BatchReport;

#[cfg(test)]
mod tests;
