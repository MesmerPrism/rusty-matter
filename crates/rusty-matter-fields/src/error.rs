use core::fmt;

/// Surface-field validation failure.
#[derive(Clone, Debug, PartialEq)]
pub enum MatterFieldError {
    /// Payload schema did not match the expected schema.
    UnexpectedSchema {
        /// Expected schema ID.
        expected: &'static str,
        /// Actual schema ID.
        actual: String,
    },
    /// Surface-field substrate ID was empty.
    EmptySubstrateId,
    /// Surface-field node ID was empty.
    EmptyNodeId,
    /// Surface-field field ID was empty.
    EmptyFieldId,
    /// Surface-field state ID was empty.
    EmptyStateId,
    /// Surface-field perturbation ID was empty.
    EmptyPerturbationId,
    /// Surface-field runtime config ID was empty.
    EmptyRuntimeConfigId,
    /// Surface-field summary ID was empty.
    EmptyRunSummaryId,
    /// Surface-field substrate was invalid.
    InvalidSubstrate(&'static str),
    /// Surface-field scalar or vector field was invalid.
    InvalidField(&'static str),
    /// Surface-field perturbation was invalid.
    InvalidPerturbation(&'static str),
    /// Surface-field runtime config was invalid.
    InvalidRuntimeConfig(&'static str),
    /// Surface-field run summary was invalid.
    InvalidRunSummary(&'static str),
    /// Field buffer length did not match the substrate node count.
    NodeCountMismatch {
        /// Expected node count.
        expected: usize,
        /// Actual buffer count.
        actual: usize,
    },
    /// Scalar field value was non-finite.
    NonFiniteScalar {
        /// Rejected field ID.
        field_id: String,
        /// Rejected value index.
        index: usize,
    },
    /// Vector field value was non-finite.
    NonFiniteVector {
        /// Rejected field ID.
        field_id: String,
        /// Rejected value index.
        index: usize,
    },
    /// Neighbor target was invalid.
    InvalidNeighbor {
        /// Source node index.
        node_index: usize,
        /// Rejected neighbor index.
        neighbor_index: usize,
    },
    /// Neighbor list referenced its source node.
    SelfNeighbor {
        /// Source node index.
        node_index: usize,
    },
    /// Neighbor list repeated a target.
    DuplicateNeighbor {
        /// Source node index.
        node_index: usize,
        /// Repeated neighbor index.
        neighbor_index: usize,
    },
    /// Field IDs must be unique within a state.
    DuplicateFieldId {
        /// Repeated field ID.
        field_id: String,
    },
    /// Perturbation node list repeated a target.
    DuplicatePerturbationNode {
        /// Repeated node index.
        node_index: usize,
    },
    /// Perturbation referenced a node outside the substrate.
    InvalidPerturbationNode {
        /// Rejected node index.
        node_index: usize,
        /// Available node count.
        node_count: usize,
    },
}

impl fmt::Display for MatterFieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedSchema { expected, actual } => {
                write!(formatter, "expected schema {expected}, found {actual}")
            }
            Self::EmptySubstrateId => {
                formatter.write_str("surface-field substrate id must not be empty")
            }
            Self::EmptyNodeId => formatter.write_str("surface-field node id must not be empty"),
            Self::EmptyFieldId => formatter.write_str("surface-field field id must not be empty"),
            Self::EmptyStateId => formatter.write_str("surface-field state id must not be empty"),
            Self::EmptyPerturbationId => {
                formatter.write_str("surface-field perturbation id must not be empty")
            }
            Self::EmptyRuntimeConfigId => {
                formatter.write_str("surface-field runtime config id must not be empty")
            }
            Self::EmptyRunSummaryId => {
                formatter.write_str("surface-field run summary id must not be empty")
            }
            Self::InvalidSubstrate(reason) => {
                write!(formatter, "invalid surface-field substrate: {reason}")
            }
            Self::InvalidField(reason) => write!(formatter, "invalid surface field: {reason}"),
            Self::InvalidPerturbation(reason) => {
                write!(formatter, "invalid surface-field perturbation: {reason}")
            }
            Self::InvalidRuntimeConfig(reason) => {
                write!(formatter, "invalid surface-field runtime config: {reason}")
            }
            Self::InvalidRunSummary(reason) => {
                write!(formatter, "invalid surface-field run summary: {reason}")
            }
            Self::NodeCountMismatch { expected, actual } => write!(
                formatter,
                "surface-field node count mismatch: expected {expected}, found {actual}"
            ),
            Self::NonFiniteScalar { field_id, index } => {
                write!(
                    formatter,
                    "surface scalar field {field_id} value {index} is non-finite"
                )
            }
            Self::NonFiniteVector { field_id, index } => {
                write!(
                    formatter,
                    "surface vector field {field_id} value {index} is non-finite"
                )
            }
            Self::InvalidNeighbor {
                node_index,
                neighbor_index,
            } => write!(
                formatter,
                "surface-field node {node_index} references invalid neighbor {neighbor_index}"
            ),
            Self::SelfNeighbor { node_index } => {
                write!(
                    formatter,
                    "surface-field node {node_index} references itself"
                )
            }
            Self::DuplicateNeighbor {
                node_index,
                neighbor_index,
            } => write!(
                formatter,
                "surface-field node {node_index} repeats neighbor {neighbor_index}"
            ),
            Self::DuplicateFieldId { field_id } => {
                write!(formatter, "surface-field state repeats field id {field_id}")
            }
            Self::DuplicatePerturbationNode { node_index } => {
                write!(
                    formatter,
                    "surface-field perturbation repeats node {node_index}"
                )
            }
            Self::InvalidPerturbationNode {
                node_index,
                node_count,
            } => write!(
                formatter,
                "surface-field perturbation references node {node_index}, but only {node_count} nodes exist"
            ),
        }
    }
}

impl std::error::Error for MatterFieldError {}
