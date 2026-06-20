use crate::{
    default_planarian_source_dynamics_targets, default_planarian_species_like_head_taxonomy,
    default_planarian_xr_display_bridge_fixture, default_planarian_xr_display_substrate_request,
    default_planformdb_derived_fixture, MatterFieldError,
};

#[test]
fn source_dynamics_targets_preserve_non_calibrated_boundaries() {
    let fixture =
        default_planarian_source_dynamics_targets().expect("source dynamics fixture validates");

    assert_eq!(
        fixture.fixture_id,
        "fixture.fields.planarian_ap.source_dynamics_targets"
    );
    assert!(fixture.source_policy.contains("not calibrated"));
    assert!(fixture
        .targets
        .iter()
        .any(|target| target.target_id == "ap_transient_memory"
            && target.matter_scenario_ids.contains(
                &"bioelectric.planarian_ap.transient_depolarization_memory.synthetic".to_owned()
            )));
    let gap_target = fixture
        .targets
        .iter()
        .find(|target| target.target_id == "gap_block_conductance")
        .expect("gap-block target exists");
    assert_eq!(gap_target.planformdb_record_ids.len(), 14);
    assert!(gap_target
        .planformdb_record_ids
        .contains(&"planformdb:experiment:444:resultset:496".to_owned()));
    assert!(gap_target
        .planformdb_record_ids
        .contains(&"planformdb:experiment:450:resultset:502".to_owned()));
    assert!(gap_target
        .blocked_uses
        .iter()
        .any(|blocked| blocked.contains("stochastic simulation")));
    assert!(fixture.targets.iter().all(|target| target
        .blocked_uses
        .iter()
        .any(|blocked| blocked.contains("calibrated physiology"))));
}

#[test]
fn damaged_source_dynamics_targets_are_rejected() {
    let mut fixture =
        default_planarian_source_dynamics_targets().expect("fixture validates before damage");
    fixture.targets[0].blocked_uses.clear();

    let error = fixture
        .validate()
        .expect_err("missing blocked uses rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));

    let mut fixture =
        default_planarian_source_dynamics_targets().expect("fixture validates before damage");
    fixture.targets[1].planformdb_record_ids.clear();

    let error = fixture
        .validate()
        .expect_err("PlanformDB target without record IDs rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));

    let mut fixture =
        default_planarian_source_dynamics_targets().expect("fixture validates before damage");
    fixture.targets[1].planformdb_record_ids[0] = "local:experiment:415".to_owned();

    let error = fixture
        .validate()
        .expect_err("malformed PlanformDB record IDs reject");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));
}

#[test]
fn planarian_xr_display_bridge_preserves_matter_boundaries() {
    let fixture = default_planarian_xr_display_bridge_fixture().expect("bridge fixture validates");

    assert_eq!(
        fixture.fixture_id,
        "fixture.fields.planarian_xr.neuron_cloud_display_bridge.v0"
    );
    assert_eq!(fixture.matter_substrate_role, "display_substrate");
    assert_eq!(fixture.matter_authority, "rusty-matter");
    assert_eq!(fixture.optics_authority, "rusty-optics");
    assert_eq!(fixture.source_element_count, 3_467);
    assert_eq!(fixture.source_element_count, fixture.mapped_element_count);
    assert!(fixture
        .capability_policy
        .allowed_capabilities
        .contains(&"display_substrate".to_owned()));
    assert!(fixture
        .capability_policy
        .blocked_capabilities
        .contains(&"observed_dynamics_binding".to_owned()));
    assert!(fixture
        .matter_use_policy
        .iter()
        .any(|policy| policy.contains("not runtime dynamics")));
    assert!(fixture
        .caveats
        .iter()
        .any(|caveat| caveat.contains("not measured")));
    assert!(fixture.public_inputs.iter().any(|input| {
        input.kind == "bridge_manifest"
            && input.sha256 == "7a0ce4c93162ff7ec4308222155f5c6ca31ff20305e06af477655750b481ca2f"
    }));
}

#[test]
fn damaged_planarian_xr_display_bridge_is_rejected() {
    let mut fixture =
        default_planarian_xr_display_bridge_fixture().expect("fixture validates before damage");
    fixture.schema_id = "rusty.matter.fields.planarian_xr_display_bridge_fixture.v0".to_owned();

    let error = fixture.validate().expect_err("wrong schema rejects");
    assert!(matches!(error, MatterFieldError::UnexpectedSchema { .. }));

    let mut fixture =
        default_planarian_xr_display_bridge_fixture().expect("fixture validates before damage");
    fixture.source_map_sha256 = "0".repeat(64);

    let error = fixture
        .validate()
        .expect_err("source-map hash mismatch rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));

    let mut fixture =
        default_planarian_xr_display_bridge_fixture().expect("fixture validates before damage");
    fixture.input_geometry_path = "raw/neuron-cell-cloud.glb".to_owned();

    let error = fixture.validate().expect_err("unsafe path rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));

    let mut fixture =
        default_planarian_xr_display_bridge_fixture().expect("fixture validates before damage");
    fixture.evidence_type = "observed".to_owned();

    let error = fixture.validate().expect_err("evidence overclaim rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));

    let mut fixture =
        default_planarian_xr_display_bridge_fixture().expect("fixture validates before damage");
    fixture
        .capability_policy
        .blocked_capabilities
        .retain(|capability| capability != "observed_dynamics_binding");
    fixture
        .capability_policy
        .allowed_capabilities
        .push("observed_dynamics_binding".to_owned());

    let error = fixture
        .validate()
        .expect_err("capability overclaim rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));
}

#[test]
fn planarian_xr_display_substrate_request_is_request_only() {
    let request =
        default_planarian_xr_display_substrate_request().expect("substrate request validates");

    assert_eq!(
        request.request_id,
        "request.fields.planarian_xr.neuron_cloud_display_substrate.v0"
    );
    assert_eq!(
        request.source_bridge_fixture_id,
        "fixture.fields.planarian_xr.neuron_cloud_display_bridge.v0"
    );
    assert_eq!(request.source_element_count, 3_467);
    assert_eq!(request.requested_node_count, 3_467);
    assert_eq!(
        request.graph_policy.materialization_status,
        "request_only_not_materialized"
    );
    assert_eq!(request.graph_policy.nearest_neighbors_per_node, 4);
    assert!(request
        .allowed_outputs
        .contains(&"display_substrate_graph_fixture".to_owned()));
    assert!(request
        .blocked_outputs
        .contains(&"observed_dynamics_binding".to_owned()));
    assert!(request
        .caveats
        .iter()
        .any(|caveat| caveat.contains("not runtime dynamics")));
}

#[test]
fn damaged_planarian_xr_display_substrate_request_is_rejected() {
    let mut request =
        default_planarian_xr_display_substrate_request().expect("request validates before damage");
    request.schema_id = "rusty.matter.fields.planarian_xr_display_substrate_request.v0".to_owned();

    let error = request.validate().expect_err("wrong schema rejects");
    assert!(matches!(error, MatterFieldError::UnexpectedSchema { .. }));

    let mut request =
        default_planarian_xr_display_substrate_request().expect("request validates before damage");
    request.requested_node_count = request.requested_node_count.saturating_sub(1);

    let error = request.validate().expect_err("count mismatch rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));

    let mut request =
        default_planarian_xr_display_substrate_request().expect("request validates before damage");
    request.source_map_path = "raw/neuron-cloud-source-map.json".to_owned();

    let error = request.validate().expect_err("unsafe path rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));

    let mut request =
        default_planarian_xr_display_substrate_request().expect("request validates before damage");
    request
        .blocked_outputs
        .retain(|output| output != "observed_dynamics_binding");
    request
        .allowed_outputs
        .push("observed_dynamics_binding".to_owned());

    let error = request
        .validate()
        .expect_err("blocked-output overclaim rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));
}

#[test]
fn planformdb_derived_fixture_preserves_source_notice_and_records() {
    let fixture = default_planformdb_derived_fixture().expect("PlanformDB fixture validates");

    assert_eq!(fixture.fixture_id, "planformdb-derived-v0");
    assert_eq!(fixture.records.len(), 14);
    assert!(fixture.notice_text.contains("notice may not be removed"));
    assert!(fixture
        .selection_policy
        .non_scope
        .contains(&"Matter runtime dynamics".to_owned()));
    assert!(fixture.records.iter().any(|record| record
        .normalized_labels
        .manipulation
        .contains("vnc_disruption_t1d")));
    assert!(fixture
        .records
        .iter()
        .any(
            |record| record.normalized_labels.teaching_target == "innexin_gap_junction_label"
                && record.source_ids.experiment_id == 450
        ));
    for record in &fixture.records {
        assert_eq!(record.evidence_type, "derived_planformdb_record");
        let frequency_sum = record
            .resultant_morphologies
            .iter()
            .map(|morphology| morphology.frequency)
            .sum::<f32>();
        assert!((frequency_sum - 1.0).abs() <= 0.001);
        assert!(record
            .source_citation_ids
            .contains(&"planformdb_250".to_owned()));
    }
}

#[test]
fn damaged_planformdb_fixture_is_rejected() {
    let mut fixture =
        default_planformdb_derived_fixture().expect("fixture validates before damage");
    fixture.records[0].resultant_morphologies[0].frequency = 1.2;

    let error = fixture
        .validate()
        .expect_err("invalid morphology frequency rejects");
    assert!(matches!(error, MatterFieldError::InvalidField(_)));

    let mut fixture =
        default_planformdb_derived_fixture().expect("fixture validates before damage");
    fixture.notice_text = "missing required notice".to_owned();
    let error = fixture
        .validate()
        .expect_err("missing notice phrase rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));
}

#[test]
fn species_like_head_taxonomy_is_rights_safe() {
    let taxonomy = default_planarian_species_like_head_taxonomy().expect("head taxonomy validates");

    assert_eq!(taxonomy.labels.len(), 6);
    assert!(taxonomy.image_policy.contains("no paper figure reuse"));
    assert!(taxonomy
        .labels
        .iter()
        .any(|label| label.label_id == "unclassified_teaching_abstraction"));
    assert!(taxonomy
        .labels
        .iter()
        .all(|label| label.visual_policy.contains("generated")));
}

#[test]
fn damaged_species_like_head_taxonomy_is_rejected() {
    let mut taxonomy =
        default_planarian_species_like_head_taxonomy().expect("taxonomy validates before damage");
    taxonomy.labels[1].label_id = taxonomy.labels[0].label_id.clone();

    let error = taxonomy.validate().expect_err("duplicate label rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));

    let mut taxonomy =
        default_planarian_species_like_head_taxonomy().expect("taxonomy validates before damage");
    taxonomy.labels[0].visual_policy = "reuse source figure crop".to_owned();
    let error = taxonomy
        .validate()
        .expect_err("non-generated visual policy rejects");
    assert!(matches!(error, MatterFieldError::InvalidField(_)));
}
