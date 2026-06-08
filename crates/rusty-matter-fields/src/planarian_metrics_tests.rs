use crate::{
    MatterFieldError, PlanarianBioelectricOutcomeTrace, PlanarianBioelectricPresetConfig,
    PlanarianBioelectricScenarioKind, PlanarianBioelectricScenarioRun,
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

fn planarian_run(kind: PlanarianBioelectricScenarioKind) -> PlanarianBioelectricScenarioRun {
    PlanarianBioelectricScenarioRun::build(
        kind,
        PlanarianBioelectricPresetConfig {
            sample_count: 80,
            step_count: 150,
            frame_stride: 15,
            seed: 130_363,
            ..PlanarianBioelectricPresetConfig::default()
        },
    )
    .expect("planarian scenario validates")
}
