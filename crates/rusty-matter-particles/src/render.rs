use rusty_matter_model::Vec3;

use crate::{
    ParticleError, ParticleSet, ParticleState, PARTICLE_RENDER_PAYLOAD_SCHEMA_ID,
    PARTICLE_RENDER_SAMPLE_SCHEMA_ID,
};

/// One render-neutral particle sample prepared from simulation state.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleRenderSample {
    /// Schema identifier.
    pub schema_id: String,
    /// Source particle identifier.
    pub particle_id: String,
    /// Particle position.
    pub position: Vec3,
    /// Particle radius.
    pub radius: f32,
    /// Particle velocity.
    pub velocity: Vec3,
    /// Particle speed in units per second.
    pub speed: f32,
    /// Particle age in seconds.
    pub age_seconds: f32,
    /// Domain-neutral particle flags.
    pub flags: u32,
}

impl ParticleRenderSample {
    /// Builds a render-neutral sample from one particle.
    #[must_use]
    pub fn from_particle(particle: &ParticleState) -> Self {
        Self {
            schema_id: PARTICLE_RENDER_SAMPLE_SCHEMA_ID.to_owned(),
            particle_id: particle.particle_id.clone(),
            position: particle.position,
            radius: particle.radius,
            velocity: particle.velocity,
            speed: particle.velocity.length(),
            age_seconds: particle.age_seconds,
            flags: particle.flags,
        }
    }

    /// Validates render-neutral sample data.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the sample is invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if self.schema_id != PARTICLE_RENDER_SAMPLE_SCHEMA_ID {
            return Err(ParticleError::UnexpectedSchema {
                expected: PARTICLE_RENDER_SAMPLE_SCHEMA_ID,
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
        if !self.speed.is_finite() || self.speed < 0.0 {
            return Err(ParticleError::InvalidRenderPayload(
                "sample speed must be finite and non-negative",
            ));
        }
        if !self.age_seconds.is_finite() || self.age_seconds < 0.0 {
            return Err(ParticleError::InvalidAge {
                particle_id: self.particle_id.clone(),
            });
        }
        Ok(())
    }
}

/// Render-neutral particle payload for Optics or renderer adapters to consume.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleRenderPayload {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable payload identifier.
    pub payload_id: String,
    /// Source particle set identifier.
    pub source_set_id: String,
    /// Source set time in seconds.
    pub time_seconds: f32,
    /// Render-neutral particle samples.
    pub samples: Vec<ParticleRenderSample>,
    /// Minimum particle bounds, including radius. None when the set is empty.
    pub bounds_min: Option<Vec3>,
    /// Maximum particle bounds, including radius. None when the set is empty.
    pub bounds_max: Option<Vec3>,
}

impl ParticleRenderPayload {
    /// Builds a render-neutral payload from a particle set.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the set or payload ID is invalid.
    pub fn from_particle_set(
        payload_id: impl Into<String>,
        set: &ParticleSet,
    ) -> Result<Self, ParticleError> {
        set.validate()?;
        let payload_id = payload_id.into();
        if payload_id.trim().is_empty() {
            return Err(ParticleError::EmptyRenderPayloadId);
        }
        let samples = set
            .particles
            .iter()
            .map(ParticleRenderSample::from_particle)
            .collect::<Vec<_>>();
        let (bounds_min, bounds_max) = particle_render_bounds(&samples);
        let payload = Self {
            schema_id: PARTICLE_RENDER_PAYLOAD_SCHEMA_ID.to_owned(),
            payload_id,
            source_set_id: set.set_id.clone(),
            time_seconds: set.time_seconds,
            samples,
            bounds_min,
            bounds_max,
        };
        payload.validate()?;
        Ok(payload)
    }

    /// Validates payload shape.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the payload is invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if self.schema_id != PARTICLE_RENDER_PAYLOAD_SCHEMA_ID {
            return Err(ParticleError::UnexpectedSchema {
                expected: PARTICLE_RENDER_PAYLOAD_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.payload_id.trim().is_empty() {
            return Err(ParticleError::EmptyRenderPayloadId);
        }
        if self.source_set_id.trim().is_empty() {
            return Err(ParticleError::EmptySetId);
        }
        if !self.time_seconds.is_finite() || self.time_seconds < 0.0 {
            return Err(ParticleError::InvalidSetTime);
        }
        for sample in &self.samples {
            sample.validate()?;
        }
        match (self.bounds_min, self.bounds_max) {
            (Some(min), Some(max)) => {
                if !min.is_finite()
                    || !max.is_finite()
                    || min.x > max.x
                    || min.y > max.y
                    || min.z > max.z
                {
                    return Err(ParticleError::InvalidRenderPayload(
                        "bounds must be finite and ordered",
                    ));
                }
            }
            (None, None) if self.samples.is_empty() => {}
            _ => {
                return Err(ParticleError::InvalidRenderPayload(
                    "bounds must both be present for non-empty payloads",
                ));
            }
        }
        Ok(())
    }
}

fn particle_render_bounds(samples: &[ParticleRenderSample]) -> (Option<Vec3>, Option<Vec3>) {
    let Some(first) = samples.first() else {
        return (None, None);
    };
    let radius = Vec3::new(first.radius, first.radius, first.radius);
    let mut min = first.position - radius;
    let mut max = first.position + radius;
    for sample in samples.iter().skip(1) {
        let radius = Vec3::new(sample.radius, sample.radius, sample.radius);
        min = min.min(sample.position - radius);
        max = max.max(sample.position + radius);
    }
    (Some(min), Some(max))
}
