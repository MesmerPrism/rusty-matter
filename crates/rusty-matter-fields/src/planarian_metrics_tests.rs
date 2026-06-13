use crate::{
    planarian_comparison_scenario_kinds, MatterFieldError, PlanarianBioelectricOutcomeTrace,
    PlanarianBioelectricOutcomeTraceSet, PlanarianBioelectricPresetConfig,
    PlanarianBioelectricScenarioKind, PlanarianBioelectricScenarioRun,
    PlanarianNormalizedMorphologyMetrics,
};

#[test]
fn planarian_outcome_trace_captures_memory_vs_control() {
    let memory = planarian_run(PlanarianBioelectricScenarioKind::TransientDepolarizationMemory);
    let control =
        planarian_run(PlanarianBioelectricScenarioKind::TransientDepolarizationNoMemoryControl);

    let memory_trace =
        PlanarianBioelectricOutcomeTrace::from_scenario_run("trace.planarian.memory", &memory)
            .expect("memory trace validates");
    let control_trace =
        PlanarianBioelectricOutcomeTrace::from_scenario_run("trace.planarian.control", &control)
            .expect("control trace validates");
    let memory_final = memory_trace.samples.last().expect("memory sample");
    let control_final = control_trace.samples.last().expect("control sample");

    assert_eq!(memory_trace.sample_columns.len(), 7);
    assert!(memory_final.posterior_memory_average > 0.35);
    assert!(
        memory_final.posterior_head_identity_average
            > control_final.posterior_head_identity_average + 0.20
    );
    assert_eq!(control_final.posterior_memory_average, 0.0);
}

#[test]
fn planarian_outcome_trace_captures_gap_block_conductance() {
    let baseline = planarian_run(PlanarianBioelectricScenarioKind::Baseline);
    let gap_block = planarian_run(PlanarianBioelectricScenarioKind::GapBlock);

    let baseline_trace =
        PlanarianBioelectricOutcomeTrace::from_scenario_run("trace.planarian.baseline", &baseline)
            .expect("baseline trace validates");
    let gap_trace =
        PlanarianBioelectricOutcomeTrace::from_scenario_run("trace.planarian.gap", &gap_block)
            .expect("gap trace validates");

    assert!(baseline_trace.cross_cut_base_conductance_average > 0.0);
    assert!(
        gap_trace.cross_cut_base_conductance_average
            < baseline_trace.cross_cut_base_conductance_average * 0.15
    );
}

#[test]
fn damaged_planarian_outcome_trace_is_rejected() {
    let run = planarian_run(PlanarianBioelectricScenarioKind::TransientDepolarizationMemory);
    let mut trace =
        PlanarianBioelectricOutcomeTrace::from_scenario_run("trace.planarian.damaged", &run)
            .expect("trace validates before damage");

    trace.schema_id = "rusty.matter.fields.wrong.v1".to_owned();
    let error = trace.validate().expect_err("wrong schema rejects");
    assert!(matches!(error, MatterFieldError::UnexpectedSchema { .. }));

    let mut trace =
        PlanarianBioelectricOutcomeTrace::from_scenario_run("trace.planarian.damaged", &run)
            .expect("trace validates before sample damage");
    trace.samples[0].posterior_memory_average = f32::NAN;
    let error = trace.validate().expect_err("non-finite sample rejects");
    assert!(matches!(error, MatterFieldError::InvalidField(_)));
}

#[test]
fn planarian_outcome_trace_set_captures_comparison_family() {
    let trace_set = PlanarianBioelectricOutcomeTraceSet::from_preset_config(
        "trace_set.planarian.comparison",
        &planarian_comparison_scenario_kinds(),
        test_planarian_config(),
    )
    .expect("trace set validates");

    assert_eq!(trace_set.traces.len(), 5);
    assert_eq!(trace_set.sample_columns.len(), 7);
    assert!(trace_set
        .trace_for_scenario(PlanarianBioelectricScenarioKind::Baseline)
        .is_some());
    let memory = trace_set
        .trace_for_scenario(PlanarianBioelectricScenarioKind::TransientDepolarizationMemory)
        .expect("memory trace");
    let control = trace_set
        .trace_for_scenario(
            PlanarianBioelectricScenarioKind::TransientDepolarizationNoMemoryControl,
        )
        .expect("control trace");
    let memory_final = memory.samples.last().expect("memory final");
    let control_final = control.samples.last().expect("control final");

    assert!(memory_final.posterior_memory_average > 0.35);
    assert!(
        memory_final.posterior_head_identity_average
            > control_final.posterior_head_identity_average + 0.20
    );
}

#[test]
fn damaged_planarian_outcome_trace_set_is_rejected() {
    let mut trace_set = PlanarianBioelectricOutcomeTraceSet::from_preset_config(
        "trace_set.planarian.damaged",
        &planarian_comparison_scenario_kinds(),
        test_planarian_config(),
    )
    .expect("trace set validates before damage");

    let duplicate = trace_set.traces[0].clone();
    trace_set.traces.push(duplicate);
    let error = trace_set
        .validate()
        .expect_err("duplicate scenario trace rejects");

    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));
}

#[test]
fn normalized_morphology_metrics_preserve_non_calibrated_source_target() {
    let run = planarian_run(PlanarianBioelectricScenarioKind::TransientDepolarizationMemory);
    let metrics = PlanarianNormalizedMorphologyMetrics::from_scenario_run(
        "metrics.planarian.normalized_morphology",
        &run,
    )
    .expect("normalized morphology metrics validate");

    assert_eq!(metrics.region_extents.len(), 5);
    assert!(metrics.unit_policy.contains("not calibrated"));
    assert_eq!(
        metrics.source_target_anchors,
        vec!["source:beane_2013_dev::target:head_size_scaling::future_metric".to_owned()]
    );
    assert!(metrics.head_region_extent_normalized > 0.0);
    assert!(metrics.pharyngeal_region_extent_normalized > 0.0);
    assert!((0.0..=1.0).contains(&metrics.head_identity_extent_normalized));
}

#[test]
fn damaged_normalized_morphology_metrics_are_rejected() {
    let run = planarian_run(PlanarianBioelectricScenarioKind::TransientDepolarizationMemory);
    let mut metrics = PlanarianNormalizedMorphologyMetrics::from_scenario_run(
        "metrics.planarian.normalized_morphology.damaged",
        &run,
    )
    .expect("normalized morphology metrics validate before damage");

    metrics.head_region_extent_normalized = f32::NAN;
    let error = metrics
        .validate()
        .expect_err("non-finite normalized metric rejects");
    assert!(matches!(error, MatterFieldError::InvalidField(_)));

    let mut metrics = PlanarianNormalizedMorphologyMetrics::from_scenario_run(
        "metrics.planarian.normalized_morphology.damaged_anchor",
        &run,
    )
    .expect("normalized morphology metrics validate before anchor damage");
    metrics.source_target_anchors.clear();
    let error = metrics
        .validate()
        .expect_err("missing source-target anchor rejects");
    assert!(matches!(error, MatterFieldError::InvalidRunSummary(_)));
}

fn planarian_run(kind: PlanarianBioelectricScenarioKind) -> PlanarianBioelectricScenarioRun {
    PlanarianBioelectricScenarioRun::build(kind, test_planarian_config())
        .expect("planarian scenario validates")
}

fn test_planarian_config() -> PlanarianBioelectricPresetConfig {
    PlanarianBioelectricPresetConfig {
        sample_count: 80,
        step_count: 150,
        frame_stride: 15,
        seed: 130_363,
        ..PlanarianBioelectricPresetConfig::default()
    }
}
