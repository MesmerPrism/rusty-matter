use core::fmt;

use rusty_matter_batch::BatchError;
use rusty_matter_model::MatterModelError;

/// SDF operation failure.
#[derive(Clone, Debug, PartialEq)]
pub enum SdfError {
    /// Model validation failed.
    Model(MatterModelError),
    /// Grid schema was unexpected.
    UnexpectedSchema {
        /// Expected schema ID.
        expected: &'static str,
        /// Actual schema ID.
        actual: String,
    },
    /// Grid ID was empty.
    EmptyGridId,
    /// Origin was non-finite.
    NonFiniteOrigin,
    /// Voxel size was invalid.
    InvalidVoxelSize(f32),
    /// Max voxel budget was invalid.
    InvalidVoxelBudget,
    /// Dimension was zero.
    ZeroDimension,
    /// Voxel count overflowed.
    VoxelCountOverflow,
    /// Voxel budget was exceeded.
    VoxelBudgetExceeded {
        /// Requested voxel count.
        requested: usize,
        /// Maximum allowed voxel count.
        max: usize,
    },
    /// Packed distance count did not match dimensions.
    DistanceCountMismatch {
        /// Expected sample count.
        expected: usize,
        /// Actual sample count.
        actual: usize,
    },
    /// Packed distance was non-finite.
    NonFiniteDistance {
        /// Rejected distance index.
        index: usize,
    },
    /// Triangle had no area.
    DegenerateTriangle,
    /// Batch executor creation failed.
    BatchExecution(BatchError),
}

impl fmt::Display for SdfError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(formatter, "{error}"),
            Self::UnexpectedSchema { expected, actual } => {
                write!(formatter, "expected schema {expected}, found {actual}")
            }
            Self::EmptyGridId => formatter.write_str("grid id must not be empty"),
            Self::NonFiniteOrigin => formatter.write_str("grid origin must be finite"),
            Self::InvalidVoxelSize(value) => {
                write!(formatter, "voxel size must be finite and positive: {value}")
            }
            Self::InvalidVoxelBudget => formatter.write_str("max voxel budget must be non-zero"),
            Self::ZeroDimension => formatter.write_str("grid dimensions must be non-zero"),
            Self::VoxelCountOverflow => formatter.write_str("grid voxel count overflowed"),
            Self::VoxelBudgetExceeded { requested, max } => {
                write!(formatter, "requested {requested} voxels, max is {max}")
            }
            Self::DistanceCountMismatch { expected, actual } => {
                write!(formatter, "expected {expected} distances, found {actual}")
            }
            Self::NonFiniteDistance { index } => {
                write!(formatter, "distance {index} is non-finite")
            }
            Self::DegenerateTriangle => formatter.write_str("triangle has no area"),
            Self::BatchExecution(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for SdfError {}

impl From<MatterModelError> for SdfError {
    fn from(value: MatterModelError) -> Self {
        Self::Model(value)
    }
}

impl From<BatchError> for SdfError {
    fn from(value: BatchError) -> Self {
        Self::BatchExecution(value)
    }
}
