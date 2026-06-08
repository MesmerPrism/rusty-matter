use rusty_matter_fields::{
    BioelectricCircuitConfig, BioelectricCircuitRuntime, BioelectricCircuitState,
    BioelectricCircuitStepDiagnostics, BioelectricConductanceEdge, BioelectricCurrentKind,
    BioelectricCurrentTerm, BioelectricGate, BioelectricGateSource, BioelectricMemoryState,
    BioelectricReadoutLayer, BioelectricVoltageField, BioelectricVoltageUnit,
    SurfaceFieldDebugFrame, SurfaceFieldDebugFrameSequence, SurfaceFieldPerturbation,
    SurfaceFieldPerturbationEffect, SurfaceFieldRunSummary, SurfaceFieldRuntime,
    SurfaceFieldRuntimeConfig, SurfaceFieldState, SurfaceFieldSubstrate, SurfaceScalarField,
    SurfaceScalarFieldKind, SurfaceVectorField, SurfaceVectorFieldKind,
};
use rusty_matter_mesh::{MeshSurfaceSampleConfig, MeshSurfaceSamplePattern, TriangleMeshSurface};
use rusty_matter_model::Vec3;

use crate::error::CliError;

pub(crate) fn surface_field_contract_summary(
    surface: &TriangleMeshSurface,
) -> Result<SurfaceFieldRunSummary, CliError> {
    let (substrate, state, perturbations) = surface_field_contracts(surface)?;

    SurfaceFieldRunSummary::from_contracts(
        "fixture.fields.unit_square_contract.v1",
        &substrate,
        &state,
        &SurfaceFieldRuntimeConfig::default(),
        &perturbations,
    )
    .map_err(CliError::Field)
}

pub(crate) fn surface_field_debug_frame(
    surface: &TriangleMeshSurface,
) -> Result<SurfaceFieldDebugFrame, CliError> {
    let (substrate, state, perturbations) = surface_field_contracts(surface)?;
    SurfaceFieldDebugFrame::from_contracts(
        "fixture.fields.unit_square_debug_frame.v1",
        &substrate,
        &state,
        &perturbations,
    )
    .map_err(CliError::Field)
}

pub(crate) fn surface_field_debug_sequence(
    surface: &TriangleMeshSurface,
) -> Result<SurfaceFieldDebugFrameSequence, CliError> {
    let (substrate, state, perturbations) = surface_field_dynamic_contracts(surface)?;
    let config = SurfaceFieldRuntimeConfig {
        config_id: "fields.runtime.dynamic_fixture".to_owned(),
        fixed_step_seconds: 1.0 / 30.0,
        max_steps_per_run: 240,
        scalar_diffusion_rate: 2.8,
        scalar_decay_rate: 0.18,
        second_tier_coupling_weight: 0.42,
        vector_alignment_rate: 3.2,
        vector_gradient_rate: 1.9,
        ..SurfaceFieldRuntimeConfig::default()
    };
    let runtime = SurfaceFieldRuntime::new(config).map_err(CliError::Field)?;
    runtime
        .run_debug_sequence(
            "fixture.fields.unit_square_debug_sequence.v1",
            &substrate,
            &state,
            &perturbations,
            120,
            3,
        )
        .map_err(CliError::Field)
}

pub(crate) fn bioelectric_circuit_config() -> Result<BioelectricCircuitConfig, CliError> {
    let config = BioelectricCircuitConfig {
        config_id: "fields.bioelectric_circuit.fixture".to_owned(),
        fixed_step_seconds: 1.0 / 60.0,
        max_steps_per_run: 180,
        voltage_clamp_min: -1.0,
        voltage_clamp_max: 1.0,
        conductance_clamp_min: 0.0,
        conductance_clamp_max: 3.0,
        current_clamp_absolute: 5.0,
        ..BioelectricCircuitConfig::default()
    };
    config.validate().map_err(CliError::Field)?;
    Ok(config)
}

pub(crate) fn bioelectric_circuit_state(
    surface: &TriangleMeshSurface,
) -> Result<BioelectricCircuitState, CliError> {
    let (substrate, mut circuit) = bioelectric_circuit_contracts(surface)?;
    let runtime =
        BioelectricCircuitRuntime::new(bioelectric_circuit_config()?).map_err(CliError::Field)?;
    runtime
        .step_fixed(&substrate, &mut circuit, 0)
        .map_err(CliError::Field)?;
    Ok(circuit)
}

pub(crate) fn bioelectric_circuit_step_diagnostics(
    surface: &TriangleMeshSurface,
) -> Result<BioelectricCircuitStepDiagnostics, CliError> {
    let (substrate, mut circuit) = bioelectric_circuit_contracts(surface)?;
    let runtime =
        BioelectricCircuitRuntime::new(bioelectric_circuit_config()?).map_err(CliError::Field)?;
    runtime
        .step_fixed(&substrate, &mut circuit, 0)
        .map_err(CliError::Field)
}

fn surface_field_contracts(
    surface: &TriangleMeshSurface,
) -> Result<
    (
        SurfaceFieldSubstrate,
        SurfaceFieldState,
        Vec<SurfaceFieldPerturbation>,
    ),
    CliError,
> {
    let substrate = surface_field_substrate(surface)?;
    let node_count = substrate.node_count();
    let mut wound_values = vec![0.0; node_count];
    for &node_index in &[0_usize, 1, 2] {
        if let Some(value) = wound_values.get_mut(node_index) {
            *value = 1.0 - node_index as f32 * 0.24;
        }
    }
    let morphogen_values = substrate
        .nodes
        .iter()
        .map(|node| node.position.x.clamp(0.0, 1.0))
        .collect::<Vec<_>>();
    let polarity_vectors = substrate
        .nodes
        .iter()
        .map(|node| {
            if node.node_index == 3 || node.node_index == 4 {
                Vec3::new(-1.0, 0.0, 0.0)
            } else {
                Vec3::new(1.0, 0.0, 0.0)
            }
        })
        .collect::<Vec<_>>();
    let state = SurfaceFieldState::new(
        "fields.state.unit_square_contract",
        &substrate,
        vec![
            SurfaceScalarField::constant(
                "field.vmem_like",
                SurfaceScalarFieldKind::VmemLike,
                node_count,
                0.5,
            ),
            SurfaceScalarField::new(
                "field.wound_signal",
                SurfaceScalarFieldKind::WoundSignal,
                wound_values,
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
    .map_err(CliError::Field)?;
    let perturbations = vec![
        SurfaceFieldPerturbation::new(
            "perturbation.wound.center",
            Some("field.wound_signal".to_owned()),
            vec![0, 1, 2],
            SurfaceFieldPerturbationEffect::WoundRegion { signal_value: 1.0 },
        ),
        SurfaceFieldPerturbation::new(
            "perturbation.polarity.invert",
            Some("field.polarity".to_owned()),
            vec![3, 4],
            SurfaceFieldPerturbationEffect::PolarityInversion,
        ),
    ];
    Ok((substrate, state, perturbations))
}

fn surface_field_dynamic_contracts(
    surface: &TriangleMeshSurface,
) -> Result<
    (
        SurfaceFieldSubstrate,
        SurfaceFieldState,
        Vec<SurfaceFieldPerturbation>,
    ),
    CliError,
> {
    let substrate = surface_field_dynamic_substrate(surface)?;
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
        "fields.state.unit_square_dynamic",
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
    .map_err(CliError::Field)?;

    let wound_nodes = nearest_nodes(&substrate, Vec3::new(0.28, 0.64, 0.0), 6);
    let vmem_nodes = nearest_nodes(&substrate, Vec3::new(0.50, 0.48, 0.0), 10);
    let polarity_nodes = nearest_nodes(&substrate, Vec3::new(0.72, 0.34, 0.0), 8);
    let coupling_nodes = nearest_nodes(&substrate, Vec3::new(0.36, 0.58, 0.0), 14);

    let mut wound = SurfaceFieldPerturbation::new(
        "perturbation.wound.dynamic_center",
        Some("field.wound_signal".to_owned()),
        wound_nodes,
        SurfaceFieldPerturbationEffect::WoundRegion { signal_value: 1.0 },
    );
    wound.duration_steps = 30;

    let mut vmem = SurfaceFieldPerturbation::new(
        "perturbation.vmem.dynamic_offset",
        Some("field.vmem_like".to_owned()),
        vmem_nodes,
        SurfaceFieldPerturbationEffect::DepolarizeRegion { delta: 0.12 },
    );
    vmem.start_step = 10;
    vmem.duration_steps = 36;

    let mut polarity = SurfaceFieldPerturbation::new(
        "perturbation.polarity.dynamic_inversion",
        Some("field.polarity".to_owned()),
        polarity_nodes,
        SurfaceFieldPerturbationEffect::PolarityInversion,
    );
    polarity.start_step = 18;

    let mut coupling = SurfaceFieldPerturbation::new(
        "perturbation.coupling.dynamic_wound_shell",
        None,
        coupling_nodes,
        SurfaceFieldPerturbationEffect::CouplingMultiplierChange { multiplier: 1.45 },
    );
    coupling.duration_steps = 90;

    Ok((substrate, state, vec![wound, vmem, polarity, coupling]))
}

fn bioelectric_circuit_contracts(
    surface: &TriangleMeshSurface,
) -> Result<(SurfaceFieldSubstrate, BioelectricCircuitState), CliError> {
    let substrate = surface_field_substrate(surface)?;
    let node_count = substrate.node_count();
    let voltage_values = substrate
        .nodes
        .iter()
        .map(|node| 0.18 * (node.position.x - 0.5) + 0.10 * (node.position.y - 0.5))
        .collect::<Vec<_>>();
    let gate = BioelectricGate::new(
        "gate.voltage_difference.synthetic",
        BioelectricGateSource::VoltageDifference,
        0.07,
        0.018,
        0.3,
        1.65,
    );
    let conductance_edges =
        BioelectricConductanceEdge::from_substrate_neighbors(&substrate, 0.16, 0.045, Some(gate))
            .map_err(CliError::Field)?;

    let source_nodes = nearest_nodes(&substrate, Vec3::new(0.28, 0.62, 0.0), 5);
    let sink_nodes = nearest_nodes(&substrate, Vec3::new(0.76, 0.38, 0.0), 5);
    let mut source = BioelectricCurrentTerm::new(
        "current.fixture.transient_source",
        source_nodes,
        BioelectricCurrentKind::Constant { current: 0.85 },
    );
    source.duration_steps = 24;
    let mut sink = BioelectricCurrentTerm::new(
        "current.fixture.transient_sink",
        sink_nodes,
        BioelectricCurrentKind::Constant { current: -0.35 },
    );
    sink.start_step = 8;
    sink.duration_steps = 40;
    let current_terms = vec![
        BioelectricCurrentTerm::new(
            "current.fixture.leak",
            Vec::new(),
            BioelectricCurrentKind::Leak {
                conductance: 0.16,
                reversal_voltage: 0.0,
            },
        ),
        BioelectricCurrentTerm::new(
            "current.fixture.pump",
            Vec::new(),
            BioelectricCurrentKind::Pump {
                rate: 0.10,
                target_voltage: 0.0,
            },
        ),
        BioelectricCurrentTerm::new(
            "current.fixture.generic_voltage_gate",
            Vec::new(),
            BioelectricCurrentKind::VoltageGated {
                max_conductance: 0.06,
                reversal_voltage: -0.25,
                threshold: 0.16,
                slope: 0.05,
            },
        ),
        source,
        sink,
    ];
    let memory = BioelectricMemoryState::zeroed(
        "memory.fixture.hysteresis",
        node_count,
        0.24,
        -0.16,
        1.9,
        0.55,
    );
    let readout = BioelectricReadoutLayer::new(
        "readout.fixture.voltage_to_morphogen",
        vec![0.0; node_count],
        0.8,
        0.45,
        0.08,
        1.25,
        -1.0,
        1.0,
    );
    let state = BioelectricCircuitState::new(
        "circuit.fixture.bioelectric_unit_square",
        &substrate,
        BioelectricVoltageField::new(
            "field.bioelectric_voltage",
            BioelectricVoltageUnit::Normalized,
            0.0,
            voltage_values,
        ),
        conductance_edges,
        current_terms,
        Some(memory),
        vec![readout],
    )
    .map_err(CliError::Field)?;
    Ok((substrate, state))
}

fn surface_field_substrate(
    surface: &TriangleMeshSurface,
) -> Result<SurfaceFieldSubstrate, CliError> {
    let config = MeshSurfaceSampleConfig {
        sample_config_id: "mesh.surface_sample.field_fixture".to_owned(),
        sample_set_id: "mesh.surface_samples.field_fixture".to_owned(),
        point_count: 12,
        first_tier_neighbor_count: 3,
        second_tier_neighbor_count: 3,
        seed: 48_161,
        pattern: MeshSurfaceSamplePattern::LowDiscrepancy,
        ..MeshSurfaceSampleConfig::default()
    };
    let samples = surface.sample_points(&config).map_err(CliError::Mesh)?;
    SurfaceFieldSubstrate::from_sample_set("fields.substrate.unit_square_fixture", &samples)
        .map_err(CliError::Field)
}

fn surface_field_dynamic_substrate(
    surface: &TriangleMeshSurface,
) -> Result<SurfaceFieldSubstrate, CliError> {
    let config = MeshSurfaceSampleConfig {
        sample_config_id: "mesh.surface_sample.field_dynamic_fixture".to_owned(),
        sample_set_id: "mesh.surface_samples.field_dynamic_fixture".to_owned(),
        point_count: 64,
        first_tier_neighbor_count: 4,
        second_tier_neighbor_count: 4,
        seed: 65_537,
        pattern: MeshSurfaceSamplePattern::LowDiscrepancy,
        ..MeshSurfaceSampleConfig::default()
    };
    let samples = surface.sample_points(&config).map_err(CliError::Mesh)?;
    SurfaceFieldSubstrate::from_sample_set("fields.substrate.unit_square_dynamic", &samples)
        .map_err(CliError::Field)
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
