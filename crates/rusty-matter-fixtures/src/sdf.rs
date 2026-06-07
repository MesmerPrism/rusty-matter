use rusty_matter_model::{TriangleMeshSnapshot, Vec3};
use rusty_matter_sdf::{MeshSdfSignMode, MeshToSdfConfig, PackedSdfGrid};

use crate::summary::SdfFixtureSummary;

#[derive(Clone, Copy)]
pub(crate) struct SdfFixtureCase {
    pub(crate) fixture_id: &'static str,
    pub(crate) mesh_path: &'static str,
    pub(crate) grid_path: &'static str,
    pub(crate) summary_path: &'static str,
    pub(crate) mesh: fn() -> TriangleMeshSnapshot,
    pub(crate) config: MeshToSdfConfig,
}

pub(crate) fn sdf_fixture_cases() -> Vec<SdfFixtureCase> {
    vec![
        SdfFixtureCase {
            fixture_id: "fixture.sdf.unit_triangle.v1",
            mesh_path: "fixtures/mesh/unit-triangle.json",
            grid_path: "fixtures/sdf/unit-triangle-packed-grid.json",
            summary_path: "fixtures/sdf/unit-triangle-sdf-summary.json",
            mesh: unit_triangle_mesh,
            config: MeshToSdfConfig {
                voxel_size: 0.5,
                padding_voxels: 1,
                max_voxels: 1_000,
                sign_mode: MeshSdfSignMode::TriangleNormal,
            },
        },
        SdfFixtureCase {
            fixture_id: "fixture.sdf.unit_tetrahedron.v1",
            mesh_path: "fixtures/mesh/unit-tetrahedron.json",
            grid_path: "fixtures/sdf/unit-tetrahedron-packed-grid.json",
            summary_path: "fixtures/sdf/unit-tetrahedron-sdf-summary.json",
            mesh: unit_tetrahedron_mesh,
            config: MeshToSdfConfig {
                voxel_size: 0.5,
                padding_voxels: 1,
                max_voxels: 1_000,
                sign_mode: MeshSdfSignMode::UnsignedOnly,
            },
        },
    ]
}

pub(crate) fn summarize_sdf_fixture(
    fixture_id: impl Into<String>,
    mesh: &TriangleMeshSnapshot,
    grid: &PackedSdfGrid,
) -> SdfFixtureSummary {
    let min_distance = grid
        .distances
        .iter()
        .copied()
        .reduce(f32::min)
        .expect("grid has samples");
    let max_distance = grid
        .distances
        .iter()
        .copied()
        .reduce(f32::max)
        .expect("grid has samples");

    SdfFixtureSummary {
        schema_id: "rusty.matter.fixture.sdf_summary.v1".to_owned(),
        fixture_id: fixture_id.into(),
        mesh_id: mesh.mesh_id.clone(),
        grid_id: grid.grid_id.clone(),
        dimensions: grid.dimensions,
        voxel_size: grid.voxel_size,
        sample_count: grid.sample_count(),
        min_distance,
        max_distance,
        origin: grid.origin,
    }
}

pub(crate) fn unit_triangle_mesh() -> TriangleMeshSnapshot {
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

pub(crate) fn unit_tetrahedron_mesh() -> TriangleMeshSnapshot {
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
