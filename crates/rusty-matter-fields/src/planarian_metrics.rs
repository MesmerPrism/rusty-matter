use crate::{
    BioelectricCircuitDebugFrame, BioelectricConductanceEdge, MatterFieldError, PlanarianAxisMap,
    PlanarianAxisRegion, PlanarianBioelectricPresetConfig, PlanarianBioelectricScenarioKind,
    PlanarianBioelectricScenarioRun, PLANARIAN_BIOELECTRIC_OUTCOME_TRACE_SCHEMA_ID,
    PLANARIAN_BIOELECTRIC_OUTCOME_TRACE_SET_SCHEMA_ID,
    PLANARIAN_NORMALIZED_MORPHOLOGY_METRICS_SCHEMA_ID,
};

const HEAD_IDENTITY_READOUT_ID: &str = "readout.planarian_ap.head_identity";
const TAIL_IDENTITY_READOUT_ID: &str = "readout.planarian_ap.tail_identity";
const DEFAULT_CUT_Z: f32 = 0.16;
const DEFAULT_CUT_HALF_WIDTH: f32 = 0.11;
const HEAD_SIZE_SCALING_SOURCE_TARGET_ANCHOR: &str =
    "source:beane_2013_dev::target:head_size_scaling::future_metric";
const NORMALIZED_MORPHOLOGY_UNIT_POLICY: &str =
    "mesh-normalized educational geometry/readout extents; not calibrated area or physiology";

/// One sampled educational outcome metric row for a planarian bioelectric run.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianBioelectricOutcomeSample {
    /// Fixed-step index represented by this sample.
    pub step_index: u32,
    /// Simulation time in seconds represented by this sample.
    pub time_seconds: f32,
    /// Mean hysteresis-memory value in posterior reprogramming nodes.
    pub posterior_memory_average: f32,
    /// Mean head-identity readout in posterior reprogramming nodes.
    pub posterior_head_identity_average: f32,
    /// Mean head-identity readout in the anatomical head region.
    pub head_identity_at_head_average: f32,
    /// Mean tail-identity readout in the anatomical tail region.
    pub tail_identity_at_tail_average: f32,
    /// Mean voltage in the educational transverse-cut band.
    pub cut_band_voltage_average: f32,
}

impl PlanarianBioelectricOutcomeSample {
    fn validate(&self, previous_step: Option<u32>) -> Result<(), MatterFieldError> {
        if let Some(previous_step) = previous_step {
            if self.step_index <= previous_step {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian outcome trace steps must be increasing",
                ));
            }
        }
        if !self.time_seconds.is_finite() || self.time_seconds < 0.0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian outcome trace sample time must be finite and non-negative",
            ));
        }
        validate_fraction(
            self.posterior_memory_average,
            "planarian posterior memory outcome",
        )?;
        validate_fraction(
            self.posterior_head_identity_average,
            "planarian posterior head-identity outcome",
        )?;
        validate_fraction(
            self.head_identity_at_head_average,
            "planarian head-region head-identity outcome",
        )?;
        validate_fraction(
            self.tail_identity_at_tail_average,
            "planarian tail-region tail-identity outcome",
        )?;
        if !self.cut_band_voltage_average.is_finite() {
            return Err(MatterFieldError::InvalidField(
                "planarian cut-band voltage outcome must be finite",
            ));
        }
        Ok(())
    }
}

/// Compact Matter-owned outcome trace for one educational planarian scenario.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianBioelectricOutcomeTrace {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable trace identifier.
    pub trace_id: String,
    /// Source scenario identifier.
    pub scenario_id: String,
    /// Source scenario kind.
    pub scenario_kind: PlanarianBioelectricScenarioKind,
    /// Expected qualitative behavior copied from the source run.
    pub expected_outcome: String,
    /// Source evidence type copied from the source run.
    pub evidence_type: String,
    /// Source substrate identifier.
    pub substrate_id: String,
    /// Source surface identifier.
    pub surface_id: String,
    /// Fixed-step duration in seconds.
    pub fixed_step_seconds: f32,
    /// Scenario frame stride used for samples.
    pub frame_stride: u32,
    /// Mean initial base conductance across the educational transverse-cut band.
    pub cross_cut_base_conductance_average: f32,
    /// Sample columns exported to browser Wasm in order.
    pub sample_columns: Vec<String>,
    /// Outcome samples over the executed scenario sequence.
    pub samples: Vec<PlanarianBioelectricOutcomeSample>,
}

impl PlanarianBioelectricOutcomeTrace {
    /// Builds a compact outcome trace from a validated planarian scenario run.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the source run or generated trace is
    /// invalid.
    pub fn from_scenario_run(
        trace_id: impl Into<String>,
        run: &PlanarianBioelectricScenarioRun,
    ) -> Result<Self, MatterFieldError> {
        run.validate()?;
        let posterior_nodes = posterior_comparison_nodes(&run.axis_map);
        let head_nodes = run.axis_map.nodes_in_region(PlanarianAxisRegion::Head);
        let tail_nodes = run.axis_map.nodes_in_region(PlanarianAxisRegion::Tail);
        let cut_band_nodes = run
            .axis_map
            .nodes_in_z_band(DEFAULT_CUT_Z, DEFAULT_CUT_HALF_WIDTH);
        if posterior_nodes.is_empty()
            || head_nodes.is_empty()
            || tail_nodes.is_empty()
            || cut_band_nodes.is_empty()
        {
            return Err(MatterFieldError::InvalidSubstrate(
                "planarian outcome trace regions must contain sampled nodes",
            ));
        }
        let samples = run
            .sequence
            .frames
            .iter()
            .map(|frame| {
                outcome_sample_from_frame(
                    frame,
                    &posterior_nodes,
                    &head_nodes,
                    &tail_nodes,
                    &cut_band_nodes,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let trace = Self {
            schema_id: PLANARIAN_BIOELECTRIC_OUTCOME_TRACE_SCHEMA_ID.to_owned(),
            trace_id: trace_id.into(),
            scenario_id: run.scenario_id.clone(),
            scenario_kind: run.scenario_kind,
            expected_outcome: run.expected_outcome.clone(),
            evidence_type: run.evidence_type.clone(),
            substrate_id: run.substrate.substrate_id.clone(),
            surface_id: run.source_surface.surface_id.clone(),
            fixed_step_seconds: run.circuit_config.fixed_step_seconds,
            frame_stride: run.sequence.frame_stride,
            cross_cut_base_conductance_average: average_cross_cut_base_conductance(
                &run.initial_circuit.conductance_edges,
                &run.axis_map,
                DEFAULT_CUT_Z,
            )?,
            sample_columns: vec![
                "step_index".to_owned(),
                "time_seconds".to_owned(),
                "posterior_memory_average".to_owned(),
                "posterior_head_identity_average".to_owned(),
                "head_identity_at_head_average".to_owned(),
                "tail_identity_at_tail_average".to_owned(),
                "cut_band_voltage_average".to_owned(),
            ],
            samples,
        };
        trace.validate()?;
        Ok(trace)
    }

    /// Validates the outcome-trace contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, IDs, timing, or values are
    /// invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_BIOELECTRIC_OUTCOME_TRACE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_BIOELECTRIC_OUTCOME_TRACE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.trace_id.trim().is_empty()
            || self.scenario_id != self.scenario_kind.scenario_id()
            || self.expected_outcome.trim().is_empty()
            || self.evidence_type.trim().is_empty()
            || self.substrate_id.trim().is_empty()
            || self.surface_id.trim().is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian outcome trace metadata must be populated and consistent",
            ));
        }
        if !self.fixed_step_seconds.is_finite() || self.fixed_step_seconds <= 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "planarian outcome trace fixed step must be positive",
            ));
        }
        if self.frame_stride == 0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian outcome trace frame stride must be positive",
            ));
        }
        if !self.cross_cut_base_conductance_average.is_finite()
            || self.cross_cut_base_conductance_average < 0.0
        {
            return Err(MatterFieldError::InvalidField(
                "planarian outcome trace conductance average must be finite and non-negative",
            ));
        }
        if self.samples.is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian outcome trace must include samples",
            ));
        }
        if self.sample_columns.len() != 7 || self.sample_columns.iter().any(|id| id.is_empty()) {
            return Err(MatterFieldError::InvalidField(
                "planarian outcome trace columns must match the exported layout",
            ));
        }
        let mut previous_step = None;
        for sample in &self.samples {
            sample.validate(previous_step)?;
            previous_step = Some(sample.step_index);
        }
        Ok(())
    }
}

/// Matter-owned comparison bundle over several deterministic planarian traces.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianBioelectricOutcomeTraceSet {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable comparison-set identifier.
    pub trace_set_id: String,
    /// Shared source substrate identifier.
    pub substrate_id: String,
    /// Shared source surface identifier.
    pub surface_id: String,
    /// Shared evidence type.
    pub evidence_type: String,
    /// Fixed-step duration in seconds shared by all traces.
    pub fixed_step_seconds: f32,
    /// Scenario frame stride shared by all traces.
    pub frame_stride: u32,
    /// Sample columns exported to browser Wasm in order.
    pub sample_columns: Vec<String>,
    /// Scenario traces included in this comparison bundle.
    pub traces: Vec<PlanarianBioelectricOutcomeTrace>,
}

impl PlanarianBioelectricOutcomeTraceSet {
    /// Builds traces for a scenario family using one shared preset config.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when any generated run or trace is invalid,
    /// or when traces cannot be compared as one bundle.
    pub fn from_preset_config(
        trace_set_id: impl Into<String>,
        scenario_kinds: &[PlanarianBioelectricScenarioKind],
        config: PlanarianBioelectricPresetConfig,
    ) -> Result<Self, MatterFieldError> {
        config.validate()?;
        if scenario_kinds.is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian outcome trace set must include scenarios",
            ));
        }
        let traces = scenario_kinds
            .iter()
            .copied()
            .map(|scenario_kind| {
                let run = PlanarianBioelectricScenarioRun::build(scenario_kind, config.clone())?;
                PlanarianBioelectricOutcomeTrace::from_scenario_run(
                    format!("{}.outcome_trace", run.scenario_id),
                    &run,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let Some(first_trace) = traces.first() else {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian outcome trace set must include traces",
            ));
        };
        let trace_set = Self {
            schema_id: PLANARIAN_BIOELECTRIC_OUTCOME_TRACE_SET_SCHEMA_ID.to_owned(),
            trace_set_id: trace_set_id.into(),
            substrate_id: first_trace.substrate_id.clone(),
            surface_id: first_trace.surface_id.clone(),
            evidence_type: first_trace.evidence_type.clone(),
            fixed_step_seconds: first_trace.fixed_step_seconds,
            frame_stride: first_trace.frame_stride,
            sample_columns: first_trace.sample_columns.clone(),
            traces,
        };
        trace_set.validate()?;
        Ok(trace_set)
    }

    /// Returns a trace by scenario kind.
    #[must_use]
    pub fn trace_for_scenario(
        &self,
        scenario_kind: PlanarianBioelectricScenarioKind,
    ) -> Option<&PlanarianBioelectricOutcomeTrace> {
        self.traces
            .iter()
            .find(|trace| trace.scenario_kind == scenario_kind)
    }

    /// Validates the comparison-bundle contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when metadata, trace identity, shared timing,
    /// or scenario uniqueness is invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_BIOELECTRIC_OUTCOME_TRACE_SET_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_BIOELECTRIC_OUTCOME_TRACE_SET_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.trace_set_id.trim().is_empty()
            || self.substrate_id.trim().is_empty()
            || self.surface_id.trim().is_empty()
            || self.evidence_type.trim().is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian outcome trace set metadata must be populated",
            ));
        }
        if !self.fixed_step_seconds.is_finite() || self.fixed_step_seconds <= 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "planarian outcome trace set fixed step must be positive",
            ));
        }
        if self.frame_stride == 0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian outcome trace set frame stride must be positive",
            ));
        }
        if self.sample_columns.len() != 7 || self.sample_columns.iter().any(|id| id.is_empty()) {
            return Err(MatterFieldError::InvalidField(
                "planarian outcome trace set columns must match the exported layout",
            ));
        }
        if self.traces.is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian outcome trace set must include traces",
            ));
        }
        let mut scenario_kinds = Vec::with_capacity(self.traces.len());
        for trace in &self.traces {
            trace.validate()?;
            if trace.substrate_id != self.substrate_id
                || trace.surface_id != self.surface_id
                || trace.evidence_type != self.evidence_type
                || (trace.fixed_step_seconds - self.fixed_step_seconds).abs() > f32::EPSILON
                || trace.frame_stride != self.frame_stride
                || trace.sample_columns != self.sample_columns
            {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian outcome trace set traces must share source, timing, and columns",
                ));
            }
            if scenario_kinds.contains(&trace.scenario_kind) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian outcome trace set must not repeat scenarios",
                ));
            }
            scenario_kinds.push(trace.scenario_kind);
        }
        Ok(())
    }
}

/// One normalized AP-region geometry summary over a planarian sampled graph.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianRegionExtentMetric {
    /// Region enum.
    pub region: PlanarianAxisRegion,
    /// Stable region identifier.
    pub region_id: String,
    /// Human-readable label.
    pub label: String,
    /// Sampled nodes classified into the region.
    pub node_count: usize,
    /// Fraction of all sampled nodes in this region.
    pub node_fraction: f32,
    /// Minimum sampled AP coordinate in 0..=1.
    pub ap_min: f32,
    /// Maximum sampled AP coordinate in 0..=1.
    pub ap_max: f32,
    /// Sampled AP span in 0..=1.
    pub sampled_ap_span_normalized: f32,
    /// Mean absolute lateral coordinate over sampled nodes.
    pub mean_abs_lateral_coordinate: f32,
}

impl PlanarianRegionExtentMetric {
    fn from_axis_map(
        axis_map: &PlanarianAxisMap,
        region: PlanarianAxisRegion,
    ) -> Result<Self, MatterFieldError> {
        let region_nodes = axis_map
            .node_regions
            .iter()
            .filter(|node| node.region == region)
            .collect::<Vec<_>>();
        if region_nodes.is_empty() {
            return Err(MatterFieldError::InvalidField(
                "planarian morphology metric requires every AP region",
            ));
        }
        let node_count = region_nodes.len();
        let total_count = axis_map.node_regions.len();
        let mut ap_min = f32::INFINITY;
        let mut ap_max = f32::NEG_INFINITY;
        let mut lateral_sum = 0.0;
        for node in region_nodes {
            ap_min = ap_min.min(node.ap_coordinate);
            ap_max = ap_max.max(node.ap_coordinate);
            lateral_sum += node.lateral_coordinate.abs();
        }
        let metric = Self {
            region,
            region_id: region.region_id().to_owned(),
            label: region.label().to_owned(),
            node_count,
            node_fraction: node_count as f32 / total_count as f32,
            ap_min,
            ap_max,
            sampled_ap_span_normalized: ap_max - ap_min,
            mean_abs_lateral_coordinate: lateral_sum / node_count as f32,
        };
        metric.validate(total_count)?;
        Ok(metric)
    }

    fn validate(&self, total_node_count: usize) -> Result<(), MatterFieldError> {
        if self.region_id != self.region.region_id()
            || self.label.trim().is_empty()
            || self.node_count == 0
            || self.node_count > total_node_count
        {
            return Err(MatterFieldError::InvalidField(
                "planarian region extent metadata must match sampled regions",
            ));
        }
        validate_fraction(self.node_fraction, "planarian region node fraction")?;
        validate_fraction(self.ap_min, "planarian region AP minimum")?;
        validate_fraction(self.ap_max, "planarian region AP maximum")?;
        validate_fraction(
            self.sampled_ap_span_normalized,
            "planarian sampled region AP span",
        )?;
        if self.ap_min > self.ap_max {
            return Err(MatterFieldError::InvalidField(
                "planarian region AP extent must be ordered",
            ));
        }
        if !self.mean_abs_lateral_coordinate.is_finite() || self.mean_abs_lateral_coordinate < 0.0 {
            return Err(MatterFieldError::InvalidField(
                "planarian region lateral coordinate summary must be finite",
            ));
        }
        Ok(())
    }
}

/// Normalized planarian morphology/readout metrics for educational checks.
///
/// These values summarize Matter-owned sampled geometry and readouts. They are
/// not calibrated head-size, organ-size, or physiology predictions.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianNormalizedMorphologyMetrics {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable metric bundle identifier.
    pub metrics_id: String,
    /// Source scenario identifier.
    pub scenario_id: String,
    /// Source substrate identifier.
    pub source_substrate_id: String,
    /// Source surface identifier.
    pub source_surface_id: String,
    /// Source axis-map identifier.
    pub source_axis_map_id: String,
    /// Evidence type copied from the scenario run.
    pub evidence_type: String,
    /// Unit policy and non-calibration boundary.
    pub unit_policy: String,
    /// Source-target status for this first metric slice.
    pub source_target_status: String,
    /// Source-target anchors represented by the metric bundle.
    pub source_target_anchors: Vec<String>,
    /// Sampled AP-region extent metrics.
    pub region_extents: Vec<PlanarianRegionExtentMetric>,
    /// Head AP-region sampled extent in normalized body coordinates.
    pub head_region_extent_normalized: f32,
    /// Pharyngeal trunk AP-region sampled extent in normalized body coordinates.
    pub pharyngeal_region_extent_normalized: f32,
    /// Fraction of sampled nodes whose final head-identity readout is at least 0.5.
    pub head_identity_extent_normalized: f32,
}

impl PlanarianNormalizedMorphologyMetrics {
    /// Builds normalized educational metrics from one validated planarian run.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the source run or generated metric
    /// contract is invalid.
    pub fn from_scenario_run(
        metrics_id: impl Into<String>,
        run: &PlanarianBioelectricScenarioRun,
    ) -> Result<Self, MatterFieldError> {
        run.validate()?;
        let region_extents = PlanarianAxisRegion::all()
            .into_iter()
            .map(|region| PlanarianRegionExtentMetric::from_axis_map(&run.axis_map, region))
            .collect::<Result<Vec<_>, _>>()?;
        let final_frame = run
            .sequence
            .frames
            .last()
            .ok_or(MatterFieldError::InvalidRunSummary(
                "planarian normalized morphology metrics require a final frame",
            ))?;
        let head_readout = readout_values(final_frame, HEAD_IDENTITY_READOUT_ID)?;
        let head_identity_extent_normalized = head_readout
            .iter()
            .filter(|value| value.is_finite() && **value >= 0.5)
            .count() as f32
            / head_readout.len() as f32;
        let metrics = Self {
            schema_id: PLANARIAN_NORMALIZED_MORPHOLOGY_METRICS_SCHEMA_ID.to_owned(),
            metrics_id: metrics_id.into(),
            scenario_id: run.scenario_id.clone(),
            source_substrate_id: run.substrate.substrate_id.clone(),
            source_surface_id: run.source_surface.surface_id.clone(),
            source_axis_map_id: run.axis_map.map_id.clone(),
            evidence_type: run.evidence_type.clone(),
            unit_policy: NORMALIZED_MORPHOLOGY_UNIT_POLICY.to_owned(),
            source_target_status: "source_reviewed_metric_path_without_calibrated_thresholds"
                .to_owned(),
            source_target_anchors: vec![HEAD_SIZE_SCALING_SOURCE_TARGET_ANCHOR.to_owned()],
            head_region_extent_normalized: region_span(&region_extents, PlanarianAxisRegion::Head)?,
            pharyngeal_region_extent_normalized: region_span(
                &region_extents,
                PlanarianAxisRegion::PharyngealTrunk,
            )?,
            head_identity_extent_normalized,
            region_extents,
        };
        metrics.validate()?;
        Ok(metrics)
    }

    /// Validates the metric bundle.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, source metadata, unit policy,
    /// anchors, or normalized values are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_NORMALIZED_MORPHOLOGY_METRICS_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_NORMALIZED_MORPHOLOGY_METRICS_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.metrics_id.trim().is_empty()
            || self.scenario_id.trim().is_empty()
            || self.source_substrate_id.trim().is_empty()
            || self.source_surface_id.trim().is_empty()
            || self.source_axis_map_id.trim().is_empty()
            || self.evidence_type.trim().is_empty()
            || !self.unit_policy.contains("not calibrated")
            || self.source_target_status.trim().is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian normalized morphology metric metadata must be populated",
            ));
        }
        if self.source_target_anchors != [HEAD_SIZE_SCALING_SOURCE_TARGET_ANCHOR.to_owned()] {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian normalized morphology metrics must keep the head-size source-target anchor",
            ));
        }
        validate_fraction(
            self.head_region_extent_normalized,
            "planarian normalized head-region extent",
        )?;
        validate_fraction(
            self.pharyngeal_region_extent_normalized,
            "planarian normalized pharyngeal-region extent",
        )?;
        validate_fraction(
            self.head_identity_extent_normalized,
            "planarian normalized head-identity extent",
        )?;
        if self.region_extents.len() != PlanarianAxisRegion::all().len() {
            return Err(MatterFieldError::InvalidField(
                "planarian normalized morphology metrics require every AP region",
            ));
        }
        let mut total_nodes = 0_usize;
        for (metric, expected_region) in self.region_extents.iter().zip(PlanarianAxisRegion::all())
        {
            if metric.region != expected_region {
                return Err(MatterFieldError::InvalidField(
                    "planarian normalized morphology region metrics must be posterior-to-anterior",
                ));
            }
            total_nodes += metric.node_count;
        }
        if total_nodes == 0 {
            return Err(MatterFieldError::InvalidField(
                "planarian normalized morphology metrics require sampled nodes",
            ));
        }
        for metric in &self.region_extents {
            metric.validate(total_nodes)?;
        }
        if (region_span(&self.region_extents, PlanarianAxisRegion::Head)?
            - self.head_region_extent_normalized)
            .abs()
            > 1.0e-5
            || (region_span(&self.region_extents, PlanarianAxisRegion::PharyngealTrunk)?
                - self.pharyngeal_region_extent_normalized)
                .abs()
                > 1.0e-5
        {
            return Err(MatterFieldError::InvalidField(
                "planarian normalized morphology summary fields must match region metrics",
            ));
        }
        Ok(())
    }
}

/// Returns the default educational comparison scenario order.
#[must_use]
pub const fn planarian_comparison_scenario_kinds() -> [PlanarianBioelectricScenarioKind; 5] {
    [
        PlanarianBioelectricScenarioKind::Baseline,
        PlanarianBioelectricScenarioKind::TransverseCutWound,
        PlanarianBioelectricScenarioKind::GapBlock,
        PlanarianBioelectricScenarioKind::TransientDepolarizationMemory,
        PlanarianBioelectricScenarioKind::TransientDepolarizationNoMemoryControl,
    ]
}

fn outcome_sample_from_frame(
    frame: &BioelectricCircuitDebugFrame,
    posterior_nodes: &[usize],
    head_nodes: &[usize],
    tail_nodes: &[usize],
    cut_band_nodes: &[usize],
) -> Result<PlanarianBioelectricOutcomeSample, MatterFieldError> {
    let head = readout_values(frame, HEAD_IDENTITY_READOUT_ID)?;
    let tail = readout_values(frame, TAIL_IDENTITY_READOUT_ID)?;
    Ok(PlanarianBioelectricOutcomeSample {
        step_index: frame.step_index,
        time_seconds: frame.time_seconds,
        posterior_memory_average: frame
            .memory_values
            .as_ref()
            .map_or(0.0, |values| average_nodes(values, posterior_nodes)),
        posterior_head_identity_average: average_nodes(head, posterior_nodes),
        head_identity_at_head_average: average_nodes(head, head_nodes),
        tail_identity_at_tail_average: average_nodes(tail, tail_nodes),
        cut_band_voltage_average: average_nodes(&frame.voltage_values, cut_band_nodes),
    })
}

fn readout_values<'a>(
    frame: &'a BioelectricCircuitDebugFrame,
    layer_id: &str,
) -> Result<&'a [f32], MatterFieldError> {
    frame
        .readout_layers
        .iter()
        .find(|layer| layer.layer_id == layer_id)
        .map(|layer| layer.values.as_slice())
        .ok_or(MatterFieldError::InvalidField(
            "planarian outcome trace requires head and tail readout layers",
        ))
}

fn posterior_comparison_nodes(axis_map: &PlanarianAxisMap) -> Vec<usize> {
    let mut nodes = axis_map.nodes_in_region(PlanarianAxisRegion::Tail);
    nodes.extend(axis_map.nodes_in_region(PlanarianAxisRegion::PostpharyngealTrunk));
    nodes
}

fn average_cross_cut_base_conductance(
    edges: &[BioelectricConductanceEdge],
    axis_map: &PlanarianAxisMap,
    cut_z: f32,
) -> Result<f32, MatterFieldError> {
    let mut sum = 0.0;
    let mut count = 0_usize;
    for edge in edges {
        let from_z = axis_map.node_normalized_z(edge.from_node).ok_or(
            MatterFieldError::InvalidSubstrate(
                "planarian outcome trace edge source must have AP metadata",
            ),
        )?;
        let to_z =
            axis_map
                .node_normalized_z(edge.to_node)
                .ok_or(MatterFieldError::InvalidSubstrate(
                    "planarian outcome trace edge target must have AP metadata",
                ))?;
        if (from_z < cut_z && to_z >= cut_z) || (to_z < cut_z && from_z >= cut_z) {
            sum += edge.base_conductance;
            count += 1;
        }
    }
    if count == 0 {
        return Err(MatterFieldError::InvalidSubstrate(
            "planarian outcome trace requires cross-cut conductance edges",
        ));
    }
    Ok(sum / count as f32)
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

fn region_span(
    metrics: &[PlanarianRegionExtentMetric],
    region: PlanarianAxisRegion,
) -> Result<f32, MatterFieldError> {
    metrics
        .iter()
        .find(|metric| metric.region == region)
        .map(|metric| metric.sampled_ap_span_normalized)
        .ok_or(MatterFieldError::InvalidField(
            "planarian normalized morphology metric is missing a region",
        ))
}

fn validate_fraction(value: f32, label: &'static str) -> Result<(), MatterFieldError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(MatterFieldError::InvalidField(label))
    }
}
