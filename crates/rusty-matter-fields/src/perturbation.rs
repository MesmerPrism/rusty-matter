use rusty_matter_model::Vec3;

use crate::{MatterFieldError, SURFACE_FIELD_PERTURBATION_SCHEMA_ID};

/// Schematic effect carried by a surface-field perturbation descriptor.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceFieldPerturbationEffect {
    /// Initialize or overwrite polarity with the supplied direction.
    NormalPolarity {
        /// Target polarity vector.
        vector: Vec3,
    },
    /// Raise or set a wound signal in a region.
    WoundRegion {
        /// Schematic wound signal value.
        signal_value: f32,
    },
    /// Apply a schematic scalar offset in a region.
    DepolarizeRegion {
        /// Scalar offset; sign decides depolarizing or hyperpolarizing direction.
        delta: f32,
    },
    /// Change local coupling strength in a region.
    CouplingMultiplierChange {
        /// Coupling multiplier.
        multiplier: f32,
    },
    /// Invert polarity in a region.
    PolarityInversion,
}

/// Scheduled perturbation descriptor over surface-field nodes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldPerturbation {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable perturbation identifier.
    pub perturbation_id: String,
    /// Optional target field identifier.
    pub target_field_id: Option<String>,
    /// Target node indices.
    pub node_indices: Vec<usize>,
    /// First step where this perturbation may apply.
    pub start_step: u32,
    /// Number of steps covered by the descriptor.
    pub duration_steps: u32,
    /// Perturbation effect payload.
    pub effect: SurfaceFieldPerturbationEffect,
}

impl SurfaceFieldPerturbation {
    /// Creates a perturbation descriptor.
    #[must_use]
    pub fn new(
        perturbation_id: impl Into<String>,
        target_field_id: Option<String>,
        node_indices: Vec<usize>,
        effect: SurfaceFieldPerturbationEffect,
    ) -> Self {
        Self {
            schema_id: SURFACE_FIELD_PERTURBATION_SCHEMA_ID.to_owned(),
            perturbation_id: perturbation_id.into(),
            target_field_id,
            node_indices,
            start_step: 0,
            duration_steps: 1,
            effect,
        }
    }

    /// Validates the perturbation against a substrate node count.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when metadata, targets, or effect values
    /// are invalid.
    pub fn validate(&self, node_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_FIELD_PERTURBATION_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_FIELD_PERTURBATION_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.perturbation_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyPerturbationId);
        }
        if self
            .target_field_id
            .as_ref()
            .is_some_and(|field_id| field_id.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidPerturbation(
                "target field id must not be empty when present",
            ));
        }
        if node_count == 0 {
            return Err(MatterFieldError::InvalidPerturbation(
                "node count must be non-zero",
            ));
        }
        if self.node_indices.is_empty() {
            return Err(MatterFieldError::InvalidPerturbation(
                "perturbation must target at least one node",
            ));
        }
        let mut seen = Vec::with_capacity(self.node_indices.len());
        for &node_index in &self.node_indices {
            if node_index >= node_count {
                return Err(MatterFieldError::InvalidPerturbationNode {
                    node_index,
                    node_count,
                });
            }
            if seen.contains(&node_index) {
                return Err(MatterFieldError::DuplicatePerturbationNode { node_index });
            }
            seen.push(node_index);
        }
        if self.duration_steps == 0 {
            return Err(MatterFieldError::InvalidPerturbation(
                "duration_steps must be non-zero",
            ));
        }
        validate_effect(&self.effect)?;
        Ok(())
    }
}

fn validate_effect(effect: &SurfaceFieldPerturbationEffect) -> Result<(), MatterFieldError> {
    match effect {
        SurfaceFieldPerturbationEffect::NormalPolarity { vector } => {
            if !vector.is_finite() || vector.length_squared() <= 1.0e-10 {
                return Err(MatterFieldError::InvalidPerturbation(
                    "normal polarity vector must be finite and non-zero",
                ));
            }
        }
        SurfaceFieldPerturbationEffect::WoundRegion { signal_value } => {
            if !signal_value.is_finite() {
                return Err(MatterFieldError::InvalidPerturbation(
                    "wound signal value must be finite",
                ));
            }
        }
        SurfaceFieldPerturbationEffect::DepolarizeRegion { delta } => {
            if !delta.is_finite() {
                return Err(MatterFieldError::InvalidPerturbation(
                    "depolarization delta must be finite",
                ));
            }
        }
        SurfaceFieldPerturbationEffect::CouplingMultiplierChange { multiplier } => {
            if !multiplier.is_finite() || *multiplier < 0.0 {
                return Err(MatterFieldError::InvalidPerturbation(
                    "coupling multiplier must be finite and non-negative",
                ));
            }
        }
        SurfaceFieldPerturbationEffect::PolarityInversion => {}
    }
    Ok(())
}
