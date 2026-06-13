use crate::{
    default_planarian_species_like_head_taxonomy, default_planformdb_derived_fixture,
    MatterFieldError,
};

#[test]
fn planformdb_derived_fixture_preserves_source_notice_and_records() {
    let fixture = default_planformdb_derived_fixture().expect("PlanformDB fixture validates");

    assert_eq!(fixture.fixture_id, "planformdb-derived-v0");
    assert_eq!(fixture.records.len(), 4);
    assert!(fixture.notice_text.contains("notice may not be removed"));
    assert!(fixture
        .selection_policy
        .non_scope
        .contains(&"Matter runtime dynamics".to_owned()));
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
