use crate::{
    circuit::validate_circuit_for_substrate, BioelectricCircuitRuntime, BioelectricCircuitState,
    BioelectricCurrentKind, BioelectricCurrentTerm, MatterFieldError, SurfaceFieldSubstrate,
    BIOELECTRIC_CIRCUIT_EDIT_RESULT_SCHEMA_ID, BIOELECTRIC_CIRCUIT_EDIT_SCHEMA_ID,
};

/// Interactive bioelectric circuit mutation requested by a UI, agent, or later
/// command surface.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricCircuitEdit {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable edit identifier for audit and replay.
    pub edit_id: String,
    /// Optional state revision the requester believes it is editing.
    pub expected_revision: Option<u64>,
    /// Typed edit operation.
    pub operation: BioelectricCircuitEditOperation,
}

impl BioelectricCircuitEdit {
    /// Creates an edit request.
    #[must_use]
    pub fn new(
        edit_id: impl Into<String>,
        expected_revision: Option<u64>,
        operation: BioelectricCircuitEditOperation,
    ) -> Self {
        Self {
            schema_id: BIOELECTRIC_CIRCUIT_EDIT_SCHEMA_ID.to_owned(),
            edit_id: edit_id.into(),
            expected_revision,
            operation,
        }
    }

    /// Validates edit metadata and target bounds.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when IDs, numeric values, or explicit
    /// targets are invalid.
    pub fn validate(&self, node_count: usize, edge_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != BIOELECTRIC_CIRCUIT_EDIT_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_CIRCUIT_EDIT_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.edit_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidField(
                "bioelectric edit id must not be empty",
            ));
        }
        self.operation.validate(node_count, edge_count)
    }
}

/// Typed bioelectric circuit edit operation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum BioelectricCircuitEditOperation {
    /// Set one node's voltage to a runtime-clamped value.
    SetNodeVoltage {
        /// Target node index.
        node_index: usize,
        /// Requested voltage value.
        voltage: f32,
    },
    /// Add a runtime-clamped voltage delta to one node.
    AddNodeVoltage {
        /// Target node index.
        node_index: usize,
        /// Requested voltage delta.
        delta: f32,
    },
    /// Set one node's hysteresis memory value to a 0..=1 clamped value.
    SetNodeMemory {
        /// Target node index.
        node_index: usize,
        /// Requested memory value.
        memory_value: f32,
    },
    /// Scale all conductance edges incident on a node.
    ScaleIncidentConductance {
        /// Target node index.
        node_index: usize,
        /// Multiplicative conductance scale.
        scale: f32,
    },
    /// Set a gate threshold and optionally slope for one conductance edge.
    SetEdgeGateThreshold {
        /// Target conductance edge index.
        edge_index: usize,
        /// Requested gate threshold.
        threshold: f32,
        /// Optional replacement gate slope.
        slope: Option<f32>,
    },
    /// Set gate multiplier bounds for one conductance edge.
    SetEdgeGateMultiplierBounds {
        /// Target conductance edge index.
        edge_index: usize,
        /// Requested lower multiplier bound.
        min_multiplier: f32,
        /// Requested upper multiplier bound.
        max_multiplier: f32,
    },
    /// Add a scheduled transient constant current source term.
    AddTransientCurrent {
        /// Stable current term identifier.
        term_id: String,
        /// Target node indices. Empty means all nodes.
        target_node_indices: Vec<usize>,
        /// Current contribution while active.
        current: f32,
        /// First active fixed step.
        start_step: u32,
        /// Active duration in fixed steps.
        duration_steps: u32,
    },
}

impl BioelectricCircuitEditOperation {
    fn validate(&self, node_count: usize, edge_count: usize) -> Result<(), MatterFieldError> {
        match self {
            Self::SetNodeVoltage {
                node_index,
                voltage,
            } => {
                validate_node_index(*node_index, node_count)?;
                validate_finite(*voltage, "bioelectric edit voltage must be finite")?;
            }
            Self::AddNodeVoltage { node_index, delta } => {
                validate_node_index(*node_index, node_count)?;
                validate_finite(*delta, "bioelectric edit voltage delta must be finite")?;
            }
            Self::SetNodeMemory {
                node_index,
                memory_value,
            } => {
                validate_node_index(*node_index, node_count)?;
                validate_finite(
                    *memory_value,
                    "bioelectric edit memory value must be finite",
                )?;
            }
            Self::ScaleIncidentConductance { node_index, scale } => {
                validate_node_index(*node_index, node_count)?;
                if !scale.is_finite() || *scale < 0.0 {
                    return Err(MatterFieldError::InvalidField(
                        "bioelectric edit conductance scale must be finite and non-negative",
                    ));
                }
            }
            Self::SetEdgeGateThreshold {
                edge_index,
                threshold,
                slope,
            } => {
                validate_edge_index(*edge_index, edge_count)?;
                validate_finite(*threshold, "bioelectric edit gate threshold must be finite")?;
                if slope.is_some_and(|value| !value.is_finite() || value == 0.0) {
                    return Err(MatterFieldError::InvalidField(
                        "bioelectric edit gate slope must be finite and non-zero",
                    ));
                }
            }
            Self::SetEdgeGateMultiplierBounds {
                edge_index,
                min_multiplier,
                max_multiplier,
            } => {
                validate_edge_index(*edge_index, edge_count)?;
                if !min_multiplier.is_finite()
                    || !max_multiplier.is_finite()
                    || *min_multiplier < 0.0
                    || *min_multiplier > *max_multiplier
                {
                    return Err(MatterFieldError::InvalidField(
                        "bioelectric edit gate multiplier bounds must be finite, non-negative, and increasing",
                    ));
                }
            }
            Self::AddTransientCurrent {
                term_id,
                target_node_indices,
                current,
                duration_steps,
                ..
            } => {
                if term_id.trim().is_empty() {
                    return Err(MatterFieldError::InvalidField(
                        "bioelectric edit current term id must not be empty",
                    ));
                }
                validate_finite(*current, "bioelectric edit current must be finite")?;
                if *duration_steps == 0 {
                    return Err(MatterFieldError::InvalidField(
                        "bioelectric edit current duration must be non-zero",
                    ));
                }
                validate_node_targets(target_node_indices, node_count)?;
            }
        }
        Ok(())
    }
}

/// Result of one attempted bioelectric circuit edit.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricCircuitEditResult {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable result identifier.
    pub result_id: String,
    /// Source edit identifier.
    pub edit_id: String,
    /// Whether the edit was accepted and applied.
    pub accepted: bool,
    /// Rejection reason when the edit was not applied.
    pub rejection_reason: Option<String>,
    /// Circuit revision before the edit attempt.
    pub revision_before: u64,
    /// Circuit revision after the edit attempt.
    pub revision_after: u64,
    /// Number of values clamped while applying the edit.
    pub clamped_values: usize,
    /// Affected node indices.
    pub affected_node_indices: Vec<usize>,
    /// Affected conductance edge indices.
    pub affected_edge_indices: Vec<usize>,
    /// Affected current term identifiers.
    pub affected_current_term_ids: Vec<String>,
}

impl BioelectricCircuitEditResult {
    fn accepted(
        edit_id: &str,
        revision_before: u64,
        revision_after: u64,
        clamped_values: usize,
        affected_node_indices: Vec<usize>,
        affected_edge_indices: Vec<usize>,
        affected_current_term_ids: Vec<String>,
    ) -> Self {
        Self {
            schema_id: BIOELECTRIC_CIRCUIT_EDIT_RESULT_SCHEMA_ID.to_owned(),
            result_id: format!("{edit_id}.result"),
            edit_id: edit_id.to_owned(),
            accepted: true,
            rejection_reason: None,
            revision_before,
            revision_after,
            clamped_values,
            affected_node_indices,
            affected_edge_indices,
            affected_current_term_ids,
        }
    }

    fn rejected(edit_id: &str, revision: u64, reason: impl Into<String>) -> Self {
        Self {
            schema_id: BIOELECTRIC_CIRCUIT_EDIT_RESULT_SCHEMA_ID.to_owned(),
            result_id: format!("{edit_id}.result"),
            edit_id: edit_id.to_owned(),
            accepted: false,
            rejection_reason: Some(reason.into()),
            revision_before: revision,
            revision_after: revision,
            clamped_values: 0,
            affected_node_indices: Vec::new(),
            affected_edge_indices: Vec::new(),
            affected_current_term_ids: Vec::new(),
        }
    }

    /// Validates the edit result.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, IDs, revision ordering, or
    /// affected target lists are invalid.
    pub fn validate(&self, node_count: usize, edge_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != BIOELECTRIC_CIRCUIT_EDIT_RESULT_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_CIRCUIT_EDIT_RESULT_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.result_id.trim().is_empty() || self.edit_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric edit result ids must not be empty",
            ));
        }
        if self.accepted {
            if self.rejection_reason.is_some() || self.revision_after <= self.revision_before {
                return Err(MatterFieldError::InvalidRunSummary(
                    "accepted bioelectric edit result must advance revision without rejection reason",
                ));
            }
        } else if self
            .rejection_reason
            .as_ref()
            .map_or(true, |reason| reason.trim().is_empty())
            || self.revision_after != self.revision_before
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "rejected bioelectric edit result must preserve revision and include a reason",
            ));
        }
        validate_node_targets(&self.affected_node_indices, node_count)?;
        validate_edge_targets(&self.affected_edge_indices, edge_count)?;
        if self
            .affected_current_term_ids
            .iter()
            .any(|term_id| term_id.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric edit result affected term ids must not be empty",
            ));
        }
        Ok(())
    }
}

impl BioelectricCircuitRuntime {
    /// Applies one validated interactive edit to a circuit state.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the substrate, state, edit shape, or
    /// generated state is invalid. Revision mismatches and unavailable optional
    /// targets are returned as rejected edit results.
    pub fn apply_edit(
        &self,
        substrate: &SurfaceFieldSubstrate,
        state: &mut BioelectricCircuitState,
        edit: &BioelectricCircuitEdit,
    ) -> Result<BioelectricCircuitEditResult, MatterFieldError> {
        validate_circuit_for_substrate(substrate, state)?;
        self.config().validate()?;
        edit.validate(state.node_count, state.conductance_edges.len())?;
        if edit
            .expected_revision
            .is_some_and(|revision| revision != state.revision)
        {
            let result = BioelectricCircuitEditResult::rejected(
                &edit.edit_id,
                state.revision,
                "bioelectric circuit revision mismatch",
            );
            result.validate(state.node_count, state.conductance_edges.len())?;
            return Ok(result);
        }

        let revision_before = state.revision;
        let applied = apply_operation(self, state, &edit.operation)?;
        let Some(applied) = applied else {
            let result = BioelectricCircuitEditResult::rejected(
                &edit.edit_id,
                revision_before,
                "bioelectric edit target is unavailable",
            );
            result.validate(state.node_count, state.conductance_edges.len())?;
            return Ok(result);
        };

        state.advance_revision()?;
        state.validate()?;
        let result = BioelectricCircuitEditResult::accepted(
            &edit.edit_id,
            revision_before,
            state.revision,
            applied.clamped_values,
            applied.affected_node_indices,
            applied.affected_edge_indices,
            applied.affected_current_term_ids,
        );
        result.validate(state.node_count, state.conductance_edges.len())?;
        Ok(result)
    }
}

struct AppliedEdit {
    clamped_values: usize,
    affected_node_indices: Vec<usize>,
    affected_edge_indices: Vec<usize>,
    affected_current_term_ids: Vec<String>,
}

impl AppliedEdit {
    fn nodes(node_index: usize, clamped_values: usize) -> Self {
        Self {
            clamped_values,
            affected_node_indices: vec![node_index],
            affected_edge_indices: Vec::new(),
            affected_current_term_ids: Vec::new(),
        }
    }

    fn edges(edge_indices: Vec<usize>, clamped_values: usize) -> Self {
        Self {
            clamped_values,
            affected_node_indices: Vec::new(),
            affected_edge_indices: edge_indices,
            affected_current_term_ids: Vec::new(),
        }
    }

    fn term(term_id: String, affected_node_indices: Vec<usize>) -> Self {
        Self {
            clamped_values: 0,
            affected_node_indices,
            affected_edge_indices: Vec::new(),
            affected_current_term_ids: vec![term_id],
        }
    }
}

fn apply_operation(
    runtime: &BioelectricCircuitRuntime,
    state: &mut BioelectricCircuitState,
    operation: &BioelectricCircuitEditOperation,
) -> Result<Option<AppliedEdit>, MatterFieldError> {
    match operation {
        BioelectricCircuitEditOperation::SetNodeVoltage {
            node_index,
            voltage,
        } => Ok(Some(apply_set_node_voltage(
            runtime,
            state,
            *node_index,
            *voltage,
        ))),
        BioelectricCircuitEditOperation::AddNodeVoltage { node_index, delta } => Ok(Some(
            apply_add_node_voltage(runtime, state, *node_index, *delta),
        )),
        BioelectricCircuitEditOperation::SetNodeMemory {
            node_index,
            memory_value,
        } => Ok(apply_set_node_memory(state, *node_index, *memory_value)),
        BioelectricCircuitEditOperation::ScaleIncidentConductance { node_index, scale } => {
            Ok(Some(apply_scale_incident_conductance(
                runtime,
                state,
                *node_index,
                *scale,
            )))
        }
        BioelectricCircuitEditOperation::SetEdgeGateThreshold {
            edge_index,
            threshold,
            slope,
        } => Ok(apply_set_edge_gate_threshold(
            state,
            *edge_index,
            *threshold,
            *slope,
        )),
        BioelectricCircuitEditOperation::SetEdgeGateMultiplierBounds {
            edge_index,
            min_multiplier,
            max_multiplier,
        } => Ok(apply_set_edge_gate_multiplier_bounds(
            state,
            *edge_index,
            *min_multiplier,
            *max_multiplier,
        )),
        BioelectricCircuitEditOperation::AddTransientCurrent {
            term_id,
            target_node_indices,
            current,
            start_step,
            duration_steps,
        } => apply_add_transient_current(
            state,
            term_id,
            target_node_indices,
            *current,
            *start_step,
            *duration_steps,
        ),
    }
}

fn apply_set_node_voltage(
    runtime: &BioelectricCircuitRuntime,
    state: &mut BioelectricCircuitState,
    node_index: usize,
    voltage: f32,
) -> AppliedEdit {
    let clamped = voltage.clamp(
        runtime.config().voltage_clamp_min,
        runtime.config().voltage_clamp_max,
    );
    state.voltage.values[node_index] = clamped;
    AppliedEdit::nodes(node_index, clamped_count(voltage, clamped))
}

fn apply_add_node_voltage(
    runtime: &BioelectricCircuitRuntime,
    state: &mut BioelectricCircuitState,
    node_index: usize,
    delta: f32,
) -> AppliedEdit {
    let next = state.voltage.values[node_index] + delta;
    let clamped = next.clamp(
        runtime.config().voltage_clamp_min,
        runtime.config().voltage_clamp_max,
    );
    state.voltage.values[node_index] = clamped;
    AppliedEdit::nodes(node_index, clamped_count(next, clamped))
}

fn apply_set_node_memory(
    state: &mut BioelectricCircuitState,
    node_index: usize,
    memory_value: f32,
) -> Option<AppliedEdit> {
    let memory = state.memory.as_mut()?;
    let clamped = memory_value.clamp(0.0, 1.0);
    memory.values[node_index] = clamped;
    Some(AppliedEdit::nodes(
        node_index,
        clamped_count(memory_value, clamped),
    ))
}

fn apply_scale_incident_conductance(
    runtime: &BioelectricCircuitRuntime,
    state: &mut BioelectricCircuitState,
    node_index: usize,
    scale: f32,
) -> AppliedEdit {
    let mut affected_edges = Vec::new();
    let mut clamped_values = 0;
    for (edge_index, edge) in state.conductance_edges.iter_mut().enumerate() {
        if edge.from_node != node_index && edge.to_node != node_index {
            continue;
        }
        let next_base = edge.base_conductance * scale;
        let clamped_base = next_base.clamp(
            runtime.config().conductance_clamp_min,
            runtime.config().conductance_clamp_max,
        );
        let next_conductance = edge.conductance * scale;
        let clamped_conductance = next_conductance.clamp(
            runtime.config().conductance_clamp_min,
            runtime.config().conductance_clamp_max,
        );
        clamped_values += clamped_count(next_base, clamped_base);
        clamped_values += clamped_count(next_conductance, clamped_conductance);
        edge.base_conductance = clamped_base;
        edge.conductance = clamped_conductance;
        affected_edges.push(edge_index);
    }
    AppliedEdit::edges(affected_edges, clamped_values)
}

fn apply_set_edge_gate_threshold(
    state: &mut BioelectricCircuitState,
    edge_index: usize,
    threshold: f32,
    slope: Option<f32>,
) -> Option<AppliedEdit> {
    let gate = state.conductance_edges[edge_index].gate.as_mut()?;
    gate.threshold = threshold;
    if let Some(slope) = slope {
        gate.slope = slope;
    }
    Some(AppliedEdit::edges(vec![edge_index], 0))
}

fn apply_set_edge_gate_multiplier_bounds(
    state: &mut BioelectricCircuitState,
    edge_index: usize,
    min_multiplier: f32,
    max_multiplier: f32,
) -> Option<AppliedEdit> {
    let gate = state.conductance_edges[edge_index].gate.as_mut()?;
    gate.min_multiplier = min_multiplier;
    gate.max_multiplier = max_multiplier;
    Some(AppliedEdit::edges(vec![edge_index], 0))
}

fn apply_add_transient_current(
    state: &mut BioelectricCircuitState,
    term_id: &str,
    target_node_indices: &[usize],
    current: f32,
    start_step: u32,
    duration_steps: u32,
) -> Result<Option<AppliedEdit>, MatterFieldError> {
    if state
        .current_terms
        .iter()
        .any(|term| term.term_id == term_id)
    {
        return Ok(None);
    }
    let mut term = BioelectricCurrentTerm::new(
        term_id.to_owned(),
        target_node_indices.to_vec(),
        BioelectricCurrentKind::Constant { current },
    );
    term.start_step = start_step;
    term.duration_steps = duration_steps;
    term.validate(state.node_count)?;
    state.current_terms.push(term);
    Ok(Some(AppliedEdit::term(
        term_id.to_owned(),
        target_node_indices.to_vec(),
    )))
}

fn clamped_count(requested: f32, clamped: f32) -> usize {
    usize::from(requested.to_bits() != clamped.to_bits())
}

fn validate_finite(value: f32, reason: &'static str) -> Result<(), MatterFieldError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(MatterFieldError::InvalidField(reason))
    }
}

fn validate_node_index(node_index: usize, node_count: usize) -> Result<(), MatterFieldError> {
    if node_index < node_count {
        Ok(())
    } else {
        Err(MatterFieldError::InvalidPerturbationNode {
            node_index,
            node_count,
        })
    }
}

fn validate_edge_index(edge_index: usize, edge_count: usize) -> Result<(), MatterFieldError> {
    if edge_index < edge_count {
        Ok(())
    } else {
        Err(MatterFieldError::InvalidNeighbor {
            node_index: edge_index,
            neighbor_index: edge_count,
        })
    }
}

fn validate_node_targets(
    node_indices: &[usize],
    node_count: usize,
) -> Result<(), MatterFieldError> {
    let mut targets = Vec::with_capacity(node_indices.len());
    for &node_index in node_indices {
        validate_node_index(node_index, node_count)?;
        if targets.contains(&node_index) {
            return Err(MatterFieldError::DuplicatePerturbationNode { node_index });
        }
        targets.push(node_index);
    }
    Ok(())
}

fn validate_edge_targets(
    edge_indices: &[usize],
    edge_count: usize,
) -> Result<(), MatterFieldError> {
    let mut targets = Vec::with_capacity(edge_indices.len());
    for &edge_index in edge_indices {
        validate_edge_index(edge_index, edge_count)?;
        if targets.contains(&edge_index) {
            return Err(MatterFieldError::DuplicateNeighbor {
                node_index: edge_index,
                neighbor_index: edge_index,
            });
        }
        targets.push(edge_index);
    }
    Ok(())
}
