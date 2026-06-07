use core::fmt;

/// Model validation failure.
#[derive(Clone, Debug, PartialEq)]
pub enum MatterModelError {
    /// Identifier was empty.
    EmptyId,
    /// Identifier had an empty segment.
    EmptyIdSegment,
    /// Identifier segment had an invalid edge character.
    InvalidIdSegmentEdge(String),
    /// Identifier segment had an invalid character.
    InvalidIdCharacter(char),
    /// Schema did not start with `rusty.matter`.
    InvalidSchemaPrefix(String),
    /// Schema version was invalid.
    InvalidSchemaVersion(String),
    /// Schema did not match the expected payload.
    UnexpectedSchema {
        /// Expected schema ID.
        expected: &'static str,
        /// Actual schema ID.
        actual: String,
    },
    /// Mesh identifier was empty.
    EmptyMeshId,
    /// Point set was empty.
    EmptyPointSet,
    /// Index set was empty.
    EmptyIndexSet,
    /// Point contained a non-finite coordinate.
    NonFinitePoint {
        /// Rejected point index.
        index: usize,
    },
    /// Bounds contained a non-finite coordinate.
    NonFiniteBounds,
    /// Bounds min exceeded max.
    InvertedBounds,
    /// Padding was invalid.
    InvalidPadding,
    /// Triangle repeated at least one vertex.
    DegenerateTriangle {
        /// Rejected triangle index.
        triangle_index: usize,
    },
    /// Triangle index was outside the position array.
    IndexOutOfRange {
        /// Rejected triangle index.
        triangle_index: usize,
        /// Rejected vertex index.
        vertex_index: u32,
        /// Available vertex count.
        vertex_count: usize,
    },
}

impl fmt::Display for MatterModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => formatter.write_str("dotted id must not be empty"),
            Self::EmptyIdSegment => formatter.write_str("dotted id segments must not be empty"),
            Self::InvalidIdSegmentEdge(segment) => {
                write!(formatter, "invalid dotted id segment edge: {segment}")
            }
            Self::InvalidIdCharacter(character) => {
                write!(formatter, "invalid dotted id character: {character}")
            }
            Self::InvalidSchemaPrefix(value) => {
                write!(formatter, "schema id must start with rusty.matter: {value}")
            }
            Self::InvalidSchemaVersion(value) => {
                write!(formatter, "schema id version must be v<positive integer>: {value}")
            }
            Self::UnexpectedSchema { expected, actual } => {
                write!(formatter, "expected schema {expected}, found {actual}")
            }
            Self::EmptyMeshId => formatter.write_str("mesh id must not be empty"),
            Self::EmptyPointSet => formatter.write_str("point set must not be empty"),
            Self::EmptyIndexSet => formatter.write_str("index set must not be empty"),
            Self::NonFinitePoint { index } => write!(formatter, "point {index} is non-finite"),
            Self::NonFiniteBounds => formatter.write_str("bounds must be finite"),
            Self::InvertedBounds => formatter.write_str("bounds min must not exceed max"),
            Self::InvalidPadding => formatter.write_str("padding must be finite and non-negative"),
            Self::DegenerateTriangle { triangle_index } => {
                write!(formatter, "triangle {triangle_index} repeats a vertex")
            }
            Self::IndexOutOfRange {
                triangle_index,
                vertex_index,
                vertex_count,
            } => write!(
                formatter,
                "triangle {triangle_index} references vertex {vertex_index}, but only {vertex_count} vertices exist"
            ),
        }
    }
}

impl std::error::Error for MatterModelError {}
