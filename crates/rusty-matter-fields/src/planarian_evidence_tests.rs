use crate::{
    default_planarian_source_dynamics_targets, default_planarian_species_like_head_taxonomy,
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
