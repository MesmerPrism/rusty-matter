#![allow(clippy::cast_precision_loss)]

use js_sys::{Float32Array, Uint32Array};
use rusty_matter_fields::{
    planarian_comparison_scenario_kinds, BioelectricCircuitEdit, BioelectricCircuitEditOperation,
    BioelectricCircuitEditResult, BioelectricCircuitRuntime, BioelectricCircuitState,
    BioelectricCircuitStepDiagnostics, PlanarianAxisRegion, PlanarianBioelectricOutcomeTrace,
    PlanarianBioelectricOutcomeTraceSet, PlanarianBioelectricPresetConfig,
    PlanarianBioelectricScenarioKind, PlanarianBioelectricScenarioRun, SurfaceFieldPerturbation,
    SurfaceFieldPerturbationEffect, SurfaceFieldRuntime, SurfaceFieldRuntimeConfig,
    SurfaceFieldState, SurfaceFieldStepDiagnostics, SurfaceFieldSubstrate, SurfaceScalarField,
    SurfaceScalarFieldKind, SurfaceVectorField, SurfaceVectorFieldKind,
};
use rusty_matter_mesh::{MeshSurfaceSampleConfig, MeshSurfaceSamplePattern, TriangleMeshSurface};
use rusty_matter_model::Vec3;
use wasm_bindgen::prelude::*;

const PLANARIAN_WASM_EDIT_EVENT_CAPACITY: usize = 12;
const PLANARIAN_WASM_EDIT_EVENT_STRIDE: usize = 15;
const PLANARIAN_WASM_EDIT_TARGET_STRIDE: usize = 8;

/// Realtime Matter surface-field runtime exported to browser Wasm.
///
/// The browser owns controls and drawing. This runtime owns the substrate,
/// state, perturbation schedule, sparse neighbor plan, and fixed-step updates.
#[wasm_bindgen]
pub struct SurfaceFieldRealtimeRuntime {
    runtime: SurfaceFieldRuntime,
    substrate: SurfaceFieldSubstrate,
    initial_state: SurfaceFieldState,
    state: SurfaceFieldState,
    perturbations: Vec<SurfaceFieldPerturbation>,
    step_index: u32,
    last_step: SurfaceFieldStepDiagnostics,
}

#[wasm_bindgen]
impl SurfaceFieldRealtimeRuntime {
    /// Creates the deterministic unit-square realtime demo runtime.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when the Matter substrate, fields, runtime
    /// config, or perturbation contracts fail validation.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let (substrate, state, perturbations) = dynamic_contracts()?;
        let runtime = SurfaceFieldRuntime::new(SurfaceFieldRuntimeConfig {
            config_id: "fields.runtime.wasm_dynamic".to_owned(),
            fixed_step_seconds: 1.0 / 30.0,
            max_steps_per_run: 240,
            scalar_diffusion_rate: 2.8,
            scalar_decay_rate: 0.18,
            second_tier_coupling_weight: 0.42,
            vector_alignment_rate: 3.2,
            vector_gradient_rate: 1.9,
            ..SurfaceFieldRuntimeConfig::default()
        })
        .map_err(to_js_error)?;
        Ok(Self {
            runtime,
            substrate,
            initial_state: state.clone(),
            state,
            perturbations,
            step_index: 0,
            last_step: SurfaceFieldStepDiagnostics::empty(0),
        })
    }

    /// Resets the state and timestep to the deterministic initial condition.
    pub fn reset(&mut self) {
        self.state = self.initial_state.clone();
        self.step_index = 0;
        self.last_step = SurfaceFieldStepDiagnostics::empty(0);
    }

    /// Advances one or more Matter fixed steps and returns runtime stats.
    ///
    /// The returned `Float32Array` layout matches `stats()`. Each call is
    /// bounded to at most eight fixed steps so a browser frame cannot request an
    /// unbounded simulation burst.
    pub fn step(&mut self, requested_steps: u32) -> Result<Float32Array, JsValue> {
        let steps = requested_steps.clamp(1, 8);
        for _ in 0..steps {
            self.last_step = self
                .runtime
                .step_fixed(
                    &self.substrate,
                    &mut self.state,
                    &self.perturbations,
                    self.step_index,
                )
                .map_err(to_js_error)?;
            self.step_index += 1;
            self.state.time_seconds =
                self.step_index as f32 * self.runtime.config().fixed_step_seconds;
        }
        Ok(self.stats())
    }

    /// Returns static node topology.
    ///
    /// The returned `Float32Array` layout is six floats per node:
    /// `[x, y, z, nx, ny, nz]`.
    #[must_use]
    pub fn nodes(&self) -> Float32Array {
        let mut values = Vec::with_capacity(self.substrate.node_count() * 6);
        for node in &self.substrate.nodes {
            values.extend_from_slice(&[
                node.position.x,
                node.position.y,
                node.position.z,
                node.normal.x,
                node.normal.y,
                node.normal.z,
            ]);
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns static sparse neighbor edges.
    ///
    /// The returned `Uint32Array` layout is three unsigned integers per edge:
    /// `[from, to, tier]`.
    #[must_use]
    pub fn edges(&self) -> Uint32Array {
        let mut values = Vec::with_capacity(
            self.substrate.first_tier_edge_count() + self.substrate.second_tier_edge_count(),
        );
        for node in &self.substrate.nodes {
            values.extend(
                node.first_tier_neighbors
                    .iter()
                    .copied()
                    .flat_map(|to| [usize_to_u32(node.node_index), usize_to_u32(to), 1]),
            );
            values.extend(
                node.second_tier_neighbors
                    .iter()
                    .copied()
                    .flat_map(|to| [usize_to_u32(node.node_index), usize_to_u32(to), 2]),
            );
        }
        Uint32Array::from(values.as_slice())
    }

    /// Returns perturbation region metadata.
    ///
    /// The returned `Uint32Array` layout is four unsigned integers per region:
    /// `[effect_code, target_code, node_offset, node_count]`.
    #[must_use]
    pub fn region_metadata(&self) -> Uint32Array {
        let mut offset = 0_u32;
        let mut values = Vec::with_capacity(self.perturbations.len() * 4);
        for perturbation in &self.perturbations {
            let len = usize_to_u32(perturbation.node_indices.len());
            values.extend_from_slice(&[
                effect_code(&perturbation.effect),
                target_code(perturbation.target_field_id.as_deref()),
                offset,
                len,
            ]);
            offset = offset.saturating_add(len);
        }
        Uint32Array::from(values.as_slice())
    }

    /// Returns flattened perturbation region node indices.
    #[must_use]
    pub fn region_nodes(&self) -> Uint32Array {
        let values = self
            .perturbations
            .iter()
            .flat_map(|perturbation| perturbation.node_indices.iter().copied())
            .map(usize_to_u32)
            .collect::<Vec<_>>();
        Uint32Array::from(values.as_slice())
    }

    /// Returns current scalar and vector values.
    ///
    /// The returned `Float32Array` layout is six floats per node:
    /// `[vmem_like, wound_signal, morphogen, polarity_x, polarity_y, polarity_z]`.
    #[must_use]
    pub fn snapshot(&self) -> Float32Array {
        let vmem = scalar_values(&self.state, "field.vmem_like");
        let wound = scalar_values(&self.state, "field.wound_signal");
        let morphogen = scalar_values(&self.state, "field.morphogen");
        let polarity = vector_values(&self.state, "field.polarity");
        let mut values = Vec::with_capacity(self.substrate.node_count() * 6);
        for node_index in 0..self.substrate.node_count() {
            let vector = polarity[node_index];
            values.extend_from_slice(&[
                vmem[node_index],
                wound[node_index],
                morphogen[node_index],
                vector.x,
                vector.y,
                vector.z,
            ]);
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns the latest runtime stats.
    ///
    /// The returned `Float32Array` layout is:
    /// `[step, time_seconds, node_count, edge_count, scalar_fields,
    /// vector_fields, active_perturbations, neighbor_links_visited,
    /// clamped_scalars, clamped_vectors, fixed_step_seconds]`.
    #[must_use]
    pub fn stats(&self) -> Float32Array {
        Float32Array::from(
            &[
                self.step_index as f32,
                self.state.time_seconds,
                self.substrate.node_count() as f32,
                (self.substrate.first_tier_edge_count() + self.substrate.second_tier_edge_count())
                    as f32,
                self.state.scalar_fields.len() as f32,
                self.state.vector_fields.len() as f32,
                self.last_step.active_perturbations as f32,
                self.last_step.neighbor_links_visited as f32,
                self.last_step.clamped_scalars as f32,
                self.last_step.clamped_vectors as f32,
                self.runtime.config().fixed_step_seconds,
            ][..],
        )
    }

    /// Returns substrate node count.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.substrate.node_count()
    }
}

#[derive(Clone, Debug)]
struct PlanarianWasmEditEvent {
    event_index: u64,
    step_index: u32,
    time_seconds: f32,
    operation_code: u32,
    target_kind_code: u32,
    target_index: Option<usize>,
    value_a: f32,
    value_b: f32,
    result: BioelectricCircuitEditResult,
}

impl PlanarianWasmEditEvent {
    fn from_operation(
        event_index: u64,
        step_index: u32,
        time_seconds: f32,
        operation: &BioelectricCircuitEditOperation,
        result: BioelectricCircuitEditResult,
    ) -> Self {
        let (operation_code, target_kind_code, target_index, value_a, value_b) = match operation {
            BioelectricCircuitEditOperation::SetNodeVoltage {
                node_index,
                voltage,
            } => (1, 1, Some(*node_index), *voltage, 0.0),
            BioelectricCircuitEditOperation::AddNodeVoltage { node_index, delta } => {
                (2, 1, Some(*node_index), *delta, 0.0)
            }
            BioelectricCircuitEditOperation::SetNodeMemory {
                node_index,
                memory_value,
            } => (3, 1, Some(*node_index), *memory_value, 0.0),
            BioelectricCircuitEditOperation::ScaleIncidentConductance { node_index, scale } => {
                (4, 1, Some(*node_index), *scale, 0.0)
            }
            BioelectricCircuitEditOperation::SetEdgeGateThreshold {
                edge_index,
                threshold,
                slope,
            } => (5, 2, Some(*edge_index), *threshold, slope.unwrap_or(0.0)),
            BioelectricCircuitEditOperation::SetEdgeGateMultiplierBounds {
                edge_index,
                min_multiplier,
                max_multiplier,
            } => (6, 2, Some(*edge_index), *min_multiplier, *max_multiplier),
            BioelectricCircuitEditOperation::AddTransientCurrent {
                target_node_indices,
                current,
                duration_steps,
                ..
            } => (
                7,
                if target_node_indices.is_empty() { 3 } else { 1 },
                target_node_indices.first().copied(),
                *current,
                *duration_steps as f32,
            ),
        };
        Self {
            event_index,
            step_index,
            time_seconds,
            operation_code,
            target_kind_code,
            target_index,
            value_a,
            value_b,
            result,
        }
    }
}

/// Realtime Matter planarian bioelectric runtime exported to browser Wasm.
///
/// Matter owns the source body surface, sampled substrate, circuit state,
/// edit/revision semantics, and fixed-step dynamics. Browser code may render
/// and request edits, but it does not compute bioelectric state.
#[wasm_bindgen]
pub struct PlanarianBioelectricRealtimeRuntime {
    runtime: BioelectricCircuitRuntime,
    source_run: PlanarianBioelectricScenarioRun,
    outcome_trace: PlanarianBioelectricOutcomeTrace,
    outcome_trace_set: PlanarianBioelectricOutcomeTraceSet,
    initial_circuit: BioelectricCircuitState,
    circuit: BioelectricCircuitState,
    step_index: u32,
    last_step: BioelectricCircuitStepDiagnostics,
    last_edit: Option<BioelectricCircuitEditResult>,
    edit_events: Vec<PlanarianWasmEditEvent>,
    edit_event_sequence: u64,
}

#[wasm_bindgen]
impl PlanarianBioelectricRealtimeRuntime {
    /// Creates the deterministic GLB-derived planarian bioelectric runtime.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when the Matter planarian scenario, circuit,
    /// or runtime contracts fail validation.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        Self::for_scenario(PlanarianBioelectricScenarioKind::TransientDepolarizationMemory)
    }

    /// Resets circuit state and timestep to the deterministic initial state.
    pub fn reset(&mut self) {
        self.circuit = self.initial_circuit.clone();
        self.step_index = 0;
        self.last_step = BioelectricCircuitStepDiagnostics::empty(0);
        self.last_edit = None;
        self.edit_events.clear();
        self.edit_event_sequence = 0;
    }

    /// Rebuilds the runtime for a Matter-owned scenario code and returns stats.
    ///
    /// Codes are:
    /// `0=baseline`, `1=transverse wound`, `2=gap block`,
    /// `3=transient memory`, `4=no-memory control`.
    pub fn reset_to_scenario(&mut self, scenario_code: u32) -> Result<Float32Array, JsValue> {
        let next = Self::for_scenario_with_trace_set(
            scenario_kind_from_code(scenario_code)?,
            self.outcome_trace_set.clone(),
        )?;
        self.runtime = next.runtime;
        self.source_run = next.source_run;
        self.outcome_trace = next.outcome_trace;
        self.outcome_trace_set = next.outcome_trace_set;
        self.initial_circuit = next.initial_circuit;
        self.circuit = next.circuit;
        self.step_index = next.step_index;
        self.last_step = next.last_step;
        self.last_edit = next.last_edit;
        self.edit_events = next.edit_events;
        self.edit_event_sequence = next.edit_event_sequence;
        Ok(self.stats())
    }

    /// Advances one or more Matter fixed steps and returns realtime stats.
    ///
    /// The call is bounded to eight fixed steps so a browser frame cannot
    /// request an unbounded simulation burst.
    pub fn step(&mut self, requested_steps: u32) -> Result<Float32Array, JsValue> {
        let steps = requested_steps.clamp(1, 8);
        for _ in 0..steps {
            self.last_step = self
                .runtime
                .step_fixed(
                    &self.source_run.substrate,
                    &mut self.circuit,
                    self.step_index,
                )
                .map_err(to_js_error)?;
            self.step_index += 1;
        }
        Ok(self.stats())
    }

    /// Returns source body vertices.
    ///
    /// The returned `Float32Array` layout is three floats per vertex:
    /// `[x, y, z]`.
    #[must_use]
    pub fn body_vertices(&self) -> Float32Array {
        let mut values = Vec::with_capacity(self.source_run.source_surface.positions.len() * 3);
        for position in &self.source_run.source_surface.positions {
            values.extend_from_slice(&[position.x, position.y, position.z]);
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns source body triangle indices.
    ///
    /// The returned `Uint32Array` layout is three indices per triangle.
    #[must_use]
    pub fn body_triangles(&self) -> Uint32Array {
        let values = self
            .source_run
            .source_surface
            .triangles
            .iter()
            .flat_map(|triangle| triangle.iter().copied())
            .collect::<Vec<_>>();
        Uint32Array::from(values.as_slice())
    }

    /// Returns sampled bioelectric node geometry and AP metadata.
    ///
    /// The returned `Float32Array` layout is nine floats per node:
    /// `[x, y, z, nx, ny, nz, region_code, ap_coordinate, lateral_coordinate]`.
    #[must_use]
    pub fn nodes(&self) -> Float32Array {
        let mut values = Vec::with_capacity(self.source_run.substrate.node_count() * 9);
        for node in &self.source_run.substrate.nodes {
            let region = &self.source_run.axis_map.node_regions[node.node_index];
            values.extend_from_slice(&[
                node.position.x,
                node.position.y,
                node.position.z,
                node.normal.x,
                node.normal.y,
                node.normal.z,
                region_code(region.region) as f32,
                region.ap_coordinate,
                region.lateral_coordinate,
            ]);
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns mesh-anchor metadata for each sampled bioelectric node.
    ///
    /// The returned `Float32Array` layout is four floats per node:
    /// `[source_triangle_index, barycentric_a, barycentric_b, barycentric_c]`.
    /// This lets renderers and later adapters inspect the GLB-derived
    /// mesh-attached data contract without using the GLB as runtime authority.
    #[must_use]
    pub fn node_surface_anchors(&self) -> Float32Array {
        let mut values = Vec::with_capacity(self.source_run.substrate.node_count() * 4);
        for node in &self.source_run.substrate.nodes {
            values.extend_from_slice(&[
                node.triangle_index as f32,
                node.barycentric[0],
                node.barycentric[1],
                node.barycentric[2],
            ]);
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns the number of floats per node in `node_surface_anchors()`.
    #[must_use]
    pub fn node_surface_anchor_stride(&self) -> usize {
        4
    }

    /// Returns conductance edge metadata.
    ///
    /// The returned `Uint32Array` layout is four unsigned integers per edge:
    /// `[from_node, to_node, tier, has_gate]`.
    #[must_use]
    pub fn conductance_edges(&self) -> Uint32Array {
        let values = self
            .circuit
            .conductance_edges
            .iter()
            .flat_map(|edge| {
                [
                    usize_to_u32(edge.from_node),
                    usize_to_u32(edge.to_node),
                    u32::from(edge.tier),
                    u32::from(edge.gate.is_some()),
                ]
            })
            .collect::<Vec<_>>();
        Uint32Array::from(values.as_slice())
    }

    /// Returns dynamic conductance and gate values.
    ///
    /// The returned `Float32Array` layout is six floats per edge:
    /// `[base_conductance, conductance, threshold, slope, min_multiplier,
    /// max_multiplier]`. Missing gates use zeros for gate fields.
    #[must_use]
    pub fn conductance_values(&self) -> Float32Array {
        let mut values = Vec::with_capacity(self.circuit.conductance_edges.len() * 6);
        for edge in &self.circuit.conductance_edges {
            if let Some(gate) = &edge.gate {
                values.extend_from_slice(&[
                    edge.base_conductance,
                    edge.conductance,
                    gate.threshold,
                    gate.slope,
                    gate.min_multiplier,
                    gate.max_multiplier,
                ]);
            } else {
                values.extend_from_slice(&[
                    edge.base_conductance,
                    edge.conductance,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                ]);
            }
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns a Matter-owned readout for one selected node.
    ///
    /// The returned `Float32Array` layout is:
    /// `[node_index, region_code, ap_coordinate, lateral_coordinate, voltage,
    /// memory, head_identity, tail_identity, incident_edge_count,
    /// outgoing_edge_count]`.
    pub fn node_state(&self, node_index: u32) -> Result<Float32Array, JsValue> {
        let node_index = u32_to_usize(node_index);
        let Some(region) = self.source_run.axis_map.node_regions.get(node_index) else {
            return Err(JsValue::from_str(
                "planarian node state target is unavailable",
            ));
        };
        let Some(voltage) = self.circuit.voltage.values.get(node_index).copied() else {
            return Err(JsValue::from_str(
                "planarian node state voltage is unavailable",
            ));
        };
        let memory = self
            .circuit
            .memory
            .as_ref()
            .and_then(|state| state.values.get(node_index))
            .copied()
            .unwrap_or(0.0);
        let head = readout_values(&self.circuit, "readout.planarian_ap.head_identity")
            .and_then(|values| values.get(node_index))
            .copied()
            .unwrap_or(0.0);
        let tail = readout_values(&self.circuit, "readout.planarian_ap.tail_identity")
            .and_then(|values| values.get(node_index))
            .copied()
            .unwrap_or(0.0);
        let incident_edge_count = self
            .circuit
            .conductance_edges
            .iter()
            .filter(|edge| edge.from_node == node_index || edge.to_node == node_index)
            .count();
        let outgoing_edge_count = self
            .circuit
            .conductance_edges
            .iter()
            .filter(|edge| edge.from_node == node_index)
            .count();
        Ok(Float32Array::from(
            &[
                node_index as f32,
                region_code(region.region) as f32,
                region.ap_coordinate,
                region.lateral_coordinate,
                voltage,
                memory,
                head,
                tail,
                incident_edge_count as f32,
                outgoing_edge_count as f32,
            ][..],
        ))
    }

    /// Returns a Matter-owned readout for one selected conductance edge.
    ///
    /// The returned `Float32Array` layout is:
    /// `[edge_index, from_node, to_node, tier, has_gate, base_conductance,
    /// conductance, gate_threshold, gate_slope, gate_min_multiplier,
    /// gate_max_multiplier]`. Missing gates use zeros for gate fields.
    pub fn conductance_edge_state(&self, edge_index: u32) -> Result<Float32Array, JsValue> {
        let edge_index = u32_to_usize(edge_index);
        let Some(edge) = self.circuit.conductance_edges.get(edge_index) else {
            return Err(JsValue::from_str(
                "planarian conductance edge state target is unavailable",
            ));
        };
        let (has_gate, threshold, slope, min_multiplier, max_multiplier) = edge
            .gate
            .as_ref()
            .map_or((0.0, 0.0, 0.0, 0.0, 0.0), |gate| {
                (
                    1.0,
                    gate.threshold,
                    gate.slope,
                    gate.min_multiplier,
                    gate.max_multiplier,
                )
            });
        Ok(Float32Array::from(
            &[
                edge_index as f32,
                edge.from_node as f32,
                edge.to_node as f32,
                f32::from(edge.tier),
                has_gate,
                edge.base_conductance,
                edge.conductance,
                threshold,
                slope,
                min_multiplier,
                max_multiplier,
            ][..],
        ))
    }

    /// Returns current node state.
    ///
    /// The returned `Float32Array` layout is four floats per node:
    /// `[voltage, memory, head_identity, tail_identity]`.
    #[must_use]
    pub fn snapshot(&self) -> Float32Array {
        let memory = self
            .circuit
            .memory
            .as_ref()
            .map(|state| state.values.as_slice());
        let head = readout_values(&self.circuit, "readout.planarian_ap.head_identity");
        let tail = readout_values(&self.circuit, "readout.planarian_ap.tail_identity");
        let mut values = Vec::with_capacity(self.circuit.node_count * 4);
        for node_index in 0..self.circuit.node_count {
            values.extend_from_slice(&[
                self.circuit.voltage.values[node_index],
                memory.map_or(0.0, |values| values[node_index]),
                head.map_or(0.0, |values| values[node_index]),
                tail.map_or(0.0, |values| values[node_index]),
            ]);
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns the Matter-owned deterministic outcome trace for this scenario.
    ///
    /// The returned `Float32Array` layout is seven floats per sample:
    /// `[step_index, time_seconds, posterior_memory_average,
    /// posterior_head_identity_average, head_identity_at_head_average,
    /// tail_identity_at_tail_average, cut_band_voltage_average]`.
    #[must_use]
    pub fn outcome_trace(&self) -> Float32Array {
        outcome_trace_values(&self.outcome_trace)
    }

    /// Returns the number of floats per `outcome_trace()` sample.
    #[must_use]
    pub fn outcome_trace_stride(&self) -> usize {
        7
    }

    /// Returns the number of samples in the current deterministic outcome trace.
    #[must_use]
    pub fn outcome_trace_sample_count(&self) -> usize {
        self.outcome_trace.samples.len()
    }

    /// Returns the initial cross-cut conductance average for this scenario.
    #[must_use]
    pub fn outcome_trace_cross_cut_conductance(&self) -> f32 {
        self.outcome_trace.cross_cut_base_conductance_average
    }

    /// Returns scenario codes available in the Matter comparison trace set.
    #[must_use]
    pub fn comparison_scenario_codes(&self) -> Uint32Array {
        let values = self
            .outcome_trace_set
            .traces
            .iter()
            .map(|trace| scenario_code(trace.scenario_kind))
            .collect::<Vec<_>>();
        Uint32Array::from(values.as_slice())
    }

    /// Returns a Matter-owned deterministic outcome trace for a scenario code.
    pub fn outcome_trace_for_scenario(&self, scenario_code: u32) -> Result<Float32Array, JsValue> {
        Ok(outcome_trace_values(
            self.trace_for_scenario_code(scenario_code)?,
        ))
    }

    /// Returns the cross-cut conductance average for a comparison scenario.
    pub fn outcome_trace_cross_cut_conductance_for_scenario(
        &self,
        scenario_code: u32,
    ) -> Result<f32, JsValue> {
        Ok(self
            .trace_for_scenario_code(scenario_code)?
            .cross_cut_base_conductance_average)
    }

    /// Returns the number of samples for a comparison scenario trace.
    pub fn outcome_trace_sample_count_for_scenario(
        &self,
        scenario_code: u32,
    ) -> Result<usize, JsValue> {
        Ok(self.trace_for_scenario_code(scenario_code)?.samples.len())
    }

    /// Sets one node voltage and returns edit-result stats.
    pub fn set_node_voltage(
        &mut self,
        node_index: u32,
        voltage: f32,
    ) -> Result<Float32Array, JsValue> {
        self.apply_edit(BioelectricCircuitEditOperation::SetNodeVoltage {
            node_index: u32_to_usize(node_index),
            voltage,
        })
    }

    /// Adds a voltage delta to one node and returns edit-result stats.
    pub fn add_node_voltage(
        &mut self,
        node_index: u32,
        delta: f32,
    ) -> Result<Float32Array, JsValue> {
        self.apply_edit(BioelectricCircuitEditOperation::AddNodeVoltage {
            node_index: u32_to_usize(node_index),
            delta,
        })
    }

    /// Sets one node memory value and returns edit-result stats.
    pub fn set_node_memory(
        &mut self,
        node_index: u32,
        memory_value: f32,
    ) -> Result<Float32Array, JsValue> {
        self.apply_edit(BioelectricCircuitEditOperation::SetNodeMemory {
            node_index: u32_to_usize(node_index),
            memory_value,
        })
    }

    /// Scales all conductance edges incident on a node and returns edit-result
    /// stats.
    pub fn scale_incident_conductance(
        &mut self,
        node_index: u32,
        scale: f32,
    ) -> Result<Float32Array, JsValue> {
        self.apply_edit(BioelectricCircuitEditOperation::ScaleIncidentConductance {
            node_index: u32_to_usize(node_index),
            scale,
        })
    }

    /// Sets one edge gate threshold. A zero `slope` keeps the existing slope.
    pub fn set_edge_gate_threshold(
        &mut self,
        edge_index: u32,
        threshold: f32,
        slope: f32,
    ) -> Result<Float32Array, JsValue> {
        self.apply_edit(BioelectricCircuitEditOperation::SetEdgeGateThreshold {
            edge_index: u32_to_usize(edge_index),
            threshold,
            slope: (slope != 0.0).then_some(slope),
        })
    }

    /// Adds a transient constant current to one node beginning at the current
    /// fixed step.
    pub fn add_transient_current(
        &mut self,
        node_index: u32,
        current: f32,
        duration_steps: u32,
    ) -> Result<Float32Array, JsValue> {
        self.apply_edit(BioelectricCircuitEditOperation::AddTransientCurrent {
            term_id: format!(
                "current.planarian_wasm.node{}.{}",
                node_index, self.circuit.revision
            ),
            target_node_indices: vec![u32_to_usize(node_index)],
            current,
            start_step: self.step_index,
            duration_steps,
        })
    }

    /// Returns the compact layout width for recent edit events.
    ///
    /// `edit_event_history()` uses this stride so browser adapters can decode
    /// the bounded Matter-owned feedback stream without hard-coding the width.
    #[must_use]
    pub fn edit_event_history_stride(&self) -> usize {
        PLANARIAN_WASM_EDIT_EVENT_STRIDE
    }

    /// Returns a bounded recent edit event history owned by Matter.
    ///
    /// The returned `Float32Array` layout is 15 floats per event:
    /// `[event_index, step, time_seconds, operation_code, target_kind,
    /// target_index, value_a, value_b, accepted, revision_before,
    /// revision_after, clamped_values, affected_node_count,
    /// affected_edge_count, affected_current_term_count]`.
    ///
    /// Operation codes are:
    /// `1=set node voltage`, `2=add node voltage`, `3=set node memory`,
    /// `4=scale incident conductance`, `5=set edge gate threshold`,
    /// `6=set edge gate multiplier bounds`, `7=add transient current`.
    ///
    /// Target kind codes are:
    /// `1=surface node`, `2=conductance edge`, `3=current term`, `0=none`.
    #[must_use]
    pub fn edit_event_history(&self) -> Float32Array {
        let mut values =
            Vec::with_capacity(self.edit_events.len() * PLANARIAN_WASM_EDIT_EVENT_STRIDE);
        for event in &self.edit_events {
            values.extend_from_slice(&[
                event.event_index as f32,
                event.step_index as f32,
                event.time_seconds,
                event.operation_code as f32,
                event.target_kind_code as f32,
                event.target_index.map_or(-1.0, |index| index as f32),
                event.value_a,
                event.value_b,
                if event.result.accepted { 1.0 } else { 0.0 },
                event.result.revision_before as f32,
                event.result.revision_after as f32,
                event.result.clamped_values as f32,
                event.result.affected_node_indices.len() as f32,
                event.result.affected_edge_indices.len() as f32,
                event.result.affected_current_term_ids.len() as f32,
            ]);
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns the compact layout width for recent affected edit targets.
    ///
    /// `edit_event_targets()` uses this stride so browser adapters can draw
    /// Matter-owned feedback highlights without hard-coding the width.
    #[must_use]
    pub fn edit_event_targets_stride(&self) -> usize {
        PLANARIAN_WASM_EDIT_TARGET_STRIDE
    }

    /// Returns affected node and edge targets for the bounded edit history.
    ///
    /// The returned `Float32Array` layout is eight floats per target:
    /// `[event_index, step, time_seconds, operation_code, target_kind,
    /// target_index, accepted, revision_after]`.
    ///
    /// Target kind codes are:
    /// `1=surface node`, `2=conductance edge`.
    #[must_use]
    pub fn edit_event_targets(&self) -> Float32Array {
        let target_count = self
            .edit_events
            .iter()
            .map(|event| {
                event.result.affected_node_indices.len() + event.result.affected_edge_indices.len()
            })
            .sum::<usize>();
        let mut values = Vec::with_capacity(target_count * PLANARIAN_WASM_EDIT_TARGET_STRIDE);
        for event in &self.edit_events {
            for node_index in &event.result.affected_node_indices {
                push_edit_target_values(&mut values, event, 1, *node_index);
            }
            for edge_index in &event.result.affected_edge_indices {
                push_edit_target_values(&mut values, event, 2, *edge_index);
            }
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns latest runtime stats.
    ///
    /// The returned `Float32Array` layout is:
    /// `[step, time_seconds, revision, node_count, edge_count, current_terms,
    /// active_current_terms, active_gates, clamped_voltage_nodes,
    /// max_voltage_delta, fixed_step_seconds, last_edit_accepted,
    /// last_edit_revision_after, scenario_code, posterior_memory_average,
    /// posterior_head_identity_average, head_identity_at_head_average,
    /// tail_identity_at_tail_average]`.
    #[must_use]
    pub fn stats(&self) -> Float32Array {
        let last_edit_accepted =
            self.last_edit
                .as_ref()
                .map_or(0.0, |result| if result.accepted { 1.0 } else { 0.0 });
        let last_edit_revision_after = self
            .last_edit
            .as_ref()
            .map_or(self.circuit.revision as f32, |result| {
                result.revision_after as f32
            });
        let posterior_nodes = posterior_comparison_nodes(&self.source_run);
        let head_nodes = self
            .source_run
            .axis_map
            .nodes_in_region(PlanarianAxisRegion::Head);
        let tail_nodes = self
            .source_run
            .axis_map
            .nodes_in_region(PlanarianAxisRegion::Tail);
        let memory = self
            .circuit
            .memory
            .as_ref()
            .map(|memory| memory.values.as_slice());
        let head = readout_values(&self.circuit, "readout.planarian_ap.head_identity");
        let tail = readout_values(&self.circuit, "readout.planarian_ap.tail_identity");
        Float32Array::from(
            &[
                self.step_index as f32,
                self.circuit.time_seconds,
                self.circuit.revision as f32,
                self.circuit.node_count as f32,
                self.circuit.conductance_edges.len() as f32,
                self.circuit.current_terms.len() as f32,
                self.last_step.active_current_terms as f32,
                self.last_step.active_gates as f32,
                self.last_step.clamped_voltage_nodes as f32,
                self.last_step.max_voltage_delta,
                self.runtime.config().fixed_step_seconds,
                last_edit_accepted,
                last_edit_revision_after,
                scenario_code(self.source_run.scenario_kind) as f32,
                memory.map_or(0.0, |values| average_nodes(values, &posterior_nodes)),
                head.map_or(0.0, |values| average_nodes(values, &posterior_nodes)),
                head.map_or(0.0, |values| average_nodes(values, &head_nodes)),
                tail.map_or(0.0, |values| average_nodes(values, &tail_nodes)),
            ][..],
        )
    }

    /// Returns substrate node count.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.source_run.substrate.node_count()
    }

    /// Returns source body vertex count.
    #[must_use]
    pub fn body_vertex_count(&self) -> usize {
        self.source_run.source_surface.vertex_count()
    }

    /// Returns source body triangle count.
    #[must_use]
    pub fn body_triangle_count(&self) -> usize {
        self.source_run.source_surface.triangle_count()
    }
}

impl PlanarianBioelectricRealtimeRuntime {
    fn for_scenario(scenario_kind: PlanarianBioelectricScenarioKind) -> Result<Self, JsValue> {
        Self::for_scenario_with_trace_set(scenario_kind, planarian_wasm_outcome_trace_set()?)
    }

    fn for_scenario_with_trace_set(
        scenario_kind: PlanarianBioelectricScenarioKind,
        outcome_trace_set: PlanarianBioelectricOutcomeTraceSet,
    ) -> Result<Self, JsValue> {
        let source_run =
            PlanarianBioelectricScenarioRun::build(scenario_kind, planarian_wasm_config())
                .map_err(to_js_error)?;
        let mut config = source_run.circuit_config.clone();
        config.config_id.clear();
        config
            .config_id
            .push_str("fields.bioelectric_circuit.planarian_wasm_realtime");
        config.max_steps_per_run = u32::MAX;
        let runtime = BioelectricCircuitRuntime::new(config).map_err(to_js_error)?;
        let outcome_trace = outcome_trace_set
            .trace_for_scenario(scenario_kind)
            .ok_or_else(|| JsValue::from_str("missing planarian outcome trace for scenario"))?
            .clone();
        let initial_circuit = source_run.initial_circuit.clone();
        Ok(Self {
            runtime,
            source_run,
            outcome_trace,
            outcome_trace_set,
            circuit: initial_circuit.clone(),
            initial_circuit,
            step_index: 0,
            last_step: BioelectricCircuitStepDiagnostics::empty(0),
            last_edit: None,
            edit_events: Vec::new(),
            edit_event_sequence: 0,
        })
    }

    fn apply_edit(
        &mut self,
        operation: BioelectricCircuitEditOperation,
    ) -> Result<Float32Array, JsValue> {
        let edit = BioelectricCircuitEdit::new(
            format!("edit.planarian_wasm.{}", self.circuit.revision),
            None,
            operation.clone(),
        );
        let result = self
            .runtime
            .apply_edit(&self.source_run.substrate, &mut self.circuit, &edit)
            .map_err(to_js_error)?;
        let values = edit_result_values(&result);
        self.record_edit_event(&operation, &result);
        self.last_edit = Some(result);
        Ok(values)
    }

    fn record_edit_event(
        &mut self,
        operation: &BioelectricCircuitEditOperation,
        result: &BioelectricCircuitEditResult,
    ) {
        let event = PlanarianWasmEditEvent::from_operation(
            self.edit_event_sequence,
            self.step_index,
            self.circuit.time_seconds,
            operation,
            result.clone(),
        );
        self.edit_event_sequence = self.edit_event_sequence.saturating_add(1);
        if self.edit_events.len() == PLANARIAN_WASM_EDIT_EVENT_CAPACITY {
            self.edit_events.remove(0);
        }
        self.edit_events.push(event);
    }

    fn trace_for_scenario_code(
        &self,
        scenario_code_value: u32,
    ) -> Result<&PlanarianBioelectricOutcomeTrace, JsValue> {
        let scenario_kind = scenario_kind_from_code(scenario_code_value)?;
        self.outcome_trace_set
            .trace_for_scenario(scenario_kind)
            .ok_or_else(|| JsValue::from_str("missing planarian outcome trace for scenario"))
    }
}

fn dynamic_contracts() -> Result<
    (
        SurfaceFieldSubstrate,
        SurfaceFieldState,
        Vec<SurfaceFieldPerturbation>,
    ),
    JsValue,
> {
    let surface = unit_square_surface();
    let samples = surface
        .sample_points(&MeshSurfaceSampleConfig {
            sample_config_id: "mesh.surface_sample.field_wasm_dynamic".to_owned(),
            sample_set_id: "mesh.surface_samples.field_wasm_dynamic".to_owned(),
            point_count: 64,
            first_tier_neighbor_count: 4,
            second_tier_neighbor_count: 4,
            seed: 65_537,
            pattern: MeshSurfaceSamplePattern::LowDiscrepancy,
            ..MeshSurfaceSampleConfig::default()
        })
        .map_err(to_js_error)?;
    let substrate = SurfaceFieldSubstrate::from_sample_set(
        "fields.substrate.wasm_unit_square_dynamic",
        &samples,
    )
    .map_err(to_js_error)?;
    let node_count = substrate.node_count();
    let vmem_values = substrate
        .nodes
        .iter()
        .map(|node| 0.16 + (node.position.y - 0.5) * 0.18)
        .collect::<Vec<_>>();
    let morphogen_values = substrate
        .nodes
        .iter()
        .map(|node| node.position.x.clamp(0.0, 1.0))
        .collect::<Vec<_>>();
    let polarity_vectors = substrate
        .nodes
        .iter()
        .map(|node| normalize(Vec3::new(1.0, (node.position.y - 0.5) * 0.45, 0.0)))
        .collect::<Vec<_>>();
    let state = SurfaceFieldState::new(
        "fields.state.wasm_unit_square_dynamic",
        &substrate,
        vec![
            SurfaceScalarField::new(
                "field.vmem_like",
                SurfaceScalarFieldKind::VmemLike,
                vmem_values,
            ),
            SurfaceScalarField::constant(
                "field.wound_signal",
                SurfaceScalarFieldKind::WoundSignal,
                node_count,
                0.0,
            ),
            SurfaceScalarField::new(
                "field.morphogen",
                SurfaceScalarFieldKind::Morphogen,
                morphogen_values,
            ),
        ],
        vec![SurfaceVectorField::new(
            "field.polarity",
            SurfaceVectorFieldKind::Polarity,
            polarity_vectors,
        )],
    )
    .map_err(to_js_error)?;

    let mut wound = SurfaceFieldPerturbation::new(
        "perturbation.wound.dynamic_center",
        Some("field.wound_signal".to_owned()),
        nearest_nodes(&substrate, Vec3::new(0.28, 0.64, 0.0), 6),
        SurfaceFieldPerturbationEffect::WoundRegion { signal_value: 1.0 },
    );
    wound.duration_steps = 30;
    let mut vmem = SurfaceFieldPerturbation::new(
        "perturbation.vmem.dynamic_offset",
        Some("field.vmem_like".to_owned()),
        nearest_nodes(&substrate, Vec3::new(0.50, 0.48, 0.0), 10),
        SurfaceFieldPerturbationEffect::DepolarizeRegion { delta: 0.12 },
    );
    vmem.start_step = 10;
    vmem.duration_steps = 36;
    let mut polarity = SurfaceFieldPerturbation::new(
        "perturbation.polarity.dynamic_inversion",
        Some("field.polarity".to_owned()),
        nearest_nodes(&substrate, Vec3::new(0.72, 0.34, 0.0), 8),
        SurfaceFieldPerturbationEffect::PolarityInversion,
    );
    polarity.start_step = 18;
    let mut coupling = SurfaceFieldPerturbation::new(
        "perturbation.coupling.dynamic_wound_shell",
        None,
        nearest_nodes(&substrate, Vec3::new(0.36, 0.58, 0.0), 14),
        SurfaceFieldPerturbationEffect::CouplingMultiplierChange { multiplier: 1.45 },
    );
    coupling.duration_steps = 90;

    Ok((substrate, state, vec![wound, vmem, polarity, coupling]))
}

fn planarian_wasm_config() -> PlanarianBioelectricPresetConfig {
    PlanarianBioelectricPresetConfig {
        sample_count: 160,
        first_tier_neighbor_count: 5,
        second_tier_neighbor_count: 5,
        step_count: 240,
        frame_stride: 12,
        seed: 196_613,
        ..PlanarianBioelectricPresetConfig::default()
    }
}

fn planarian_wasm_outcome_trace_set() -> Result<PlanarianBioelectricOutcomeTraceSet, JsValue> {
    PlanarianBioelectricOutcomeTraceSet::from_preset_config(
        "fields.planarian_ap.wasm_comparison_outcome_trace_set",
        &planarian_comparison_scenario_kinds(),
        planarian_wasm_comparison_config(),
    )
    .map_err(to_js_error)
}

fn planarian_wasm_comparison_config() -> PlanarianBioelectricPresetConfig {
    PlanarianBioelectricPresetConfig {
        sample_count: 128,
        step_count: 240,
        frame_stride: 12,
        seed: 196_613,
        ..PlanarianBioelectricPresetConfig::default()
    }
}

fn scenario_kind_from_code(code: u32) -> Result<PlanarianBioelectricScenarioKind, JsValue> {
    match code {
        0 => Ok(PlanarianBioelectricScenarioKind::Baseline),
        1 => Ok(PlanarianBioelectricScenarioKind::TransverseCutWound),
        2 => Ok(PlanarianBioelectricScenarioKind::GapBlock),
        3 => Ok(PlanarianBioelectricScenarioKind::TransientDepolarizationMemory),
        4 => Ok(PlanarianBioelectricScenarioKind::TransientDepolarizationNoMemoryControl),
        _ => Err(JsValue::from_str("unknown planarian scenario code")),
    }
}

fn scenario_code(kind: PlanarianBioelectricScenarioKind) -> u32 {
    match kind {
        PlanarianBioelectricScenarioKind::Baseline => 0,
        PlanarianBioelectricScenarioKind::TransverseCutWound => 1,
        PlanarianBioelectricScenarioKind::GapBlock => 2,
        PlanarianBioelectricScenarioKind::TransientDepolarizationMemory => 3,
        PlanarianBioelectricScenarioKind::TransientDepolarizationNoMemoryControl => 4,
    }
}

fn posterior_comparison_nodes(source_run: &PlanarianBioelectricScenarioRun) -> Vec<usize> {
    let mut nodes = source_run
        .axis_map
        .nodes_in_region(PlanarianAxisRegion::Tail);
    nodes.extend(
        source_run
            .axis_map
            .nodes_in_region(PlanarianAxisRegion::PostpharyngealTrunk),
    );
    nodes
}

fn average_nodes(values: &[f32], nodes: &[usize]) -> f32 {
    if nodes.is_empty() {
        return 0.0;
    }
    nodes
        .iter()
        .copied()
        .filter_map(|node_index| values.get(node_index).copied())
        .sum::<f32>()
        / nodes.len() as f32
}

fn outcome_trace_values(trace: &PlanarianBioelectricOutcomeTrace) -> Float32Array {
    let mut values = Vec::with_capacity(trace.samples.len() * 7);
    for sample in &trace.samples {
        values.extend_from_slice(&[
            sample.step_index as f32,
            sample.time_seconds,
            sample.posterior_memory_average,
            sample.posterior_head_identity_average,
            sample.head_identity_at_head_average,
            sample.tail_identity_at_tail_average,
            sample.cut_band_voltage_average,
        ]);
    }
    Float32Array::from(values.as_slice())
}

fn scalar_values<'a>(state: &'a SurfaceFieldState, field_id: &str) -> &'a [f32] {
    state
        .scalar_field(field_id)
        .map_or(&[], |field| field.values.as_slice())
}

fn vector_values<'a>(state: &'a SurfaceFieldState, field_id: &str) -> &'a [Vec3] {
    state
        .vector_field(field_id)
        .map_or(&[], |field| field.vectors.as_slice())
}

fn unit_square_surface() -> TriangleMeshSurface {
    TriangleMeshSurface::new(
        "mesh.unit_square_surface",
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    )
}

fn nearest_nodes(substrate: &SurfaceFieldSubstrate, center: Vec3, count: usize) -> Vec<usize> {
    let mut nodes = substrate
        .nodes
        .iter()
        .map(|node| (node.node_index, node.position.distance_squared(center)))
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.1.total_cmp(&right.1));
    nodes
        .into_iter()
        .take(count.min(substrate.node_count()))
        .map(|(node_index, _)| node_index)
        .collect()
}

fn normalize(vector: Vec3) -> Vec3 {
    let length = vector.length();
    if length > 1.0e-6 {
        vector / length
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    }
}

fn effect_code(effect: &SurfaceFieldPerturbationEffect) -> u32 {
    match effect {
        SurfaceFieldPerturbationEffect::WoundRegion { .. } => 1,
        SurfaceFieldPerturbationEffect::DepolarizeRegion { .. } => 2,
        SurfaceFieldPerturbationEffect::PolarityInversion => 3,
        SurfaceFieldPerturbationEffect::CouplingMultiplierChange { .. } => 4,
        SurfaceFieldPerturbationEffect::NormalPolarity { .. } => 5,
    }
}

fn target_code(target_field_id: Option<&str>) -> u32 {
    match target_field_id {
        Some("field.wound_signal") => 1,
        Some("field.vmem_like") => 2,
        Some("field.polarity") => 3,
        Some("field.morphogen") => 4,
        Some(_) => 99,
        None => 0,
    }
}

fn region_code(region: PlanarianAxisRegion) -> u32 {
    match region {
        PlanarianAxisRegion::Tail => 1,
        PlanarianAxisRegion::PostpharyngealTrunk => 2,
        PlanarianAxisRegion::PharyngealTrunk => 3,
        PlanarianAxisRegion::PrepharyngealTrunk => 4,
        PlanarianAxisRegion::Head => 5,
    }
}

fn readout_values<'a>(state: &'a BioelectricCircuitState, layer_id: &str) -> Option<&'a [f32]> {
    state
        .readout_layers
        .iter()
        .find(|layer| layer.layer_id == layer_id)
        .map(|layer| layer.values.as_slice())
}

fn push_edit_target_values(
    values: &mut Vec<f32>,
    event: &PlanarianWasmEditEvent,
    target_kind_code: u32,
    target_index: usize,
) {
    values.extend_from_slice(&[
        event.event_index as f32,
        event.step_index as f32,
        event.time_seconds,
        event.operation_code as f32,
        target_kind_code as f32,
        target_index as f32,
        if event.result.accepted { 1.0 } else { 0.0 },
        event.result.revision_after as f32,
    ]);
}

fn edit_result_values(result: &BioelectricCircuitEditResult) -> Float32Array {
    Float32Array::from(
        &[
            if result.accepted { 1.0 } else { 0.0 },
            result.revision_before as f32,
            result.revision_after as f32,
            result.clamped_values as f32,
            result.affected_node_indices.len() as f32,
            result.affected_edge_indices.len() as f32,
            result.affected_current_term_ids.len() as f32,
        ][..],
    )
}

fn usize_to_u32(value: usize) -> u32 {
    value.try_into().unwrap_or(u32::MAX)
}

fn u32_to_usize(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

#[allow(clippy::needless_pass_by_value)]
fn to_js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
