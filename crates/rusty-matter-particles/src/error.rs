use core::fmt;

/// Particle validation or simulation failure.
#[derive(Clone, Debug, PartialEq)]
pub enum ParticleError {
    /// Payload schema did not match the expected schema.
    UnexpectedSchema {
        /// Expected schema ID.
        expected: &'static str,
        /// Actual schema ID.
        actual: String,
    },
    /// Particle ID was empty.
    EmptyParticleId,
    /// Particle set ID was empty.
    EmptySetId,
    /// Render payload ID was empty.
    EmptyRenderPayloadId,
    /// SDF interaction ID was empty.
    EmptyInteractionId,
    /// Particle interaction bundle ID was empty.
    EmptyInteractionsId,
    /// Particle influence point ID was empty.
    EmptyInfluenceId,
    /// Particle impulse ID was empty.
    EmptyImpulseId,
    /// Particle interaction body ID was empty.
    EmptyBodyId,
    /// Fixed-step config ID was empty.
    EmptyStepConfigId,
    /// Particle position was non-finite.
    NonFinitePosition {
        /// Rejected particle ID.
        particle_id: String,
    },
    /// Particle velocity was non-finite.
    NonFiniteVelocity {
        /// Rejected particle ID.
        particle_id: String,
    },
    /// Particle radius was invalid.
    InvalidRadius {
        /// Rejected particle ID.
        particle_id: String,
    },
    /// Particle inverse mass was invalid.
    InvalidInverseMass {
        /// Rejected particle ID.
        particle_id: String,
    },
    /// Particle age was invalid.
    InvalidAge {
        /// Rejected particle ID.
        particle_id: String,
    },
    /// Particle set time was invalid.
    InvalidSetTime,
    /// Interaction config was invalid.
    InvalidInteractionConfig(&'static str),
    /// Neighbor interaction config was invalid.
    InvalidNeighborConfig(&'static str),
    /// Influence point config was invalid.
    InvalidInfluenceConfig(&'static str),
    /// Particle impulse config was invalid.
    InvalidImpulseConfig(&'static str),
    /// Particle interaction body config was invalid.
    InvalidBodyConfig(&'static str),
    /// Particle execution config was invalid.
    InvalidExecutionConfig(&'static str),
    /// Batch execution failed.
    BatchExecution(String),
    /// Render-neutral payload was invalid.
    InvalidRenderPayload(&'static str),
    /// Spatial hash cell size was invalid.
    InvalidSpatialHashCellSize,
    /// Fixed step was invalid.
    InvalidFixedStep,
    /// Maximum fixed steps was invalid.
    InvalidMaxSteps,
}

impl fmt::Display for ParticleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedSchema { expected, actual } => {
                write!(formatter, "expected schema {expected}, found {actual}")
            }
            Self::EmptyParticleId => formatter.write_str("particle id must not be empty"),
            Self::EmptySetId => formatter.write_str("particle set id must not be empty"),
            Self::EmptyRenderPayloadId => {
                formatter.write_str("particle render payload id must not be empty")
            }
            Self::EmptyInteractionId => formatter.write_str("interaction id must not be empty"),
            Self::EmptyInteractionsId => {
                formatter.write_str("particle interactions id must not be empty")
            }
            Self::EmptyInfluenceId => formatter.write_str("influence id must not be empty"),
            Self::EmptyImpulseId => formatter.write_str("impulse id must not be empty"),
            Self::EmptyBodyId => formatter.write_str("body id must not be empty"),
            Self::EmptyStepConfigId => {
                formatter.write_str("fixed-step config id must not be empty")
            }
            Self::NonFinitePosition { particle_id } => {
                write!(formatter, "particle {particle_id} position is non-finite")
            }
            Self::NonFiniteVelocity { particle_id } => {
                write!(formatter, "particle {particle_id} velocity is non-finite")
            }
            Self::InvalidRadius { particle_id } => {
                write!(formatter, "particle {particle_id} radius is invalid")
            }
            Self::InvalidInverseMass { particle_id } => {
                write!(formatter, "particle {particle_id} inverse mass is invalid")
            }
            Self::InvalidAge { particle_id } => {
                write!(formatter, "particle {particle_id} age is invalid")
            }
            Self::InvalidSetTime => formatter.write_str("particle set time is invalid"),
            Self::InvalidInteractionConfig(reason) => {
                write!(formatter, "invalid SDF interaction config: {reason}")
            }
            Self::InvalidNeighborConfig(reason) => {
                write!(formatter, "invalid neighbor interaction config: {reason}")
            }
            Self::InvalidInfluenceConfig(reason) => {
                write!(formatter, "invalid influence point config: {reason}")
            }
            Self::InvalidImpulseConfig(reason) => {
                write!(formatter, "invalid particle impulse config: {reason}")
            }
            Self::InvalidBodyConfig(reason) => {
                write!(
                    formatter,
                    "invalid particle interaction body config: {reason}"
                )
            }
            Self::InvalidExecutionConfig(reason) => {
                write!(formatter, "invalid particle execution config: {reason}")
            }
            Self::BatchExecution(reason) => {
                write!(formatter, "particle batch execution failed: {reason}")
            }
            Self::InvalidRenderPayload(reason) => {
                write!(formatter, "invalid particle render payload: {reason}")
            }
            Self::InvalidSpatialHashCellSize => {
                formatter.write_str("spatial hash cell size must be finite and positive")
            }
            Self::InvalidFixedStep => formatter.write_str("fixed step must be finite and positive"),
            Self::InvalidMaxSteps => formatter.write_str("max steps per frame must be non-zero"),
        }
    }
}

impl std::error::Error for ParticleError {}
