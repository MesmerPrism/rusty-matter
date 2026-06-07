use rusty_matter_model::Vec3;

use crate::{
    ParticleError, PARTICLE_IMPULSE_SCHEMA_ID, PARTICLE_INFLUENCE_POINT_SCHEMA_ID,
    PARTICLE_INTERACTIONS_SCHEMA_ID, PARTICLE_INTERACTION_BODY_SCHEMA_ID,
};

/// Influence mode for point interactions.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParticleInfluenceMode {
    /// Push particles away from the point.
    Repel,
    /// Pull particles toward the point.
    Attract,
    /// Pull particles toward the point with Gaussian falloff.
    GaussianAttract,
}

/// Point influence applied during particle simulation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleInfluencePoint {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable influence identifier.
    pub influence_id: String,
    /// Influence center.
    pub position: Vec3,
    /// Influence radius.
    pub radius: f32,
    /// Acceleration scale.
    pub strength: f32,
    /// Influence mode.
    pub mode: ParticleInfluenceMode,
}

impl ParticleInfluencePoint {
    /// Creates an influence point.
    #[must_use]
    pub fn new(
        influence_id: impl Into<String>,
        position: Vec3,
        radius: f32,
        strength: f32,
        mode: ParticleInfluenceMode,
    ) -> Self {
        Self {
            schema_id: PARTICLE_INFLUENCE_POINT_SCHEMA_ID.to_owned(),
            influence_id: influence_id.into(),
            position,
            radius,
            strength,
            mode,
        }
    }

    /// Validates the influence point.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the influence point is invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if self.schema_id != PARTICLE_INFLUENCE_POINT_SCHEMA_ID {
            return Err(ParticleError::UnexpectedSchema {
                expected: PARTICLE_INFLUENCE_POINT_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.influence_id.trim().is_empty() {
            return Err(ParticleError::EmptyInfluenceId);
        }
        if !self.position.is_finite() {
            return Err(ParticleError::InvalidInfluenceConfig(
                "position must be finite",
            ));
        }
        if !self.radius.is_finite() || self.radius < 0.0 {
            return Err(ParticleError::InvalidInfluenceConfig(
                "radius must be finite and non-negative",
            ));
        }
        if !self.strength.is_finite() {
            return Err(ParticleError::InvalidInfluenceConfig(
                "strength must be finite",
            ));
        }
        Ok(())
    }
}

/// One-shot impulse applied during the next fixed simulation step.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleImpulse {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable impulse identifier.
    pub impulse_id: String,
    /// Impulse center.
    pub position: Vec3,
    /// Impulse radius.
    pub radius: f32,
    /// Velocity delta at the impulse center.
    pub velocity_delta: Vec3,
}

impl ParticleImpulse {
    /// Creates an impulse.
    #[must_use]
    pub fn new(
        impulse_id: impl Into<String>,
        position: Vec3,
        radius: f32,
        velocity_delta: Vec3,
    ) -> Self {
        Self {
            schema_id: PARTICLE_IMPULSE_SCHEMA_ID.to_owned(),
            impulse_id: impulse_id.into(),
            position,
            radius,
            velocity_delta,
        }
    }

    /// Validates the impulse.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the impulse is invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if self.schema_id != PARTICLE_IMPULSE_SCHEMA_ID {
            return Err(ParticleError::UnexpectedSchema {
                expected: PARTICLE_IMPULSE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.impulse_id.trim().is_empty() {
            return Err(ParticleError::EmptyImpulseId);
        }
        if !self.position.is_finite() || !self.velocity_delta.is_finite() {
            return Err(ParticleError::InvalidImpulseConfig(
                "position and velocity_delta must be finite",
            ));
        }
        if !self.radius.is_finite() || self.radius < 0.0 {
            return Err(ParticleError::InvalidImpulseConfig(
                "radius must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

/// Simple interaction shape.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParticleInteractionShape {
    /// Sphere body.
    Sphere {
        /// Sphere center.
        center: Vec3,
        /// Sphere radius.
        radius: f32,
    },
    /// Axis-aligned box body.
    AxisAlignedBox {
        /// Minimum corner.
        min: Vec3,
        /// Maximum corner.
        max: Vec3,
    },
}

/// Simple collision body for particle simulation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleInteractionBody {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable body identifier.
    pub body_id: String,
    /// Body shape.
    pub shape: ParticleInteractionShape,
    /// Velocity restitution along the collision normal.
    pub restitution: f32,
}

impl ParticleInteractionBody {
    /// Creates a sphere body.
    #[must_use]
    pub fn sphere(body_id: impl Into<String>, center: Vec3, radius: f32) -> Self {
        Self {
            schema_id: PARTICLE_INTERACTION_BODY_SCHEMA_ID.to_owned(),
            body_id: body_id.into(),
            shape: ParticleInteractionShape::Sphere { center, radius },
            restitution: 0.0,
        }
    }

    /// Creates an axis-aligned box body.
    #[must_use]
    pub fn axis_aligned_box(body_id: impl Into<String>, min: Vec3, max: Vec3) -> Self {
        Self {
            schema_id: PARTICLE_INTERACTION_BODY_SCHEMA_ID.to_owned(),
            body_id: body_id.into(),
            shape: ParticleInteractionShape::AxisAlignedBox { min, max },
            restitution: 0.0,
        }
    }

    /// Validates the body.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the body is invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if self.schema_id != PARTICLE_INTERACTION_BODY_SCHEMA_ID {
            return Err(ParticleError::UnexpectedSchema {
                expected: PARTICLE_INTERACTION_BODY_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.body_id.trim().is_empty() {
            return Err(ParticleError::EmptyBodyId);
        }
        if !self.restitution.is_finite() || self.restitution < 0.0 {
            return Err(ParticleError::InvalidBodyConfig(
                "restitution must be finite and non-negative",
            ));
        }
        match self.shape {
            ParticleInteractionShape::Sphere { center, radius } => {
                if !center.is_finite() || !radius.is_finite() || radius < 0.0 {
                    return Err(ParticleError::InvalidBodyConfig(
                        "sphere center must be finite and radius must be non-negative",
                    ));
                }
            }
            ParticleInteractionShape::AxisAlignedBox { min, max } => {
                if !min.is_finite()
                    || !max.is_finite()
                    || min.x > max.x
                    || min.y > max.y
                    || min.z > max.z
                {
                    return Err(ParticleError::InvalidBodyConfig(
                        "box bounds must be finite and ordered",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Non-SDF particle interaction bundle.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleInteractions {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable bundle identifier.
    pub interactions_id: String,
    /// Point influences.
    pub influence_points: Vec<ParticleInfluencePoint>,
    /// Simple bodies.
    pub bodies: Vec<ParticleInteractionBody>,
}

impl Default for ParticleInteractions {
    fn default() -> Self {
        Self {
            schema_id: PARTICLE_INTERACTIONS_SCHEMA_ID.to_owned(),
            interactions_id: "particle.interactions.default".to_owned(),
            influence_points: Vec::new(),
            bodies: Vec::new(),
        }
    }
}

impl ParticleInteractions {
    /// Validates the interaction bundle.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when an interaction is invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if self.schema_id != PARTICLE_INTERACTIONS_SCHEMA_ID {
            return Err(ParticleError::UnexpectedSchema {
                expected: PARTICLE_INTERACTIONS_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.interactions_id.trim().is_empty() {
            return Err(ParticleError::EmptyInteractionsId);
        }
        for point in &self.influence_points {
            point.validate()?;
        }
        for body in &self.bodies {
            body.validate()?;
        }
        Ok(())
    }
}
