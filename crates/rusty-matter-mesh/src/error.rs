use core::fmt;

/// Mesh validation or sampling failure.
#[derive(Clone, Debug, PartialEq)]
pub enum MatterMeshError {
    /// Payload schema did not match the expected schema.
    UnexpectedSchema {
        /// Expected schema ID.
        expected: &'static str,
        /// Actual schema ID.
        actual: String,
    },
    /// Surface ID was empty.
    EmptySurfaceId,
    /// Sample config ID was empty.
    EmptySampleConfigId,
    /// Sample set ID was empty.
    EmptySampleSetId,
    /// Coordinate map ID was empty.
    EmptyCoordinateMapId,
    /// Coordinate frame config ID was empty.
    EmptyCoordinateFrameConfigId,
    /// Coordinate frame set ID was empty.
    EmptyCoordinateFrameSetId,
    /// Collider config ID was empty.
    EmptyColliderConfigId,
    /// Hand rig capture ID was empty.
    EmptyHandRigCaptureId,
    /// Hand joint frame ID was empty.
    EmptyHandJointFrameId,
    /// Hand mesh frame ID was empty.
    EmptyHandFrameId,
    /// Position was non-finite.
    NonFinitePosition {
        /// Rejected position index.
        index: usize,
    },
    /// Triangle repeated a vertex.
    DegenerateTriangle {
        /// Rejected triangle index.
        triangle_index: usize,
    },
    /// Triangle referenced an out-of-range vertex.
    IndexOutOfRange {
        /// Rejected triangle index.
        triangle_index: usize,
        /// Rejected vertex index.
        vertex_index: u32,
        /// Available vertex count.
        vertex_count: usize,
    },
    /// Surface was invalid.
    InvalidSurface(&'static str),
    /// Sample config was invalid.
    InvalidSampleConfig(&'static str),
    /// Collider config was invalid.
    InvalidColliderConfig(&'static str),
    /// Coordinate-frame config was invalid.
    InvalidCoordinateFrameConfig(&'static str),
    /// Coordinate map was invalid.
    InvalidCoordinateMap(&'static str),
    /// Hand payload was invalid.
    InvalidHandPayload(&'static str),
    /// Surface topology changed.
    ChangedTopology,
}

impl fmt::Display for MatterMeshError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedSchema { expected, actual } => {
                write!(formatter, "expected schema {expected}, found {actual}")
            }
            Self::EmptySurfaceId => formatter.write_str("surface id must not be empty"),
            Self::EmptySampleConfigId => {
                formatter.write_str("sample config id must not be empty")
            }
            Self::EmptySampleSetId => formatter.write_str("sample set id must not be empty"),
            Self::EmptyCoordinateMapId => {
                formatter.write_str("coordinate map id must not be empty")
            }
            Self::EmptyCoordinateFrameConfigId => {
                formatter.write_str("coordinate frame config id must not be empty")
            }
            Self::EmptyCoordinateFrameSetId => {
                formatter.write_str("coordinate frame set id must not be empty")
            }
            Self::EmptyColliderConfigId => {
                formatter.write_str("collider config id must not be empty")
            }
            Self::EmptyHandRigCaptureId => {
                formatter.write_str("hand rig capture id must not be empty")
            }
            Self::EmptyHandJointFrameId => {
                formatter.write_str("hand joint frame id must not be empty")
            }
            Self::EmptyHandFrameId => formatter.write_str("hand mesh frame id must not be empty"),
            Self::NonFinitePosition { index } => {
                write!(formatter, "surface position {index} is non-finite")
            }
            Self::DegenerateTriangle { triangle_index } => {
                write!(formatter, "surface triangle {triangle_index} is degenerate")
            }
            Self::IndexOutOfRange {
                triangle_index,
                vertex_index,
                vertex_count,
            } => write!(
                formatter,
                "surface triangle {triangle_index} references vertex {vertex_index}, but only {vertex_count} vertices exist"
            ),
            Self::InvalidSurface(reason) => write!(formatter, "invalid mesh surface: {reason}"),
            Self::InvalidSampleConfig(reason) => {
                write!(formatter, "invalid mesh surface sample config: {reason}")
            }
            Self::InvalidColliderConfig(reason) => {
                write!(formatter, "invalid dynamic mesh collider config: {reason}")
            }
            Self::InvalidCoordinateFrameConfig(reason) => {
                write!(formatter, "invalid mesh coordinate frame config: {reason}")
            }
            Self::InvalidCoordinateMap(reason) => {
                write!(formatter, "invalid mesh coordinate map: {reason}")
            }
            Self::InvalidHandPayload(reason) => write!(formatter, "invalid hand payload: {reason}"),
            Self::ChangedTopology => formatter.write_str("mesh surface topology changed"),
        }
    }
}

impl std::error::Error for MatterMeshError {}
