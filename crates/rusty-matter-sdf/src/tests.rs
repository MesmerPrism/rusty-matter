use super::*;
use std::num::NonZeroUsize;

use rusty_matter_batch::{BatchBackendKind, BatchConfig};
use rusty_matter_model::{MatterModelError, TriangleMeshSnapshot, Vec3, TRIANGLE_MESH_SCHEMA_ID};

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

fn tetrahedron_mesh() -> TriangleMeshSnapshot {
    TriangleMeshSnapshot::new(
        "mesh.unit_tetrahedron",
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
        vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
    )
}

#[test]
fn mesh_to_sdf_builds_packed_grid() {
    let grid = build_sdf_from_mesh(
        &triangle_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.5,
            padding_voxels: 1,
            max_voxels: 1_000,
            sign_mode: MeshSdfSignMode::TriangleNormal,
        },
    )
    .expect("grid builds");

    assert_eq!(grid.schema_id, PACKED_SDF_GRID_SCHEMA_ID);
    assert_eq!(grid.dimensions, [4, 4, 2]);
    assert_eq!(grid.sample_count(), 32);
    assert!(grid.distances.iter().all(|distance| distance.is_finite()));
}

#[test]
fn mesh_to_sdf_report_exposes_serial_diagnostics() {
    let report = build_sdf_from_mesh_report(
        &triangle_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.5,
            padding_voxels: 1,
            max_voxels: 1_000,
            sign_mode: MeshSdfSignMode::UnsignedOnly,
        },
    )
    .expect("grid builds");

    assert_eq!(report.grid.dimensions, [4, 4, 2]);
    assert_eq!(report.diagnostics.voxel_count, 32);
    assert_eq!(report.diagnostics.triangle_count, 1);
    assert_eq!(report.diagnostics.triangle_tests, 32);
    assert_eq!(report.diagnostics.rejected_voxels, 0);
}

#[test]
fn mesh_to_sdf_batched_matches_serial_grid_and_diagnostics() {
    let config = MeshToSdfConfig {
        voxel_size: 0.5,
        padding_voxels: 1,
        max_voxels: 1_000,
        sign_mode: MeshSdfSignMode::TriangleNormal,
    };
    let serial = build_sdf_from_mesh_report(&triangle_mesh(), config).expect("serial grid builds");
    let batched = build_sdf_from_mesh_batched(
        &triangle_mesh(),
        config,
        BatchConfig {
            backend: BatchBackendKind::Serial,
            batch_size: NonZeroUsize::new(3).expect("batch size is non-zero"),
            max_threads: None,
        },
    )
    .expect("batched grid builds");

    assert_eq!(batched.build, serial);
    assert_eq!(batched.execution.backend, BatchBackendKind::Serial);
    assert_eq!(batched.execution.batch_size, 3);
    assert_eq!(batched.execution.chunk_count, 11);
    assert_eq!(batched.execution.worker_count, 1);
}

#[test]
fn mesh_to_sdf_batched_rejects_invalid_batch_config() {
    let error = build_sdf_from_mesh_batched(
        &triangle_mesh(),
        MeshToSdfConfig::default(),
        BatchConfig {
            backend: BatchBackendKind::Serial,
            batch_size: NonZeroUsize::new(8).expect("batch size is non-zero"),
            max_threads: Some(0),
        },
    )
    .unwrap_err();

    assert!(matches!(error, SdfError::BatchExecution(_)));
}

#[cfg(feature = "parallel")]
#[test]
fn mesh_to_sdf_rayon_batched_matches_serial_grid_and_diagnostics() {
    let config = MeshToSdfConfig {
        voxel_size: 0.5,
        padding_voxels: 1,
        max_voxels: 1_000,
        sign_mode: MeshSdfSignMode::UnsignedOnly,
    };
    let serial = build_sdf_from_mesh_report(&tetrahedron_mesh(), config).expect("serial builds");
    let rayon = build_sdf_from_mesh_batched(
        &tetrahedron_mesh(),
        config,
        BatchConfig {
            backend: BatchBackendKind::Rayon,
            batch_size: NonZeroUsize::new(5).expect("batch size is non-zero"),
            max_threads: Some(2),
        },
    )
    .expect("rayon build succeeds");

    assert_eq!(rayon.build, serial);
    assert_eq!(rayon.execution.backend, BatchBackendKind::Rayon);
    assert_eq!(rayon.execution.batch_size, 5);
    assert_eq!(rayon.execution.chunk_count, 13);
    assert_eq!(rayon.execution.worker_count, 2);
}

#[test]
fn tetrahedron_fixture_builds_unsigned_grid() {
    let grid = build_sdf_from_mesh(
        &tetrahedron_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.5,
            padding_voxels: 1,
            max_voxels: 1_000,
            sign_mode: MeshSdfSignMode::UnsignedOnly,
        },
    )
    .expect("grid builds");

    assert_eq!(grid.dimensions, [4, 4, 4]);
    assert_eq!(grid.sample_count(), 64);
    assert!(grid.distances.iter().all(|distance| *distance >= 0.0));
}

#[test]
fn mesh_to_sdf_rejects_bad_indices() {
    let mesh = TriangleMeshSnapshot {
        schema_id: TRIANGLE_MESH_SCHEMA_ID.to_owned(),
        mesh_id: "mesh.bad".to_owned(),
        positions: vec![Vec3::ZERO],
        indices: vec![[0, 1, 2]],
    };
    let error = build_sdf_from_mesh(&mesh, MeshToSdfConfig::default()).unwrap_err();
    assert!(matches!(
        error,
        SdfError::Model(MatterModelError::IndexOutOfRange { .. })
    ));
}

#[test]
fn mesh_to_sdf_rejects_invalid_voxel_size() {
    let error = build_sdf_from_mesh(
        &triangle_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.0,
            ..MeshToSdfConfig::default()
        },
    )
    .unwrap_err();
    assert_eq!(error, SdfError::InvalidVoxelSize(0.0));
}

#[test]
fn sample_nearest_returns_distance() {
    let grid = build_sdf_from_mesh(
        &triangle_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.5,
            padding_voxels: 1,
            max_voxels: 1_000,
            sign_mode: MeshSdfSignMode::UnsignedOnly,
        },
    )
    .expect("grid builds");
    let sample = grid
        .sample_nearest(Vec3::new(0.0, 0.0, 0.0))
        .expect("sample exists");
    assert!(sample.distance >= 0.0);
}

#[test]
fn grid_linear_cell_helpers_round_trip_x_fastest_order() {
    let grid = build_sdf_from_mesh(
        &triangle_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.5,
            padding_voxels: 1,
            max_voxels: 1_000,
            sign_mode: MeshSdfSignMode::UnsignedOnly,
        },
    )
    .expect("grid builds");

    for linear in 0..grid.sample_count() {
        let [x, y, z] = grid.linear_to_cell(linear).expect("cell exists");
        assert_eq!(grid.packed_index(x, y, z), Some(linear));
        assert_eq!(grid.cell_center(x, y, z), grid.linear_cell_center(linear));
    }
    assert_eq!(grid.linear_to_cell(grid.sample_count()), None);
    assert_eq!(grid.linear_cell_center(grid.sample_count()), None);
}

#[test]
fn grid_checked_and_clamped_sampling_have_explicit_boundary_behavior() {
    let grid = build_sdf_from_mesh(
        &triangle_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.5,
            padding_voxels: 1,
            max_voxels: 1_000,
            sign_mode: MeshSdfSignMode::UnsignedOnly,
        },
    )
    .expect("grid builds");

    let outside = grid.origin - Vec3::new(10.0, 10.0, 10.0);
    assert_eq!(grid.sample_nearest_checked(outside), None);
    assert_eq!(
        grid.sample_nearest_clamped(outside)
            .expect("clamped sample exists")
            .cell,
        [0, 0, 0]
    );
}

#[test]
fn grid_gradient_nearest_returns_finite_vector() {
    let grid = build_sdf_from_mesh(
        &triangle_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.5,
            padding_voxels: 1,
            max_voxels: 1_000,
            sign_mode: MeshSdfSignMode::UnsignedOnly,
        },
    )
    .expect("grid builds");
    let point = grid
        .cell_center(1, 1, 0)
        .expect("interior-ish cell center exists");
    let gradient = grid
        .gradient_nearest(point)
        .expect("gradient sample exists");

    assert!(gradient.is_finite());
    assert!(gradient.length() <= 1.0 + 1.0e-6);
}

#[test]
fn grid_validation_rejects_distance_count_mismatch() {
    let grid = PackedSdfGrid::new("sdf.bad_count", Vec3::ZERO, 1.0, [2, 2, 2], vec![0.0; 7]);

    assert_eq!(
        grid.validate(),
        Err(SdfError::DistanceCountMismatch {
            expected: 8,
            actual: 7
        })
    );
}

#[test]
fn grid_validation_rejects_non_finite_distances() {
    let grid = PackedSdfGrid::new("sdf.non_finite", Vec3::ZERO, 1.0, [1, 1, 1], vec![f32::NAN]);

    assert_eq!(
        grid.validate(),
        Err(SdfError::NonFiniteDistance { index: 0 })
    );
}

#[test]
fn voxel_budget_is_enforced() {
    let error = build_sdf_from_mesh(
        &triangle_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.01,
            padding_voxels: 1,
            max_voxels: 10,
            sign_mode: MeshSdfSignMode::UnsignedOnly,
        },
    )
    .unwrap_err();
    assert!(matches!(error, SdfError::VoxelBudgetExceeded { .. }));
}
