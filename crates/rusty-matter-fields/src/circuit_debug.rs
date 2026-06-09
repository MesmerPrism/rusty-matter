use crate::{
    BioelectricCircuitRuntime, BioelectricCircuitState, BioelectricCircuitStepDiagnostics,
    MatterFieldError, SurfaceFieldDebugEdge, SurfaceFieldDebugNode, SurfaceFieldSubstrate,
    BIOELECTRIC_CIRCUIT_DEBUG_FRAME_SCHEMA_ID, BIOELECTRIC_CIRCUIT_DEBUG_SEQUENCE_SCHEMA_ID,
};

/// One downstream bioelectric readout layer copied into a debug frame.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricReadoutDebugLayer {
    /// Source readout layer identifier.
    pub layer_id: String,
    /// Minimum value in this frame.
    pub min_value: f32,
    /// Maximum value in this frame.
    pub max_value: f32,
    /// One readout value per debug node.
    pub values: Vec<f32>,
}

/// Policy-free debug frame over one bioelectric circuit state.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricCircuitDebugFrame {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable frame identifier.
    pub frame_id: String,
    /// Source substrate identifier.
    pub substrate_id: String,
    /// Source circuit state identifier.
    pub circuit_id: String,
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
    /// Voltage unit label copied from the source state.
    pub voltage_unit: String,
    /// Minimum voltage in this frame.
    pub voltage_min: f32,
    /// Maximum voltage in this frame.
    pub voltage_max: f32,
    /// One voltage value per debug node.
    pub voltage_values: Vec<f32>,
    /// Optional hysteresis memory value per debug node.
    pub memory_values: Option<Vec<f32>>,
    /// Voltage-driven downstream readout layers.
    pub readout_layers: Vec<BioelectricReadoutDebugLayer>,
    /// Optional diagnostics for the step that produced this frame.
    pub diagnostics: Option<BioelectricCircuitStepDiagnostics>,
}

impl BioelectricCircuitDebugFrame {
    /// Creates a debug frame from a circuit state at a fixed-step index.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when substrate, state, or diagnostics are
    /// invalid.
    pub fn from_state_at_step(
        frame_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
        state: &BioelectricCircuitState,
        step_index: u32,
        diagnostics: Option<BioelectricCircuitStepDiagnostics>,
    ) -> Result<Self, MatterFieldError> {
        validate_state_for_substrate(substrate, state)?;
        if let Some(diagnostic) = &diagnostics {
            diagnostic.validate(state.node_count, state.conductance_edges.len())?;
        }
        let Some((voltage_min, voltage_max)) = value_range(&state.voltage.values) else {
            return Err(MatterFieldError::InvalidField(
                "bioelectric debug voltage values must not be empty",
            ));
        };

        let frame = Self {
            schema_id: BIOELECTRIC_CIRCUIT_DEBUG_FRAME_SCHEMA_ID.to_owned(),
            frame_id: frame_id.into(),
            substrate_id: substrate.substrate_id.clone(),
            circuit_id: state.circuit_id.clone(),
            surface_id: substrate.surface_id.clone(),
            topology_index_hash: substrate.topology_key.index_hash,
            step_index,
            time_seconds: state.time_seconds,
            nodes: debug_nodes(substrate),
            edges: debug_edges(substrate),
            voltage_unit: format!("{:?}", state.voltage.unit),
            voltage_min,
            voltage_max,
            voltage_values: state.voltage.values.clone(),
            memory_values: state.memory.as_ref().map(|memory| memory.values.clone()),
            readout_layers: state
                .readout_layers
                .iter()
                .map(|layer| {
                    let Some((min_value, max_value)) = value_range(&layer.values) else {
                        return Err(MatterFieldError::InvalidField(
                            "bioelectric debug readout values must not be empty",
                        ));
                    };
                    Ok(BioelectricReadoutDebugLayer {
                        layer_id: layer.layer_id.clone(),
                        min_value,
                        max_value,
                        values: layer.values.clone(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            diagnostics,
        };
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
        if self.schema_id != BIOELECTRIC_CIRCUIT_DEBUG_FRAME_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_CIRCUIT_DEBUG_FRAME_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.frame_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug frame id must not be empty",
            ));
        }
        if self.substrate_id.trim().is_empty() {
            return Err(MatterFieldError::EmptySubstrateId);
        }
        if self.circuit_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyStateId);
        }
        if self.surface_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidSubstrate(
                "bioelectric debug frame surface id must not be empty",
            ));
        }
        if !self.time_seconds.is_finite() || self.time_seconds < 0.0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug frame time must be finite and non-negative",
            ));
        }
        if self.voltage_unit.trim().is_empty() {
            return Err(MatterFieldError::InvalidField(
                "bioelectric debug voltage unit must not be empty",
            ));
        }
        let node_count = self.nodes.len();
        if node_count == 0 {
            return Err(MatterFieldError::InvalidSubstrate(
                "bioelectric debug frame must contain nodes",
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
                    "bioelectric debug frame nodes must match node order and finite vectors",
                ));
            }
        }
        for edge in &self.edges {
            if edge.from >= node_count || edge.to >= node_count || edge.from == edge.to {
                return Err(MatterFieldError::InvalidRunSummary(
                    "bioelectric debug frame edges must target valid distinct nodes",
                ));
            }
            if !(1..=2).contains(&edge.tier) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "bioelectric debug frame edge tier must be 1 or 2",
                ));
            }
        }
        validate_range(
            self.voltage_min,
            self.voltage_max,
            "bioelectric voltage debug range",
        )?;
        validate_value_buffer(
            &self.voltage_values,
            node_count,
            "bioelectric voltage debug",
        )?;
        if let Some(values) = &self.memory_values {
            validate_value_buffer(values, node_count, "bioelectric memory debug")?;
            if !values.iter().all(|value| (0.0..=1.0).contains(value)) {
                return Err(MatterFieldError::InvalidField(
                    "bioelectric memory debug values must be in 0..=1",
                ));
            }
        }
        for layer in &self.readout_layers {
            validate_readout_layer(layer, node_count)?;
        }
        if let Some(diagnostic) = &self.diagnostics {
            diagnostic.validate(node_count, self.edges.len())?;
        }
        Ok(())
    }
}

/// Policy-free debug sequence over a fixed-step bioelectric circuit run.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricCircuitDebugSequence {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable sequence identifier.
    pub sequence_id: String,
    /// Source substrate identifier.
    pub substrate_id: String,
    /// Source surface identifier.
    pub surface_id: String,
    /// Initial circuit state identifier.
    pub initial_circuit_id: String,
    /// Fixed step duration in seconds.
    pub fixed_step_seconds: f32,
    /// Total executed fixed steps.
    pub step_count: u32,
    /// Step interval between emitted debug frames.
    pub frame_stride: u32,
    /// Per-step diagnostics for the full run.
    pub diagnostics: Vec<BioelectricCircuitStepDiagnostics>,
    /// Emitted debug frames.
    pub frames: Vec<BioelectricCircuitDebugFrame>,
}

impl BioelectricCircuitDebugSequence {
    /// Runs a deterministic fixed-step circuit sequence and emits debug frames.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the circuit contracts or generated
    /// sequence are invalid.
    pub fn run_fixed(
        sequence_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
        runtime: &BioelectricCircuitRuntime,
        initial_state: &BioelectricCircuitState,
        step_count: u32,
        frame_stride: u32,
    ) -> Result<Self, MatterFieldError> {
        if frame_stride == 0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug sequence frame stride must be non-zero",
            ));
        }
        runtime.validate_contracts(substrate, initial_state)?;
        let fixed_step_seconds = runtime.config().fixed_step_seconds;
        let mut state = initial_state.clone();
        let mut diagnostics = Vec::with_capacity(step_count as usize);
        let mut frames = vec![BioelectricCircuitDebugFrame::from_state_at_step(
            format!("{}.frame.{:04}", initial_state.circuit_id, 0),
            substrate,
            &state,
            0,
            None,
        )?];

        for step_index in 0..step_count {
            let diagnostic = runtime.step_fixed(substrate, &mut state, step_index)?;
            let emitted_step = step_index + 1;
            let should_emit = emitted_step % frame_stride == 0 || emitted_step == step_count;
            if should_emit {
                frames.push(BioelectricCircuitDebugFrame::from_state_at_step(
                    format!("{}.frame.{emitted_step:04}", initial_state.circuit_id),
                    substrate,
                    &state,
                    emitted_step,
                    Some(diagnostic.clone()),
                )?);
            }
            diagnostics.push(diagnostic);
        }

        Self::new(
            sequence_id,
            fixed_step_seconds,
            step_count,
            frame_stride,
            diagnostics,
            frames,
        )
    }

    /// Creates and validates a debug sequence.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the sequence contract is invalid.
    pub fn new(
        sequence_id: impl Into<String>,
        fixed_step_seconds: f32,
        step_count: u32,
        frame_stride: u32,
        diagnostics: Vec<BioelectricCircuitStepDiagnostics>,
        frames: Vec<BioelectricCircuitDebugFrame>,
    ) -> Result<Self, MatterFieldError> {
        let Some(first_frame) = frames.first() else {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug sequence must contain at least one frame",
            ));
        };
        let sequence = Self {
            schema_id: BIOELECTRIC_CIRCUIT_DEBUG_SEQUENCE_SCHEMA_ID.to_owned(),
            sequence_id: sequence_id.into(),
            substrate_id: first_frame.substrate_id.clone(),
            surface_id: first_frame.surface_id.clone(),
            initial_circuit_id: first_frame.circuit_id.clone(),
            fixed_step_seconds,
            step_count,
            frame_stride,
            diagnostics,
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
        if self.schema_id != BIOELECTRIC_CIRCUIT_DEBUG_SEQUENCE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_CIRCUIT_DEBUG_SEQUENCE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.sequence_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug sequence id must not be empty",
            ));
        }
        if self.substrate_id.trim().is_empty() {
            return Err(MatterFieldError::EmptySubstrateId);
        }
        if self.surface_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidSubstrate(
                "bioelectric debug sequence surface id must not be empty",
            ));
        }
        if self.initial_circuit_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyStateId);
        }
        if !self.fixed_step_seconds.is_finite() || self.fixed_step_seconds <= 0.0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug sequence fixed step must be finite and positive",
            ));
        }
        if self.frame_stride == 0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug sequence frame stride must be non-zero",
            ));
        }
        if self.diagnostics.len() != self.step_count as usize {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug sequence diagnostics must match step count",
            ));
        }
        if self.frames.is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug sequence must contain frames",
            ));
        }

        let node_count = self.frames[0].nodes.len();
        let edge_count = self.frames[0].edges.len();
        for diagnostic in &self.diagnostics {
            diagnostic.validate(node_count, edge_count)?;
        }

        let mut previous_step = None::<u32>;
        for frame in &self.frames {
            frame.validate()?;
            if frame.substrate_id != self.substrate_id
                || frame.surface_id != self.surface_id
                || frame.circuit_id != self.initial_circuit_id
            {
                return Err(MatterFieldError::InvalidRunSummary(
                    "bioelectric debug sequence frame source must match sequence",
                ));
            }
            if frame.step_index > self.step_count {
                return Err(MatterFieldError::InvalidRunSummary(
                    "bioelectric debug sequence frame step must not exceed step count",
                ));
            }
            if previous_step.is_some_and(|step| frame.step_index <= step) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "bioelectric debug sequence frame steps must be increasing",
                ));
            }
            previous_step = Some(frame.step_index);
        }
        if self
            .frames
            .first()
            .is_some_and(|frame| frame.step_index != 0)
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug sequence must start at step 0",
            ));
        }
        if self.step_count > 0
            && self
                .frames
                .last()
                .is_some_and(|frame| frame.step_index != self.step_count)
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric debug sequence final frame must match step count",
            ));
        }
        Ok(())
    }
}

fn validate_state_for_substrate(
    substrate: &SurfaceFieldSubstrate,
    state: &BioelectricCircuitState,
) -> Result<(), MatterFieldError> {
    substrate.validate()?;
    state.validate()?;
    if state.substrate_id != substrate.substrate_id {
        return Err(MatterFieldError::InvalidRunSummary(
            "bioelectric debug state substrate id must match substrate",
        ));
    }
    if state.node_count != substrate.node_count() {
        return Err(MatterFieldError::NodeCountMismatch {
            expected: substrate.node_count(),
            actual: state.node_count,
        });
    }
    Ok(())
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

fn validate_readout_layer(
    layer: &BioelectricReadoutDebugLayer,
    node_count: usize,
) -> Result<(), MatterFieldError> {
    if layer.layer_id.trim().is_empty() {
        return Err(MatterFieldError::EmptyFieldId);
    }
    validate_range(
        layer.min_value,
        layer.max_value,
        "bioelectric readout debug range",
    )?;
    validate_value_buffer(&layer.values, node_count, "bioelectric readout debug")
}

fn validate_value_buffer(
    values: &[f32],
    node_count: usize,
    label: &'static str,
) -> Result<(), MatterFieldError> {
    if values.len() != node_count {
        return Err(MatterFieldError::NodeCountMismatch {
            expected: node_count,
            actual: values.len(),
        });
    }
    if !values.iter().all(|value| value.is_finite()) {
        return Err(MatterFieldError::InvalidField(
            "bioelectric debug values must be finite",
        ));
    }
    let Some((min_value, max_value)) = value_range(values) else {
        return Err(MatterFieldError::InvalidField(label));
    };
    validate_range(min_value, max_value, label)
}

fn validate_range(
    min_value: f32,
    max_value: f32,
    label: &'static str,
) -> Result<(), MatterFieldError> {
    if !min_value.is_finite() || !max_value.is_finite() || min_value > max_value {
        Err(MatterFieldError::InvalidField(label))
    } else {
        Ok(())
    }
}

fn value_range(values: &[f32]) -> Option<(f32, f32)> {
    values.iter().copied().fold(None, |range, value| {
        range.map_or(Some((value, value)), |(min_value, max_value)| {
            Some((min_value.min(value), max_value.max(value)))
        })
    })
}
