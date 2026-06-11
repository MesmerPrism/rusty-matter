use core::fmt;

use rusty_matter_adf::AdfError;
use rusty_matter_batch::BatchError;
use rusty_matter_mesh::MatterMeshError;
use rusty_matter_particles::ParticleError;
use rusty_matter_sdf::SdfError;

/// Surface runtime orchestration failure.
#[derive(Clone, Debug, PartialEq)]
pub enum MatterSurfaceRuntimeError {
    /// Runtime ID was empty.
    EmptyRuntimeId,
    /// Particle set ID was empty.
    EmptyParticleSetId,
    /// Particle render payload ID was empty.
    EmptyRenderPayloadId,
    /// No surface sampler has been installed yet.
    DistanceSamplerUnavailable,
    /// No current surface is available.
    SurfaceUnavailable,
    /// Frame time was non-finite or negative.
    InvalidFrameTime,
    /// Particle count was outside the supported deterministic range.
    InvalidParticleCount,
    /// Particle reset radius values were invalid.
    InvalidParticleReset(&'static str),
    /// Mesh operation failed.
    Mesh(MatterMeshError),
    /// Particle operation failed.
    Particle(ParticleError),
    /// SDF operation failed.
    Sdf(SdfError),
    /// ADF operation failed.
    Adf(AdfError),
    /// Batch execution failed.
    Batch(BatchError),
}

impl fmt::Display for MatterSurfaceRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRuntimeId => formatter.write_str("surface runtime id must not be empty"),
            Self::EmptyParticleSetId => {
                formatter.write_str("surface runtime particle set id must not be empty")
            }
            Self::EmptyRenderPayloadId => {
                formatter.write_str("particle render payload id must not be empty")
            }
            Self::DistanceSamplerUnavailable => {
                formatter.write_str("surface distance sampler is unavailable")
            }
            Self::SurfaceUnavailable => {
                formatter.write_str("surface runtime has no current surface")
            }
            Self::InvalidFrameTime => {
                formatter.write_str("frame time must be finite and non-negative")
            }
            Self::InvalidParticleCount => {
                formatter.write_str("particle count must be within the deterministic runtime range")
            }
            Self::InvalidParticleReset(reason) => {
                write!(formatter, "invalid particle reset: {reason}")
            }
            Self::Mesh(error) => write!(formatter, "{error}"),
            Self::Particle(error) => write!(formatter, "{error}"),
            Self::Sdf(error) => write!(formatter, "{error}"),
            Self::Adf(error) => write!(formatter, "{error}"),
            Self::Batch(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for MatterSurfaceRuntimeError {}

impl From<MatterMeshError> for MatterSurfaceRuntimeError {
    fn from(value: MatterMeshError) -> Self {
        Self::Mesh(value)
    }
}

impl From<ParticleError> for MatterSurfaceRuntimeError {
    fn from(value: ParticleError) -> Self {
        Self::Particle(value)
    }
}

impl From<SdfError> for MatterSurfaceRuntimeError {
    fn from(value: SdfError) -> Self {
        Self::Sdf(value)
    }
}

impl From<AdfError> for MatterSurfaceRuntimeError {
    fn from(value: AdfError) -> Self {
        Self::Adf(value)
    }
}

impl From<BatchError> for MatterSurfaceRuntimeError {
    fn from(value: BatchError) -> Self {
        Self::Batch(value)
    }
}
