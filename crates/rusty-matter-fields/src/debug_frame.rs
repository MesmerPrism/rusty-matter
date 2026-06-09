use rusty_matter_model::Vec3;

use crate::{
    MatterFieldError, SurfaceFieldPerturbation, SurfaceFieldPerturbationEffect,
    SurfaceFieldRunSummary, SurfaceFieldState, SurfaceFieldStepDiagnostics, SurfaceFieldSubstrate,
    SurfaceScalarField, SurfaceVectorField, SURFACE_FIELD_DEBUG_FRAME_SCHEMA_ID,
    SURFACE_FIELD_DEBUG_SEQUENCE_SCHEMA_ID,
};

/// One surface-field node record for debug and visualization adapters.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldDebugNode {
    /// Stable node identifier.
    pub node_id: String,
    /// Source mesh sample identifier.
    pub sample_id: String,
    /// Node index within the substrate.
    pub node_index: usize,
    /// Node position.
    pub position: Vec3,
    /// Node normal.
    pub normal: Vec3,
}

/// One directed same-surface neighbor edge.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceFieldDebugEdge {
    /// Source node index.
    pub from: usize,
    /// Target node index.
    pub to: usize,
    /// Neighbor tier, starting at 1.
    pub tier: u8,
}

/// Scalar field layer copied from a surface-field state.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldScalarDebugLayer {
    /// Source field identifier.
    pub field_id: String,
    /// Field kind label.
    pub kind: String,
    /// Minimum scalar value.
    pub min_value: f32,
    /// Maximum scalar value.
    pub max_value: f32,
    /// One scalar per debug node.
    pub values: Vec<f32>,
}

/// Vector field layer copied from a surface-field state.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldVectorDebugLayer {
    /// Source field identifier.
    pub field_id: String,
    /// Field kind label.
    pub kind: String,
    /// Maximum vector length.
    pub max_length: f32,
    /// One vector per debug node.
    pub vectors: Vec<Vec3>,
}

/// Perturbation region copied from scheduled perturbation descriptors.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldDebugPerturbationRegion {
    /// Source perturbation identifier.
    pub perturbation_id: String,
    /// Optional target field identifier.
    pub target_field_id: Option<String>,
    /// Region node indices.
    pub node_indices: Vec<usize>,
    /// Effect label for visualization adapters.
    pub effect_kind: String,
}

/// Policy-free debug frame over one surface-field state.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldDebugFrame {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable frame identifier.
    pub frame_id: String,
    /// Source substrate identifier.
    pub substrate_id: String,
    /// Source state identifier.
    pub state_id: String,
    /// Source surface identifier.
    pub surface_id: String,
    /// Source topology hash.
    pub topology_index_hash: u64,
    /// Fixed-step index represented by this frame.
    pub step_index: u32,
    /// State time in seconds represented by this frame.
    pub time_seconds: f32,
    /// Surface-field nodes.
    pub nodes: Vec<SurfaceFieldDebugNode>,
    /// Directed same-surface neighbor edges.
    pub edges: Vec<SurfaceFieldDebugEdge>,
    /// Scalar field layers.
    pub scalar_layers: Vec<SurfaceFieldScalarDebugLayer>,
    /// Vector field layers.
    pub vector_layers: Vec<SurfaceFieldVectorDebugLayer>,
    /// Perturbation regions.
    pub perturbation_regions: Vec<SurfaceFieldDebugPerturbationRegion>,
}

impl SurfaceFieldDebugFrame {
    /// Creates a debug frame from validated surface-field contracts.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when substrate, state, or perturbations are
    /// invalid.
    pub fn from_contracts(
        frame_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
        state: &SurfaceFieldState,
        perturbations: &[SurfaceFieldPerturbation],
    ) -> Result<Self, MatterFieldError> {
        substrate.validate()?;
        state.validate()?;
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

        let frame = Self {
            schema_id: SURFACE_FIELD_DEBUG_FRAME_SCHEMA_ID.to_owned(),
            frame_id: frame_id.into(),
            substrate_id: substrate.substrate_id.clone(),
            state_id: state.state_id.clone(),
            surface_id: substrate.surface_id.clone(),
            topology_index_hash: substrate.topology_key.index_hash,
            step_index: 0,
            time_seconds: state.time_seconds,
            nodes: debug_nodes(substrate),
            edges: debug_edges(substrate),
            scalar_layers: state
                .scalar_fields
                .iter()
                .map(debug_scalar_layer)
                .collect::<Result<Vec<_>, _>>()?,
            vector_layers: state
                .vector_fields
                .iter()
                .map(debug_vector_layer)
                .collect::<Result<Vec<_>, _>>()?,
            perturbation_regions: perturbations
                .iter()
                .map(debug_perturbation_region)
                .collect(),
        };
        frame.validate()?;
        Ok(frame)
    }

    /// Creates a debug frame from a state at a specific fixed-step index.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when substrate, state, or perturbations are
    /// invalid.
    pub fn from_state_at_step(
        frame_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
        state: &SurfaceFieldState,
        perturbations: &[SurfaceFieldPerturbation],
        step_index: u32,
    ) -> Result<Self, MatterFieldError> {
        let mut frame = Self::from_contracts(frame_id, substrate, state, perturbations)?;
        frame.step_index = step_index;
        frame.time_seconds = state.time_seconds;
        frame.validate()?;
        Ok(frame)
    }

    /// Validates the debug-frame contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, IDs, counts, or values are
    /// invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_FIELD_DEBUG_FRAME_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_FIELD_DEBUG_FRAME_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.frame_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "debug frame id must not be empty",
            ));
        }
        if self.substrate_id.trim().is_empty() {
            return Err(MatterFieldError::EmptySubstrateId);
        }
        if self.state_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyStateId);
        }
        if self.surface_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidSubstrate(
                "surface id must not be empty",
            ));
        }
        if !self.time_seconds.is_finite() || self.time_seconds < 0.0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "debug frame time must be finite and non-negative",
            ));
        }
        let node_count = self.nodes.len();
        if node_count == 0 {
            return Err(MatterFieldError::InvalidSubstrate(
                "debug frame must contain nodes",
            ));
        }
        for (expected_index, node) in self.nodes.iter().enumerate() {
            if node.node_index != expected_index
                || node.node_id.trim().is_empty()
                || node.sample_id.trim().is_empty()
                || !node.position.is_finite()
                || !node.normal.is_finite()
            {
                return Err(MatterFieldError::InvalidSubstrate(
                    "debug frame node must match node order and finite vectors",
                ));
            }
        }
        for edge in &self.edges {
            if edge.from >= node_count || edge.to >= node_count || edge.from == edge.to {
                return Err(MatterFieldError::InvalidRunSummary(
                    "debug frame edges must target valid distinct nodes",
                ));
            }
            if !(1..=2).contains(&edge.tier) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "debug frame edge tier must be 1 or 2",
                ));
            }
        }
        for layer in &self.scalar_layers {
            validate_scalar_layer(layer, node_count)?;
        }
        for layer in &self.vector_layers {
            validate_vector_layer(layer, node_count)?;
        }
        for region in &self.perturbation_regions {
            if region.perturbation_id.trim().is_empty() || region.effect_kind.trim().is_empty() {
                return Err(MatterFieldError::InvalidPerturbation(
                    "debug perturbation region ids must not be empty",
                ));
            }
            for &node_index in &region.node_indices {
                if node_index >= node_count {
                    return Err(MatterFieldError::InvalidPerturbationNode {
                        node_index,
                        node_count,
                    });
                }
            }
        }
        Ok(())
    }
}

fn debug_nodes(substrate: &SurfaceFieldSubstrate) -> Vec<SurfaceFieldDebugNode> {
    substrate
        .nodes
        .iter()
        .map(|node| SurfaceFieldDebugNode {
            node_id: node.node_id.clone(),
            sample_id: node.sample_id.clone(),
            node_index: node.node_index,
            position: node.position,
            normal: node.normal,
        })
        .collect()
}

fn debug_edges(substrate: &SurfaceFieldSubstrate) -> Vec<SurfaceFieldDebugEdge> {
    let mut edges = Vec::new();
    for node in &substrate.nodes {
        edges.extend(
            node.first_tier_neighbors
                .iter()
                .copied()
                .map(|to| SurfaceFieldDebugEdge {
                    from: node.node_index,
                    to,
                    tier: 1,
                }),
        );
        edges.extend(
            node.second_tier_neighbors
                .iter()
                .copied()
                .map(|to| SurfaceFieldDebugEdge {
                    from: node.node_index,
                    to,
                    tier: 2,
                }),
        );
    }
    edges
}

fn debug_scalar_layer(
    field: &SurfaceScalarField,
) -> Result<SurfaceFieldScalarDebugLayer, MatterFieldError> {
    let Some((min_value, max_value)) = field.value_range() else {
        return Err(MatterFieldError::InvalidField(
            "scalar debug layer must not be empty",
        ));
    };
    Ok(SurfaceFieldScalarDebugLayer {
        field_id: field.field_id.clone(),
        kind: format!("{:?}", field.kind),
        min_value,
        max_value,
        values: field.values.clone(),
    })
}

fn debug_vector_layer(
    field: &SurfaceVectorField,
) -> Result<SurfaceFieldVectorDebugLayer, MatterFieldError> {
    let Some(max_length) = field.max_vector_length() else {
        return Err(MatterFieldError::InvalidField(
            "vector debug layer must not be empty",
        ));
    };
    Ok(SurfaceFieldVectorDebugLayer {
        field_id: field.field_id.clone(),
        kind: format!("{:?}", field.kind),
        max_length,
        vectors: field.vectors.clone(),
    })
}

fn debug_perturbation_region(
    perturbation: &SurfaceFieldPerturbation,
) -> SurfaceFieldDebugPerturbationRegion {
    SurfaceFieldDebugPerturbationRegion {
        perturbation_id: perturbation.perturbation_id.clone(),
        target_field_id: perturbation.target_field_id.clone(),
        node_indices: perturbation.node_indices.clone(),
        effect_kind: perturbation_effect_label(&perturbation.effect).to_owned(),
    }
}

fn perturbation_effect_label(effect: &SurfaceFieldPerturbationEffect) -> &'static str {
    match effect {
        SurfaceFieldPerturbationEffect::NormalPolarity { .. } => "normal_polarity",
        SurfaceFieldPerturbationEffect::WoundRegion { .. } => "wound_region",
        SurfaceFieldPerturbationEffect::DepolarizeRegion { .. } => "depolarize_region",
        SurfaceFieldPerturbationEffect::CouplingMultiplierChange { .. } => {
            "coupling_multiplier_change"
        }
        SurfaceFieldPerturbationEffect::PolarityInversion => "polarity_inversion",
    }
}

fn validate_scalar_layer(
    layer: &SurfaceFieldScalarDebugLayer,
    node_count: usize,
) -> Result<(), MatterFieldError> {
    if layer.field_id.trim().is_empty() || layer.kind.trim().is_empty() {
        return Err(MatterFieldError::EmptyFieldId);
    }
    if layer.values.len() != node_count {
        return Err(MatterFieldError::NodeCountMismatch {
            expected: node_count,
            actual: layer.values.len(),
        });
    }
    if !layer.min_value.is_finite()
        || !layer.max_value.is_finite()
        || layer.min_value > layer.max_value
    {
        return Err(MatterFieldError::InvalidField(
            "scalar debug range must be finite and increasing",
        ));
    }
    if !layer.values.iter().all(|value| value.is_finite()) {
        return Err(MatterFieldError::InvalidField(
            "scalar debug values must be finite",
        ));
    }
    Ok(())
}

fn validate_vector_layer(
    layer: &SurfaceFieldVectorDebugLayer,
    node_count: usize,
) -> Result<(), MatterFieldError> {
    if layer.field_id.trim().is_empty() || layer.kind.trim().is_empty() {
        return Err(MatterFieldError::EmptyFieldId);
    }
    if layer.vectors.len() != node_count {
        return Err(MatterFieldError::NodeCountMismatch {
            expected: node_count,
            actual: layer.vectors.len(),
        });
    }
    if !layer.max_length.is_finite() || layer.max_length < 0.0 {
        return Err(MatterFieldError::InvalidField(
            "vector debug max length must be finite and non-negative",
        ));
    }
    if !layer.vectors.iter().copied().all(Vec3::is_finite) {
        return Err(MatterFieldError::InvalidField(
            "vector debug values must be finite",
        ));
    }
    Ok(())
}

/// Policy-free debug sequence over a fixed-step surface-field run.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldDebugFrameSequence {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable sequence identifier.
    pub sequence_id: String,
    /// Source substrate identifier.
    pub substrate_id: String,
    /// Source surface identifier.
    pub surface_id: String,
    /// Initial state identifier.
    pub initial_state_id: String,
    /// Fixed step duration in seconds.
    pub fixed_step_seconds: f32,
    /// Total executed fixed steps.
    pub step_count: u32,
    /// Step interval between emitted debug frames.
    pub frame_stride: u32,
    /// Per-step diagnostics for the full run.
    pub diagnostics: Vec<SurfaceFieldStepDiagnostics>,
    /// Summary over the final state.
    pub summary: SurfaceFieldRunSummary,
    /// Emitted debug frames.
    pub frames: Vec<SurfaceFieldDebugFrame>,
}

impl SurfaceFieldDebugFrameSequence {
    /// Creates and validates a debug frame sequence.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the sequence contract is invalid.
    pub fn new(
        sequence_id: impl Into<String>,
        fixed_step_seconds: f32,
        step_count: u32,
        frame_stride: u32,
        diagnostics: Vec<SurfaceFieldStepDiagnostics>,
        summary: SurfaceFieldRunSummary,
        frames: Vec<SurfaceFieldDebugFrame>,
    ) -> Result<Self, MatterFieldError> {
        let Some(first_frame) = frames.first() else {
            return Err(MatterFieldError::InvalidRunSummary(
                "debug sequence must contain at least one frame",
            ));
        };
        let sequence = Self {
            schema_id: SURFACE_FIELD_DEBUG_SEQUENCE_SCHEMA_ID.to_owned(),
            sequence_id: sequence_id.into(),
            substrate_id: first_frame.substrate_id.clone(),
            surface_id: first_frame.surface_id.clone(),
            initial_state_id: first_frame.state_id.clone(),
            fixed_step_seconds,
            step_count,
            frame_stride,
            diagnostics,
            summary,
            frames,
        };
        sequence.validate()?;
        Ok(sequence)
    }

    /// Validates the debug sequence.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, IDs, frame ordering, or
    /// diagnostics are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_FIELD_DEBUG_SEQUENCE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_FIELD_DEBUG_SEQUENCE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.sequence_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "debug sequence id must not be empty",
            ));
        }
        if self.substrate_id.trim().is_empty() {
            return Err(MatterFieldError::EmptySubstrateId);
        }
        if self.surface_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidSubstrate(
                "debug sequence surface id must not be empty",
            ));
        }
        if self.initial_state_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyStateId);
        }
        if !self.fixed_step_seconds.is_finite() || self.fixed_step_seconds <= 0.0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "debug sequence fixed step must be finite and positive",
            ));
        }
        if self.frame_stride == 0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "debug sequence frame stride must be non-zero",
            ));
        }
        if self.diagnostics.len() != self.step_count as usize {
            return Err(MatterFieldError::InvalidRunSummary(
                "debug sequence diagnostics must match step count",
            ));
        }
        self.summary.validate()?;
        if self.summary.step_count != self.step_count {
            return Err(MatterFieldError::InvalidRunSummary(
                "debug sequence summary step count must match",
            ));
        }
        if self.frames.is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "debug sequence must contain frames",
            ));
        }
        let node_count = self.summary.node_count;
        for diagnostic in &self.diagnostics {
            diagnostic.validate(node_count)?;
        }
        let mut previous_step = None::<u32>;
        for frame in &self.frames {
            frame.validate()?;
            if frame.substrate_id != self.substrate_id || frame.surface_id != self.surface_id {
                return Err(MatterFieldError::InvalidRunSummary(
                    "debug sequence frame source must match sequence",
                ));
            }
            if frame.step_index > self.step_count {
                return Err(MatterFieldError::InvalidRunSummary(
                    "debug sequence frame step must not exceed step count",
                ));
            }
            if previous_step.is_some_and(|step| frame.step_index <= step) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "debug sequence frame steps must be increasing",
                ));
            }
            previous_step = Some(frame.step_index);
        }
        Ok(())
    }
}
