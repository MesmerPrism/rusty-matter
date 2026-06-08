use rusty_matter_fields::{
    SurfaceFieldPerturbation, SurfaceFieldPerturbationEffect, SurfaceFieldRunSummary,
    SurfaceFieldRuntimeConfig, SurfaceFieldState, SurfaceFieldSubstrate, SurfaceScalarField,
    SurfaceScalarFieldKind, SurfaceVectorField, SurfaceVectorFieldKind,
};
use rusty_matter_mesh::{MeshSurfaceSampleConfig, MeshSurfaceSamplePattern, TriangleMeshSurface};
use rusty_matter_model::Vec3;

use crate::error::CliError;

pub(crate) fn surface_field_contract_summary(
    surface: &TriangleMeshSurface,
) -> Result<SurfaceFieldRunSummary, CliError> {
    let substrate = surface_field_substrate(surface)?;
    let node_count = substrate.node_count();
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
            SurfaceScalarField::constant(
                "field.wound_signal",
                SurfaceScalarFieldKind::WoundSignal,
                node_count,
                0.0,
            ),
            SurfaceScalarField::constant(
                "field.morphogen",
                SurfaceScalarFieldKind::Morphogen,
                node_count,
                0.25,
            ),
        ],
        vec![SurfaceVectorField::constant(
            "field.polarity",
            SurfaceVectorFieldKind::Polarity,
            node_count,
            Vec3::new(1.0, 0.0, 0.0),
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

    SurfaceFieldRunSummary::from_contracts(
        "fixture.fields.unit_square_contract.v1",
        &substrate,
        &state,
        &SurfaceFieldRuntimeConfig::default(),
        &perturbations,
    )
    .map_err(CliError::Field)
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
