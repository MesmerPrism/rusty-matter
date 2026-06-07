use rusty_matter_model::Vec3;

use crate::math::normalize_or;
use crate::surface::triangle_area;
use crate::{MatterMeshError, MeshSurfaceTopologyKey, TriangleMeshSurface};

/// Schema ID for mesh surface sample configuration.
pub const MESH_SURFACE_SAMPLE_CONFIG_SCHEMA_ID: &str = "rusty.matter.mesh.surface_sample_config.v1";
/// Schema ID for mesh surface samples.
pub const MESH_SURFACE_SAMPLE_SCHEMA_ID: &str = "rusty.matter.mesh.surface_sample.v1";
/// Schema ID for mesh surface sample sets.
pub const MESH_SURFACE_SAMPLE_SET_SCHEMA_ID: &str = "rusty.matter.mesh.surface_sample_set.v1";
/// Schema ID for cross-surface neighborhoods.
pub const MESH_SURFACE_CROSS_NEIGHBORHOOD_SCHEMA_ID: &str =
    "rusty.matter.mesh.surface_cross_neighborhood.v1";

/// Deterministic mesh-surface sampling pattern.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeshSurfaceSamplePattern {
    /// Area-stratified triangle selection with seeded per-triangle barycentric points.
    AreaStratified,
    /// Low-discrepancy area traversal for less clumped coordinate placement.
    LowDiscrepancy,
}

impl Default for MeshSurfaceSamplePattern {
    fn default() -> Self {
        Self::AreaStratified
    }
}

/// Deterministic mesh-surface sampling configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshSurfaceSampleConfig {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable config identifier.
    pub sample_config_id: String,
    /// Stable output sample-set identifier.
    pub sample_set_id: String,
    /// Exact requested output count for valid surfaces.
    pub point_count: usize,
    /// Same-surface nearest-neighbor count for the first tier.
    pub first_tier_neighbor_count: usize,
    /// Same-surface nearest-neighbor count for the second tier.
    pub second_tier_neighbor_count: usize,
    /// Deterministic sampling seed.
    pub seed: u64,
    /// Deterministic sampling pattern.
    pub pattern: MeshSurfaceSamplePattern,
}

impl Default for MeshSurfaceSampleConfig {
    fn default() -> Self {
        Self {
            schema_id: MESH_SURFACE_SAMPLE_CONFIG_SCHEMA_ID.to_owned(),
            sample_config_id: "mesh.surface_sample.default".to_owned(),
            sample_set_id: "mesh.surface_samples.default".to_owned(),
            point_count: 256,
            first_tier_neighbor_count: 6,
            second_tier_neighbor_count: 12,
            seed: 11_337,
            pattern: MeshSurfaceSamplePattern::AreaStratified,
        }
    }
}

impl MeshSurfaceSampleConfig {
    /// Creates a high-quality surface-coordinate distribution config.
    #[must_use]
    pub fn high_quality_surface_points(point_count: usize) -> Self {
        Self {
            sample_config_id: "mesh.surface_sample.high_quality".to_owned(),
            sample_set_id: "mesh.surface_samples.high_quality".to_owned(),
            point_count,
            first_tier_neighbor_count: 6,
            second_tier_neighbor_count: 14,
            seed: 29_791,
            pattern: MeshSurfaceSamplePattern::LowDiscrepancy,
            ..Self::default()
        }
    }

    /// Validates the sample config.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the config is invalid.
    pub fn validate(&self) -> Result<(), MatterMeshError> {
        if self.schema_id != MESH_SURFACE_SAMPLE_CONFIG_SCHEMA_ID {
            return Err(MatterMeshError::UnexpectedSchema {
                expected: MESH_SURFACE_SAMPLE_CONFIG_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.sample_config_id.trim().is_empty() {
            return Err(MatterMeshError::EmptySampleConfigId);
        }
        if self.sample_set_id.trim().is_empty() {
            return Err(MatterMeshError::EmptySampleSetId);
        }
        if self.point_count == 0 {
            return Err(MatterMeshError::InvalidSampleConfig(
                "point_count must be non-zero",
            ));
        }
        Ok(())
    }
}

/// One sampled coordinate on a triangle mesh surface.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshSurfaceSample {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable sample identifier within a sample set.
    pub sample_id: String,
    /// Evaluated position.
    pub position: Vec3,
    /// Evaluated surface normal.
    pub normal: Vec3,
    /// Source triangle index.
    pub triangle_index: usize,
    /// Barycentric coordinates within the source triangle.
    pub barycentric: [f32; 3],
}

impl MeshSurfaceSample {
    /// Returns whether this sample is internally consistent.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.schema_id == MESH_SURFACE_SAMPLE_SCHEMA_ID
            && !self.sample_id.trim().is_empty()
            && self.position.is_finite()
            && self.normal.is_finite()
            && self.normal.length_squared() > 1.0e-10
            && self
                .barycentric
                .iter()
                .all(|value| value.is_finite() && *value >= -1.0e-5 && *value <= 1.0 + 1.0e-5)
            && (self.barycentric[0] + self.barycentric[1] + self.barycentric[2] - 1.0).abs()
                <= 1.0e-4
    }
}

/// Sampled mesh coordinates and nearest-neighbor tiers.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshSurfaceSampleSet {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable sample set identifier.
    pub sample_set_id: String,
    /// Source surface identifier.
    pub surface_id: String,
    /// Source topology key.
    pub topology_key: MeshSurfaceTopologyKey,
    /// Surface samples.
    pub samples: Vec<MeshSurfaceSample>,
    /// First nearest-neighbor tier per sample.
    pub first_tier_neighbors: Vec<Vec<usize>>,
    /// Second nearest-neighbor tier per sample.
    pub second_tier_neighbors: Vec<Vec<usize>>,
}

impl MeshSurfaceSampleSet {
    /// Returns the sample count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Returns whether there are no samples.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Returns sample positions.
    #[must_use]
    pub fn positions(&self) -> Vec<Vec3> {
        self.samples.iter().map(|sample| sample.position).collect()
    }

    /// Re-evaluates sample anchors against a deformed mesh with the same
    /// topology.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when topology changed or anchors cannot be
    /// evaluated.
    pub fn update_positions_from_surface(
        &mut self,
        surface: &TriangleMeshSurface,
    ) -> Result<(), MatterMeshError> {
        surface.validate()?;
        let next_key = surface.topology_key();
        if self.topology_key != next_key {
            return Err(MatterMeshError::ChangedTopology);
        }

        let mut updates = Vec::with_capacity(self.samples.len());
        for sample in &self.samples {
            let Some((position, normal)) =
                evaluate_surface_anchor(surface, sample.triangle_index, sample.barycentric)
            else {
                return Err(MatterMeshError::InvalidSurface(
                    "sample anchor could not be evaluated",
                ));
            };
            updates.push((position, normal));
        }
        for (sample, (position, normal)) in self.samples.iter_mut().zip(updates) {
            sample.position = position;
            sample.normal = normal;
        }
        Ok(())
    }

    /// Rebuilds nearest-neighbor tiers from current sample positions.
    pub fn rebuild_neighbor_tiers(&mut self, first_tier_count: usize, second_tier_count: usize) {
        let (first, second) =
            build_nearest_neighbor_tiers(&self.positions(), first_tier_count, second_tier_count);
        self.first_tier_neighbors = first;
        self.second_tier_neighbors = second;
    }

    /// Validates sample-set consistency.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let count = self.samples.len();
        self.schema_id == MESH_SURFACE_SAMPLE_SET_SCHEMA_ID
            && !self.sample_set_id.trim().is_empty()
            && !self.surface_id.trim().is_empty()
            && count > 0
            && self.samples.iter().all(MeshSurfaceSample::is_valid)
            && self.first_tier_neighbors.len() == count
            && self.second_tier_neighbors.len() == count
            && self
                .first_tier_neighbors
                .iter()
                .enumerate()
                .all(|(origin, neighbors)| neighbor_list_is_valid(origin, count, neighbors))
            && self
                .second_tier_neighbors
                .iter()
                .enumerate()
                .all(|(origin, neighbors)| neighbor_list_is_valid(origin, count, neighbors))
    }
}

/// Status from one live mesh sampler update.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiveMeshSurfaceUpdateStatus {
    /// The first valid sample set was produced.
    Initialized,
    /// Existing samples were re-evaluated against new positions.
    Updated,
    /// Topology changed and samples were rebuilt.
    ResampledTopology,
    /// The input surface or sample config was invalid.
    InvalidSurface,
}

/// Summary from one live mesh sampler update.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct LiveMeshSurfaceUpdate {
    /// Update status.
    pub status: LiveMeshSurfaceUpdateStatus,
    /// Current topology key, if available.
    pub topology_key: Option<MeshSurfaceTopologyKey>,
    /// Current sample count.
    pub sample_count: usize,
}

/// Stable sampler for deformed mesh surfaces.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveMeshSurfaceSampler {
    config: MeshSurfaceSampleConfig,
    samples: Option<MeshSurfaceSampleSet>,
}

impl LiveMeshSurfaceSampler {
    /// Creates a live sampler.
    #[must_use]
    pub fn new(config: MeshSurfaceSampleConfig) -> Self {
        Self {
            config,
            samples: None,
        }
    }

    /// Returns samples, if initialized.
    #[must_use]
    pub fn samples(&self) -> Option<&MeshSurfaceSampleSet> {
        self.samples.as_ref()
    }

    /// Returns current topology key, if initialized.
    #[must_use]
    pub fn topology_key(&self) -> Option<&MeshSurfaceTopologyKey> {
        self.samples.as_ref().map(|samples| &samples.topology_key)
    }

    /// Updates samples from a mesh surface.
    #[must_use]
    pub fn update_from_surface(&mut self, surface: &TriangleMeshSurface) -> LiveMeshSurfaceUpdate {
        if surface.validate().is_err() || self.config.validate().is_err() {
            return self.update_summary(LiveMeshSurfaceUpdateStatus::InvalidSurface);
        }

        let next_key = surface.topology_key();
        let needs_resample = match self.samples.as_ref() {
            Some(samples) => samples.topology_key != next_key,
            None => true,
        };
        if needs_resample {
            let status = if self.samples.is_some() {
                LiveMeshSurfaceUpdateStatus::ResampledTopology
            } else {
                LiveMeshSurfaceUpdateStatus::Initialized
            };
            let Ok(samples) = sample_mesh_surface_points(surface, &self.config) else {
                return self.update_summary(LiveMeshSurfaceUpdateStatus::InvalidSurface);
            };
            self.samples = Some(samples);
            return self.update_summary(status);
        }

        if let Some(samples) = self.samples.as_mut() {
            if samples.update_positions_from_surface(surface).is_err() {
                return self.update_summary(LiveMeshSurfaceUpdateStatus::InvalidSurface);
            }
        }
        self.update_summary(LiveMeshSurfaceUpdateStatus::Updated)
    }

    fn update_summary(&self, status: LiveMeshSurfaceUpdateStatus) -> LiveMeshSurfaceUpdate {
        LiveMeshSurfaceUpdate {
            status,
            topology_key: self
                .samples
                .as_ref()
                .map(|samples| samples.topology_key.clone()),
            sample_count: self.samples.as_ref().map_or(0, MeshSurfaceSampleSet::len),
        }
    }
}

/// Configuration for cross-surface nearest-neighbor links.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshSurfaceCrossNeighborConfig {
    /// Neighbor count per source point.
    pub neighbors_per_point: usize,
    /// Maximum neighbor distance. Zero disables the distance gate.
    pub max_distance: f32,
}

impl Default for MeshSurfaceCrossNeighborConfig {
    fn default() -> Self {
        Self {
            neighbors_per_point: 1,
            max_distance: 0.0,
        }
    }
}

/// Bidirectional nearest-neighbor links between two sampled surfaces.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshSurfaceCrossNeighborhood {
    /// Schema identifier.
    pub schema_id: String,
    /// Links from A samples to B samples.
    pub a_to_b_neighbors: Vec<Vec<usize>>,
    /// Links from B samples to A samples.
    pub b_to_a_neighbors: Vec<Vec<usize>>,
}

impl MeshSurfaceCrossNeighborhood {
    /// Returns whether all neighbor targets are valid.
    #[must_use]
    pub fn is_valid(&self, a_count: usize, b_count: usize) -> bool {
        self.schema_id == MESH_SURFACE_CROSS_NEIGHBORHOOD_SCHEMA_ID
            && self.a_to_b_neighbors.len() == a_count
            && self.b_to_a_neighbors.len() == b_count
            && self
                .a_to_b_neighbors
                .iter()
                .all(|neighbors| neighbor_targets_are_valid(b_count, neighbors))
            && self
                .b_to_a_neighbors
                .iter()
                .all(|neighbors| neighbor_targets_are_valid(a_count, neighbors))
    }
}

/// Builds deterministic surface samples.
///
/// # Errors
///
/// Returns [`MatterMeshError`] when the surface or config is invalid.
pub fn sample_mesh_surface_points(
    surface: &TriangleMeshSurface,
    config: &MeshSurfaceSampleConfig,
) -> Result<MeshSurfaceSampleSet, MatterMeshError> {
    surface.validate()?;
    config.validate()?;

    let records = surface_triangle_records(surface);
    if records.is_empty() {
        return Err(MatterMeshError::InvalidSurface(
            "surface has no valid triangles",
        ));
    }
    let total_area = records.last().map_or(0.0, |record| record.cumulative_area);
    if !total_area.is_finite() || total_area <= 1.0e-9 {
        return Err(MatterMeshError::InvalidSurface(
            "surface area must be positive",
        ));
    }

    let mut per_triangle_counts = vec![0_usize; surface.triangles.len()];
    let mut samples = Vec::with_capacity(config.point_count);
    for sample_index in 0..config.point_count {
        let area_fraction = match config.pattern {
            MeshSurfaceSamplePattern::AreaStratified => {
                (sample_index as f32 + 0.5) / config.point_count as f32
            }
            MeshSurfaceSamplePattern::LowDiscrepancy => {
                halton01(sample_index + 1, 2, config.seed).clamp(1.0e-6, 0.999_999)
            }
        };
        let area_target = (area_fraction * total_area).min(total_area);
        let record_index = select_surface_triangle(&records, area_target);
        let record = records[record_index];
        let local_index = per_triangle_counts[record.triangle_index];
        per_triangle_counts[record.triangle_index] += 1;
        let barycentric = sample_barycentric(local_index, config.seed, record.triangle_index);
        let [a, b, c] = record.indices;
        let position = surface.positions[a] * barycentric[0]
            + surface.positions[b] * barycentric[1]
            + surface.positions[c] * barycentric[2];
        samples.push(MeshSurfaceSample {
            schema_id: MESH_SURFACE_SAMPLE_SCHEMA_ID.to_owned(),
            sample_id: format!("{}.sample.{sample_index:04}", config.sample_set_id),
            position,
            normal: record.normal,
            triangle_index: record.triangle_index,
            barycentric,
        });
    }
    let positions = samples
        .iter()
        .map(|sample| sample.position)
        .collect::<Vec<_>>();
    let (first_tier_neighbors, second_tier_neighbors) = build_nearest_neighbor_tiers(
        &positions,
        config.first_tier_neighbor_count,
        config.second_tier_neighbor_count,
    );
    let sample_set = MeshSurfaceSampleSet {
        schema_id: MESH_SURFACE_SAMPLE_SET_SCHEMA_ID.to_owned(),
        sample_set_id: config.sample_set_id.clone(),
        surface_id: surface.surface_id.clone(),
        topology_key: surface.topology_key(),
        samples,
        first_tier_neighbors,
        second_tier_neighbors,
    };
    if sample_set.is_valid() {
        Ok(sample_set)
    } else {
        Err(MatterMeshError::InvalidSurface(
            "generated sample set did not validate",
        ))
    }
}

/// Builds cross-neighborhood links between two sample sets.
#[must_use]
pub fn build_mesh_surface_cross_neighborhood(
    a_positions: &[Vec3],
    b_positions: &[Vec3],
    config: MeshSurfaceCrossNeighborConfig,
) -> MeshSurfaceCrossNeighborhood {
    let max_distance_squared = if config.max_distance.is_finite() && config.max_distance > 0.0 {
        config.max_distance * config.max_distance
    } else {
        f32::INFINITY
    };
    MeshSurfaceCrossNeighborhood {
        schema_id: MESH_SURFACE_CROSS_NEIGHBORHOOD_SCHEMA_ID.to_owned(),
        a_to_b_neighbors: build_cross_neighbor_lists(
            a_positions,
            b_positions,
            config.neighbors_per_point,
            max_distance_squared,
        ),
        b_to_a_neighbors: build_cross_neighbor_lists(
            b_positions,
            a_positions,
            config.neighbors_per_point,
            max_distance_squared,
        ),
    }
}

#[derive(Clone, Copy, Debug)]
struct SurfaceTriangleRecord {
    indices: [usize; 3],
    triangle_index: usize,
    normal: Vec3,
    cumulative_area: f32,
}

fn surface_triangle_records(surface: &TriangleMeshSurface) -> Vec<SurfaceTriangleRecord> {
    let mut records = Vec::new();
    let mut cumulative_area = 0.0;
    for (triangle_index, triangle) in surface.triangles.iter().copied().enumerate() {
        let Some(area) = triangle_area(&surface.positions, triangle) else {
            continue;
        };
        let [a, b, c] = triangle;
        let Ok(a) = usize::try_from(a) else {
            continue;
        };
        let Ok(b) = usize::try_from(b) else {
            continue;
        };
        let Ok(c) = usize::try_from(c) else {
            continue;
        };
        let normal = normalize_or(
            (surface.positions[b] - surface.positions[a])
                .cross(surface.positions[c] - surface.positions[a]),
            Vec3::new(0.0, 1.0, 0.0),
        );
        cumulative_area += area;
        records.push(SurfaceTriangleRecord {
            indices: [a, b, c],
            triangle_index,
            normal,
            cumulative_area,
        });
    }
    records
}

fn select_surface_triangle(records: &[SurfaceTriangleRecord], area_target: f32) -> usize {
    let mut low = 0;
    let mut high = records.len();
    while low < high {
        let mid = low + (high - low) / 2;
        if area_target <= records[mid].cumulative_area {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    low.min(records.len().saturating_sub(1))
}

fn sample_barycentric(local_index: usize, seed: u64, triangle_index: usize) -> [f32; 3] {
    let u = quasirandom01(local_index, seed, triangle_index, 0);
    let v = quasirandom01(local_index, seed, triangle_index, 1);
    let sqrt_u = u.sqrt();
    [1.0 - sqrt_u, sqrt_u * (1.0 - v), sqrt_u * v]
}

fn halton01(index: usize, base: u32, seed: u64) -> f32 {
    let base = base.max(2);
    let mut denominator = 1.0_f32;
    let mut value = 0.0_f32;
    let mut current = index + usize::try_from(seed % u64::from(base)).unwrap_or(0);
    while current > 0 {
        denominator *= base as f32;
        value += (current % base as usize) as f32 / denominator;
        current /= base as usize;
    }
    value.fract()
}

fn quasirandom01(local_index: usize, seed: u64, triangle_index: usize, axis: u32) -> f32 {
    let seed_mix = (seed as u32)
        ^ ((seed >> 32) as u32)
        ^ (triangle_index as u32).wrapping_mul(0x9E37_79B9)
        ^ axis.wrapping_mul(0x85EB_CA6B);
    let offset = hash01(seed_mix);
    let step = if axis == 0 { 0.618_034 } else { 0.754_877_7 };
    ((local_index as f32 + 0.5) * step + offset)
        .fract()
        .clamp(1.0e-6, 0.999_999)
}

fn hash01(value: u32) -> f32 {
    let mut x = value;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x & 0x00FF_FFFF) as f32 / 16_777_215.0
}

fn evaluate_surface_anchor(
    surface: &TriangleMeshSurface,
    triangle_index: usize,
    barycentric: [f32; 3],
) -> Option<(Vec3, Vec3)> {
    if !barycentric.iter().all(|value| value.is_finite()) {
        return None;
    }
    let triangle = *surface.triangles.get(triangle_index)?;
    let [a, b, c] = triangle;
    let a = usize::try_from(a).ok()?;
    let b = usize::try_from(b).ok()?;
    let c = usize::try_from(c).ok()?;
    let v0 = *surface.positions.get(a)?;
    let v1 = *surface.positions.get(b)?;
    let v2 = *surface.positions.get(c)?;
    if !v0.is_finite() || !v1.is_finite() || !v2.is_finite() {
        return None;
    }
    let normal = normalize_or((v1 - v0).cross(v2 - v0), Vec3::ZERO);
    if normal == Vec3::ZERO {
        return None;
    }
    Some((
        v0 * barycentric[0] + v1 * barycentric[1] + v2 * barycentric[2],
        normal,
    ))
}

fn build_nearest_neighbor_tiers(
    positions: &[Vec3],
    first_tier_count: usize,
    second_tier_count: usize,
) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let count = positions.len();
    let first_tier_count = first_tier_count.min(count.saturating_sub(1));
    let second_tier_count =
        second_tier_count.min(count.saturating_sub(1).saturating_sub(first_tier_count));
    let mut first = Vec::with_capacity(count);
    let mut second = Vec::with_capacity(count);
    for origin in 0..count {
        let mut distances = Vec::with_capacity(count.saturating_sub(1));
        for candidate in 0..count {
            if origin == candidate {
                continue;
            }
            let distance = positions[origin].distance_squared(positions[candidate]);
            let distance = if distance.is_finite() {
                distance
            } else {
                f32::INFINITY
            };
            distances.push((distance, candidate));
        }
        distances.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));
        first.push(
            distances
                .iter()
                .take(first_tier_count)
                .map(|(_, index)| *index)
                .collect(),
        );
        second.push(
            distances
                .iter()
                .skip(first_tier_count)
                .take(second_tier_count)
                .map(|(_, index)| *index)
                .collect(),
        );
    }
    (first, second)
}

fn build_cross_neighbor_lists(
    source_positions: &[Vec3],
    target_positions: &[Vec3],
    neighbors_per_point: usize,
    max_distance_squared: f32,
) -> Vec<Vec<usize>> {
    if source_positions.is_empty() {
        return Vec::new();
    }
    if target_positions.is_empty() || neighbors_per_point == 0 {
        return vec![Vec::new(); source_positions.len()];
    }
    source_positions
        .iter()
        .map(|source| {
            let mut distances = target_positions
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(target_index, target)| {
                    let distance = source.distance_squared(target);
                    (distance.is_finite() && distance <= max_distance_squared)
                        .then_some((distance, target_index))
                })
                .collect::<Vec<_>>();
            distances.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));
            distances
                .iter()
                .take(neighbors_per_point)
                .map(|(_, index)| *index)
                .collect()
        })
        .collect()
}

fn neighbor_list_is_valid(origin: usize, count: usize, neighbors: &[usize]) -> bool {
    let mut seen = Vec::with_capacity(neighbors.len());
    neighbors.iter().all(|neighbor| {
        *neighbor < count && *neighbor != origin && push_unique(&mut seen, *neighbor)
    })
}

fn neighbor_targets_are_valid(count: usize, neighbors: &[usize]) -> bool {
    let mut seen = Vec::with_capacity(neighbors.len());
    neighbors
        .iter()
        .all(|neighbor| *neighbor < count && push_unique(&mut seen, *neighbor))
}

fn push_unique(values: &mut Vec<usize>, value: usize) -> bool {
    if values.contains(&value) {
        false
    } else {
        values.push(value);
        true
    }
}
