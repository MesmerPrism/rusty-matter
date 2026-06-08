use rusty_matter_mesh::{MeshSurfaceSampleConfig, MeshSurfaceSamplePattern, TriangleMeshSurface};
use rusty_matter_model::Vec3;

use crate::{
    BioelectricCircuitConfig, BioelectricCircuitDebugSequence, BioelectricCircuitRuntime,
    BioelectricCircuitState, BioelectricConductanceEdge, BioelectricCurrentKind,
    BioelectricCurrentTerm, BioelectricGate, BioelectricGateSource, BioelectricMemoryState,
    BioelectricReadoutLayer, BioelectricVoltageField, BioelectricVoltageUnit, MatterFieldError,
    SurfaceFieldSubstrate, PLANARIAN_AXIS_MAP_SCHEMA_ID,
    PLANARIAN_BIOELECTRIC_SCENARIO_RUN_SCHEMA_ID,
};

/// Synthetic planarian anterior/posterior region bands.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanarianAxisRegion {
    /// Posterior tail band.
    Tail,
    /// Posterior trunk band behind the pharynx.
    PostpharyngealTrunk,
    /// Pharyngeal trunk band.
    PharyngealTrunk,
    /// Anterior trunk band in front of the pharynx.
    PrepharyngealTrunk,
    /// Anterior head band.
    Head,
}

impl PlanarianAxisRegion {
    /// Returns regions in posterior-to-anterior order.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Tail,
            Self::PostpharyngealTrunk,
            Self::PharyngealTrunk,
            Self::PrepharyngealTrunk,
            Self::Head,
        ]
    }

    /// Returns the region identifier used by the reviewed Planarian XR atlas.
    #[must_use]
    pub const fn region_id(self) -> &'static str {
        match self {
            Self::Tail => "region_tail",
            Self::PostpharyngealTrunk => "region_postpharyngeal_trunk",
            Self::PharyngealTrunk => "region_pharyngeal_trunk",
            Self::PrepharyngealTrunk => "region_prepharyngeal_trunk",
            Self::Head => "region_head",
        }
    }

    /// Returns a human-readable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Tail => "tail",
            Self::PostpharyngealTrunk => "postpharyngeal trunk",
            Self::PharyngealTrunk => "pharyngeal trunk",
            Self::PrepharyngealTrunk => "prepharyngeal trunk",
            Self::Head => "head",
        }
    }

    /// Returns an educational normalized voltage target for this region.
    #[must_use]
    pub const fn target_voltage(self) -> f32 {
        match self {
            Self::Tail => -0.28,
            Self::PostpharyngealTrunk => -0.14,
            Self::PharyngealTrunk => 0.0,
            Self::PrepharyngealTrunk => 0.14,
            Self::Head => 0.28,
        }
    }

    fn from_z(z: f32) -> Self {
        if z < -0.62 {
            Self::Tail
        } else if z < -0.18 {
            Self::PostpharyngealTrunk
        } else if z < 0.24 {
            Self::PharyngealTrunk
        } else if z < 0.66 {
            Self::PrepharyngealTrunk
        } else {
            Self::Head
        }
    }
}

/// One AP region band over the normalized planarian z axis.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianAxisRegionBand {
    /// Region enum.
    pub region: PlanarianAxisRegion,
    /// Stable atlas-style region identifier.
    pub region_id: String,
    /// Human-readable label.
    pub label: String,
    /// Minimum normalized AP z coordinate.
    pub z_min: f32,
    /// Maximum normalized AP z coordinate.
    pub z_max: f32,
}

impl PlanarianAxisRegionBand {
    fn new(region: PlanarianAxisRegion, z_min: f32, z_max: f32) -> Self {
        Self {
            region,
            region_id: region.region_id().to_owned(),
            label: region.label().to_owned(),
            z_min,
            z_max,
        }
    }

    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.region_id != self.region.region_id() || self.label.trim().is_empty() {
            return Err(MatterFieldError::InvalidField(
                "planarian axis band metadata must match region",
            ));
        }
        if !self.z_min.is_finite() || !self.z_max.is_finite() || self.z_min >= self.z_max {
            return Err(MatterFieldError::InvalidField(
                "planarian axis band range must be finite and increasing",
            ));
        }
        Ok(())
    }
}

/// One surface node's AP-region metadata.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianAxisNodeRegion {
    /// Surface node index.
    pub node_index: usize,
    /// Classified AP region.
    pub region: PlanarianAxisRegion,
    /// Stable atlas-style region identifier.
    pub region_id: String,
    /// Normalized AP coordinate in 0..=1, posterior to anterior.
    pub ap_coordinate: f32,
    /// Lateral coordinate normalized by local half-width.
    pub lateral_coordinate: f32,
}

impl PlanarianAxisNodeRegion {
    fn from_node(node_index: usize, position: Vec3) -> Self {
        let region = PlanarianAxisRegion::from_z(position.z);
        let half_width = planarian_half_width_at_z(position.z).max(1.0e-6);
        Self {
            node_index,
            region,
            region_id: region.region_id().to_owned(),
            ap_coordinate: ((position.z + 1.0) * 0.5).clamp(0.0, 1.0),
            lateral_coordinate: (position.x / half_width).clamp(-1.5, 1.5),
        }
    }

    fn validate(&self, expected_index: usize) -> Result<(), MatterFieldError> {
        if self.node_index != expected_index {
            return Err(MatterFieldError::InvalidSubstrate(
                "planarian axis node indices must match node order",
            ));
        }
        if self.region_id != self.region.region_id() {
            return Err(MatterFieldError::InvalidField(
                "planarian axis node region id must match region",
            ));
        }
        if !self.ap_coordinate.is_finite()
            || !(0.0..=1.0).contains(&self.ap_coordinate)
            || !self.lateral_coordinate.is_finite()
        {
            return Err(MatterFieldError::InvalidField(
                "planarian axis node coordinates must be finite",
            ));
        }
        Ok(())
    }
}

/// AP-region metadata over one synthetic planarian surface-field substrate.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianAxisMap {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable axis-map identifier.
    pub map_id: String,
    /// Source substrate identifier.
    pub substrate_id: String,
    /// Posterior-to-anterior region bands.
    pub bands: Vec<PlanarianAxisRegionBand>,
    /// One classified AP region per substrate node.
    pub node_regions: Vec<PlanarianAxisNodeRegion>,
}

impl PlanarianAxisMap {
    /// Builds AP-region metadata from a substrate.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the substrate or generated map is
    /// invalid.
    pub fn from_substrate(
        map_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
    ) -> Result<Self, MatterFieldError> {
        substrate.validate()?;
        let map = Self {
            schema_id: PLANARIAN_AXIS_MAP_SCHEMA_ID.to_owned(),
            map_id: map_id.into(),
            substrate_id: substrate.substrate_id.clone(),
            bands: planarian_axis_bands(),
            node_regions: substrate
                .nodes
                .iter()
                .map(|node| PlanarianAxisNodeRegion::from_node(node.node_index, node.position))
                .collect(),
        };
        map.validate(substrate)?;
        Ok(map)
    }

    /// Returns node indices in a region.
    #[must_use]
    pub fn nodes_in_region(&self, region: PlanarianAxisRegion) -> Vec<usize> {
        self.node_regions
            .iter()
            .filter_map(|node| (node.region == region).then_some(node.node_index))
            .collect()
    }

    /// Returns node indices whose AP coordinate is inside a z band.
    #[must_use]
    pub fn nodes_in_z_band(&self, center_z: f32, half_width: f32) -> Vec<usize> {
        if !center_z.is_finite() || !half_width.is_finite() || half_width <= 0.0 {
            return Vec::new();
        }
        self.node_regions
            .iter()
            .filter_map(|node| {
                let z = node.ap_coordinate * 2.0 - 1.0;
                ((z - center_z).abs() <= half_width).then_some(node.node_index)
            })
            .collect()
    }

    /// Validates the axis map against its substrate.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, IDs, bands, or node metadata
    /// are invalid.
    pub fn validate(&self, substrate: &SurfaceFieldSubstrate) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_AXIS_MAP_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_AXIS_MAP_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.map_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian axis map id must not be empty",
            ));
        }
        if self.substrate_id != substrate.substrate_id {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian axis map substrate id must match substrate",
            ));
        }
        if self.bands.len() != PlanarianAxisRegion::all().len() {
            return Err(MatterFieldError::InvalidField(
                "planarian axis map must include all AP bands",
            ));
        }
        for (band, expected_region) in self.bands.iter().zip(PlanarianAxisRegion::all()) {
            band.validate()?;
            if band.region != expected_region {
                return Err(MatterFieldError::InvalidField(
                    "planarian axis bands must be posterior-to-anterior",
                ));
            }
        }
        if self.node_regions.len() != substrate.node_count() {
            return Err(MatterFieldError::NodeCountMismatch {
                expected: substrate.node_count(),
                actual: self.node_regions.len(),
            });
        }
        let mut region_counts = [0_usize; 5];
        for (expected_index, node_region) in self.node_regions.iter().enumerate() {
            node_region.validate(expected_index)?;
            let expected_region =
                PlanarianAxisRegion::from_z(substrate.nodes[expected_index].position.z);
            if node_region.region != expected_region {
                return Err(MatterFieldError::InvalidField(
                    "planarian axis node region must match substrate AP coordinate",
                ));
            }
            region_counts[region_slot(node_region.region)] += 1;
        }
        if region_counts.contains(&0) {
            return Err(MatterFieldError::InvalidField(
                "planarian axis map must sample every AP region",
            ));
        }
        Ok(())
    }
}

/// Scenario type for synthetic planarian AP bioelectric runs.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanarianBioelectricScenarioKind {
    /// Baseline AP polarization with no perturbation.
    Baseline,
    /// Localized transverse wound depolarization around a cut band.
    TransverseCutWound,
    /// Gap-junction-like coupling reduction across a cut band.
    GapBlock,
    /// Transient posterior depolarization with hysteresis memory enabled.
    TransientDepolarizationMemory,
    /// Same transient perturbation with memory disabled as a control.
    TransientDepolarizationNoMemoryControl,
}

impl PlanarianBioelectricScenarioKind {
    /// Stable scenario identifier.
    #[must_use]
    pub const fn scenario_id(self) -> &'static str {
        match self {
            Self::Baseline => "bioelectric.planarian_ap.baseline.synthetic",
            Self::TransverseCutWound => "bioelectric.planarian_ap.transverse_cut_wound.synthetic",
            Self::GapBlock => "bioelectric.planarian_ap.gap_block.synthetic",
            Self::TransientDepolarizationMemory => {
                "bioelectric.planarian_ap.transient_depolarization_memory.synthetic"
            }
            Self::TransientDepolarizationNoMemoryControl => {
                "bioelectric.planarian_ap.transient_depolarization_no_memory_control.synthetic"
            }
        }
    }

    /// Expected qualitative behavior for educational validation.
    #[must_use]
    pub const fn expected_outcome(self) -> &'static str {
        match self {
            Self::Baseline => "stable anterior-positive/posterior-negative AP voltage gradient",
            Self::TransverseCutWound => "localized wound depolarization over a transverse cut band",
            Self::GapBlock => "reduced cross-band coupling across a transverse gap block",
            Self::TransientDepolarizationMemory => {
                "posterior transient depolarization can persist as head-identity readout memory"
            }
            Self::TransientDepolarizationNoMemoryControl => {
                "posterior transient depolarization relaxes without persistent readout memory"
            }
        }
    }
}

/// Configuration for synthetic planarian bioelectric presets.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianBioelectricPresetConfig {
    /// Number of surface sample nodes.
    pub sample_count: usize,
    /// Same-surface first-tier neighbor count.
    pub first_tier_neighbor_count: usize,
    /// Same-surface second-tier neighbor count.
    pub second_tier_neighbor_count: usize,
    /// Deterministic sampling seed.
    pub seed: u64,
    /// Axial mesh segments along the AP axis.
    pub axial_segments: usize,
    /// Lateral mesh segments across the body width.
    pub lateral_segments: usize,
    /// Synthetic transverse cut center in normalized AP z coordinates.
    pub cut_z: f32,
    /// Half-width around the cut used for perturbation targets.
    pub cut_half_width: f32,
    /// Fixed step duration in seconds.
    pub fixed_step_seconds: f32,
    /// Number of fixed steps in generated scenario sequences.
    pub step_count: u32,
    /// Step interval between emitted debug frames.
    pub frame_stride: u32,
}

impl Default for PlanarianBioelectricPresetConfig {
    fn default() -> Self {
        Self {
            sample_count: 96,
            first_tier_neighbor_count: 4,
            second_tier_neighbor_count: 4,
            seed: 104_729,
            axial_segments: 28,
            lateral_segments: 8,
            cut_z: 0.16,
            cut_half_width: 0.11,
            fixed_step_seconds: 1.0 / 30.0,
            step_count: 180,
            frame_stride: 10,
        }
    }
}

impl PlanarianBioelectricPresetConfig {
    /// Validates the preset config.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when counts, ranges, or timing values are
    /// invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.sample_count < 32 {
            return Err(MatterFieldError::InvalidSubstrate(
                "planarian preset sample_count must be at least 32",
            ));
        }
        if self.first_tier_neighbor_count == 0
            || self.first_tier_neighbor_count + self.second_tier_neighbor_count >= self.sample_count
        {
            return Err(MatterFieldError::InvalidSubstrate(
                "planarian preset neighbor counts must be non-zero and below sample count",
            ));
        }
        if self.axial_segments < 8 || self.lateral_segments < 3 {
            return Err(MatterFieldError::InvalidSubstrate(
                "planarian preset mesh segments are too low",
            ));
        }
        if !self.cut_z.is_finite() || !(-0.9..=0.9).contains(&self.cut_z) {
            return Err(MatterFieldError::InvalidField(
                "planarian preset cut_z must be finite in -0.9..=0.9",
            ));
        }
        if !self.cut_half_width.is_finite()
            || self.cut_half_width <= 0.0
            || self.cut_half_width > 0.5
        {
            return Err(MatterFieldError::InvalidField(
                "planarian preset cut_half_width must be finite in 0..=0.5",
            ));
        }
        if !self.fixed_step_seconds.is_finite() || self.fixed_step_seconds <= 0.0 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "planarian preset fixed_step_seconds must be finite and positive",
            ));
        }
        if self.step_count == 0 || self.step_count > 4096 {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "planarian preset step_count must be in 1..=4096",
            ));
        }
        if self.frame_stride == 0 || self.frame_stride > self.step_count {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian preset frame_stride must be in 1..=step_count",
            ));
        }
        Ok(())
    }
}

/// Synthetic planarian AP bioelectric scenario run.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianBioelectricScenarioRun {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable run identifier.
    pub run_id: String,
    /// Stable scenario identifier.
    pub scenario_id: String,
    /// Scenario kind.
    pub scenario_kind: PlanarianBioelectricScenarioKind,
    /// Evidence type. Current presets are educational abstractions, not data fits.
    pub evidence_type: String,
    /// Expected qualitative behavior.
    pub expected_outcome: String,
    /// Matter-owned synthetic body surface sampled by the field substrate.
    pub source_surface: TriangleMeshSurface,
    /// Source surface-field substrate.
    pub substrate: SurfaceFieldSubstrate,
    /// AP-region metadata over the substrate.
    pub axis_map: PlanarianAxisMap,
    /// Circuit runtime configuration.
    pub circuit_config: BioelectricCircuitConfig,
    /// Initial circuit state.
    pub initial_circuit: BioelectricCircuitState,
    /// Executed circuit debug sequence.
    pub sequence: BioelectricCircuitDebugSequence,
    /// Compact literature/design anchors used to shape the abstraction.
    pub literature_anchors: Vec<String>,
}

impl PlanarianBioelectricScenarioRun {
    /// Builds a synthetic planarian AP bioelectric scenario run.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when generated substrate, circuit, or
    /// sequence contracts are invalid.
    pub fn build(
        scenario_kind: PlanarianBioelectricScenarioKind,
        config: PlanarianBioelectricPresetConfig,
    ) -> Result<Self, MatterFieldError> {
        config.validate()?;
        let surface = synthetic_planarian_axis_surface(&config)?;
        let sample_config = MeshSurfaceSampleConfig {
            sample_config_id: "mesh.surface_sample.planarian_ap.synthetic".to_owned(),
            sample_set_id: "mesh.surface_samples.planarian_ap.synthetic".to_owned(),
            point_count: config.sample_count,
            first_tier_neighbor_count: config.first_tier_neighbor_count,
            second_tier_neighbor_count: config.second_tier_neighbor_count,
            seed: config.seed,
            pattern: MeshSurfaceSamplePattern::LowDiscrepancy,
            ..MeshSurfaceSampleConfig::default()
        };
        let samples = surface.sample_points(&sample_config).map_err(|_| {
            MatterFieldError::InvalidSubstrate("planarian sample set must validate")
        })?;
        let substrate = SurfaceFieldSubstrate::from_sample_set(
            "fields.substrate.planarian_ap.synthetic",
            &samples,
        )?;
        let axis_map =
            PlanarianAxisMap::from_substrate("fields.planarian_ap.axis_map.synthetic", &substrate)?;
        let circuit_config = BioelectricCircuitConfig {
            config_id: "fields.bioelectric_circuit.planarian_ap.synthetic".to_owned(),
            fixed_step_seconds: config.fixed_step_seconds,
            max_steps_per_run: config.step_count,
            voltage_clamp_min: -1.0,
            voltage_clamp_max: 1.0,
            conductance_clamp_min: 0.0,
            conductance_clamp_max: 4.0,
            current_clamp_absolute: 8.0,
            ..BioelectricCircuitConfig::default()
        };
        let runtime = BioelectricCircuitRuntime::new(circuit_config.clone())?;
        let initial_circuit =
            build_planarian_circuit(scenario_kind, &config, &substrate, &axis_map)?;
        let sequence = BioelectricCircuitDebugSequence::run_fixed(
            format!("{}.sequence", scenario_kind.scenario_id()),
            &substrate,
            &runtime,
            &initial_circuit,
            config.step_count,
            config.frame_stride,
        )?;
        let run = Self {
            schema_id: PLANARIAN_BIOELECTRIC_SCENARIO_RUN_SCHEMA_ID.to_owned(),
            run_id: format!("{}.run", scenario_kind.scenario_id()),
            scenario_id: scenario_kind.scenario_id().to_owned(),
            scenario_kind,
            evidence_type: "synthetic_educational_abstraction".to_owned(),
            expected_outcome: scenario_kind.expected_outcome().to_owned(),
            source_surface: surface,
            substrate,
            axis_map,
            circuit_config,
            initial_circuit,
            sequence,
            literature_anchors: literature_anchors(scenario_kind),
        };
        run.validate()?;
        Ok(run)
    }

    /// Validates the scenario run.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, metadata, substrate, circuit,
    /// or sequence contracts are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_BIOELECTRIC_SCENARIO_RUN_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_BIOELECTRIC_SCENARIO_RUN_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.run_id.trim().is_empty()
            || self.scenario_id != self.scenario_kind.scenario_id()
            || self.evidence_type.trim().is_empty()
            || self.expected_outcome.trim().is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian scenario metadata must be populated and consistent",
            ));
        }
        self.source_surface.validate().map_err(|_| {
            MatterFieldError::InvalidSubstrate("planarian source surface must validate")
        })?;
        self.substrate.validate()?;
        if self.source_surface.surface_id != self.substrate.surface_id {
            return Err(MatterFieldError::InvalidSubstrate(
                "planarian source surface id must match substrate surface id",
            ));
        }
        if self.source_surface.topology_key() != self.substrate.topology_key {
            return Err(MatterFieldError::InvalidSubstrate(
                "planarian source surface topology must match substrate topology key",
            ));
        }
        self.axis_map.validate(&self.substrate)?;
        self.circuit_config.validate()?;
        self.initial_circuit.validate()?;
        if self.initial_circuit.substrate_id != self.substrate.substrate_id {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian initial circuit substrate id must match substrate",
            ));
        }
        self.sequence.validate()?;
        if self.sequence.substrate_id != self.substrate.substrate_id
            || self.sequence.initial_circuit_id != self.initial_circuit.circuit_id
            || (self.sequence.fixed_step_seconds - self.circuit_config.fixed_step_seconds).abs()
                > f32::EPSILON
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian sequence metadata must match circuit and substrate",
            ));
        }
        if self.literature_anchors.is_empty()
            || self
                .literature_anchors
                .iter()
                .any(|anchor| anchor.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian scenario must include compact literature anchors",
            ));
        }
        Ok(())
    }
}

/// Builds the synthetic tapered planarian AP surface used by the presets.
///
/// # Errors
///
/// Returns [`MatterFieldError`] when generated geometry is invalid.
pub fn synthetic_planarian_axis_surface(
    config: &PlanarianBioelectricPresetConfig,
) -> Result<TriangleMeshSurface, MatterFieldError> {
    config.validate()?;
    let axial_count = config.axial_segments + 1;
    let lateral_count = config.lateral_segments + 1;
    let mut positions = Vec::with_capacity(axial_count * lateral_count);
    for axial_index in 0..axial_count {
        let axial_fraction = axial_index as f32 / config.axial_segments as f32;
        let z = -1.0 + 2.0 * axial_fraction;
        let half_width = planarian_half_width_at_z(z);
        for lateral_index in 0..lateral_count {
            let lateral_fraction = lateral_index as f32 / config.lateral_segments as f32;
            let lateral = -1.0 + 2.0 * lateral_fraction;
            let x = lateral * half_width;
            let y = 0.018 * (1.0 - lateral * lateral) * (1.0 - 0.15 * z.abs());
            positions.push(Vec3::new(x, y, z));
        }
    }

    let mut triangles = Vec::with_capacity(config.axial_segments * config.lateral_segments * 2);
    for axial_index in 0..config.axial_segments {
        for lateral_index in 0..config.lateral_segments {
            let a = mesh_grid_index(axial_index, lateral_index, lateral_count)?;
            let b = mesh_grid_index(axial_index + 1, lateral_index, lateral_count)?;
            let c = mesh_grid_index(axial_index, lateral_index + 1, lateral_count)?;
            let d = mesh_grid_index(axial_index + 1, lateral_index + 1, lateral_count)?;
            triangles.push([a, b, c]);
            triangles.push([c, b, d]);
        }
    }
    let surface =
        TriangleMeshSurface::new("mesh.planarian_ap.synthetic_surface", positions, triangles);
    surface.validate().map_err(|_| {
        MatterFieldError::InvalidSubstrate("synthetic planarian surface must validate")
    })?;
    Ok(surface)
}

fn build_planarian_circuit(
    scenario_kind: PlanarianBioelectricScenarioKind,
    config: &PlanarianBioelectricPresetConfig,
    substrate: &SurfaceFieldSubstrate,
    axis_map: &PlanarianAxisMap,
) -> Result<BioelectricCircuitState, MatterFieldError> {
    let node_count = substrate.node_count();
    let voltage_values = axis_map
        .node_regions
        .iter()
        .map(|node| node.region.target_voltage())
        .collect::<Vec<_>>();
    let gate = BioelectricGate::new(
        "gate.planarian_ap.voltage_difference",
        BioelectricGateSource::VoltageDifference,
        0.12,
        0.035,
        0.35,
        1.45,
    );
    let mut conductance_edges =
        BioelectricConductanceEdge::from_substrate_neighbors(substrate, 0.12, 0.03, Some(gate))?;
    if scenario_kind == PlanarianBioelectricScenarioKind::GapBlock {
        for edge in &mut conductance_edges {
            if edge_crosses_cut(edge, substrate, config.cut_z) {
                edge.base_conductance *= 0.08;
                edge.conductance = edge.base_conductance;
            }
        }
    }

    let mut current_terms = vec![BioelectricCurrentTerm::new(
        "current.planarian_ap.leak",
        Vec::new(),
        BioelectricCurrentKind::Leak {
            conductance: 0.05,
            reversal_voltage: 0.0,
        },
    )];
    for region in PlanarianAxisRegion::all() {
        current_terms.push(BioelectricCurrentTerm::new(
            format!("current.planarian_ap.pump.{}", region.region_id()),
            axis_map.nodes_in_region(region),
            BioelectricCurrentKind::Pump {
                rate: 1.15,
                target_voltage: region.target_voltage(),
            },
        ));
    }

    match scenario_kind {
        PlanarianBioelectricScenarioKind::Baseline | PlanarianBioelectricScenarioKind::GapBlock => {
        }
        PlanarianBioelectricScenarioKind::TransverseCutWound => {
            let mut wound = BioelectricCurrentTerm::new(
                "current.planarian_ap.transverse_wound_depolarization",
                axis_map.nodes_in_z_band(config.cut_z, config.cut_half_width),
                BioelectricCurrentKind::Constant { current: 1.65 },
            );
            wound.duration_steps = 36;
            current_terms.push(wound);
        }
        PlanarianBioelectricScenarioKind::TransientDepolarizationMemory
        | PlanarianBioelectricScenarioKind::TransientDepolarizationNoMemoryControl => {
            let mut transient = BioelectricCurrentTerm::new(
                "current.planarian_ap.posterior_transient_depolarization",
                posterior_reprogramming_nodes(axis_map),
                BioelectricCurrentKind::Constant { current: 2.75 },
            );
            transient.start_step = 2;
            transient.duration_steps = 30;
            current_terms.push(transient);
        }
    }

    let memory = (scenario_kind
        != PlanarianBioelectricScenarioKind::TransientDepolarizationNoMemoryControl)
        .then(|| {
            BioelectricMemoryState::zeroed(
                "memory.planarian_ap.pattern_hysteresis",
                node_count,
                0.25,
                -0.95,
                4.0,
                0.04,
            )
        });
    let head_readout = BioelectricReadoutLayer::new(
        "readout.planarian_ap.head_identity",
        voltage_values
            .iter()
            .copied()
            .map(initial_head_readout)
            .collect(),
        1.50,
        0.70,
        0.50,
        2.0,
        0.0,
        1.0,
    );
    let tail_readout = BioelectricReadoutLayer::new(
        "readout.planarian_ap.tail_identity",
        voltage_values
            .iter()
            .copied()
            .map(initial_tail_readout)
            .collect(),
        -1.50,
        -0.50,
        0.50,
        2.0,
        0.0,
        1.0,
    );

    BioelectricCircuitState::new(
        format!("circuit.{}", scenario_kind.scenario_id()),
        substrate,
        BioelectricVoltageField::new(
            "field.planarian_ap.bioelectric_voltage",
            BioelectricVoltageUnit::Normalized,
            0.0,
            voltage_values,
        ),
        conductance_edges,
        current_terms,
        memory,
        vec![head_readout, tail_readout],
    )
}

fn planarian_axis_bands() -> Vec<PlanarianAxisRegionBand> {
    vec![
        PlanarianAxisRegionBand::new(PlanarianAxisRegion::Tail, -1.0, -0.62),
        PlanarianAxisRegionBand::new(PlanarianAxisRegion::PostpharyngealTrunk, -0.62, -0.18),
        PlanarianAxisRegionBand::new(PlanarianAxisRegion::PharyngealTrunk, -0.18, 0.24),
        PlanarianAxisRegionBand::new(PlanarianAxisRegion::PrepharyngealTrunk, 0.24, 0.66),
        PlanarianAxisRegionBand::new(PlanarianAxisRegion::Head, 0.66, 1.0),
    ]
}

fn planarian_half_width_at_z(z: f32) -> f32 {
    const WIDTHS: [(f32, f32); 6] = [
        (-1.00, 0.045),
        (-0.62, 0.110),
        (-0.18, 0.150),
        (0.24, 0.145),
        (0.66, 0.105),
        (1.00, 0.060),
    ];
    let z = z.clamp(-1.0, 1.0);
    for pair in WIDTHS.windows(2) {
        let [(z0, width0), (z1, width1)] = [pair[0], pair[1]];
        if z >= z0 && z <= z1 {
            let t = (z - z0) / (z1 - z0);
            return width0 + (width1 - width0) * t;
        }
    }
    WIDTHS[WIDTHS.len() - 1].1
}

fn mesh_grid_index(
    axial_index: usize,
    lateral_index: usize,
    lateral_count: usize,
) -> Result<u32, MatterFieldError> {
    let index = axial_index
        .checked_mul(lateral_count)
        .and_then(|base| base.checked_add(lateral_index))
        .ok_or(MatterFieldError::InvalidSubstrate(
            "planarian mesh index overflow",
        ))?;
    u32::try_from(index)
        .map_err(|_| MatterFieldError::InvalidSubstrate("planarian mesh index exceeds u32 range"))
}

fn edge_crosses_cut(
    edge: &BioelectricConductanceEdge,
    substrate: &SurfaceFieldSubstrate,
    cut_z: f32,
) -> bool {
    let from_z = substrate.nodes[edge.from_node].position.z;
    let to_z = substrate.nodes[edge.to_node].position.z;
    (from_z < cut_z && to_z >= cut_z) || (to_z < cut_z && from_z >= cut_z)
}

fn posterior_reprogramming_nodes(axis_map: &PlanarianAxisMap) -> Vec<usize> {
    let mut nodes = axis_map.nodes_in_region(PlanarianAxisRegion::Tail);
    nodes.extend(axis_map.nodes_in_region(PlanarianAxisRegion::PostpharyngealTrunk));
    nodes
}

fn initial_head_readout(voltage: f32) -> f32 {
    clamp01(0.50 + 1.50 * voltage)
}

fn initial_tail_readout(voltage: f32) -> f32 {
    clamp01(0.50 - 1.50 * voltage)
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn region_slot(region: PlanarianAxisRegion) -> usize {
    match region {
        PlanarianAxisRegion::Tail => 0,
        PlanarianAxisRegion::PostpharyngealTrunk => 1,
        PlanarianAxisRegion::PharyngealTrunk => 2,
        PlanarianAxisRegion::PrepharyngealTrunk => 3,
        PlanarianAxisRegion::Head => 4,
    }
}

fn literature_anchors(scenario_kind: PlanarianBioelectricScenarioKind) -> Vec<String> {
    let mut anchors = vec![
        "planarian_ap_axis_regeneration".to_owned(),
        "gap_junction_bioelectric_patterning".to_owned(),
        "voltage_driven_downstream_pattern_readout".to_owned(),
    ];
    if matches!(
        scenario_kind,
        PlanarianBioelectricScenarioKind::TransientDepolarizationMemory
            | PlanarianBioelectricScenarioKind::TransientDepolarizationNoMemoryControl
    ) {
        anchors.push("transient_depolarization_and_pattern_memory_control".to_owned());
    }
    anchors
}
