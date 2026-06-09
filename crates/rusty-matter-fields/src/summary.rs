use crate::{
    MatterFieldError, SurfaceFieldPerturbation, SurfaceFieldRuntimeConfig, SurfaceFieldState,
    SurfaceFieldSubstrate, SURFACE_FIELD_RUN_SUMMARY_SCHEMA_ID,
    SURFACE_FIELD_STEP_DIAGNOSTICS_SCHEMA_ID,
};

/// Per-step diagnostic contract for later field dynamics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldStepDiagnostics {
    /// Schema identifier.
    pub schema_id: String,
    /// Step index.
    pub step_index: u32,
    /// Number of scalar fields visited.
    pub scalar_field_count: usize,
    /// Number of vector fields visited.
    pub vector_field_count: usize,
    /// Number of nodes updated.
    pub updated_nodes: usize,
    /// Number of scalar values clamped.
    pub clamped_scalars: usize,
    /// Number of vector values clamped.
    pub clamped_vectors: usize,
    /// Number of active perturbations applied before this step.
    pub active_perturbations: usize,
    /// Number of sparse neighbor links visited by update kernels.
    pub neighbor_links_visited: usize,
    /// Number of node updates rejected.
    pub rejected_nodes: usize,
}

impl SurfaceFieldStepDiagnostics {
    /// Creates zeroed diagnostics for a step.
    #[must_use]
    pub fn empty(step_index: u32) -> Self {
        Self {
            schema_id: SURFACE_FIELD_STEP_DIAGNOSTICS_SCHEMA_ID.to_owned(),
            step_index,
            scalar_field_count: 0,
            vector_field_count: 0,
            updated_nodes: 0,
            clamped_scalars: 0,
            clamped_vectors: 0,
            active_perturbations: 0,
            neighbor_links_visited: 0,
            rejected_nodes: 0,
        }
    }

    /// Validates diagnostic counts against a substrate node count.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema or counts are invalid.
    pub fn validate(&self, node_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_FIELD_STEP_DIAGNOSTICS_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_FIELD_STEP_DIAGNOSTICS_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.updated_nodes > node_count || self.rejected_nodes > node_count {
            return Err(MatterFieldError::InvalidRunSummary(
                "step diagnostic node counts must not exceed node count",
            ));
        }
        Ok(())
    }
}

/// Contract summary for a surface-field run or pre-run validation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldRunSummary {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable summary identifier.
    pub summary_id: String,
    /// Source substrate identifier.
    pub substrate_id: String,
    /// Source state identifier.
    pub state_id: String,
    /// Source runtime config identifier.
    pub runtime_config_id: String,
    /// Number of substrate nodes.
    pub node_count: usize,
    /// Number of scalar fields.
    pub scalar_field_count: usize,
    /// Number of vector fields.
    pub vector_field_count: usize,
    /// Number of validated perturbations.
    pub perturbation_count: usize,
    /// Directed first-tier edge count.
    pub first_tier_edge_count: usize,
    /// Directed second-tier edge count.
    pub second_tier_edge_count: usize,
    /// Executed fixed steps.
    pub step_count: u32,
    /// Minimum scalar value in the summarized state.
    pub scalar_min: Option<f32>,
    /// Maximum scalar value in the summarized state.
    pub scalar_max: Option<f32>,
    /// Maximum vector length in the summarized state.
    pub max_vector_length: Option<f32>,
}

impl SurfaceFieldRunSummary {
    /// Creates a zero-step contract summary from validated inputs.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when any input contract is invalid.
    pub fn from_contracts(
        summary_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
        state: &SurfaceFieldState,
        config: &SurfaceFieldRuntimeConfig,
        perturbations: &[SurfaceFieldPerturbation],
    ) -> Result<Self, MatterFieldError> {
        substrate.validate()?;
        state.validate()?;
        config.validate()?;
        if state.substrate_id != substrate.substrate_id {
            return Err(MatterFieldError::InvalidRunSummary(
                "state substrate id must match substrate",
            ));
        }
        if state.node_count != substrate.node_count() {
            return Err(MatterFieldError::NodeCountMismatch {
                expected: substrate.node_count(),
                actual: state.node_count,
            });
        }
        for perturbation in perturbations {
            perturbation.validate(substrate.node_count())?;
        }

        let (scalar_min, scalar_max) = scalar_range(state);
        let summary = Self {
            schema_id: SURFACE_FIELD_RUN_SUMMARY_SCHEMA_ID.to_owned(),
            summary_id: summary_id.into(),
            substrate_id: substrate.substrate_id.clone(),
            state_id: state.state_id.clone(),
            runtime_config_id: config.config_id.clone(),
            node_count: substrate.node_count(),
            scalar_field_count: state.scalar_fields.len(),
            vector_field_count: state.vector_fields.len(),
            perturbation_count: perturbations.len(),
            first_tier_edge_count: substrate.first_tier_edge_count(),
            second_tier_edge_count: substrate.second_tier_edge_count(),
            step_count: 0,
            scalar_min,
            scalar_max,
            max_vector_length: max_vector_length(state),
        };
        summary.validate()?;
        Ok(summary)
    }

    /// Creates a run summary from a final state and executed step count.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when contracts or the resulting summary are
    /// invalid.
    pub fn from_run(
        summary_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
        final_state: &SurfaceFieldState,
        config: &SurfaceFieldRuntimeConfig,
        perturbations: &[SurfaceFieldPerturbation],
        step_count: u32,
    ) -> Result<Self, MatterFieldError> {
        let mut summary =
            Self::from_contracts(summary_id, substrate, final_state, config, perturbations)?;
        summary.step_count = step_count;
        summary.validate()?;
        Ok(summary)
    }

    /// Validates the run summary contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, IDs, or summary ranges are
    /// invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_FIELD_RUN_SUMMARY_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_FIELD_RUN_SUMMARY_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.summary_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyRunSummaryId);
        }
        if self.substrate_id.trim().is_empty() {
            return Err(MatterFieldError::EmptySubstrateId);
        }
        if self.state_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyStateId);
        }
        if self.runtime_config_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyRuntimeConfigId);
        }
        if self.node_count == 0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "node_count must be non-zero",
            ));
        }
        if self.scalar_field_count == 0 && self.vector_field_count == 0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "summary must cover at least one field",
            ));
        }
        if let (Some(min_value), Some(max_value)) = (self.scalar_min, self.scalar_max) {
            if !min_value.is_finite() || !max_value.is_finite() || min_value > max_value {
                return Err(MatterFieldError::InvalidRunSummary(
                    "scalar range must be finite and increasing",
                ));
            }
        } else if self.scalar_min.is_some() || self.scalar_max.is_some() {
            return Err(MatterFieldError::InvalidRunSummary(
                "scalar range endpoints must both be present or absent",
            ));
        }
        if self
            .max_vector_length
            .is_some_and(|length| !length.is_finite() || length < 0.0)
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "max vector length must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

fn scalar_range(state: &SurfaceFieldState) -> (Option<f32>, Option<f32>) {
    let mut min_value = None::<f32>;
    let mut max_value = None::<f32>;
    for field in &state.scalar_fields {
        if let Some((field_min, field_max)) = field.value_range() {
            min_value = Some(min_value.map_or(field_min, |current| current.min(field_min)));
            max_value = Some(max_value.map_or(field_max, |current| current.max(field_max)));
        }
    }
    (min_value, max_value)
}

fn max_vector_length(state: &SurfaceFieldState) -> Option<f32> {
    state
        .vector_fields
        .iter()
        .filter_map(|field| field.max_vector_length())
        .max_by(f32::total_cmp)
}
