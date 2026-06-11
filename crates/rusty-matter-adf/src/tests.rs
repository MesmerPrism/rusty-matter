use rusty_matter_model::{TriangleMeshSnapshot, Vec3};
use rusty_matter_sdf::{build_sdf_from_mesh, MeshSdfSignMode, MeshToSdfConfig, PackedSdfGrid};

use crate::*;

fn triangle_mesh() -> TriangleMeshSnapshot {
    TriangleMeshSnapshot::new(
        "mesh.unit_triangle",
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        vec![[0, 1, 2]],
    )
}

fn sdf_grid() -> PackedSdfGrid {
    build_sdf_from_mesh(
        &triangle_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.5,
            padding_voxels: 1,
            max_voxels: 1_000,
            sign_mode: MeshSdfSignMode::UnsignedOnly,
        },
    )
    .expect("SDF grid builds")
}

#[test]
fn adf_builds_from_sdf_grid() {
    let grid = sdf_grid();
    let report = build_adf_from_sdf_grid_report(
        &grid,
        AdfBuildConfig {
            max_depth: 2,
            max_cells: 512,
            error_tolerance: 0.01,
        },
    )
    .expect("ADF builds");

    assert_eq!(report.field.schema_id, ADAPTIVE_DISTANCE_FIELD_SCHEMA_ID);
    assert_eq!(report.field.source_grid_id, grid.grid_id);
    assert!(report.field.cell_count() > 1);
    assert_eq!(report.diagnostics.source_sample_count, grid.sample_count());
    assert_eq!(report.diagnostics.cell_count, report.field.cell_count());
    assert!(report.diagnostics.max_level <= 2);
    assert!(report
        .field
        .cells
        .iter()
        .all(|cell| cell.center_distance.is_finite()));
}

#[test]
fn adf_large_tolerance_collapses_to_root_cell() {
    let grid = sdf_grid();
    let report = build_adf_from_sdf_grid_report(
        &grid,
        AdfBuildConfig {
            max_depth: 4,
            max_cells: 8,
            error_tolerance: 100.0,
        },
    )
    .expect("ADF builds");

    assert_eq!(report.field.cell_count(), 1);
    assert_eq!(report.field.cells[0].level, 0);
    assert_eq!(report.diagnostics.split_count, 0);
}

#[test]
fn adf_enforces_cell_budget() {
    let error = build_adf_from_sdf_grid_report(
        &sdf_grid(),
        AdfBuildConfig {
            max_depth: 2,
            max_cells: 1,
            error_tolerance: 0.0,
        },
    )
    .unwrap_err();

    assert!(matches!(error, AdfError::CellBudgetExceeded { .. }));
}

#[test]
fn adf_samples_containing_leaf_cell() {
    let grid = sdf_grid();
    let field = build_adf_from_sdf_grid(
        &grid,
        AdfBuildConfig {
            max_depth: 2,
            max_cells: 512,
            error_tolerance: 0.01,
        },
    )
    .expect("ADF builds");
    let point = grid
        .linear_cell_center(0)
        .expect("source sample center exists");
    let sample = field.sample_nearest(point).expect("ADF sample exists");

    assert!(sample.distance.is_finite());
    assert!(sample.level <= field.max_depth);
    assert!(sample.cell_index < field.cells.len());
}

#[test]
fn adf_rejects_invalid_config() {
    let error = build_adf_from_sdf_grid_report(
        &sdf_grid(),
        AdfBuildConfig {
            error_tolerance: f32::NAN,
            ..AdfBuildConfig::default()
        },
    )
    .unwrap_err();

    assert!(matches!(error, AdfError::InvalidErrorTolerance(_)));
}

#[test]
fn adf_validation_rejects_empty_cells() {
    let field = AdaptiveDistanceField::new("adf.empty", "sdf.source", Vec3::ZERO, 1.0, 1, vec![]);

    assert_eq!(field.validate(), Err(AdfError::EmptyCells));
}

#[test]
fn adf_validation_rejects_unexpected_schema() {
    let mut field =
        AdaptiveDistanceField::new("adf.schema", "sdf.source", Vec3::ZERO, 1.0, 1, vec![]);
    field.schema_id = "rusty.matter.sdf.packed_grid.v1".to_owned();

    assert_eq!(
        field.validate(),
        Err(AdfError::UnexpectedSchema {
            expected: ADAPTIVE_DISTANCE_FIELD_SCHEMA_ID,
            actual: "rusty.matter.sdf.packed_grid.v1".to_owned(),
        })
    );
}
