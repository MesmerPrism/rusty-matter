//! Native animated mesh surface runtime facade.
//!
//! This crate coordinates Matter-owned mesh distance, dynamic collider, SDF,
//! and particle primitives for native app adapters. It does not own renderer
//! policy, platform APIs, settings resolution, command routing, or browser
//! WebAssembly exports.

mod error;
mod runtime;

pub use error::*;
pub use runtime::*;
pub use rusty_matter_batch::{BatchBackendKind, BatchConfig, BatchExecutor};
pub use rusty_matter_particles::{ParticleExecutionBackend, ParticleExecutionConfig};

#[cfg(test)]
mod tests;
