use crate::{MatterFieldError, SURFACE_FIELD_RUNTIME_CONFIG_SCHEMA_ID};

/// Runtime configuration for later fixed-step surface-field dynamics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldRuntimeConfig {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable config identifier.
    pub config_id: String,
    /// Fixed step duration in seconds.
    pub fixed_step_seconds: f32,
    /// Maximum number of fixed steps accepted by one run request.
    pub max_steps_per_run: u32,
    /// Number of same-surface neighbor tiers used by runtime work.
    pub enabled_neighbor_tiers: u8,
    /// Minimum allowed scalar value after clamping.
    pub scalar_clamp_min: f32,
    /// Maximum allowed scalar value after clamping.
    pub scalar_clamp_max: f32,
    /// Maximum allowed vector length after clamping.
    pub vector_clamp_length: f32,
    /// Scalar diffusion coefficient over the sparse surface-neighbor graph.
    pub scalar_diffusion_rate: f32,
    /// Scalar decay coefficient toward zero.
    pub scalar_decay_rate: f32,
    /// Weight applied to second-tier neighbors relative to first-tier links.
    pub second_tier_coupling_weight: f32,
    /// Vector alignment coefficient over the sparse surface-neighbor graph.
    pub vector_alignment_rate: f32,
    /// Vector response coefficient to the active scalar gradient.
    pub vector_gradient_rate: f32,
}

impl Default for SurfaceFieldRuntimeConfig {
    fn default() -> Self {
        Self {
            schema_id: SURFACE_FIELD_RUNTIME_CONFIG_SCHEMA_ID.to_owned(),
            config_id: "fields.runtime.default".to_owned(),
            fixed_step_seconds: 1.0 / 30.0,
            max_steps_per_run: 512,
            enabled_neighbor_tiers: 2,
            scalar_clamp_min: -1.0,
            scalar_clamp_max: 1.0,
            vector_clamp_length: 1.0,
            scalar_diffusion_rate: 2.4,
            scalar_decay_rate: 0.25,
            second_tier_coupling_weight: 0.35,
            vector_alignment_rate: 3.0,
            vector_gradient_rate: 1.6,
        }
    }
}

impl SurfaceFieldRuntimeConfig {
    /// Validates the runtime config contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, IDs, or numeric ranges are
    /// invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_FIELD_RUNTIME_CONFIG_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_FIELD_RUNTIME_CONFIG_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.config_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyRuntimeConfigId);
        }
        if !self.fixed_step_seconds.is_finite() || self.fixed_step_seconds <= 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "fixed_step_seconds must be finite and positive",
            ));
        }
        if self.max_steps_per_run == 0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "max_steps_per_run must be non-zero",
            ));
        }
        if !(1..=2).contains(&self.enabled_neighbor_tiers) {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "enabled_neighbor_tiers must be 1 or 2",
            ));
        }
        if !self.scalar_clamp_min.is_finite()
            || !self.scalar_clamp_max.is_finite()
            || self.scalar_clamp_min >= self.scalar_clamp_max
        {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "scalar clamp range must be finite and increasing",
            ));
        }
        if !self.vector_clamp_length.is_finite() || self.vector_clamp_length <= 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "vector_clamp_length must be finite and positive",
            ));
        }
        if !self.scalar_diffusion_rate.is_finite() || self.scalar_diffusion_rate < 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "scalar_diffusion_rate must be finite and non-negative",
            ));
        }
        if !self.scalar_decay_rate.is_finite() || self.scalar_decay_rate < 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "scalar_decay_rate must be finite and non-negative",
            ));
        }
        if !self.second_tier_coupling_weight.is_finite()
            || !(0.0..=1.0).contains(&self.second_tier_coupling_weight)
        {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "second_tier_coupling_weight must be finite in 0..=1",
            ));
        }
        if !self.vector_alignment_rate.is_finite() || self.vector_alignment_rate < 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "vector_alignment_rate must be finite and non-negative",
            ));
        }
        if !self.vector_gradient_rate.is_finite() || self.vector_gradient_rate < 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "vector_gradient_rate must be finite and non-negative",
            ));
        }
        Ok(())
    }
}
