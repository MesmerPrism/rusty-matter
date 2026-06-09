use crate::{
    MatterFieldError, SurfaceFieldSubstrate, BIOELECTRIC_CIRCUIT_CONFIG_SCHEMA_ID,
    BIOELECTRIC_CIRCUIT_STATE_SCHEMA_ID, BIOELECTRIC_CONDUCTANCE_EDGE_SCHEMA_ID,
    BIOELECTRIC_CURRENT_TERM_SCHEMA_ID, BIOELECTRIC_READOUT_LAYER_SCHEMA_ID,
    BIOELECTRIC_STEP_DIAGNOSTICS_SCHEMA_ID, BIOELECTRIC_VOLTAGE_FIELD_SCHEMA_ID,
};

/// Unit contract for membrane-voltage-like circuit state.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BioelectricVoltageUnit {
    /// Calibrated normalized state. Fixtures use this until a unit calibration
    /// pass declares quantitative millivolt semantics.
    Normalized,
    /// Explicit millivolt state.
    Millivolts,
}

/// Per-node membrane-voltage-like state over a surface-field substrate.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricVoltageField {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable voltage field identifier.
    pub field_id: String,
    /// Voltage unit policy.
    pub unit: BioelectricVoltageUnit,
    /// Resting voltage for validation and source terms.
    pub resting_voltage: f32,
    /// One voltage value per substrate node.
    pub values: Vec<f32>,
}

impl BioelectricVoltageField {
    /// Creates a voltage field.
    #[must_use]
    pub fn new(
        field_id: impl Into<String>,
        unit: BioelectricVoltageUnit,
        resting_voltage: f32,
        values: Vec<f32>,
    ) -> Self {
        Self {
            schema_id: BIOELECTRIC_VOLTAGE_FIELD_SCHEMA_ID.to_owned(),
            field_id: field_id.into(),
            unit,
            resting_voltage,
            values,
        }
    }

    /// Creates a constant voltage field.
    #[must_use]
    pub fn constant(
        field_id: impl Into<String>,
        unit: BioelectricVoltageUnit,
        node_count: usize,
        resting_voltage: f32,
        value: f32,
    ) -> Self {
        Self::new(field_id, unit, resting_voltage, vec![value; node_count])
    }

    /// Validates the voltage field against a substrate node count.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, ID, count, or values are
    /// invalid.
    pub fn validate(&self, node_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != BIOELECTRIC_VOLTAGE_FIELD_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_VOLTAGE_FIELD_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.field_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyFieldId);
        }
        if !self.resting_voltage.is_finite() {
            return Err(MatterFieldError::InvalidField(
                "bioelectric resting voltage must be finite",
            ));
        }
        if self.values.len() != node_count {
            return Err(MatterFieldError::NodeCountMismatch {
                expected: node_count,
                actual: self.values.len(),
            });
        }
        for (index, value) in self.values.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(MatterFieldError::NonFiniteScalar {
                    field_id: self.field_id.clone(),
                    index,
                });
            }
        }
        Ok(())
    }
}

/// Signal used by a conductance gate.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BioelectricGateSource {
    /// Absolute voltage difference between the source and target node.
    VoltageDifference,
    /// Source-node voltage.
    SourceVoltage,
    /// Target-node voltage.
    TargetVoltage,
    /// Source-node hysteresis memory value.
    SourceMemory,
}

/// Smooth gate that maps voltage or memory state to a conductance multiplier.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricGate {
    /// Stable gate identifier.
    pub gate_id: String,
    /// Source signal for the gate.
    pub source: BioelectricGateSource,
    /// Gate midpoint threshold.
    pub threshold: f32,
    /// Gate slope. Positive opens above threshold; negative closes above it.
    pub slope: f32,
    /// Lower multiplier bound.
    pub min_multiplier: f32,
    /// Upper multiplier bound.
    pub max_multiplier: f32,
}

impl BioelectricGate {
    /// Creates a gate.
    #[must_use]
    pub fn new(
        gate_id: impl Into<String>,
        source: BioelectricGateSource,
        threshold: f32,
        slope: f32,
        min_multiplier: f32,
        max_multiplier: f32,
    ) -> Self {
        Self {
            gate_id: gate_id.into(),
            source,
            threshold,
            slope,
            min_multiplier,
            max_multiplier,
        }
    }

    fn multiplier(&self, source_value: f32) -> f32 {
        let scaled = ((source_value - self.threshold) / self.slope).clamp(-50.0, 50.0);
        let open_fraction = 1.0 / (1.0 + (-scaled).exp());
        self.min_multiplier + (self.max_multiplier - self.min_multiplier) * open_fraction
    }

    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.gate_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidField(
                "bioelectric gate id must not be empty",
            ));
        }
        if !self.threshold.is_finite() || !self.slope.is_finite() || self.slope == 0.0 {
            return Err(MatterFieldError::InvalidField(
                "bioelectric gate threshold and non-zero slope must be finite",
            ));
        }
        if !self.min_multiplier.is_finite()
            || !self.max_multiplier.is_finite()
            || self.min_multiplier < 0.0
            || self.min_multiplier > self.max_multiplier
        {
            return Err(MatterFieldError::InvalidField(
                "bioelectric gate multipliers must be finite, non-negative, and increasing",
            ));
        }
        Ok(())
    }
}

/// Directed sparse conductance edge representing gap-junction-like coupling.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricConductanceEdge {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable edge identifier.
    pub edge_id: String,
    /// Source node index.
    pub from_node: usize,
    /// Target node index.
    pub to_node: usize,
    /// Neighbor tier, starting at 1.
    pub tier: u8,
    /// Baseline conductance before gating.
    pub base_conductance: f32,
    /// Current conductance after gating.
    pub conductance: f32,
    /// Optional gate controlling conductance.
    pub gate: Option<BioelectricGate>,
}

impl BioelectricConductanceEdge {
    /// Creates a conductance edge.
    #[must_use]
    pub fn new(
        edge_id: impl Into<String>,
        from_node: usize,
        to_node: usize,
        tier: u8,
        base_conductance: f32,
        gate: Option<BioelectricGate>,
    ) -> Self {
        Self {
            schema_id: BIOELECTRIC_CONDUCTANCE_EDGE_SCHEMA_ID.to_owned(),
            edge_id: edge_id.into(),
            from_node,
            to_node,
            tier,
            base_conductance,
            conductance: base_conductance,
            gate,
        }
    }

    /// Builds directed conductance edges from substrate neighbor tiers.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the substrate, conductance values, or
    /// generated edges are invalid.
    pub fn from_substrate_neighbors(
        substrate: &SurfaceFieldSubstrate,
        first_tier_conductance: f32,
        second_tier_conductance: f32,
        gate: Option<BioelectricGate>,
    ) -> Result<Vec<Self>, MatterFieldError> {
        substrate.validate()?;
        if !first_tier_conductance.is_finite() || first_tier_conductance < 0.0 {
            return Err(MatterFieldError::InvalidField(
                "first-tier conductance must be finite and non-negative",
            ));
        }
        if !second_tier_conductance.is_finite() || second_tier_conductance < 0.0 {
            return Err(MatterFieldError::InvalidField(
                "second-tier conductance must be finite and non-negative",
            ));
        }
        if let Some(gate) = &gate {
            gate.validate()?;
        }

        let mut edges = Vec::new();
        for node in &substrate.nodes {
            edges.extend(node.first_tier_neighbors.iter().copied().map(|target| {
                Self::new(
                    format!("conductance.{}.{}.tier1", node.node_index, target),
                    node.node_index,
                    target,
                    1,
                    first_tier_conductance,
                    gate.clone(),
                )
            }));
            if second_tier_conductance > 0.0 {
                edges.extend(node.second_tier_neighbors.iter().copied().map(|target| {
                    Self::new(
                        format!("conductance.{}.{}.tier2", node.node_index, target),
                        node.node_index,
                        target,
                        2,
                        second_tier_conductance,
                        gate.clone(),
                    )
                }));
            }
        }

        for edge in &edges {
            edge.validate(substrate.node_count())?;
        }
        Ok(edges)
    }

    /// Validates the conductance edge.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, IDs, node indices, or
    /// conductance values are invalid.
    pub fn validate(&self, node_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != BIOELECTRIC_CONDUCTANCE_EDGE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_CONDUCTANCE_EDGE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.edge_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidField(
                "bioelectric conductance edge id must not be empty",
            ));
        }
        if self.from_node >= node_count {
            return Err(MatterFieldError::InvalidNeighbor {
                node_index: self.from_node,
                neighbor_index: self.to_node,
            });
        }
        if self.to_node >= node_count {
            return Err(MatterFieldError::InvalidNeighbor {
                node_index: self.from_node,
                neighbor_index: self.to_node,
            });
        }
        if self.from_node == self.to_node {
            return Err(MatterFieldError::SelfNeighbor {
                node_index: self.from_node,
            });
        }
        if !(1..=2).contains(&self.tier) {
            return Err(MatterFieldError::InvalidField(
                "bioelectric conductance edge tier must be 1 or 2",
            ));
        }
        if !self.base_conductance.is_finite()
            || !self.conductance.is_finite()
            || self.base_conductance < 0.0
            || self.conductance < 0.0
        {
            return Err(MatterFieldError::InvalidField(
                "bioelectric conductance values must be finite and non-negative",
            ));
        }
        if let Some(gate) = &self.gate {
            gate.validate()?;
        }
        Ok(())
    }
}

/// Configurable current source term.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BioelectricCurrentKind {
    /// Passive leak toward a reversal voltage.
    Leak {
        /// Leak conductance.
        conductance: f32,
        /// Reversal voltage.
        reversal_voltage: f32,
    },
    /// Constant current source or sink.
    Constant {
        /// Current contribution. Positive depolarizes in normalized fixtures.
        current: f32,
    },
    /// Pump-like drive toward a target voltage.
    Pump {
        /// Pump rate.
        rate: f32,
        /// Target voltage.
        target_voltage: f32,
    },
    /// Generic voltage-gated current. This is a configurable source term, not
    /// a named-ion physiology model.
    VoltageGated {
        /// Maximum conductance.
        max_conductance: f32,
        /// Reversal voltage.
        reversal_voltage: f32,
        /// Gate midpoint threshold.
        threshold: f32,
        /// Gate slope. Positive opens above threshold; negative closes above it.
        slope: f32,
    },
}

impl BioelectricCurrentKind {
    fn current_for_voltage(self, voltage: f32) -> f32 {
        match self {
            Self::Leak {
                conductance,
                reversal_voltage,
            } => conductance * (reversal_voltage - voltage),
            Self::Constant { current } => current,
            Self::Pump {
                rate,
                target_voltage,
            } => rate * (target_voltage - voltage),
            Self::VoltageGated {
                max_conductance,
                reversal_voltage,
                threshold,
                slope,
            } => {
                let scaled = ((voltage - threshold) / slope).clamp(-50.0, 50.0);
                let open_fraction = 1.0 / (1.0 + (-scaled).exp());
                max_conductance * open_fraction * (reversal_voltage - voltage)
            }
        }
    }

    fn validate(self) -> Result<(), MatterFieldError> {
        match self {
            Self::Leak {
                conductance,
                reversal_voltage,
            } => {
                if !conductance.is_finite() || conductance < 0.0 || !reversal_voltage.is_finite() {
                    return Err(MatterFieldError::InvalidField(
                        "leak current needs finite non-negative conductance and finite reversal voltage",
                    ));
                }
            }
            Self::Constant { current } => {
                if !current.is_finite() {
                    return Err(MatterFieldError::InvalidField(
                        "constant current must be finite",
                    ));
                }
            }
            Self::Pump {
                rate,
                target_voltage,
            } => {
                if !rate.is_finite() || rate < 0.0 || !target_voltage.is_finite() {
                    return Err(MatterFieldError::InvalidField(
                        "pump current needs finite non-negative rate and finite target voltage",
                    ));
                }
            }
            Self::VoltageGated {
                max_conductance,
                reversal_voltage,
                threshold,
                slope,
            } => {
                if !max_conductance.is_finite()
                    || max_conductance < 0.0
                    || !reversal_voltage.is_finite()
                    || !threshold.is_finite()
                    || !slope.is_finite()
                    || slope == 0.0
                {
                    return Err(MatterFieldError::InvalidField(
                        "voltage-gated current parameters must be finite with non-zero slope",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Current term scheduled over all nodes or a specific node subset.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricCurrentTerm {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable current term identifier.
    pub term_id: String,
    /// Target nodes. Empty means all nodes.
    pub target_node_indices: Vec<usize>,
    /// First active step.
    pub start_step: u32,
    /// Active duration in fixed steps.
    pub duration_steps: u32,
    /// Current source behavior.
    pub kind: BioelectricCurrentKind,
}

impl BioelectricCurrentTerm {
    /// Creates a current term. Empty target nodes apply to all nodes.
    #[must_use]
    pub fn new(
        term_id: impl Into<String>,
        target_node_indices: Vec<usize>,
        kind: BioelectricCurrentKind,
    ) -> Self {
        Self {
            schema_id: BIOELECTRIC_CURRENT_TERM_SCHEMA_ID.to_owned(),
            term_id: term_id.into(),
            target_node_indices,
            start_step: 0,
            duration_steps: u32::MAX,
            kind,
        }
    }

    fn is_active(&self, step_index: u32) -> bool {
        step_index >= self.start_step && step_index - self.start_step < self.duration_steps
    }

    /// Validates the current term.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, IDs, schedule, target nodes,
    /// or source parameters are invalid.
    pub fn validate(&self, node_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != BIOELECTRIC_CURRENT_TERM_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_CURRENT_TERM_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.term_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidField(
                "bioelectric current term id must not be empty",
            ));
        }
        if self.duration_steps == 0 {
            return Err(MatterFieldError::InvalidField(
                "bioelectric current term duration must be non-zero",
            ));
        }
        let mut targets = Vec::with_capacity(self.target_node_indices.len());
        for &node_index in &self.target_node_indices {
            if node_index >= node_count {
                return Err(MatterFieldError::InvalidPerturbationNode {
                    node_index,
                    node_count,
                });
            }
            if targets.contains(&node_index) {
                return Err(MatterFieldError::DuplicatePerturbationNode { node_index });
            }
            targets.push(node_index);
        }
        self.kind.validate()
    }
}

/// Per-node hysteresis state for transient-to-persistent pattern memory.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricMemoryState {
    /// Stable memory identifier.
    pub memory_id: String,
    /// One memory value per substrate node, clamped to 0..=1.
    pub values: Vec<f32>,
    /// Voltage threshold that activates memory.
    pub activation_threshold: f32,
    /// Voltage threshold that releases memory.
    pub release_threshold: f32,
    /// Activation rate in reciprocal seconds.
    pub activation_rate: f32,
    /// Release rate in reciprocal seconds.
    pub release_rate: f32,
}

impl BioelectricMemoryState {
    /// Creates a memory state.
    #[must_use]
    pub fn new(
        memory_id: impl Into<String>,
        values: Vec<f32>,
        activation_threshold: f32,
        release_threshold: f32,
        activation_rate: f32,
        release_rate: f32,
    ) -> Self {
        Self {
            memory_id: memory_id.into(),
            values,
            activation_threshold,
            release_threshold,
            activation_rate,
            release_rate,
        }
    }

    /// Creates a zeroed memory state.
    #[must_use]
    pub fn zeroed(
        memory_id: impl Into<String>,
        node_count: usize,
        activation_threshold: f32,
        release_threshold: f32,
        activation_rate: f32,
        release_rate: f32,
    ) -> Self {
        Self::new(
            memory_id,
            vec![0.0; node_count],
            activation_threshold,
            release_threshold,
            activation_rate,
            release_rate,
        )
    }

    fn validate(&self, node_count: usize) -> Result<(), MatterFieldError> {
        if self.memory_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidField(
                "bioelectric memory id must not be empty",
            ));
        }
        if self.values.len() != node_count {
            return Err(MatterFieldError::NodeCountMismatch {
                expected: node_count,
                actual: self.values.len(),
            });
        }
        for value in &self.values {
            if !value.is_finite() || !(0.0..=1.0).contains(value) {
                return Err(MatterFieldError::InvalidField(
                    "bioelectric memory values must be finite in 0..=1",
                ));
            }
        }
        if !self.activation_threshold.is_finite()
            || !self.release_threshold.is_finite()
            || self.activation_threshold <= self.release_threshold
        {
            return Err(MatterFieldError::InvalidField(
                "bioelectric memory activation threshold must exceed release threshold",
            ));
        }
        if !self.activation_rate.is_finite()
            || self.activation_rate < 0.0
            || !self.release_rate.is_finite()
            || self.release_rate < 0.0
        {
            return Err(MatterFieldError::InvalidField(
                "bioelectric memory rates must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

/// Voltage-driven downstream readout layer.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricReadoutLayer {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable layer identifier.
    pub layer_id: String,
    /// One readout value per substrate node.
    pub values: Vec<f32>,
    /// Voltage contribution weight.
    pub voltage_weight: f32,
    /// Memory contribution weight.
    pub memory_weight: f32,
    /// Readout bias.
    pub bias: f32,
    /// Readout relaxation rate.
    pub relaxation_rate: f32,
    /// Minimum readout value after clamping.
    pub clamp_min: f32,
    /// Maximum readout value after clamping.
    pub clamp_max: f32,
}

impl BioelectricReadoutLayer {
    /// Creates a readout layer.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        layer_id: impl Into<String>,
        values: Vec<f32>,
        voltage_weight: f32,
        memory_weight: f32,
        bias: f32,
        relaxation_rate: f32,
        clamp_min: f32,
        clamp_max: f32,
    ) -> Self {
        Self {
            schema_id: BIOELECTRIC_READOUT_LAYER_SCHEMA_ID.to_owned(),
            layer_id: layer_id.into(),
            values,
            voltage_weight,
            memory_weight,
            bias,
            relaxation_rate,
            clamp_min,
            clamp_max,
        }
    }

    fn validate(&self, node_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != BIOELECTRIC_READOUT_LAYER_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_READOUT_LAYER_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.layer_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyFieldId);
        }
        if self.values.len() != node_count {
            return Err(MatterFieldError::NodeCountMismatch {
                expected: node_count,
                actual: self.values.len(),
            });
        }
        if !self.values.iter().all(|value| value.is_finite()) {
            return Err(MatterFieldError::InvalidField(
                "bioelectric readout values must be finite",
            ));
        }
        if !self.voltage_weight.is_finite()
            || !self.memory_weight.is_finite()
            || !self.bias.is_finite()
            || !self.relaxation_rate.is_finite()
            || self.relaxation_rate < 0.0
        {
            return Err(MatterFieldError::InvalidField(
                "bioelectric readout coefficients must be finite",
            ));
        }
        if !self.clamp_min.is_finite()
            || !self.clamp_max.is_finite()
            || self.clamp_min >= self.clamp_max
        {
            return Err(MatterFieldError::InvalidField(
                "bioelectric readout clamp range must be finite and increasing",
            ));
        }
        Ok(())
    }
}

/// Runtime configuration for deterministic bioelectric circuit stepping.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricCircuitConfig {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable config identifier.
    pub config_id: String,
    /// Fixed step duration in seconds.
    pub fixed_step_seconds: f32,
    /// Maximum number of fixed steps accepted by one run request.
    pub max_steps_per_run: u32,
    /// Minimum voltage after clamping.
    pub voltage_clamp_min: f32,
    /// Maximum voltage after clamping.
    pub voltage_clamp_max: f32,
    /// Minimum conductance after clamping.
    pub conductance_clamp_min: f32,
    /// Maximum conductance after clamping.
    pub conductance_clamp_max: f32,
    /// Maximum absolute current contribution from one term or edge.
    pub current_clamp_absolute: f32,
}

impl Default for BioelectricCircuitConfig {
    fn default() -> Self {
        Self {
            schema_id: BIOELECTRIC_CIRCUIT_CONFIG_SCHEMA_ID.to_owned(),
            config_id: "fields.bioelectric_circuit.default".to_owned(),
            fixed_step_seconds: 1.0 / 60.0,
            max_steps_per_run: 1024,
            voltage_clamp_min: -1.0,
            voltage_clamp_max: 1.0,
            conductance_clamp_min: 0.0,
            conductance_clamp_max: 4.0,
            current_clamp_absolute: 8.0,
        }
    }
}

impl BioelectricCircuitConfig {
    /// Validates the circuit config.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, IDs, or numeric ranges are
    /// invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != BIOELECTRIC_CIRCUIT_CONFIG_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_CIRCUIT_CONFIG_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.config_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyRuntimeConfigId);
        }
        if !self.fixed_step_seconds.is_finite() || self.fixed_step_seconds <= 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "bioelectric fixed_step_seconds must be finite and positive",
            ));
        }
        if self.max_steps_per_run == 0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "bioelectric max_steps_per_run must be non-zero",
            ));
        }
        if !self.voltage_clamp_min.is_finite()
            || !self.voltage_clamp_max.is_finite()
            || self.voltage_clamp_min >= self.voltage_clamp_max
        {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "bioelectric voltage clamp range must be finite and increasing",
            ));
        }
        if !self.conductance_clamp_min.is_finite()
            || !self.conductance_clamp_max.is_finite()
            || self.conductance_clamp_min < 0.0
            || self.conductance_clamp_min > self.conductance_clamp_max
        {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "bioelectric conductance clamp range must be finite and non-negative",
            ));
        }
        if !self.current_clamp_absolute.is_finite() || self.current_clamp_absolute <= 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "bioelectric current clamp must be finite and positive",
            ));
        }
        Ok(())
    }
}

/// Bioelectric circuit state bound to one surface-field substrate.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricCircuitState {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable circuit state identifier.
    pub circuit_id: String,
    /// Source substrate identifier.
    pub substrate_id: String,
    /// Expected node count for all per-node buffers.
    pub node_count: usize,
    /// State time in seconds.
    pub time_seconds: f32,
    /// Monotonic Matter-owned state revision for steps and accepted edits.
    pub revision: u64,
    /// Membrane-voltage-like state.
    pub voltage: BioelectricVoltageField,
    /// Directed gap-junction-like conductance edges.
    pub conductance_edges: Vec<BioelectricConductanceEdge>,
    /// Configurable local current terms.
    pub current_terms: Vec<BioelectricCurrentTerm>,
    /// Optional hysteresis memory state.
    pub memory: Option<BioelectricMemoryState>,
    /// Voltage-driven downstream readout layers.
    pub readout_layers: Vec<BioelectricReadoutLayer>,
}

impl BioelectricCircuitState {
    /// Creates and validates a circuit state for a substrate.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when substrate or circuit contracts are
    /// invalid.
    pub fn new(
        circuit_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
        voltage: BioelectricVoltageField,
        conductance_edges: Vec<BioelectricConductanceEdge>,
        current_terms: Vec<BioelectricCurrentTerm>,
        memory: Option<BioelectricMemoryState>,
        readout_layers: Vec<BioelectricReadoutLayer>,
    ) -> Result<Self, MatterFieldError> {
        substrate.validate()?;
        let state = Self {
            schema_id: BIOELECTRIC_CIRCUIT_STATE_SCHEMA_ID.to_owned(),
            circuit_id: circuit_id.into(),
            substrate_id: substrate.substrate_id.clone(),
            node_count: substrate.node_count(),
            time_seconds: 0.0,
            revision: 0,
            voltage,
            conductance_edges,
            current_terms,
            memory,
            readout_layers,
        };
        state.validate()?;
        Ok(state)
    }

    /// Validates the circuit state.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when metadata, buffers, edges, terms, or
    /// readouts are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != BIOELECTRIC_CIRCUIT_STATE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_CIRCUIT_STATE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.circuit_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyStateId);
        }
        if self.substrate_id.trim().is_empty() {
            return Err(MatterFieldError::EmptySubstrateId);
        }
        if self.node_count == 0 {
            return Err(MatterFieldError::InvalidField(
                "bioelectric circuit node count must be non-zero",
            ));
        }
        if !self.time_seconds.is_finite() || self.time_seconds < 0.0 {
            return Err(MatterFieldError::InvalidField(
                "bioelectric circuit time must be finite and non-negative",
            ));
        }
        self.voltage.validate(self.node_count)?;
        if self.conductance_edges.is_empty() {
            return Err(MatterFieldError::InvalidField(
                "bioelectric circuit must contain conductance edges",
            ));
        }

        let mut edge_ids = Vec::with_capacity(self.conductance_edges.len());
        let mut edge_keys = Vec::with_capacity(self.conductance_edges.len());
        for edge in &self.conductance_edges {
            edge.validate(self.node_count)?;
            push_unique_id(&mut edge_ids, &edge.edge_id)?;
            let key = (edge.from_node, edge.to_node, edge.tier);
            if edge_keys.contains(&key) {
                return Err(MatterFieldError::DuplicateNeighbor {
                    node_index: edge.from_node,
                    neighbor_index: edge.to_node,
                });
            }
            edge_keys.push(key);
        }

        let mut term_ids = Vec::with_capacity(self.current_terms.len());
        for term in &self.current_terms {
            term.validate(self.node_count)?;
            push_unique_id(&mut term_ids, &term.term_id)?;
        }

        if let Some(memory) = &self.memory {
            memory.validate(self.node_count)?;
        }

        let mut readout_ids = Vec::with_capacity(self.readout_layers.len());
        for layer in &self.readout_layers {
            layer.validate(self.node_count)?;
            push_unique_id(&mut readout_ids, &layer.layer_id)?;
        }
        Ok(())
    }

    pub(crate) fn advance_revision(&mut self) -> Result<(), MatterFieldError> {
        self.revision = self.revision.checked_add(1).ok_or({
            MatterFieldError::InvalidRunSummary("bioelectric circuit revision overflow")
        })?;
        Ok(())
    }
}

/// Per-step diagnostics for bioelectric circuit dynamics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricCircuitStepDiagnostics {
    /// Schema identifier.
    pub schema_id: String,
    /// Step index after the update.
    pub step_index: u32,
    /// Number of nodes updated.
    pub updated_nodes: usize,
    /// Number of directed conductance edges visited.
    pub visited_edges: usize,
    /// Number of active current terms.
    pub active_current_terms: usize,
    /// Number of conductance gates evaluated.
    pub active_gates: usize,
    /// Number of node voltages clamped.
    pub clamped_voltage_nodes: usize,
    /// Number of conductance edges clamped.
    pub clamped_conductance_edges: usize,
    /// Number of memory values crossing into active state.
    pub memory_activated_nodes: usize,
    /// Number of readout layers updated.
    pub readout_layers_updated: usize,
    /// Maximum absolute voltage delta applied during this step.
    pub max_voltage_delta: f32,
    /// Sum of absolute net current over all nodes.
    pub net_current_abs_sum: f32,
}

impl BioelectricCircuitStepDiagnostics {
    /// Creates empty diagnostics for a step.
    #[must_use]
    pub fn empty(step_index: u32) -> Self {
        Self {
            schema_id: BIOELECTRIC_STEP_DIAGNOSTICS_SCHEMA_ID.to_owned(),
            step_index,
            updated_nodes: 0,
            visited_edges: 0,
            active_current_terms: 0,
            active_gates: 0,
            clamped_voltage_nodes: 0,
            clamped_conductance_edges: 0,
            memory_activated_nodes: 0,
            readout_layers_updated: 0,
            max_voltage_delta: 0.0,
            net_current_abs_sum: 0.0,
        }
    }

    /// Validates diagnostic counts against circuit sizes.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, counts, or numeric summaries
    /// are invalid.
    pub fn validate(&self, node_count: usize, edge_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != BIOELECTRIC_STEP_DIAGNOSTICS_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: BIOELECTRIC_STEP_DIAGNOSTICS_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.updated_nodes > node_count
            || self.memory_activated_nodes > node_count
            || self.visited_edges > edge_count
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric diagnostic counts must not exceed circuit sizes",
            ));
        }
        if !self.max_voltage_delta.is_finite()
            || self.max_voltage_delta < 0.0
            || !self.net_current_abs_sum.is_finite()
            || self.net_current_abs_sum < 0.0
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "bioelectric diagnostic numeric summaries must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

/// Deterministic CPU reference runtime for bioelectric circuit dynamics.
#[derive(Clone, Debug, PartialEq)]
pub struct BioelectricCircuitRuntime {
    config: BioelectricCircuitConfig,
}

impl BioelectricCircuitRuntime {
    /// Creates a circuit runtime from a config.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the config is invalid.
    pub fn new(config: BioelectricCircuitConfig) -> Result<Self, MatterFieldError> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Returns the runtime config.
    #[must_use]
    pub fn config(&self) -> &BioelectricCircuitConfig {
        &self.config
    }

    /// Validates circuit contracts without stepping dynamics.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when substrate, config, or state contracts
    /// are invalid.
    pub fn validate_contracts(
        &self,
        substrate: &SurfaceFieldSubstrate,
        state: &BioelectricCircuitState,
    ) -> Result<BioelectricCircuitStepDiagnostics, MatterFieldError> {
        validate_circuit_for_substrate(substrate, state)?;
        self.config.validate()?;
        let mut diagnostics = BioelectricCircuitStepDiagnostics::empty(0);
        diagnostics.updated_nodes = state.node_count;
        diagnostics.visited_edges = state.conductance_edges.len();
        diagnostics.readout_layers_updated = state.readout_layers.len();
        diagnostics.validate(state.node_count, state.conductance_edges.len())?;
        Ok(diagnostics)
    }

    /// Advances one fixed bioelectric circuit step in place.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when contracts or generated values are
    /// invalid.
    pub fn step_fixed(
        &self,
        substrate: &SurfaceFieldSubstrate,
        state: &mut BioelectricCircuitState,
        step_index: u32,
    ) -> Result<BioelectricCircuitStepDiagnostics, MatterFieldError> {
        if step_index >= self.config.max_steps_per_run {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "bioelectric step_index exceeds max_steps_per_run",
            ));
        }
        validate_circuit_for_substrate(substrate, state)?;
        self.config.validate()?;

        let node_count = state.node_count;
        let previous_voltage = state.voltage.values.clone();
        let previous_memory = state
            .memory
            .as_ref()
            .map(|memory| memory.values.clone())
            .unwrap_or_else(|| vec![0.0; node_count]);
        let mut net_current = vec![0.0_f32; node_count];
        let mut diagnostics = BioelectricCircuitStepDiagnostics::empty(step_index + 1);
        diagnostics.updated_nodes = node_count;

        for term in &state.current_terms {
            if !term.is_active(step_index) {
                continue;
            }
            diagnostics.active_current_terms += 1;
            if term.target_node_indices.is_empty() {
                for node_index in 0..node_count {
                    add_clamped_current(
                        &mut net_current[node_index],
                        term.kind.current_for_voltage(previous_voltage[node_index]),
                        self.config.current_clamp_absolute,
                    );
                }
            } else {
                for &node_index in &term.target_node_indices {
                    add_clamped_current(
                        &mut net_current[node_index],
                        term.kind.current_for_voltage(previous_voltage[node_index]),
                        self.config.current_clamp_absolute,
                    );
                }
            }
        }

        for edge in &mut state.conductance_edges {
            let gate_multiplier = if let Some(gate) = &edge.gate {
                diagnostics.active_gates += 1;
                let source_value =
                    gate_source_value(gate.source, edge, &previous_voltage, &previous_memory);
                gate.multiplier(source_value)
            } else {
                1.0
            };
            let next_conductance = edge.base_conductance * gate_multiplier;
            let clamped_conductance = next_conductance.clamp(
                self.config.conductance_clamp_min,
                self.config.conductance_clamp_max,
            );
            if clamped_conductance != next_conductance {
                diagnostics.clamped_conductance_edges += 1;
            }
            edge.conductance = clamped_conductance;
            let coupling_current = edge.conductance
                * (previous_voltage[edge.to_node] - previous_voltage[edge.from_node]);
            add_clamped_current(
                &mut net_current[edge.from_node],
                coupling_current,
                self.config.current_clamp_absolute,
            );
            diagnostics.visited_edges += 1;
        }

        for node_index in 0..node_count {
            let delta = self.config.fixed_step_seconds * net_current[node_index];
            let next = previous_voltage[node_index] + delta;
            let clamped = next.clamp(self.config.voltage_clamp_min, self.config.voltage_clamp_max);
            if clamped != next {
                diagnostics.clamped_voltage_nodes += 1;
            }
            diagnostics.max_voltage_delta = diagnostics
                .max_voltage_delta
                .max((clamped - previous_voltage[node_index]).abs());
            diagnostics.net_current_abs_sum += net_current[node_index].abs();
            state.voltage.values[node_index] = clamped;
        }

        update_memory(
            state,
            self.config.fixed_step_seconds,
            &mut diagnostics.memory_activated_nodes,
        );
        update_readouts(state, self.config.fixed_step_seconds);
        diagnostics.readout_layers_updated = state.readout_layers.len();

        state.time_seconds += self.config.fixed_step_seconds;
        state.advance_revision()?;
        state.validate()?;
        diagnostics.validate(node_count, state.conductance_edges.len())?;
        Ok(diagnostics)
    }
}

pub(crate) fn validate_circuit_for_substrate(
    substrate: &SurfaceFieldSubstrate,
    state: &BioelectricCircuitState,
) -> Result<(), MatterFieldError> {
    substrate.validate()?;
    state.validate()?;
    if state.substrate_id != substrate.substrate_id {
        return Err(MatterFieldError::InvalidRunSummary(
            "bioelectric circuit substrate id must match substrate",
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

fn add_clamped_current(target: &mut f32, current: f32, clamp_absolute: f32) {
    *target += current.clamp(-clamp_absolute, clamp_absolute);
}

fn gate_source_value(
    source: BioelectricGateSource,
    edge: &BioelectricConductanceEdge,
    voltage: &[f32],
    memory: &[f32],
) -> f32 {
    match source {
        BioelectricGateSource::VoltageDifference => {
            (voltage[edge.to_node] - voltage[edge.from_node]).abs()
        }
        BioelectricGateSource::SourceVoltage => voltage[edge.from_node],
        BioelectricGateSource::TargetVoltage => voltage[edge.to_node],
        BioelectricGateSource::SourceMemory => memory[edge.from_node],
    }
}

fn update_memory(
    state: &mut BioelectricCircuitState,
    fixed_step_seconds: f32,
    activated_nodes: &mut usize,
) {
    let Some(memory) = &mut state.memory else {
        return;
    };
    for (node_index, value) in memory.values.iter_mut().enumerate() {
        let before = *value;
        let voltage = state.voltage.values[node_index];
        if voltage >= memory.activation_threshold {
            *value += fixed_step_seconds * memory.activation_rate * (1.0 - *value);
        } else if voltage <= memory.release_threshold {
            *value -= fixed_step_seconds * memory.release_rate * *value;
        }
        *value = value.clamp(0.0, 1.0);
        if before < 0.5 && *value >= 0.5 {
            *activated_nodes += 1;
        }
    }
}

fn update_readouts(state: &mut BioelectricCircuitState, fixed_step_seconds: f32) {
    let memory_values = state
        .memory
        .as_ref()
        .map(|memory| memory.values.clone())
        .unwrap_or_else(|| vec![0.0; state.node_count]);
    for layer in &mut state.readout_layers {
        for (node_index, value) in layer.values.iter_mut().enumerate() {
            let target = layer.bias
                + layer.voltage_weight * state.voltage.values[node_index]
                + layer.memory_weight * memory_values[node_index];
            let next = *value + fixed_step_seconds * layer.relaxation_rate * (target - *value);
            *value = next.clamp(layer.clamp_min, layer.clamp_max);
        }
    }
}

fn push_unique_id(ids: &mut Vec<String>, id: &str) -> Result<(), MatterFieldError> {
    if ids.iter().any(|existing| existing == id) {
        Err(MatterFieldError::DuplicateFieldId {
            field_id: id.to_owned(),
        })
    } else {
        ids.push(id.to_owned());
        Ok(())
    }
}
