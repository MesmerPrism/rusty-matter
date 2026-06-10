use std::num::NonZeroUsize;

use rusty_matter_batch::{BatchBackendKind, BatchConfig};

use crate::{ParticleError, FIXED_STEP_CONFIG_SCHEMA_ID, SDF_INTERACTION_CONFIG_SCHEMA_ID};

/// Particle execution backend.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParticleExecutionBackend {
    /// Deterministic single-threaded execution.
    Serial,
    /// Rayon-backed batch execution.
    #[cfg(feature = "parallel")]
    Parallel,
}

impl ParticleExecutionBackend {
    /// Stable marker token for diagnostics and evidence.
    #[must_use]
    pub const fn marker_value(self) -> &'static str {
        match self {
            Self::Serial => "serial",
            #[cfg(feature = "parallel")]
            Self::Parallel => "rayon",
        }
    }
}

/// Particle execution configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParticleExecutionConfig {
    /// Batch execution backend.
    pub backend: ParticleExecutionBackend,
    /// Logical batch size for one particle step.
    pub batch_size: NonZeroUsize,
    /// Optional worker cap for parallel execution.
    pub max_threads: Option<usize>,
}

impl Default for ParticleExecutionConfig {
    fn default() -> Self {
        Self {
            backend: ParticleExecutionBackend::Serial,
            batch_size: NonZeroUsize::new(256).expect("default batch size is non-zero"),
            max_threads: None,
        }
    }
}

impl ParticleExecutionConfig {
    /// Validates the execution config.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when a low-rate execution setting is invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if matches!(self.max_threads, Some(0)) {
            return Err(ParticleError::InvalidExecutionConfig(
                "max_threads must be absent or positive",
            ));
        }
        Ok(())
    }

    pub(crate) fn batch_config(&self) -> Result<BatchConfig, ParticleError> {
        self.validate()?;
        Ok(BatchConfig {
            backend: match self.backend {
                ParticleExecutionBackend::Serial => BatchBackendKind::Serial,
                #[cfg(feature = "parallel")]
                ParticleExecutionBackend::Parallel => BatchBackendKind::Rayon,
            },
            batch_size: self.batch_size,
            max_threads: self.max_threads,
        })
    }
}

/// SDF interaction mode.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SdfParticleInteractionMode {
    /// Do not apply SDF acceleration.
    Disabled,
    /// Move particles toward the configured surface distance.
    AttractToSurface,
    /// Push particles away when they are inside the configured surface band.
    RepelFromSurface,
}

/// SDF interaction configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SdfParticleInteractionConfig {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable config identifier.
    pub interaction_id: String,
    /// Interaction mode.
    pub mode: SdfParticleInteractionMode,
    /// Target signed distance from the SDF surface.
    pub target_distance: f32,
    /// Acceleration scale.
    pub strength: f32,
    /// Velocity damping applied after acceleration.
    pub damping: f32,
    /// Maximum allowed speed after a fixed step.
    pub max_speed: f32,
}

impl Default for SdfParticleInteractionConfig {
    fn default() -> Self {
        Self {
            schema_id: SDF_INTERACTION_CONFIG_SCHEMA_ID.to_owned(),
            interaction_id: "interaction.sdf_surface_default".to_owned(),
            mode: SdfParticleInteractionMode::AttractToSurface,
            target_distance: 0.0,
            strength: 4.0,
            damping: 0.1,
            max_speed: 4.0,
        }
    }
}

impl SdfParticleInteractionConfig {
    /// Validates the interaction config.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when fields are non-finite or invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if self.schema_id != SDF_INTERACTION_CONFIG_SCHEMA_ID {
            return Err(ParticleError::UnexpectedSchema {
                expected: SDF_INTERACTION_CONFIG_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.interaction_id.trim().is_empty() {
            return Err(ParticleError::EmptyInteractionId);
        }
        if !self.target_distance.is_finite() {
            return Err(ParticleError::InvalidInteractionConfig(
                "target_distance must be finite",
            ));
        }
        if !self.strength.is_finite() || self.strength < 0.0 {
            return Err(ParticleError::InvalidInteractionConfig(
                "strength must be finite and non-negative",
            ));
        }
        if !self.damping.is_finite() || self.damping < 0.0 {
            return Err(ParticleError::InvalidInteractionConfig(
                "damping must be finite and non-negative",
            ));
        }
        if !self.max_speed.is_finite() || self.max_speed < 0.0 {
            return Err(ParticleError::InvalidInteractionConfig(
                "max_speed must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

/// Fixed-step simulation configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleFixedStepConfig {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable config identifier.
    pub step_config_id: String,
    /// Fixed step size in seconds.
    pub fixed_step_seconds: f32,
    /// Maximum fixed steps consumed by one frame.
    pub max_steps_per_frame: u32,
    /// Neighbor query radius. Zero disables neighbor interaction.
    pub neighbor_radius: f32,
    /// Repulsive acceleration scale for close neighbors.
    pub neighbor_repulsion_strength: f32,
}

impl Default for ParticleFixedStepConfig {
    fn default() -> Self {
        Self {
            schema_id: FIXED_STEP_CONFIG_SCHEMA_ID.to_owned(),
            step_config_id: "particle.fixed_step.default".to_owned(),
            fixed_step_seconds: 1.0 / 60.0,
            max_steps_per_frame: 4,
            neighbor_radius: 0.0,
            neighbor_repulsion_strength: 0.0,
        }
    }
}

impl ParticleFixedStepConfig {
    /// Validates the fixed-step config.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when fields are invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if self.schema_id != FIXED_STEP_CONFIG_SCHEMA_ID {
            return Err(ParticleError::UnexpectedSchema {
                expected: FIXED_STEP_CONFIG_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.step_config_id.trim().is_empty() {
            return Err(ParticleError::EmptyStepConfigId);
        }
        if !self.fixed_step_seconds.is_finite() || self.fixed_step_seconds <= 0.0 {
            return Err(ParticleError::InvalidFixedStep);
        }
        if self.max_steps_per_frame == 0 {
            return Err(ParticleError::InvalidMaxSteps);
        }
        if !self.neighbor_radius.is_finite() || self.neighbor_radius < 0.0 {
            return Err(ParticleError::InvalidNeighborConfig(
                "neighbor_radius must be finite and non-negative",
            ));
        }
        if !self.neighbor_repulsion_strength.is_finite() || self.neighbor_repulsion_strength < 0.0 {
            return Err(ParticleError::InvalidNeighborConfig(
                "neighbor_repulsion_strength must be finite and non-negative",
            ));
        }
        Ok(())
    }
}
