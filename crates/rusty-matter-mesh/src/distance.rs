use core::cmp::Ordering;

use rusty_matter_model::Vec3;

use crate::math::{closest_point_on_triangle, normalize_or};
use crate::{MatterMeshError, MeshSurfaceTopologyKey, TriangleMeshSurface};

/// Configuration for an accelerated surface-distance sampler.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceDistanceSamplerConfig {
    /// Maximum triangles kept in one leaf node.
    pub leaf_triangle_count: usize,
    /// Maximum recursive split depth.
    pub max_depth: usize,
}

impl Default for SurfaceDistanceSamplerConfig {
    fn default() -> Self {
        Self {
            leaf_triangle_count: 8,
            max_depth: 32,
        }
    }
}

impl SurfaceDistanceSamplerConfig {
    /// Returns a validated leaf triangle count.
    #[must_use]
    pub fn effective_leaf_triangle_count(&self) -> usize {
        self.leaf_triangle_count.clamp(1, 64)
    }

    /// Returns a validated maximum tree depth.
    #[must_use]
    pub fn effective_max_depth(&self) -> usize {
        self.max_depth.clamp(1, 64)
    }
}

/// Diagnostics for one accelerated surface-distance query.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SurfaceDistanceQueryDiagnostics {
    /// Number of BVH nodes whose bounds were tested.
    pub node_tests: usize,
    /// Number of leaf nodes visited.
    pub leaf_tests: usize,
    /// Number of exact triangle closest-point tests.
    pub triangle_tests: usize,
}

/// Closest surface sample from a surface-distance query.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceDistanceSample {
    /// Closest point on the mesh.
    pub point: Vec3,
    /// Triangle normal at the closest point.
    pub normal: Vec3,
    /// Euclidean distance from the query point to the mesh.
    pub distance: f32,
    /// Source triangle index.
    pub triangle_index: usize,
    /// Query diagnostics proving the accelerated path was used.
    pub diagnostics: SurfaceDistanceQueryDiagnostics,
}

/// Build-time diagnostics for a surface-distance sampler.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceDistanceSamplerStats {
    /// Number of source triangles.
    pub triangle_count: usize,
    /// Number of BVH nodes.
    pub node_count: usize,
    /// Number of leaf nodes.
    pub leaf_count: usize,
    /// Maximum leaf depth reached while building.
    pub max_depth: usize,
    /// Effective leaf triangle budget used for the tree.
    pub leaf_triangle_count: usize,
}

/// Accelerated closest-surface sampler over one triangle mesh topology.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceDistanceSampler {
    topology_key: MeshSurfaceTopologyKey,
    config: SurfaceDistanceSamplerConfig,
    triangles: Vec<DistanceTriangle>,
    triangle_order: Vec<usize>,
    nodes: Vec<DistanceNode>,
    stats: SurfaceDistanceSamplerStats,
}

impl SurfaceDistanceSampler {
    /// Builds an accelerated sampler from a triangle mesh surface.
    ///
    /// # Errors
    ///
    /// Returns [`MatterMeshError`] when the surface is invalid.
    pub fn from_surface(
        surface: &TriangleMeshSurface,
        config: SurfaceDistanceSamplerConfig,
    ) -> Result<Self, MatterMeshError> {
        surface.validate()?;
        let triangles = surface
            .triangles
            .iter()
            .copied()
            .enumerate()
            .map(|(triangle_index, triangle)| {
                DistanceTriangle::from_surface_triangle(surface, triangle_index, triangle)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if triangles.is_empty() {
            return Err(MatterMeshError::InvalidSurface(
                "distance sampler requires triangles",
            ));
        }

        let mut triangle_order = (0..triangles.len()).collect::<Vec<_>>();
        let mut nodes = Vec::new();
        let mut build_stats = DistanceBuildStats::default();
        let effective_config = SurfaceDistanceSamplerConfig {
            leaf_triangle_count: config.effective_leaf_triangle_count(),
            max_depth: config.effective_max_depth(),
        };
        build_node(
            &triangles,
            &mut triangle_order,
            &mut nodes,
            0,
            triangles.len(),
            0,
            &effective_config,
            &mut build_stats,
        );

        let stats = SurfaceDistanceSamplerStats {
            triangle_count: triangles.len(),
            node_count: nodes.len(),
            leaf_count: build_stats.leaf_count,
            max_depth: build_stats.max_depth,
            leaf_triangle_count: effective_config.leaf_triangle_count,
        };

        Ok(Self {
            topology_key: surface.topology_key(),
            config: effective_config,
            triangles,
            triangle_order,
            nodes,
            stats,
        })
    }

    /// Returns the topology key this sampler was built for.
    #[must_use]
    pub fn topology_key(&self) -> &MeshSurfaceTopologyKey {
        &self.topology_key
    }

    /// Returns build-time sampler statistics.
    #[must_use]
    pub fn stats(&self) -> &SurfaceDistanceSamplerStats {
        &self.stats
    }

    /// Returns the effective sampler configuration.
    #[must_use]
    pub fn config(&self) -> &SurfaceDistanceSamplerConfig {
        &self.config
    }

    /// Samples the closest mesh point to `point`.
    #[must_use]
    pub fn sample(&self, point: Vec3) -> Option<SurfaceDistanceSample> {
        if !point.is_finite() || self.nodes.is_empty() {
            return None;
        }

        let mut diagnostics = SurfaceDistanceQueryDiagnostics::default();
        let mut best_distance_squared = f32::INFINITY;
        let mut best: Option<SurfaceDistanceSample> = None;
        let mut stack = vec![0_usize];

        while let Some(node_index) = stack.pop() {
            let node = self.nodes.get(node_index)?;
            diagnostics.node_tests += 1;
            if node.bounds.distance_squared(point) > best_distance_squared {
                continue;
            }

            match node.kind {
                DistanceNodeKind::Leaf { start, end } => {
                    diagnostics.leaf_tests += 1;
                    for order_index in start..end {
                        let triangle_index = *self.triangle_order.get(order_index)?;
                        let triangle = self.triangles.get(triangle_index)?;
                        diagnostics.triangle_tests += 1;
                        let closest =
                            closest_point_on_triangle(point, triangle.a, triangle.b, triangle.c);
                        let distance_squared = point.distance_squared(closest);
                        if distance_squared < best_distance_squared {
                            best_distance_squared = distance_squared;
                            best = Some(SurfaceDistanceSample {
                                point: closest,
                                normal: triangle.normal,
                                distance: distance_squared.sqrt(),
                                triangle_index: triangle.triangle_index,
                                diagnostics,
                            });
                        }
                    }
                }
                DistanceNodeKind::Branch { left, right } => {
                    let left_node = self.nodes.get(left)?;
                    let right_node = self.nodes.get(right)?;
                    let left_distance = left_node.bounds.distance_squared(point);
                    let right_distance = right_node.bounds.distance_squared(point);
                    if left_distance <= right_distance {
                        push_if_possible(&mut stack, right, right_distance, best_distance_squared);
                        push_if_possible(&mut stack, left, left_distance, best_distance_squared);
                    } else {
                        push_if_possible(&mut stack, left, left_distance, best_distance_squared);
                        push_if_possible(&mut stack, right, right_distance, best_distance_squared);
                    }
                }
            }
        }

        best.map(|mut sample| {
            sample.diagnostics = diagnostics;
            sample
        })
    }
}

fn push_if_possible(
    stack: &mut Vec<usize>,
    node_index: usize,
    node_distance_squared: f32,
    best_distance_squared: f32,
) {
    if node_distance_squared <= best_distance_squared {
        stack.push(node_index);
    }
}

fn build_node(
    triangles: &[DistanceTriangle],
    triangle_order: &mut [usize],
    nodes: &mut Vec<DistanceNode>,
    start: usize,
    end: usize,
    depth: usize,
    config: &SurfaceDistanceSamplerConfig,
    stats: &mut DistanceBuildStats,
) -> usize {
    let bounds = bounds_for_range(triangles, triangle_order, start, end);
    let node_index = nodes.len();
    nodes.push(DistanceNode {
        bounds,
        kind: DistanceNodeKind::Leaf { start, end },
    });

    let count = end.saturating_sub(start);
    if count <= config.leaf_triangle_count || depth >= config.max_depth {
        stats.leaf_count += 1;
        stats.max_depth = stats.max_depth.max(depth);
        return node_index;
    }

    let axis = bounds.largest_axis();
    triangle_order[start..end].sort_by(|left, right| {
        triangles[*left]
            .centroid
            .axis(axis)
            .partial_cmp(&triangles[*right].centroid.axis(axis))
            .unwrap_or(Ordering::Equal)
    });
    let midpoint = start + count / 2;
    if midpoint == start || midpoint == end {
        stats.leaf_count += 1;
        stats.max_depth = stats.max_depth.max(depth);
        return node_index;
    }

    let left = build_node(
        triangles,
        triangle_order,
        nodes,
        start,
        midpoint,
        depth + 1,
        config,
        stats,
    );
    let right = build_node(
        triangles,
        triangle_order,
        nodes,
        midpoint,
        end,
        depth + 1,
        config,
        stats,
    );
    nodes[node_index].kind = DistanceNodeKind::Branch { left, right };
    node_index
}

fn bounds_for_range(
    triangles: &[DistanceTriangle],
    triangle_order: &[usize],
    start: usize,
    end: usize,
) -> DistanceBounds {
    let mut bounds = DistanceBounds::empty();
    for order_index in start..end {
        if let Some(triangle) = triangle_order
            .get(order_index)
            .and_then(|triangle_index| triangles.get(*triangle_index))
        {
            bounds = bounds.union(triangle.bounds);
        }
    }
    bounds
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DistanceTriangle {
    triangle_index: usize,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    normal: Vec3,
    centroid: Vec3,
    bounds: DistanceBounds,
}

impl DistanceTriangle {
    fn from_surface_triangle(
        surface: &TriangleMeshSurface,
        triangle_index: usize,
        triangle: [u32; 3],
    ) -> Result<Self, MatterMeshError> {
        let [a, b, c] = triangle;
        let a = usize::try_from(a).map_err(|_| MatterMeshError::IndexOutOfRange {
            triangle_index,
            vertex_index: triangle[0],
            vertex_count: surface.positions.len(),
        })?;
        let b = usize::try_from(b).map_err(|_| MatterMeshError::IndexOutOfRange {
            triangle_index,
            vertex_index: triangle[1],
            vertex_count: surface.positions.len(),
        })?;
        let c = usize::try_from(c).map_err(|_| MatterMeshError::IndexOutOfRange {
            triangle_index,
            vertex_index: triangle[2],
            vertex_count: surface.positions.len(),
        })?;
        let Some(a) = surface.positions.get(a).copied() else {
            return Err(MatterMeshError::IndexOutOfRange {
                triangle_index,
                vertex_index: triangle[0],
                vertex_count: surface.positions.len(),
            });
        };
        let Some(b) = surface.positions.get(b).copied() else {
            return Err(MatterMeshError::IndexOutOfRange {
                triangle_index,
                vertex_index: triangle[1],
                vertex_count: surface.positions.len(),
            });
        };
        let Some(c) = surface.positions.get(c).copied() else {
            return Err(MatterMeshError::IndexOutOfRange {
                triangle_index,
                vertex_index: triangle[2],
                vertex_count: surface.positions.len(),
            });
        };
        let normal = normalize_or((b - a).cross(c - a), Vec3::new(0.0, 1.0, 0.0));
        let centroid = (a + b + c) / 3.0;
        Ok(Self {
            triangle_index,
            a,
            b,
            c,
            normal,
            centroid,
            bounds: DistanceBounds::from_triangle(a, b, c),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DistanceNode {
    bounds: DistanceBounds,
    kind: DistanceNodeKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DistanceNodeKind {
    Leaf { start: usize, end: usize },
    Branch { left: usize, right: usize },
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DistanceBounds {
    min: Vec3,
    max: Vec3,
}

impl DistanceBounds {
    fn empty() -> Self {
        Self {
            min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    fn from_triangle(a: Vec3, b: Vec3, c: Vec3) -> Self {
        Self {
            min: a.min(b).min(c),
            max: a.max(b).max(c),
        }
    }

    fn union(self, rhs: Self) -> Self {
        Self {
            min: self.min.min(rhs.min),
            max: self.max.max(rhs.max),
        }
    }

    fn extent(self) -> Vec3 {
        self.max - self.min
    }

    fn largest_axis(self) -> Axis {
        let extent = self.extent();
        if extent.x >= extent.y && extent.x >= extent.z {
            Axis::X
        } else if extent.y >= extent.z {
            Axis::Y
        } else {
            Axis::Z
        }
    }

    fn distance_squared(self, point: Vec3) -> f32 {
        let dx = if point.x < self.min.x {
            self.min.x - point.x
        } else if point.x > self.max.x {
            point.x - self.max.x
        } else {
            0.0
        };
        let dy = if point.y < self.min.y {
            self.min.y - point.y
        } else if point.y > self.max.y {
            point.y - self.max.y
        } else {
            0.0
        };
        let dz = if point.z < self.min.z {
            self.min.z - point.z
        } else if point.z > self.max.z {
            point.z - self.max.z
        } else {
            0.0
        };
        dx.mul_add(dx, dy.mul_add(dy, dz * dz))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Axis {
    X,
    Y,
    Z,
}

trait AxisValue {
    fn axis(self, axis: Axis) -> f32;
}

impl AxisValue for Vec3 {
    fn axis(self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DistanceBuildStats {
    leaf_count: usize,
    max_depth: usize,
}
