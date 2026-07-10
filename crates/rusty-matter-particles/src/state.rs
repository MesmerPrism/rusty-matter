use rusty_matter_model::Vec3;

use crate::{ParticleError, PARTICLE_SET_SCHEMA_ID, PARTICLE_STATE_SCHEMA_ID};

/// One particle in a Matter-owned particle set.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleState {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable particle identifier within its set.
    pub particle_id: String,
    /// Particle position in the set coordinate space.
    pub position: Vec3,
    /// Particle velocity in units per second.
    pub velocity: Vec3,
    /// Particle radius.
    pub radius: f32,
    /// Inverse mass. Zero means pinned.
    pub inverse_mass: f32,
    /// Particle age in seconds.
    pub age_seconds: f32,
    /// Domain-neutral flags for downstream adapters.
    pub flags: u32,
}

impl ParticleState {
    /// Creates a particle state.
    #[must_use]
    pub fn new(particle_id: impl Into<String>, position: Vec3, radius: f32) -> Self {
        Self {
            schema_id: PARTICLE_STATE_SCHEMA_ID.to_owned(),
            particle_id: particle_id.into(),
            position,
            velocity: Vec3::ZERO,
            radius,
            inverse_mass: 1.0,
            age_seconds: 0.0,
            flags: 0,
        }
    }

    /// Validates the particle state.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the state has invalid metadata or
    /// non-finite physics fields.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if self.schema_id != PARTICLE_STATE_SCHEMA_ID {
            return Err(ParticleError::UnexpectedSchema {
                expected: PARTICLE_STATE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.particle_id.trim().is_empty() {
            return Err(ParticleError::EmptyParticleId);
        }
        if !self.position.is_finite() {
            return Err(ParticleError::NonFinitePosition {
                particle_id: self.particle_id.clone(),
            });
        }
        if !self.velocity.is_finite() {
            return Err(ParticleError::NonFiniteVelocity {
                particle_id: self.particle_id.clone(),
            });
        }
        if !self.radius.is_finite() || self.radius < 0.0 {
            return Err(ParticleError::InvalidRadius {
                particle_id: self.particle_id.clone(),
            });
        }
        if !self.inverse_mass.is_finite() || self.inverse_mass < 0.0 {
            return Err(ParticleError::InvalidInverseMass {
                particle_id: self.particle_id.clone(),
            });
        }
        if !self.age_seconds.is_finite() || self.age_seconds < 0.0 {
            return Err(ParticleError::InvalidAge {
                particle_id: self.particle_id.clone(),
            });
        }
        Ok(())
    }
}

/// Snapshot of a particle set.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleSet {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable set identifier.
    pub set_id: String,
    /// Snapshot time in seconds.
    pub time_seconds: f32,
    /// Particle states.
    pub particles: Vec<ParticleState>,
}

impl ParticleSet {
    /// Creates an empty particle set.
    #[must_use]
    pub fn new(set_id: impl Into<String>) -> Self {
        Self {
            schema_id: PARTICLE_SET_SCHEMA_ID.to_owned(),
            set_id: set_id.into(),
            time_seconds: 0.0,
            particles: Vec::new(),
        }
    }

    /// Creates a set with capacity.
    #[must_use]
    pub fn with_capacity(set_id: impl Into<String>, capacity: usize) -> Self {
        Self {
            particles: Vec::with_capacity(capacity),
            ..Self::new(set_id)
        }
    }

    /// Adds one particle.
    pub fn push(&mut self, particle: ParticleState) {
        self.particles.push(particle);
    }

    /// Returns the number of particles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    /// Returns whether the set has no particles.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Validates the particle set.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when metadata or a contained particle is
    /// invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if self.schema_id != PARTICLE_SET_SCHEMA_ID {
            return Err(ParticleError::UnexpectedSchema {
                expected: PARTICLE_SET_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.set_id.trim().is_empty() {
            return Err(ParticleError::EmptySetId);
        }
        if !self.time_seconds.is_finite() || self.time_seconds < 0.0 {
            return Err(ParticleError::InvalidSetTime);
        }
        for particle in &self.particles {
            particle.validate()?;
        }
        Ok(())
    }
}
