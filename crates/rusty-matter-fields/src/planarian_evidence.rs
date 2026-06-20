use crate::{
    MatterFieldError, PLANARIAN_SOURCE_DYNAMICS_TARGETS_SCHEMA_ID,
    PLANARIAN_SPECIES_LIKE_HEAD_TAXONOMY_SCHEMA_ID, PLANARIAN_XR_DISPLAY_BRIDGE_FIXTURE_SCHEMA_ID,
    PLANARIAN_XR_DISPLAY_SUBSTRATE_REQUEST_SCHEMA_ID, PLANFORMDB_DERIVED_FIXTURE_SCHEMA_ID,
};

const PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE: &str = "derived_planformdb_record";
const PLANARIAN_XR_DISPLAY_BRIDGE_EVIDENCE_TYPE: &str = "planarian_xr_public_display_bridge";
const PLANARIAN_XR_DISPLAY_SUBSTRATE_REQUEST_EVIDENCE_TYPE: &str =
    "planarian_xr_display_substrate_request";
const SOURCE_REVIEWED_DYNAMICS_EVIDENCE_TYPE: &str = "source_reviewed_dynamics_target";
const SPECIES_LIKE_HEAD_TAXONOMY_EVIDENCE_TYPE: &str = "rights_safe_teaching_taxonomy";
const SPECIES_LIKE_HEAD_SOURCE_TARGET_ANCHOR: &str =
    "source:emmons_bell_2015_ijms::target:species_like_head_labels::future_outcome_taxonomy";
const PLANFORMDB_NOTICE_TEXT: &str = "Planform / PlanformDB Notice\n\nSource: Lobo Lab PlanformDB 2.5.0\nSource page: https://lobolab.umbc.edu/planform/download/\n\nThis Rusty Matter fixture is a small transformed subset of PlanformDB metadata. It does not redistribute the raw SQLite database, paper figures, or morphology images.\n\nPlanform and PlanformDB are provided as-is, without any express or implied warranty. The authors are not liable for damages arising from use of this software or database.\n\nPermission is granted to use and redistribute Planform and PlanformDB freely, subject to these restrictions:\n\n1. The origin of the software and database must not be misrepresented.\n2. Works using the software or database require acknowledgment and citation of the Planform publications.\n3. This notice may not be removed or altered from any distribution.\n\nCitation for the database/application:\n\nLobo D, Malone TJ, Levin M. Planform: an application and database of graph-encoded planarian regenerative experiments. Bioinformatics 29(8), 1098-1100, 2013. DOI: 10.1093/bioinformatics/btt088";

/// One source-reviewed checkpoint for a qualitative planarian dynamics target.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianSourceDynamicsCheckpoint {
    /// Stable checkpoint identifier.
    pub checkpoint_id: String,
    /// Human-readable source relation.
    pub source_relation: String,
    /// Time or assay anchor from source review, when available.
    pub timing_anchor: String,
    /// Qualitative observation or label carried into Matter planning.
    pub qualitative_observation: String,
    /// Explicit boundary for Matter behavior.
    pub matter_boundary: String,
}

impl PlanarianSourceDynamicsCheckpoint {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.checkpoint_id.trim().is_empty()
            || self.source_relation.trim().is_empty()
            || self.timing_anchor.trim().is_empty()
            || self.qualitative_observation.trim().is_empty()
            || self.matter_boundary.trim().is_empty()
            || !self.matter_boundary.contains("not calibrated")
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian source dynamics checkpoints must be populated and non-calibrated",
            ));
        }
        Ok(())
    }
}

/// One source-reviewed dynamics target and its allowed Matter links.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianSourceDynamicsTarget {
    /// Stable implementation target identifier.
    pub target_id: String,
    /// Source identifiers backing the target.
    pub source_ids: Vec<String>,
    /// Source-target anchor used by Bioelectricity planning.
    pub source_target_anchor: String,
    /// Current implementation status.
    pub source_target_status: String,
    /// Qualitative role for Matter dynamics or annotation.
    pub dynamics_role: String,
    /// Matter scenario identifiers that may reference this target.
    pub matter_scenario_ids: Vec<String>,
    /// PlanformDB-derived record IDs linked to this target, if any.
    pub planformdb_record_ids: Vec<String>,
    /// Allowed uses of this source target.
    pub allowed_uses: Vec<String>,
    /// Explicitly blocked uses.
    pub blocked_uses: Vec<String>,
    /// Source-reviewed checkpoints carried by this target.
    pub checkpoints: Vec<PlanarianSourceDynamicsCheckpoint>,
}

impl PlanarianSourceDynamicsTarget {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.target_id.trim().is_empty()
            || self.source_ids.is_empty()
            || self.source_target_anchor.trim().is_empty()
            || self.source_target_status.trim().is_empty()
            || self.dynamics_role.trim().is_empty()
            || self.allowed_uses.is_empty()
            || self.blocked_uses.is_empty()
            || self.checkpoints.is_empty()
            || self
                .source_ids
                .iter()
                .any(|source_id| source_id.trim().is_empty())
            || self
                .allowed_uses
                .iter()
                .any(|allowed_use| allowed_use.trim().is_empty())
            || self
                .blocked_uses
                .iter()
                .any(|blocked_use| blocked_use.trim().is_empty())
            || self
                .matter_scenario_ids
                .iter()
                .any(|scenario_id| scenario_id.trim().is_empty())
            || self
                .planformdb_record_ids
                .iter()
                .any(|record_id| record_id.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian source dynamics target metadata must be populated",
            ));
        }
        if !self
            .source_target_anchor
            .contains(&format!("target:{}", self.target_id))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian source dynamics anchor must reference the target ID",
            ));
        }
        if !self
            .blocked_uses
            .iter()
            .any(|blocked_use| blocked_use.contains("calibrated physiology"))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian source dynamics targets must block calibrated physiology claims",
            ));
        }
        if self
            .source_ids
            .iter()
            .any(|source_id| source_id == "planformdb_250")
            && self.planformdb_record_ids.is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB-backed dynamics targets must preserve derived record IDs",
            ));
        }
        if self
            .source_ids
            .iter()
            .any(|source_id| source_id == "planformdb_250")
            && self
                .planformdb_record_ids
                .iter()
                .any(|record_id| !record_id.starts_with("planformdb:experiment:"))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB-backed dynamics targets must use PlanformDB record IDs",
            ));
        }
        let mut seen_checkpoint_ids = Vec::<&str>::with_capacity(self.checkpoints.len());
        for checkpoint in &self.checkpoints {
            checkpoint.validate()?;
            if seen_checkpoint_ids.contains(&checkpoint.checkpoint_id.as_str()) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian source dynamics target must not repeat checkpoint IDs",
                ));
            }
            seen_checkpoint_ids.push(checkpoint.checkpoint_id.as_str());
        }
        Ok(())
    }
}

/// Matter-owned fixture for source-reviewed planarian dynamics targets.
///
/// This fixture is annotation and validation data. It does not change the
/// synthetic educational voltage, conductance, memory, or readout stepping
/// behavior.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianSourceDynamicsTargetFixture {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable fixture identifier.
    pub fixture_id: String,
    /// Fixture schema version.
    pub schema_version: u32,
    /// Evidence type; must be `source_reviewed_dynamics_target`.
    pub evidence_type: String,
    /// Human-readable scope.
    pub scope: String,
    /// Overall non-calibration policy.
    pub source_policy: String,
    /// Source-reviewed target rows.
    pub targets: Vec<PlanarianSourceDynamicsTarget>,
}

impl PlanarianSourceDynamicsTargetFixture {
    /// Validates the source dynamics target fixture.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, policy, target identity, or
    /// non-calibration boundaries are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_SOURCE_DYNAMICS_TARGETS_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_SOURCE_DYNAMICS_TARGETS_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.fixture_id.trim().is_empty()
            || self.schema_version == 0
            || self.evidence_type != SOURCE_REVIEWED_DYNAMICS_EVIDENCE_TYPE
            || self.scope.trim().is_empty()
            || !self.source_policy.contains("not calibrated")
            || self.targets.is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian source dynamics fixture metadata must be populated and non-calibrated",
            ));
        }
        let mut seen_target_ids = Vec::<&str>::with_capacity(self.targets.len());
        for target in &self.targets {
            target.validate()?;
            if seen_target_ids.contains(&target.target_id.as_str()) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian source dynamics fixture must not repeat target IDs",
                ));
            }
            seen_target_ids.push(target.target_id.as_str());
        }
        for required in [
            "ap_transient_memory",
            "gap_block_conductance",
            "head_vs_tail_voltage",
        ] {
            if !seen_target_ids.contains(&required) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian source dynamics fixture is missing a required high-confidence target",
                ));
            }
        }
        Ok(())
    }
}

/// One public artifact referenced by a Planarian XR display-bridge fixture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianXrBridgePublicInput {
    /// Artifact role in the public bridge.
    pub kind: String,
    /// Public path from the Planarian XR static asset root.
    pub path: String,
    /// SHA-256 of the public artifact.
    pub sha256: String,
    /// Public artifact byte length.
    pub bytes: u64,
}

impl PlanarianXrBridgePublicInput {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if !matches!(
            self.kind.as_str(),
            "bridge_manifest"
                | "geometry_glb"
                | "source_map_sidecar"
                | "replay_manifest"
                | "preview_gif"
        ) {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR bridge public input kind is not supported",
            ));
        }
        if !is_sha256_hex(&self.sha256) || self.bytes == 0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR bridge public inputs must carry SHA-256 hashes and byte counts",
            ));
        }
        validate_planarian_xr_public_path(&self.path)?;
        Ok(())
    }
}

/// Allowed and blocked Matter capabilities for a public Planarian XR bridge.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianXrBridgeCapabilityPolicy {
    /// Capabilities that Matter may use from the public bridge.
    pub allowed_capabilities: Vec<String>,
    /// Capabilities that Matter must continue to reject for this bridge.
    pub blocked_capabilities: Vec<String>,
}

impl PlanarianXrBridgeCapabilityPolicy {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.allowed_capabilities.is_empty()
            || self.blocked_capabilities.is_empty()
            || self
                .allowed_capabilities
                .iter()
                .any(|capability| capability.trim().is_empty())
            || self
                .blocked_capabilities
                .iter()
                .any(|capability| capability.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR bridge capability policy must be populated",
            ));
        }
        for required in [
            "source_map_inspection",
            "read_only_element_inspection",
            "display_substrate",
            "model_inspired_replay_listing",
        ] {
            if !self
                .allowed_capabilities
                .iter()
                .any(|item| item == required)
            {
                return Err(MatterFieldError::InvalidRunSummary(
                    "Planarian XR bridge is missing a required allowed capability",
                ));
            }
        }
        for required in [
            "observed_dynamics_binding",
            "measured_bioelectric_claims",
            "predictive_regeneration_claims",
            "live_matter_stepping",
            "edit_acceptance",
            "nearest_object_annotation",
        ] {
            if !self
                .blocked_capabilities
                .iter()
                .any(|item| item == required)
            {
                return Err(MatterFieldError::InvalidRunSummary(
                    "Planarian XR bridge is missing a required blocked capability",
                ));
            }
            if self
                .allowed_capabilities
                .iter()
                .any(|item| item == required)
            {
                return Err(MatterFieldError::InvalidRunSummary(
                    "Planarian XR bridge cannot allow a blocked capability",
                ));
            }
        }
        Ok(())
    }
}

/// Matter-side import fixture for a public Planarian XR display bridge.
///
/// This fixture is a static data contract for a display substrate. It is not a
/// live Matter scenario, observed voltage trace, edit surface, or predictive
/// regeneration model.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianXrDisplayBridgeFixture {
    /// Matter schema identifier.
    pub schema_id: String,
    /// Stable Matter fixture identifier.
    pub fixture_id: String,
    /// Fixture schema version.
    pub schema_version: u32,
    /// Evidence type; must be `planarian_xr_public_display_bridge`.
    pub evidence_type: String,
    /// Source Planarian XR bridge schema identifier.
    pub source_bridge_schema: String,
    /// Source Planarian XR bridge manifest identifier.
    pub source_bridge_id: String,
    /// Public Planarian XR bridge manifest path.
    pub source_bridge_manifest_path: String,
    /// Public Planarian XR bridge manifest SHA-256.
    pub source_bridge_manifest_sha256: String,
    /// Planarian XR geometry asset identifier.
    pub atlas_geometry_asset_id: String,
    /// Planarian XR source layer identifier.
    pub source_layer_id: String,
    /// Public source DOI.
    pub source_doi: String,
    /// Public source object name.
    pub source_object_name: String,
    /// Public source object type.
    pub source_object_type: String,
    /// Public source-map sidecar path.
    pub source_map_path: String,
    /// Public source-map sidecar SHA-256.
    pub source_map_sha256: String,
    /// Public input geometry path.
    pub input_geometry_path: String,
    /// Public input geometry SHA-256.
    pub input_geometry_sha256: String,
    /// Matter role for this bridge; must be `display_substrate`.
    pub matter_substrate_role: String,
    /// Dynamics authority named by the source bridge.
    pub matter_authority: String,
    /// Visual/export authority named by the source bridge.
    pub optics_authority: String,
    /// Planarian XR role named by the source bridge.
    pub planarian_xr_role: String,
    /// Planarian XR replay identifier carried only as model-inspired display metadata.
    pub simulation_run_id: String,
    /// Number of source elements listed in the public sidecar.
    pub source_element_count: u32,
    /// Number of mapped public display elements.
    pub mapped_element_count: u32,
    /// Public replay graph node count.
    pub replay_node_count: u32,
    /// Public replay graph edge count.
    pub replay_edge_count: u32,
    /// Public replay frame count.
    pub replay_frame_count: u32,
    /// Public artifacts referenced by this bridge fixture.
    pub public_inputs: Vec<PlanarianXrBridgePublicInput>,
    /// Allowed and blocked bridge capabilities.
    pub capability_policy: PlanarianXrBridgeCapabilityPolicy,
    /// Matter-side use policy.
    pub matter_use_policy: Vec<String>,
    /// Explicit caveats preserved from the public bridge import.
    pub caveats: Vec<String>,
}

impl PlanarianXrDisplayBridgeFixture {
    /// Validates the Matter-side public Planarian XR display bridge fixture.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, public paths, hashes, counts,
    /// authority fields, or claim boundaries are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_XR_DISPLAY_BRIDGE_FIXTURE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_XR_DISPLAY_BRIDGE_FIXTURE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.fixture_id.trim().is_empty()
            || self.schema_version == 0
            || self.evidence_type != PLANARIAN_XR_DISPLAY_BRIDGE_EVIDENCE_TYPE
            || self.source_bridge_schema != "planarian-xr.rusty-dynamics-bridge.v1"
            || self.source_bridge_id.trim().is_empty()
            || self.atlas_geometry_asset_id.trim().is_empty()
            || self.source_layer_id.trim().is_empty()
            || self.source_doi.trim().is_empty()
            || self.source_object_name.trim().is_empty()
            || self.source_object_type.trim().is_empty()
            || self.simulation_run_id.trim().is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR bridge fixture metadata must be populated",
            ));
        }
        if self.matter_substrate_role != "display_substrate"
            || self.matter_authority != "rusty-matter"
            || self.optics_authority != "rusty-optics"
            || self.planarian_xr_role != "atlas_manifest_provider"
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR bridge fixture must preserve authority boundaries",
            ));
        }
        if !is_sha256_hex(&self.source_bridge_manifest_sha256)
            || !is_sha256_hex(&self.source_map_sha256)
            || !is_sha256_hex(&self.input_geometry_sha256)
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR bridge fixture hashes must be SHA-256 hex",
            ));
        }
        validate_planarian_xr_public_path(&self.source_bridge_manifest_path)?;
        validate_planarian_xr_public_path(&self.source_map_path)?;
        validate_planarian_xr_public_path(&self.input_geometry_path)?;
        if self.source_element_count == 0
            || self.mapped_element_count == 0
            || self.source_element_count != self.mapped_element_count
            || self.replay_node_count == 0
            || self.replay_edge_count == 0
            || self.replay_frame_count == 0
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR bridge fixture counts must be positive and mapped one-to-one",
            ));
        }
        if self.public_inputs.is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR bridge fixture must list public inputs",
            ));
        }
        let mut seen_public_input_kinds = Vec::<&str>::with_capacity(self.public_inputs.len());
        for public_input in &self.public_inputs {
            public_input.validate()?;
            if seen_public_input_kinds.contains(&public_input.kind.as_str()) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "Planarian XR bridge fixture must not repeat public input kinds",
                ));
            }
            seen_public_input_kinds.push(public_input.kind.as_str());
        }
        for required in [
            "bridge_manifest",
            "geometry_glb",
            "source_map_sidecar",
            "replay_manifest",
            "preview_gif",
        ] {
            if !seen_public_input_kinds.contains(&required) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "Planarian XR bridge fixture is missing a required public input",
                ));
            }
        }
        validate_public_input_link(
            &self.public_inputs,
            "bridge_manifest",
            &self.source_bridge_manifest_path,
            &self.source_bridge_manifest_sha256,
        )?;
        validate_public_input_link(
            &self.public_inputs,
            "source_map_sidecar",
            &self.source_map_path,
            &self.source_map_sha256,
        )?;
        validate_public_input_link(
            &self.public_inputs,
            "geometry_glb",
            &self.input_geometry_path,
            &self.input_geometry_sha256,
        )?;
        self.capability_policy.validate()?;
        validate_bridge_policy_text(&self.matter_use_policy)?;
        validate_bridge_policy_text(&self.caveats)?;
        if !self
            .matter_use_policy
            .iter()
            .any(|policy| policy.contains("not runtime dynamics"))
            || !self
                .matter_use_policy
                .iter()
                .any(|policy| policy.contains("not measured bioelectric data"))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR bridge fixture must state the Matter non-dynamics boundary",
            ));
        }
        if !self
            .caveats
            .iter()
            .any(|caveat| caveat.contains("not predictive"))
            || !self
                .caveats
                .iter()
                .any(|caveat| caveat.contains("not measured"))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR bridge fixture caveats must block measured and predictive claims",
            ));
        }
        Ok(())
    }
}

/// Matter-owned policy for materializing a Planarian XR public source map as a
/// display substrate.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianXrDisplaySubstrateGraphPolicy {
    /// Materialization state for this fixture.
    pub materialization_status: String,
    /// Node construction policy.
    pub node_policy: String,
    /// Coordinate interpretation policy.
    pub coordinate_policy: String,
    /// Edge construction policy.
    pub edge_policy: String,
    /// Requested nearest-neighbor count for later graph materialization.
    pub nearest_neighbors_per_node: u32,
    /// Edge weight semantics.
    pub edge_weight_policy: String,
}

impl PlanarianXrDisplaySubstrateGraphPolicy {
    fn validate(&self) -> Result<(), MatterFieldError> {
        validate_bridge_policy_text(&[
            self.materialization_status.clone(),
            self.node_policy.clone(),
            self.coordinate_policy.clone(),
            self.edge_policy.clone(),
            self.edge_weight_policy.clone(),
        ])?;
        if self.materialization_status != "request_only_not_materialized"
            || !self
                .node_policy
                .contains("one node per mapped public element")
            || !self
                .coordinate_policy
                .contains("source-map atlas positions")
            || !self.coordinate_policy.contains("not calibrated")
            || !self.edge_policy.contains("deterministic")
            || !self.edge_policy.contains("nearest")
            || self.nearest_neighbors_per_node == 0
            || self.nearest_neighbors_per_node > 12
            || !self.edge_weight_policy.contains("not conductance")
            || !self.edge_weight_policy.contains("not voltage")
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR display-substrate graph policy must remain request-only and non-dynamics",
            ));
        }
        Ok(())
    }
}

/// Matter-owned request fixture for turning a public Planarian XR bridge into
/// a deterministic display-substrate graph in a later slice.
///
/// This fixture consumes the bridge metadata and fixes the graph policy. It
/// still does not materialize nodes, voltage, conductance, or runtime stepping.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianXrDisplaySubstrateRequest {
    /// Matter schema identifier.
    pub schema_id: String,
    /// Stable request identifier.
    pub request_id: String,
    /// Request schema version.
    pub schema_version: u32,
    /// Evidence type; must be `planarian_xr_display_substrate_request`.
    pub evidence_type: String,
    /// Source bridge fixture consumed by this request.
    pub source_bridge_fixture_id: String,
    /// Source Planarian XR bridge manifest SHA-256.
    pub source_bridge_manifest_sha256: String,
    /// Public source-map sidecar path.
    pub source_map_path: String,
    /// Public source-map sidecar SHA-256.
    pub source_map_sha256: String,
    /// Public input geometry path.
    pub input_geometry_path: String,
    /// Public input geometry SHA-256.
    pub input_geometry_sha256: String,
    /// Source elements available in the public map.
    pub source_element_count: u32,
    /// Requested display-substrate graph node count.
    pub requested_node_count: u32,
    /// Graph materialization policy.
    pub graph_policy: PlanarianXrDisplaySubstrateGraphPolicy,
    /// Allowed and blocked bridge capabilities carried into the request.
    pub capability_policy: PlanarianXrBridgeCapabilityPolicy,
    /// Output families this request may produce later.
    pub allowed_outputs: Vec<String>,
    /// Output families this request must not produce.
    pub blocked_outputs: Vec<String>,
    /// Explicit caveats for the request.
    pub caveats: Vec<String>,
}

impl PlanarianXrDisplaySubstrateRequest {
    /// Builds a request from a validated public Planarian XR bridge fixture.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when either input or generated request
    /// violates the non-dynamics boundary.
    pub fn from_bridge(
        request_id: impl Into<String>,
        bridge: &PlanarianXrDisplayBridgeFixture,
        graph_policy: PlanarianXrDisplaySubstrateGraphPolicy,
    ) -> Result<Self, MatterFieldError> {
        bridge.validate()?;
        let request = Self {
            schema_id: PLANARIAN_XR_DISPLAY_SUBSTRATE_REQUEST_SCHEMA_ID.to_owned(),
            request_id: request_id.into(),
            schema_version: 1,
            evidence_type: PLANARIAN_XR_DISPLAY_SUBSTRATE_REQUEST_EVIDENCE_TYPE.to_owned(),
            source_bridge_fixture_id: bridge.fixture_id.clone(),
            source_bridge_manifest_sha256: bridge.source_bridge_manifest_sha256.clone(),
            source_map_path: bridge.source_map_path.clone(),
            source_map_sha256: bridge.source_map_sha256.clone(),
            input_geometry_path: bridge.input_geometry_path.clone(),
            input_geometry_sha256: bridge.input_geometry_sha256.clone(),
            source_element_count: bridge.source_element_count,
            requested_node_count: bridge.mapped_element_count,
            graph_policy,
            capability_policy: bridge.capability_policy.clone(),
            allowed_outputs: vec![
                "display_substrate_graph_fixture".to_owned(),
                "source_map_node_index".to_owned(),
            ],
            blocked_outputs: vec![
                "observed_dynamics_binding".to_owned(),
                "measured_bioelectric_state".to_owned(),
                "predictive_regeneration_output".to_owned(),
                "live_matter_stepping".to_owned(),
                "edit_acceptance".to_owned(),
            ],
            caveats: vec![
                "Request-only display-substrate graph policy; not runtime dynamics."
                    .to_owned(),
                "Public source-map positions are not measured bioelectric data.".to_owned(),
                "Nearest-neighbor display edges are not conductance, not voltage, and not predictive regeneration output."
                    .to_owned(),
            ],
        };
        request.validate()?;
        Ok(request)
    }

    /// Validates the display-substrate request.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, hashes, counts, graph policy,
    /// output policy, or caveats violate the request-only boundary.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_XR_DISPLAY_SUBSTRATE_REQUEST_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_XR_DISPLAY_SUBSTRATE_REQUEST_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.request_id.trim().is_empty()
            || self.schema_version == 0
            || self.evidence_type != PLANARIAN_XR_DISPLAY_SUBSTRATE_REQUEST_EVIDENCE_TYPE
            || self.source_bridge_fixture_id.trim().is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR display-substrate request metadata must be populated",
            ));
        }
        if !is_sha256_hex(&self.source_bridge_manifest_sha256)
            || !is_sha256_hex(&self.source_map_sha256)
            || !is_sha256_hex(&self.input_geometry_sha256)
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR display-substrate request hashes must be SHA-256 hex",
            ));
        }
        validate_planarian_xr_public_path(&self.source_map_path)?;
        validate_planarian_xr_public_path(&self.input_geometry_path)?;
        if self.source_element_count == 0
            || self.requested_node_count == 0
            || self.source_element_count != self.requested_node_count
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR display-substrate request must request one node per mapped element",
            ));
        }
        self.graph_policy.validate()?;
        self.capability_policy.validate()?;
        validate_bridge_policy_text(&self.allowed_outputs)?;
        validate_bridge_policy_text(&self.blocked_outputs)?;
        validate_bridge_policy_text(&self.caveats)?;
        if !self
            .allowed_outputs
            .iter()
            .any(|output| output == "display_substrate_graph_fixture")
            || !self
                .allowed_outputs
                .iter()
                .any(|output| output == "source_map_node_index")
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR display-substrate request must name allowed display outputs",
            ));
        }
        for blocked in [
            "observed_dynamics_binding",
            "measured_bioelectric_state",
            "predictive_regeneration_output",
            "live_matter_stepping",
            "edit_acceptance",
        ] {
            if !self.blocked_outputs.iter().any(|output| output == blocked)
                || self.allowed_outputs.iter().any(|output| output == blocked)
            {
                return Err(MatterFieldError::InvalidRunSummary(
                    "Planarian XR display-substrate request must keep blocked outputs blocked",
                ));
            }
        }
        if !self
            .caveats
            .iter()
            .any(|caveat| caveat.contains("not runtime dynamics"))
            || !self
                .caveats
                .iter()
                .any(|caveat| caveat.contains("not measured"))
            || !self
                .caveats
                .iter()
                .any(|caveat| caveat.contains("not predictive"))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "Planarian XR display-substrate request caveats must block dynamics and measurement claims",
            ));
        }
        Ok(())
    }
}

fn validate_planarian_xr_public_path(path: &str) -> Result<(), MatterFieldError> {
    if path.trim().is_empty()
        || path.contains('\\')
        || path.contains(':')
        || path.split('/').any(|part| part == "..")
        || contains_private_fragment(path)
        || ![
            "/data/bridges/",
            "/data/source-maps/",
            "/data/replays/",
            "/assets/geometry/",
            "/assets/bioelectricity/",
        ]
        .iter()
        .any(|root| path.starts_with(root))
    {
        return Err(MatterFieldError::InvalidRunSummary(
            "Planarian XR bridge paths must be public static asset paths",
        ));
    }
    Ok(())
}

fn validate_bridge_policy_text(items: &[String]) -> Result<(), MatterFieldError> {
    if items.is_empty() || items.iter().any(|item| item.trim().is_empty()) {
        return Err(MatterFieldError::InvalidRunSummary(
            "Planarian XR bridge policy text must be populated",
        ));
    }
    if items
        .iter()
        .any(|item| item.contains('\\') || item.contains(':') || contains_private_fragment(item))
    {
        return Err(MatterFieldError::InvalidRunSummary(
            "Planarian XR bridge policy text must not contain private artifact markers",
        ));
    }
    Ok(())
}

fn validate_public_input_link(
    public_inputs: &[PlanarianXrBridgePublicInput],
    kind: &str,
    expected_path: &str,
    expected_sha256: &str,
) -> Result<(), MatterFieldError> {
    let Some(public_input) = public_inputs.iter().find(|input| input.kind == kind) else {
        return Err(MatterFieldError::InvalidRunSummary(
            "Planarian XR bridge public input link is missing",
        ));
    };
    if public_input.path != expected_path || public_input.sha256 != expected_sha256 {
        return Err(MatterFieldError::InvalidRunSummary(
            "Planarian XR bridge public input link must match fixture hashes",
        ));
    }
    Ok(())
}

fn bridge_public_input(
    kind: &str,
    path: &str,
    sha256: &str,
    bytes: u64,
) -> PlanarianXrBridgePublicInput {
    PlanarianXrBridgePublicInput {
        kind: kind.to_owned(),
        path: path.to_owned(),
        sha256: sha256.to_owned(),
        bytes,
    }
}

fn contains_private_fragment(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "raw/",
        "/raw/",
        "artifacts/",
        "/artifacts/",
        ".togo",
        ".am.dat",
        ".am.lda",
        ".am",
        "tileminmax",
        "decoder",
        "key material",
        ".log",
        "review packet",
        "source request package",
    ]
    .iter()
    .any(|fragment| normalized.contains(fragment))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value.chars().all(|character| {
            character.is_ascii_digit()
                || ('a'..='f').contains(&character)
                || ('A'..='F').contains(&character)
        })
}

/// Source database metadata for a curated PlanformDB-derived fixture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbSourceDatabase {
    /// Public source identifier.
    pub id: String,
    /// Source database version.
    pub version: String,
    /// Public download page.
    pub source_url: String,
    /// SHA-256 of the raw source database used for derivation.
    pub raw_sha256: String,
    /// Source SQLite schema version observed during intake.
    pub sqlite_schema_version: u32,
}

impl PlanformDbSourceDatabase {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.id.trim().is_empty()
            || self.version.trim().is_empty()
            || self.source_url.trim().is_empty()
            || self.raw_sha256.trim().is_empty()
            || self.sqlite_schema_version == 0
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB source database metadata must be populated",
            ));
        }
        if self.raw_sha256.len() != 64
            || !self.raw_sha256.chars().all(|character| {
                character.is_ascii_digit()
                    || ('a'..='f').contains(&character)
                    || ('A'..='F').contains(&character)
            })
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB source database hash must be SHA-256 hex",
            ));
        }
        Ok(())
    }
}

/// Citation metadata carried with a curated PlanformDB fixture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbCitation {
    /// Stable citation identifier.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// DOI string.
    pub doi: String,
    /// Public URL.
    pub url: String,
}

impl PlanformDbCitation {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.id.trim().is_empty()
            || self.label.trim().is_empty()
            || self.doi.trim().is_empty()
            || self.url.trim().is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB citation metadata must be populated",
            ));
        }
        Ok(())
    }
}

/// Selection boundary for a curated PlanformDB-derived fixture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbSelectionPolicy {
    /// Selection date in ISO-8601 calendar form.
    pub selected_on: String,
    /// Short explanation for the selected curated subset.
    pub selection_basis: String,
    /// Explicit non-scope strings.
    pub non_scope: Vec<String>,
}

impl PlanformDbSelectionPolicy {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.selected_on.trim().is_empty()
            || self.selection_basis.trim().is_empty()
            || self.non_scope.is_empty()
            || self.non_scope.iter().any(|item| item.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB selection policy must be populated",
            ));
        }
        if !self
            .non_scope
            .iter()
            .any(|item| item == "Matter runtime dynamics")
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB fixture must explicitly exclude runtime dynamics",
            ));
        }
        Ok(())
    }
}

/// Preserved PlanformDB integer source IDs for one derived record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanformDbSourceIds {
    /// Source publication ID.
    pub publication_id: u32,
    /// Source species ID.
    pub species_id: u32,
    /// Source experiment ID.
    pub experiment_id: u32,
    /// Source manipulation ID.
    pub manipulation_id: u32,
    /// Source result-set ID.
    pub result_set_id: u32,
}

impl PlanformDbSourceIds {
    fn validate(self) -> Result<(), MatterFieldError> {
        if self.publication_id == 0
            || self.species_id == 0
            || self.experiment_id == 0
            || self.manipulation_id == 0
            || self.result_set_id == 0
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB source IDs must be positive",
            ));
        }
        Ok(())
    }
}

/// Normalized teaching labels for one PlanformDB-derived record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbNormalizedLabels {
    /// Source publication identifier used by the Bioelectricity target matrix.
    pub publication_source_id: String,
    /// Normalized species label.
    pub species: String,
    /// Normalized manipulation label.
    pub manipulation: String,
    /// Normalized perturbation labels.
    pub perturbations: Vec<String>,
    /// Teaching target linked to this record.
    pub teaching_target: String,
}

impl PlanformDbNormalizedLabels {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.publication_source_id.trim().is_empty()
            || self.species.trim().is_empty()
            || self.manipulation.trim().is_empty()
            || self.teaching_target.trim().is_empty()
            || self.perturbations.is_empty()
            || self
                .perturbations
                .iter()
                .any(|label| label.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB normalized labels must be populated",
            ));
        }
        Ok(())
    }
}

/// Assay context carried by one PlanformDB-derived record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanformDbAssayContext {
    /// Source sample count.
    pub sample_count: u32,
    /// Regeneration period in days.
    pub regeneration_period_days: f32,
}

impl PlanformDbAssayContext {
    fn validate(self) -> Result<(), MatterFieldError> {
        if self.sample_count == 0
            || !self.regeneration_period_days.is_finite()
            || self.regeneration_period_days < 0.0
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB assay context must have positive samples and non-negative duration",
            ));
        }
        Ok(())
    }
}

/// One normalized resultant morphology entry from a PlanformDB-derived record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbResultantMorphology {
    /// Source resultant morphology ID.
    pub resultant_morphology_id: u32,
    /// Source morphology ID.
    pub morphology_id: u32,
    /// Normalized outcome label.
    pub normalized_outcome: String,
    /// Source frequency normalized to 0..=1.
    pub frequency: f32,
}

impl PlanformDbResultantMorphology {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.resultant_morphology_id == 0
            || self.morphology_id == 0
            || self.normalized_outcome.trim().is_empty()
            || !self.frequency.is_finite()
            || !(0.0..=1.0).contains(&self.frequency)
        {
            return Err(MatterFieldError::InvalidField(
                "PlanformDB resultant morphology fields must be populated and normalized",
            ));
        }
        Ok(())
    }
}

/// One curated PlanformDB-derived record.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbDerivedRecord {
    /// Stable derived record ID.
    pub record_id: String,
    /// Evidence label; must be `derived_planformdb_record`.
    pub evidence_type: String,
    /// Citation/source IDs associated with the record.
    pub source_citation_ids: Vec<String>,
    /// Preserved source integer IDs.
    pub source_ids: PlanformDbSourceIds,
    /// Normalized teaching labels.
    pub normalized_labels: PlanformDbNormalizedLabels,
    /// Source assay context.
    pub assay_context: PlanformDbAssayContext,
    /// Normalized resultant morphology frequencies.
    pub resultant_morphologies: Vec<PlanformDbResultantMorphology>,
    /// Transformation notes and non-calibration boundary.
    pub transform_notes: Vec<String>,
}

impl PlanformDbDerivedRecord {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.record_id
            != format!(
                "planformdb:experiment:{}:resultset:{}",
                self.source_ids.experiment_id, self.source_ids.result_set_id
            )
            || self.evidence_type != PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE
            || self.source_citation_ids.is_empty()
            || self
                .source_citation_ids
                .iter()
                .any(|source_id| source_id.trim().is_empty())
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB derived record identity and evidence metadata must be populated",
            ));
        }
        if !self
            .source_citation_ids
            .iter()
            .any(|source_id| source_id == "planformdb_250")
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB derived records must retain the PlanformDB source citation",
            ));
        }
        self.source_ids.validate()?;
        self.normalized_labels.validate()?;
        self.assay_context.validate()?;
        if self.resultant_morphologies.is_empty() {
            return Err(MatterFieldError::InvalidField(
                "PlanformDB records require resultant morphology frequencies",
            ));
        }
        let mut frequency_sum = 0.0;
        for morphology in &self.resultant_morphologies {
            morphology.validate()?;
            frequency_sum += morphology.frequency;
        }
        if (frequency_sum - 1.0).abs() > 0.001 {
            return Err(MatterFieldError::InvalidField(
                "PlanformDB resultant morphology frequencies must sum to 1.0",
            ));
        }
        if self.transform_notes.is_empty()
            || self
                .transform_notes
                .iter()
                .any(|note| note.trim().is_empty())
            || !self
                .transform_notes
                .iter()
                .any(|note| note.contains("No calibrated bioelectric physiology"))
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB transform notes must include the non-calibration boundary",
            ));
        }
        Ok(())
    }
}

/// Curated Matter fixture containing small PlanformDB-derived review records.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanformDbDerivedFixture {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable fixture identifier.
    pub fixture_id: String,
    /// Fixture schema version.
    pub schema_version: u32,
    /// Root evidence type.
    pub evidence_type: String,
    /// Human-readable scope and boundary.
    pub scope: String,
    /// Source database metadata.
    pub source_database: PlanformDbSourceDatabase,
    /// Required notice text that must travel with the fixture.
    pub notice_text: String,
    /// Citations associated with the fixture.
    pub citations: Vec<PlanformDbCitation>,
    /// Selection and non-scope boundary.
    pub selection_policy: PlanformDbSelectionPolicy,
    /// Curated derived records.
    pub records: Vec<PlanformDbDerivedRecord>,
}

impl PlanformDbDerivedFixture {
    /// Validates the fixture contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when metadata, notice text, citations, or
    /// derived records are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANFORMDB_DERIVED_FIXTURE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANFORMDB_DERIVED_FIXTURE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.fixture_id.trim().is_empty()
            || self.schema_version == 0
            || self.evidence_type != PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE
            || self.scope.trim().is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB derived fixture metadata must be populated",
            ));
        }
        self.source_database.validate()?;
        self.selection_policy.validate()?;
        validate_planformdb_notice(&self.notice_text)?;
        if self.citations.len() < 2 || self.records.is_empty() {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB derived fixture requires citations and records",
            ));
        }
        for citation in &self.citations {
            citation.validate()?;
        }
        if !self
            .citations
            .iter()
            .any(|citation| citation.id == "lobo_2013_planform")
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB derived fixture must cite Planform",
            ));
        }
        let mut seen_record_ids = Vec::<&str>::with_capacity(self.records.len());
        for record in &self.records {
            record.validate()?;
            if seen_record_ids.contains(&record.record_id.as_str()) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "PlanformDB derived fixture must not repeat record IDs",
                ));
            }
            seen_record_ids.push(record.record_id.as_str());
        }
        Ok(())
    }
}

/// Rights-safe species-like head label entry.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianSpeciesLikeHeadLabel {
    /// Stable label identifier.
    pub label_id: String,
    /// Human-readable display label.
    pub display_label: String,
    /// `derived_source_label` or `synthetic_teaching_label`.
    pub label_kind: String,
    /// Source relation or teaching-boundary note.
    pub source_relation: String,
    /// Policy for visual assets associated with this label.
    pub visual_policy: String,
}

impl PlanarianSpeciesLikeHeadLabel {
    fn validate(&self) -> Result<(), MatterFieldError> {
        if self.label_id.trim().is_empty()
            || self.display_label.trim().is_empty()
            || self.source_relation.trim().is_empty()
            || self.visual_policy.trim().is_empty()
            || !matches!(
                self.label_kind.as_str(),
                "derived_source_label" | "synthetic_teaching_label"
            )
            || !self.visual_policy.contains("generated")
        {
            return Err(MatterFieldError::InvalidField(
                "planarian species-like head labels must be rights-safe generated labels",
            ));
        }
        Ok(())
    }
}

/// Rights-safe categorical taxonomy for species-like head-shape teaching labels.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarianSpeciesLikeHeadTaxonomy {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable taxonomy identifier.
    pub taxonomy_id: String,
    /// Evidence type for this taxonomy.
    pub evidence_type: String,
    /// Source-target anchor represented by this taxonomy.
    pub source_target_anchor: String,
    /// Source ID.
    pub source_id: String,
    /// Policy against paper-image reuse.
    pub image_policy: String,
    /// Label entries.
    pub labels: Vec<PlanarianSpeciesLikeHeadLabel>,
}

impl PlanarianSpeciesLikeHeadTaxonomy {
    /// Validates the taxonomy contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when schema, policy, source target, or
    /// labels are invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != PLANARIAN_SPECIES_LIKE_HEAD_TAXONOMY_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: PLANARIAN_SPECIES_LIKE_HEAD_TAXONOMY_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.taxonomy_id.trim().is_empty()
            || self.evidence_type != SPECIES_LIKE_HEAD_TAXONOMY_EVIDENCE_TYPE
            || self.source_target_anchor != SPECIES_LIKE_HEAD_SOURCE_TARGET_ANCHOR
            || self.source_id != "emmons_bell_2015_ijms"
            || !self.image_policy.contains("no paper figure reuse")
            || self.labels.is_empty()
        {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian species-like head taxonomy metadata must preserve source and image policy",
            ));
        }
        let mut seen_label_ids = Vec::<&str>::with_capacity(self.labels.len());
        for label in &self.labels {
            label.validate()?;
            if seen_label_ids.contains(&label.label_id.as_str()) {
                return Err(MatterFieldError::InvalidRunSummary(
                    "planarian species-like head taxonomy must not repeat labels",
                ));
            }
            seen_label_ids.push(label.label_id.as_str());
        }
        if !seen_label_ids.contains(&"unclassified_teaching_abstraction") {
            return Err(MatterFieldError::InvalidRunSummary(
                "planarian species-like head taxonomy requires an unclassified teaching label",
            ));
        }
        Ok(())
    }
}

/// Builds the default rights-safe species-like head label taxonomy.
///
/// # Errors
///
/// Returns [`MatterFieldError`] if the generated taxonomy fails validation.
pub fn default_planarian_species_like_head_taxonomy(
) -> Result<PlanarianSpeciesLikeHeadTaxonomy, MatterFieldError> {
    let taxonomy = PlanarianSpeciesLikeHeadTaxonomy {
        schema_id: PLANARIAN_SPECIES_LIKE_HEAD_TAXONOMY_SCHEMA_ID.to_owned(),
        taxonomy_id: "taxonomy.planarian.species_like_head_labels.v1".to_owned(),
        evidence_type: SPECIES_LIKE_HEAD_TAXONOMY_EVIDENCE_TYPE.to_owned(),
        source_target_anchor: SPECIES_LIKE_HEAD_SOURCE_TARGET_ANCHOR.to_owned(),
        source_id: "emmons_bell_2015_ijms".to_owned(),
        image_policy: "generated symbolic labels only; no paper figure reuse".to_owned(),
        labels: vec![
            head_label(
                "native_gd_like",
                "native G. dorotocephala-like",
                "derived_source_label",
                "control/native or pseudo-native outcome category",
            ),
            head_label(
                "pseudo_dj_like",
                "D. japonica-like",
                "derived_source_label",
                "species-like pseudo morphology category",
            ),
            head_label(
                "pseudo_pf_like",
                "P. felina-like",
                "derived_source_label",
                "species-like pseudo morphology category; source notes incomplete mimicry",
            ),
            head_label(
                "pseudo_sm_like",
                "S. mediterranea-like",
                "derived_source_label",
                "species-like pseudo morphology category",
            ),
            head_label(
                "failed_ap_regeneration",
                "failed AP regeneration",
                "derived_source_label",
                "scored failure class in source frequency panel",
            ),
            head_label(
                "unclassified_teaching_abstraction",
                "unclassified teaching abstraction",
                "synthetic_teaching_label",
                "synthetic or unresolved teaching category",
            ),
        ],
    };
    taxonomy.validate()?;
    Ok(taxonomy)
}

/// Builds the default source-reviewed planarian dynamics target fixture.
///
/// # Errors
///
/// Returns [`MatterFieldError`] if the generated target fixture fails
/// validation.
pub fn default_planarian_source_dynamics_targets(
) -> Result<PlanarianSourceDynamicsTargetFixture, MatterFieldError> {
    let fixture = PlanarianSourceDynamicsTargetFixture {
        schema_id: PLANARIAN_SOURCE_DYNAMICS_TARGETS_SCHEMA_ID.to_owned(),
        fixture_id: "fixture.fields.planarian_ap.source_dynamics_targets".to_owned(),
        schema_version: 1,
        evidence_type: SOURCE_REVIEWED_DYNAMICS_EVIDENCE_TYPE.to_owned(),
        scope: "Source-reviewed qualitative planarian dynamics targets for Matter annotation and validation; not runtime calibration.".to_owned(),
        source_policy: "Targets may label synthetic educational scenarios and future derived fixtures; source checkpoints are not calibrated physiology and do not alter Matter stepping.".to_owned(),
        targets: vec![
            source_dynamics_target(
                "ap_transient_memory",
                &["durant_2019_bpj"],
                "source:durant_2019_bpj::target:ap_transient_memory::synthetic_fixture_source_targets_scoped",
                "synthetic_fixture_source_targets_scoped",
                "Early transient depolarization and washout-memory teaching target.",
                &[
                    "bioelectric.planarian_ap.transient_depolarization_memory.synthetic",
                    "bioelectric.planarian_ap.transient_depolarization_no_memory_control.synthetic",
                ],
                &[],
                &[
                    "metadata checkpoint anchors",
                    "memory versus no-memory scenario labeling",
                    "future derived experiment fixture review",
                ],
                &[
                    "calibrated physiology",
                    "mapping source frequencies to stochastic Matter behavior",
                    "millivolt or ion-channel constants",
                ],
                vec![
                    dynamics_checkpoint(
                        "durant_2019_3hpa_window",
                        "Durant 2019 early post-amputation bioelectric window",
                        "3 hpa",
                        "Early depolarization timing is a source checkpoint for transient-memory presets.",
                    ),
                    dynamics_checkpoint(
                        "durant_2019_6hpa_context",
                        "Durant 2019 early AP-polarity context",
                        "6 hpa",
                        "Later early-window context remains annotation until derived experiment data exists.",
                    ),
                    dynamics_checkpoint(
                        "durant_2019_washout_memory",
                        "Durant 2019 washout and later morphology relation",
                        "washout/outcome",
                        "Brief perturbation followed by washout may still label persistent outcome targets.",
                    ),
                ],
            ),
            source_dynamics_target(
                "gap_block_conductance",
                &[
                    "oviedo_2010_devbiol",
                    "emmons_bell_2015_ijms",
                    "planformdb_250",
                    "lobo_2013_planform",
                ],
                "source:oviedo_2010_devbiol;source:emmons_bell_2015_ijms::target:gap_block_conductance::synthetic_fixture_source_targets_scoped",
                "synthetic_fixture_source_targets_scoped",
                "Gap-junction-like coupling reduction, VNC disruption labels, and innexin RNAi labels kept distinct.",
                &["bioelectric.planarian_ap.gap_block.synthetic"],
                &[
                    "planformdb:experiment:415:resultset:467",
                    "planformdb:experiment:416:resultset:468",
                    "planformdb:experiment:417:resultset:469",
                    "planformdb:experiment:418:resultset:470",
                    "planformdb:experiment:419:resultset:471",
                    "planformdb:experiment:441:resultset:493",
                    "planformdb:experiment:442:resultset:494",
                    "planformdb:experiment:443:resultset:495",
                    "planformdb:experiment:444:resultset:496",
                    "planformdb:experiment:446:resultset:498",
                    "planformdb:experiment:447:resultset:499",
                    "planformdb:experiment:448:resultset:500",
                    "planformdb:experiment:449:resultset:501",
                    "planformdb:experiment:450:resultset:502",
                ],
                &[
                    "qualitative conductance-block scenario labels",
                    "PlanformDB-derived phenotype/outcome annotation",
                    "future source-table threshold review",
                ],
                &[
                    "calibrated physiology",
                    "converting PlanformDB frequencies into stochastic simulation",
                    "collapsing octanol, VNC disruption, and innexin RNAi into one mechanism",
                ],
                vec![
                    dynamics_checkpoint(
                        "oviedo_2010_octanol_gap_block",
                        "Oviedo 2010 octanol gap-junction blockade records",
                        "assay/result-set",
                        "Octanol labels support conductance-block metadata, not conductance constants.",
                    ),
                    dynamics_checkpoint(
                        "oviedo_2010_vnc_disruption",
                        "Oviedo 2010 VNC disruption records",
                        "assay/result-set",
                        "VNC disruption remains a separate label from generic coupling reduction.",
                    ),
                    dynamics_checkpoint(
                        "oviedo_2010_innexin_rnai",
                        "Oviedo 2010 innexin RNAi records",
                        "35 day regeneration period",
                        "Innexin labels are evidence annotations, not direct conductance scalars.",
                    ),
                    dynamics_checkpoint(
                        "emmons_2015_species_like_heads",
                        "Emmons-Bell 2015 stochastic species-like head taxonomy",
                        "figure/table target",
                        "Species-like categories support rights-safe labels and future derived mappings.",
                    ),
                ],
            ),
            source_dynamics_target(
                "head_vs_tail_voltage",
                &["beane_2011_chembiol"],
                "source:beane_2011_chembiol::target:head_vs_tail_voltage::active_annotation_metadata",
                "active_annotation_metadata",
                "Voltage/pump perturbation annotation for head-vs-tail identity context.",
                &["bioelectric.planarian_ap.baseline.synthetic"],
                &[],
                &[
                    "normalized voltage unit-policy annotation",
                    "future named pump/channel source review",
                    "Optics display of source-target metadata",
                ],
                &[
                    "calibrated physiology",
                    "H,K-ATPase constant import",
                    "named ion-channel solver behavior",
                    "millivolt fixture without source-value extraction",
                ],
                vec![
                    dynamics_checkpoint(
                        "beane_2011_hk_atpase_annotation",
                        "Beane 2011 H,K-ATPase-mediated membrane-voltage context",
                        "assay/figure target",
                        "Pump/channel language can label source context but cannot set constants yet.",
                    ),
                    dynamics_checkpoint(
                        "beane_2011_head_tail_identity",
                        "Beane 2011 head regeneration identity context",
                        "source text/figure review pending",
                        "Head-vs-tail voltage remains normalized annotation until values are extracted.",
                    ),
                ],
            ),
            source_dynamics_target(
                "persistent_axis_recut_history",
                &["oviedo_2010_devbiol"],
                "source:oviedo_2010_devbiol::target:persistent_axis_recut_history::future_session_trace",
                "future_session_trace",
                "Persistent AP-axis and repeated-regeneration history target for future package/session fixtures.",
                &[],
                &[],
                &[
                    "future experiment/session package target",
                    "recut history annotation",
                    "Manifold audit-surface planning",
                ],
                &[
                    "calibrated physiology",
                    "claiming current Matter scenario reproduces persistent axes",
                    "storing session history inside one static scenario run",
                ],
                vec![dynamics_checkpoint(
                    "oviedo_2010_ectopic_persistent_axis",
                    "Oviedo 2010 ectopic anterior and persistent-axis target",
                    "repeated-regeneration source target",
                    "Persistent history remains future session/package evidence, not current runtime state.",
                )],
            ),
        ],
    };
    fixture.validate()?;
    Ok(fixture)
}

/// Builds the default public Planarian XR display bridge fixture.
///
/// # Errors
///
/// Returns [`MatterFieldError`] if the generated bridge fixture fails
/// validation.
pub fn default_planarian_xr_display_bridge_fixture(
) -> Result<PlanarianXrDisplayBridgeFixture, MatterFieldError> {
    let fixture = PlanarianXrDisplayBridgeFixture {
        schema_id: PLANARIAN_XR_DISPLAY_BRIDGE_FIXTURE_SCHEMA_ID.to_owned(),
        fixture_id: "fixture.fields.planarian_xr.neuron_cloud_display_bridge.v0".to_owned(),
        schema_version: 1,
        evidence_type: PLANARIAN_XR_DISPLAY_BRIDGE_EVIDENCE_TYPE.to_owned(),
        source_bridge_schema: "planarian-xr.rusty-dynamics-bridge.v1".to_owned(),
        source_bridge_id: "bridge_planarian_rusty_neuron_cloud_v0".to_owned(),
        source_bridge_manifest_path:
            "/data/bridges/planarian-rusty-neuron-cloud-bridge.json".to_owned(),
        source_bridge_manifest_sha256:
            "7a0ce4c93162ff7ec4308222155f5c6ca31ff20305e06af477655750b481ca2f"
                .to_owned(),
        atlas_geometry_asset_id: "geo_zenodo_neuron_cell_cloud_candidate".to_owned(),
        source_layer_id: "source_layer_zenodo_11724834_neuron_cell_cloud".to_owned(),
        source_doi: "10.5281/zenodo.11724834".to_owned(),
        source_object_name: "planarianneuronpool.Cloud".to_owned(),
        source_object_type: "HxCluster".to_owned(),
        source_map_path: "/data/source-maps/zenodo-11724834-neuron-cell-cloud-point-map.json"
            .to_owned(),
        source_map_sha256:
            "e0fa27071c3d95dad6df82ce1e52860b1b12f1a0532cad1bbed7940c05621b51"
                .to_owned(),
        input_geometry_path: "/assets/geometry/derived/neuron-cell-cloud.glb".to_owned(),
        input_geometry_sha256:
            "97a18266dfa0cfa0f1fac739cf01c64c5a02ea0413d5a0b7aa81e3eb24e45787"
                .to_owned(),
        matter_substrate_role: "display_substrate".to_owned(),
        matter_authority: "rusty-matter".to_owned(),
        optics_authority: "rusty-optics".to_owned(),
        planarian_xr_role: "atlas_manifest_provider".to_owned(),
        simulation_run_id: "sim_zenodo_neuron_cloud_bioelectric_replay_v0".to_owned(),
        source_element_count: 3_467,
        mapped_element_count: 3_467,
        replay_node_count: 480,
        replay_edge_count: 1_767,
        replay_frame_count: 96,
        public_inputs: vec![
            bridge_public_input(
                "bridge_manifest",
                "/data/bridges/planarian-rusty-neuron-cloud-bridge.json",
                "7a0ce4c93162ff7ec4308222155f5c6ca31ff20305e06af477655750b481ca2f",
                4_077,
            ),
            bridge_public_input(
                "geometry_glb",
                "/assets/geometry/derived/neuron-cell-cloud.glb",
                "97a18266dfa0cfa0f1fac739cf01c64c5a02ea0413d5a0b7aa81e3eb24e45787",
                250_476,
            ),
            bridge_public_input(
                "source_map_sidecar",
                "/data/source-maps/zenodo-11724834-neuron-cell-cloud-point-map.json",
                "e0fa27071c3d95dad6df82ce1e52860b1b12f1a0532cad1bbed7940c05621b51",
                1_940_611,
            ),
            bridge_public_input(
                "replay_manifest",
                "/data/replays/zenodo-neuron-cloud-bioelectric-replay.json",
                "f22dce6a6b7dde3ea2d0ee6ac27b2dd628205e0a7d4f4cdbf60b7391b948ad12",
                2_344,
            ),
            bridge_public_input(
                "preview_gif",
                "/assets/bioelectricity/zenodo-neuron-cloud-bioelectric-replay.gif",
                "de307a94e2d67ae378816362c33cd43dbf49f6f288a5bbf796e057f09ab78ee2",
                3_471_374,
            ),
        ],
        capability_policy: PlanarianXrBridgeCapabilityPolicy {
            allowed_capabilities: vec![
                "source_map_inspection".to_owned(),
                "read_only_element_inspection".to_owned(),
                "display_substrate".to_owned(),
                "model_inspired_replay_listing".to_owned(),
            ],
            blocked_capabilities: vec![
                "observed_dynamics_binding".to_owned(),
                "measured_bioelectric_claims".to_owned(),
                "predictive_regeneration_claims".to_owned(),
                "live_matter_stepping".to_owned(),
                "edit_acceptance".to_owned(),
                "nearest_object_annotation".to_owned(),
            ],
        },
        matter_use_policy: vec![
            "Treat the neuron cloud as a public display substrate, not runtime dynamics."
                .to_owned(),
            "Public display ordinals are not measured bioelectric data.".to_owned(),
            "Use this fixture only to validate bridge metadata before a separate Matter-owned scenario fixture exists."
                .to_owned(),
        ],
        caveats: vec![
            "The Planarian XR source map provides display/source ordinals only, not measured voltage and not predictive dynamics."
                .to_owned(),
            "The replay metadata remains model-inspired display output, not a Matter-owned observed-dynamics binding."
                .to_owned(),
            "Observed-dynamics binding remains blocked until reviewed Matter and Optics outputs are explicitly wired."
                .to_owned(),
        ],
    };
    fixture.validate()?;
    Ok(fixture)
}

/// Builds the default request-only Planarian XR display-substrate graph policy.
///
/// # Errors
///
/// Returns [`MatterFieldError`] if the generated request fails validation.
pub fn default_planarian_xr_display_substrate_request(
) -> Result<PlanarianXrDisplaySubstrateRequest, MatterFieldError> {
    PlanarianXrDisplaySubstrateRequest::from_bridge(
        "request.fields.planarian_xr.neuron_cloud_display_substrate.v0",
        &default_planarian_xr_display_bridge_fixture()?,
        PlanarianXrDisplaySubstrateGraphPolicy {
            materialization_status: "request_only_not_materialized".to_owned(),
            node_policy: "one node per mapped public element from the validated source map"
                .to_owned(),
            coordinate_policy:
                "source-map atlas positions after Planarian XR normalization; not calibrated physical coordinates"
                    .to_owned(),
            edge_policy:
                "deterministic nearest-neighbor display graph requested for later materialization"
                    .to_owned(),
            nearest_neighbors_per_node: 4,
            edge_weight_policy: "qualitative display adjacency only; not conductance and not voltage"
                .to_owned(),
        },
    )
}

/// Builds the default small PlanformDB-derived fixture.
///
/// # Errors
///
/// Returns [`MatterFieldError`] if the generated fixture fails validation.
pub fn default_planformdb_derived_fixture() -> Result<PlanformDbDerivedFixture, MatterFieldError> {
    let fixture = PlanformDbDerivedFixture {
        schema_id: PLANFORMDB_DERIVED_FIXTURE_SCHEMA_ID.to_owned(),
        fixture_id: "planformdb-derived-v0".to_owned(),
        schema_version: 1,
        evidence_type: PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE.to_owned(),
        scope: "Small reviewed PlanformDB-derived metadata fixture for Matter validation; not runtime dynamics.".to_owned(),
        source_database: PlanformDbSourceDatabase {
            id: "planformdb_250".to_owned(),
            version: "2.5.0".to_owned(),
            source_url: "https://lobolab.umbc.edu/planform/download/".to_owned(),
            raw_sha256: "9EFFD13DDB87664B9EF7A9B6C9C1959B502FCDD6C1B06EE016501B2D0BE83B89".to_owned(),
            sqlite_schema_version: 2,
        },
        notice_text: PLANFORMDB_NOTICE_TEXT.to_owned(),
        citations: vec![
            PlanformDbCitation {
                id: "lobo_2013_planform".to_owned(),
                label: "Lobo, Malone, and Levin 2013 - Planform".to_owned(),
                doi: "10.1093/bioinformatics/btt088".to_owned(),
                url: "https://doi.org/10.1093/bioinformatics/btt088".to_owned(),
            },
            PlanformDbCitation {
                id: "oviedo_2010_devbiol".to_owned(),
                label: "Oviedo et al. 2010 - long-range neural and gap-junction cues".to_owned(),
                doi: "10.1016/j.ydbio.2009.12.012".to_owned(),
                url: "https://doi.org/10.1016/j.ydbio.2009.12.012".to_owned(),
            },
        ],
        selection_policy: PlanformDbSelectionPolicy {
            selected_on: "2026-06-13".to_owned(),
            selection_basis: "Small curated PlanformDB records from source-reviewed Oviedo 2010 clusters that exercise octanol crop-position labels, VNC-disruption timing labels, and innexin RNAi crop-position labels.".to_owned(),
            non_scope: vec![
                "calibrated physiology".to_owned(),
                "Matter runtime dynamics".to_owned(),
                "PlanformDB row dump".to_owned(),
                "paper figure redistribution".to_owned(),
                "morphology image redistribution".to_owned(),
            ],
        },
        records: vec![
            planformdb_record(
                415,
                467,
                2,
                "head_crop",
                &["octanol_gap_junction_blockade"],
                "gap_block_conductance",
                132,
                14.0,
                &[(775, 1, "wild_type_like", 0.95), (776, 2, "double_head_two_pharynxes", 0.05)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "Normalized labels are for teaching and validation metadata only.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                    "This record is part of a curated octanol crop-position series for qualitative gap-block review.",
                ],
            ),
            planformdb_record(
                416,
                468,
                3,
                "pre_pharyngeal_crop",
                &["octanol_gap_junction_blockade"],
                "gap_block_conductance",
                118,
                14.0,
                &[(777, 1, "wild_type_like", 0.72), (778, 2, "double_head_two_pharynxes", 0.28)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "Normalized labels are for teaching and validation metadata only.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                    "This record is part of a curated octanol crop-position series for qualitative gap-block review.",
                ],
            ),
            planformdb_record(
                417,
                469,
                4,
                "pharyngeal_crop",
                &["octanol_gap_junction_blockade"],
                "gap_block_conductance",
                115,
                14.0,
                &[(779, 1, "wild_type_like", 0.50), (780, 2, "double_head_two_pharynxes", 0.50)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "Normalized labels are for teaching and validation metadata only.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                    "This record is part of a curated octanol crop-position series for qualitative gap-block review.",
                ],
            ),
            planformdb_record(
                418,
                470,
                20,
                "post_pharyngeal_crop",
                &["octanol_gap_junction_blockade"],
                "gap_block_conductance",
                145,
                14.0,
                &[(781, 2, "double_head_two_pharynxes", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "Normalized labels are for teaching and validation metadata only.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                    "This record is part of a curated octanol crop-position series for qualitative gap-block review.",
                ],
            ),
            planformdb_record(
                419,
                471,
                33,
                "tail_crop",
                &["octanol_gap_junction_blockade"],
                "gap_block_conductance",
                178,
                14.0,
                &[(782, 1, "wild_type_like", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "Normalized labels are for teaching and validation metadata only.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                    "This record is part of a curated octanol crop-position series for qualitative gap-block review.",
                ],
            ),
            planformdb_record(
                441,
                493,
                161,
                "head_plus_post_pharyngeal_crop_with_vnc_disruption_t0d",
                &["octanol_gap_junction_blockade", "vnc_disruption"],
                "gap_block_vnc_disruption_boundary",
                10,
                0.0,
                &[(824, 2, "double_head_two_pharynxes", 0.18), (825, 1, "wild_type_like", 0.82)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "This record separates VNC-disruption labeling from generic conductance semantics.",
                    "This record is part of a curated VNC-disruption timing series; timing remains source metadata, not runtime state.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                442,
                494,
                161,
                "head_plus_post_pharyngeal_crop_with_vnc_disruption_t0_125d",
                &["octanol_gap_junction_blockade", "vnc_disruption"],
                "gap_block_vnc_disruption_boundary",
                10,
                0.0,
                &[(826, 1, "wild_type_like", 0.25), (827, 2, "double_head_two_pharynxes", 0.75)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "This record separates VNC-disruption labeling from generic conductance semantics.",
                    "This record is part of a curated VNC-disruption timing series; timing remains source metadata, not runtime state.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                443,
                495,
                161,
                "head_plus_post_pharyngeal_crop_with_vnc_disruption_t0_5d",
                &["octanol_gap_junction_blockade", "vnc_disruption"],
                "gap_block_vnc_disruption_boundary",
                10,
                0.0,
                &[(828, 1, "wild_type_like", 0.80), (829, 2, "double_head_two_pharynxes", 0.20)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "This record separates VNC-disruption labeling from generic conductance semantics.",
                    "This record is part of a curated VNC-disruption timing series; timing remains source metadata, not runtime state.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                444,
                496,
                161,
                "head_plus_post_pharyngeal_crop_with_vnc_disruption_t1d",
                &["octanol_gap_junction_blockade", "vnc_disruption"],
                "gap_block_vnc_disruption_boundary",
                10,
                0.0,
                &[(830, 1, "wild_type_like", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "This record separates VNC-disruption labeling from generic conductance semantics.",
                    "This record is part of a curated VNC-disruption timing series; timing remains source metadata, not runtime state.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                446,
                498,
                2,
                "head_crop",
                &["dj_inx_12_rnai", "dj_inx_5_13_rnai"],
                "innexin_gap_junction_label",
                20,
                35.0,
                &[(832, 1, "wild_type_like", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "Innexin RNAi labels are preserved as metadata, not converted to conductance constants.",
                    "This record is part of a curated innexin RNAi crop-position series for qualitative gap-junction review.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                447,
                499,
                3,
                "pre_pharyngeal_crop",
                &["dj_inx_12_rnai", "dj_inx_5_13_rnai"],
                "innexin_gap_junction_label",
                20,
                35.0,
                &[(833, 1, "wild_type_like", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "Innexin RNAi labels are preserved as metadata, not converted to conductance constants.",
                    "This record is part of a curated innexin RNAi crop-position series for qualitative gap-junction review.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                448,
                500,
                4,
                "pharyngeal_crop",
                &["dj_inx_12_rnai", "dj_inx_5_13_rnai"],
                "innexin_gap_junction_label",
                21,
                35.0,
                &[(834, 2, "double_head_two_pharynxes", 0.20), (835, 1, "wild_type_like", 0.80)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "Innexin RNAi labels are preserved as metadata, not converted to conductance constants.",
                    "This record is part of a curated innexin RNAi crop-position series for qualitative gap-junction review.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                449,
                501,
                20,
                "post_pharyngeal_crop",
                &["dj_inx_12_rnai", "dj_inx_5_13_rnai"],
                "innexin_gap_junction_label",
                18,
                35.0,
                &[(836, 1, "wild_type_like", 0.20), (837, 2, "double_head_two_pharynxes", 0.80)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "Innexin RNAi labels are preserved as metadata, not converted to conductance constants.",
                    "This record is part of a curated innexin RNAi crop-position series for qualitative gap-junction review.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
            planformdb_record(
                450,
                502,
                33,
                "tail_crop",
                &["dj_inx_12_rnai", "dj_inx_5_13_rnai"],
                "innexin_gap_junction_label",
                20,
                35.0,
                &[(838, 1, "wild_type_like", 1.0)],
                &[
                    "Hand-selected from PlanformDB 2.5.0.",
                    "Innexin RNAi labels are preserved as metadata, not converted to conductance constants.",
                    "This record is part of a curated innexin RNAi crop-position series for qualitative gap-junction review.",
                    "PlanformDB IDs are preserved; raw database rows remain private.",
                    "No calibrated bioelectric physiology is inferred from this record.",
                ],
            ),
        ],
    };
    fixture.validate()?;
    Ok(fixture)
}

fn validate_planformdb_notice(notice_text: &str) -> Result<(), MatterFieldError> {
    for required_phrase in [
        "origin of the software and database must not be misrepresented",
        "acknowledgment and citation",
        "notice may not be removed or altered",
    ] {
        if !notice_text.contains(required_phrase) {
            return Err(MatterFieldError::InvalidRunSummary(
                "PlanformDB notice text is missing a required phrase",
            ));
        }
    }
    Ok(())
}

fn source_dynamics_target(
    target_id: &str,
    source_ids: &[&str],
    source_target_anchor: &str,
    source_target_status: &str,
    dynamics_role: &str,
    matter_scenario_ids: &[&str],
    planformdb_record_ids: &[&str],
    allowed_uses: &[&str],
    blocked_uses: &[&str],
    checkpoints: Vec<PlanarianSourceDynamicsCheckpoint>,
) -> PlanarianSourceDynamicsTarget {
    PlanarianSourceDynamicsTarget {
        target_id: target_id.to_owned(),
        source_ids: source_ids
            .iter()
            .map(|source_id| (*source_id).to_owned())
            .collect(),
        source_target_anchor: source_target_anchor.to_owned(),
        source_target_status: source_target_status.to_owned(),
        dynamics_role: dynamics_role.to_owned(),
        matter_scenario_ids: matter_scenario_ids
            .iter()
            .map(|scenario_id| (*scenario_id).to_owned())
            .collect(),
        planformdb_record_ids: planformdb_record_ids
            .iter()
            .map(|record_id| (*record_id).to_owned())
            .collect(),
        allowed_uses: allowed_uses
            .iter()
            .map(|allowed_use| (*allowed_use).to_owned())
            .collect(),
        blocked_uses: blocked_uses
            .iter()
            .map(|blocked_use| (*blocked_use).to_owned())
            .collect(),
        checkpoints,
    }
}

fn dynamics_checkpoint(
    checkpoint_id: &str,
    source_relation: &str,
    timing_anchor: &str,
    qualitative_observation: &str,
) -> PlanarianSourceDynamicsCheckpoint {
    PlanarianSourceDynamicsCheckpoint {
        checkpoint_id: checkpoint_id.to_owned(),
        source_relation: source_relation.to_owned(),
        timing_anchor: timing_anchor.to_owned(),
        qualitative_observation: qualitative_observation.to_owned(),
        matter_boundary:
            "source-reviewed metadata only; not calibrated physiology or runtime dynamics"
                .to_owned(),
    }
}

fn head_label(
    label_id: &str,
    display_label: &str,
    label_kind: &str,
    source_relation: &str,
) -> PlanarianSpeciesLikeHeadLabel {
    PlanarianSpeciesLikeHeadLabel {
        label_id: label_id.to_owned(),
        display_label: display_label.to_owned(),
        label_kind: label_kind.to_owned(),
        source_relation: source_relation.to_owned(),
        visual_policy: "generated symbolic silhouette or text label only".to_owned(),
    }
}

fn planformdb_record(
    experiment_id: u32,
    result_set_id: u32,
    manipulation_id: u32,
    manipulation: &str,
    perturbations: &[&str],
    teaching_target: &str,
    sample_count: u32,
    regeneration_period_days: f32,
    morphologies: &[(u32, u32, &str, f32)],
    transform_notes: &[&str],
) -> PlanformDbDerivedRecord {
    PlanformDbDerivedRecord {
        record_id: format!("planformdb:experiment:{experiment_id}:resultset:{result_set_id}"),
        evidence_type: PLANFORMDB_DERIVED_RECORD_EVIDENCE_TYPE.to_owned(),
        source_citation_ids: vec![
            "planformdb_250".to_owned(),
            "lobo_2013_planform".to_owned(),
            "oviedo_2010_devbiol".to_owned(),
        ],
        source_ids: PlanformDbSourceIds {
            publication_id: 1,
            species_id: 1,
            experiment_id,
            manipulation_id,
            result_set_id,
        },
        normalized_labels: PlanformDbNormalizedLabels {
            publication_source_id: "oviedo_2010_devbiol".to_owned(),
            species: "dugesia_japonica".to_owned(),
            manipulation: manipulation.to_owned(),
            perturbations: perturbations
                .iter()
                .map(|label| (*label).to_owned())
                .collect(),
            teaching_target: teaching_target.to_owned(),
        },
        assay_context: PlanformDbAssayContext {
            sample_count,
            regeneration_period_days,
        },
        resultant_morphologies: morphologies
            .iter()
            .map(
                |(resultant_morphology_id, morphology_id, normalized_outcome, frequency)| {
                    PlanformDbResultantMorphology {
                        resultant_morphology_id: *resultant_morphology_id,
                        morphology_id: *morphology_id,
                        normalized_outcome: (*normalized_outcome).to_owned(),
                        frequency: *frequency,
                    }
                },
            )
            .collect(),
        transform_notes: transform_notes
            .iter()
            .map(|note| (*note).to_owned())
            .collect(),
    }
}
