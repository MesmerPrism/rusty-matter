use rusty_matter_fields::{
    SurfaceFieldDebugFrame, SurfaceFieldPerturbation, SurfaceFieldPerturbationEffect,
    SurfaceFieldRunSummary, SurfaceFieldRuntimeConfig, SurfaceFieldState, SurfaceFieldSubstrate,
    SurfaceScalarField, SurfaceScalarFieldKind, SurfaceVectorField, SurfaceVectorFieldKind,
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
