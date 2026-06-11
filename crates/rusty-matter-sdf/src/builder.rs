use rusty_matter_batch::{BatchBackendKind, BatchConfig, BatchExecutor, BatchReduce};
use rusty_matter_model::{Bounds3, MatterModelError, TriangleMeshSnapshot, Vec3};

use crate::geometry::{nearest_signed_distance, Triangle};
use crate::grid::voxel_count;
use crate::{MeshToSdfConfig, PackedSdfGrid, SdfError};

/// Dense SDF build diagnostics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SdfBuildDiagnostics {
    /// Number of voxels in the generated dense grid.
    pub voxel_count: usize,
    /// Number of triangles prepared from the source mesh.
    pub triangle_count: usize,
    /// Exact triangle distance tests performed by the current brute-force
    /// reference builder.
    pub triangle_tests: usize,
    /// Voxels rejected while building the grid.
    pub rejected_voxels: usize,
}

/// Dense SDF build output with diagnostics.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SdfBuildReport {
    /// Built packed SDF grid.
    pub grid: PackedSdfGrid,
    /// Build diagnostics.
    pub diagnostics: SdfBuildDiagnostics,
}

/// Low-rate execution diagnostics for a batched dense SDF build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SdfBuildExecutionDiagnostics {
    /// Backend used for the build.
    pub backend: BatchBackendKind,
    /// Configured logical chunk size.
    pub batch_size: usize,
    /// Number of logical chunks executed.
    pub chunk_count: usize,
    /// Worker count reported by the batch executor.
    pub worker_count: usize,
}

/// Dense SDF build output produced through a Matter batch executor.
#[derive(Clone, Debug, PartialEq)]
pub struct SdfBatchedBuildReport {
    /// Built grid and mesh/SDF diagnostics.
    pub build: SdfBuildReport,
    /// Low-rate batch execution diagnostics.
    pub execution: SdfBuildExecutionDiagnostics,
}

/// Builds an SDF grid from a triangle mesh snapshot.
///
/// # Errors
///
/// Returns [`SdfError`] when the mesh, config, or generated grid is invalid.
pub fn build_sdf_from_mesh(
    mesh: &TriangleMeshSnapshot,
    config: MeshToSdfConfig,
) -> Result<PackedSdfGrid, SdfError> {
    build_sdf_from_mesh_report(mesh, config).map(|report| report.grid)
}

/// Builds an SDF grid from a triangle mesh snapshot and returns diagnostics.
///
/// # Errors
///
/// Returns [`SdfError`] when the mesh, config, or generated grid is invalid.
pub fn build_sdf_from_mesh_report(
    mesh: &TriangleMeshSnapshot,
    config: MeshToSdfConfig,
) -> Result<SdfBuildReport, SdfError> {
    let prepared = prepare_sdf_build(mesh, config)?;

    let mut distances = Vec::with_capacity(prepared.count);
    let mut diagnostics = SdfBuildDiagnostics {
        voxel_count: prepared.count,
        triangle_count: prepared.triangles.len(),
        ..SdfBuildDiagnostics::default()
    };
    for z in 0..prepared.dimensions[2] {
        for y in 0..prepared.dimensions[1] {
            for x in 0..prepared.dimensions[0] {
                let point = cell_center_for_indices(prepared.bounds.min, x, y, z, config);
                let distance =
                    nearest_signed_distance(point, &prepared.triangles, config.sign_mode)?;
                diagnostics.triangle_tests = diagnostics
                    .triangle_tests
                    .saturating_add(prepared.triangles.len());
                distances.push(distance);
            }
        }
    }

    let grid = packed_grid_for_build(mesh, &prepared, config, distances)?;
    Ok(SdfBuildReport { grid, diagnostics })
}

/// Builds an SDF grid with an explicit Matter batch execution config.
///
/// # Errors
///
/// Returns [`SdfError`] when the mesh, config, batch config, or generated grid
/// is invalid.
pub fn build_sdf_from_mesh_batched(
    mesh: &TriangleMeshSnapshot,
    config: MeshToSdfConfig,
    batch_config: BatchConfig,
) -> Result<SdfBatchedBuildReport, SdfError> {
    let executor = BatchExecutor::new(batch_config)?;
    build_sdf_from_mesh_with_executor(mesh, config, &executor)
}

/// Builds an SDF grid with a reusable Matter batch executor.
///
/// Use this when dense SDF builds are part of a high-rate adapter path and the
/// caller wants to reuse the executor worker pool across builds.
///
/// # Errors
///
/// Returns [`SdfError`] when the mesh, config, or generated grid is invalid.
pub fn build_sdf_from_mesh_with_executor(
    mesh: &TriangleMeshSnapshot,
    config: MeshToSdfConfig,
    executor: &BatchExecutor,
) -> Result<SdfBatchedBuildReport, SdfError> {
    let prepared = prepare_sdf_build(mesh, config)?;
    let mut distances = vec![0.0; prepared.count];
    let report = executor.run_slice_chunks(&mut distances, |chunk, output| {
        let mut diagnostics = SdfBuildDiagnostics {
            voxel_count: output.len(),
            triangle_count: prepared.triangles.len(),
            ..SdfBuildDiagnostics::default()
        };
        for (offset, distance) in output.iter_mut().enumerate() {
            let linear = chunk.range.start + offset;
            let [x, y, z] = linear_to_cell(linear, prepared.dimensions);
            let point = cell_center_for_indices(prepared.bounds.min, x, y, z, config);
            *distance = nearest_signed_distance(point, &prepared.triangles, config.sign_mode)
                .expect("prepared SDF triangles are non-empty and non-degenerate");
            diagnostics.triangle_tests = diagnostics
                .triangle_tests
                .saturating_add(prepared.triangles.len());
        }
        diagnostics
    });

    let grid = packed_grid_for_build(mesh, &prepared, config, distances)?;
    Ok(SdfBatchedBuildReport {
        build: SdfBuildReport {
            grid,
            diagnostics: report.diagnostics,
        },
        execution: SdfBuildExecutionDiagnostics {
            backend: report.backend,
            batch_size: report.batch_size,
            chunk_count: report.chunk_count,
            worker_count: report.worker_count,
        },
    })
}

impl BatchReduce for SdfBuildDiagnostics {
    fn reduce(&mut self, other: Self) {
        self.voxel_count = self.voxel_count.saturating_add(other.voxel_count);
        self.triangle_count = self.triangle_count.max(other.triangle_count);
        self.triangle_tests = self.triangle_tests.saturating_add(other.triangle_tests);
        self.rejected_voxels = self.rejected_voxels.saturating_add(other.rejected_voxels);
    }
}

struct PreparedSdfBuild {
    bounds: Bounds3,
    dimensions: [u32; 3],
    count: usize,
    triangles: Vec<Triangle>,
}

fn prepare_sdf_build(
    mesh: &TriangleMeshSnapshot,
    config: MeshToSdfConfig,
) -> Result<PreparedSdfBuild, SdfError> {
    mesh.validate().map_err(SdfError::Model)?;
    config.validate()?;

    let bounds = padded_bounds(mesh.bounds().map_err(SdfError::Model)?, config)?;
    let dimensions = dimensions_for_bounds(bounds, config)?;
    let count = voxel_count(dimensions)?;
    if count > config.max_voxels {
        return Err(SdfError::VoxelBudgetExceeded {
            requested: count,
            max: config.max_voxels,
        });
    }

    let triangles = mesh
        .indices
        .iter()
        .map(|indices| {
            let [a, b, c] = *indices;
            Triangle::new(
                mesh.positions[usize::try_from(a).expect("validated index fits usize")],
                mesh.positions[usize::try_from(b).expect("validated index fits usize")],
                mesh.positions[usize::try_from(c).expect("validated index fits usize")],
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PreparedSdfBuild {
        bounds,
        dimensions,
        count,
        triangles,
    })
}

fn packed_grid_for_build(
    mesh: &TriangleMeshSnapshot,
    prepared: &PreparedSdfBuild,
    config: MeshToSdfConfig,
    distances: Vec<f32>,
) -> Result<PackedSdfGrid, SdfError> {
    let grid = PackedSdfGrid::new(
        format!("sdf.{}", mesh.mesh_id),
        prepared.bounds.min,
        config.voxel_size,
        prepared.dimensions,
        distances,
    );
    grid.validate()?;
    Ok(grid)
}

fn linear_to_cell(linear: usize, dimensions: [u32; 3]) -> [u32; 3] {
    let [width, height, _] = dimensions;
    let width = usize::try_from(width).expect("validated SDF width fits usize");
    let height = usize::try_from(height).expect("validated SDF height fits usize");
    let plane = width
        .checked_mul(height)
        .expect("validated SDF plane fits usize");
    let z = linear / plane;
    let remainder = linear % plane;
    let y = remainder / width;
    let x = remainder % width;
    [
        u32::try_from(x).expect("validated SDF x fits u32"),
        u32::try_from(y).expect("validated SDF y fits u32"),
        u32::try_from(z).expect("validated SDF z fits u32"),
    ]
}

fn cell_center_for_indices(origin: Vec3, x: u32, y: u32, z: u32, config: MeshToSdfConfig) -> Vec3 {
    origin
        + Vec3::new(
            (x as f32 + 0.5) * config.voxel_size,
            (y as f32 + 0.5) * config.voxel_size,
            (z as f32 + 0.5) * config.voxel_size,
        )
}

fn padded_bounds(bounds: Bounds3, config: MeshToSdfConfig) -> Result<Bounds3, SdfError> {
    bounds
        .padded(config.voxel_size * config.padding_voxels as f32)
        .map_err(SdfError::Model)
}

fn dimensions_for_bounds(bounds: Bounds3, config: MeshToSdfConfig) -> Result<[u32; 3], SdfError> {
    let size = bounds.size();
    Ok([
        dimension_for_axis(size.x, config.voxel_size)?,
        dimension_for_axis(size.y, config.voxel_size)?,
        dimension_for_axis(size.z, config.voxel_size)?,
    ])
}

fn dimension_for_axis(size: f32, voxel_size: f32) -> Result<u32, SdfError> {
    if !size.is_finite() || size < 0.0 {
        return Err(SdfError::Model(MatterModelError::InvertedBounds));
    }
    let cells = (size / voxel_size).ceil().max(2.0) as u64;
    u32::try_from(cells)
        .ok()
        .filter(|value| *value > 0)
        .ok_or(SdfError::ZeroDimension)
}
