use js_sys::{Float32Array, Uint32Array};
use rusty_matter_mesh::{
    SurfaceDistanceSampler, SurfaceDistanceSamplerConfig, TriangleMeshSurface,
};
use rusty_matter_model::Vec3;
use rusty_matter_particles::{
    SurfaceParticleRuntime, SurfaceParticleRuntimeConfig, SurfaceParticleStepDiagnostics,
};
use wasm_bindgen::prelude::*;

/// Accelerated Matter hand-mesh distance runtime exported to browser Wasm.
#[wasm_bindgen]
pub struct HandMeshDistanceRuntime {
    surface: TriangleMeshSurface,
    sampler: SurfaceDistanceSampler,
}

#[wasm_bindgen]
impl HandMeshDistanceRuntime {
    /// Builds a runtime sampler from flat xyz positions and u32 triangle indices.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when the buffers are malformed or the Matter
    /// surface validation rejects the mesh.
    #[wasm_bindgen(constructor)]
    pub fn new(
        positions: &Float32Array,
        triangles: &Uint32Array,
        leaf_triangle_count: usize,
    ) -> Result<Self, JsValue> {
        let positions = positions.to_vec();
        let triangles = triangles.to_vec();
        let surface = TriangleMeshSurface::new(
            "mesh.browser_hand_runtime",
            decode_positions(&positions)?,
            decode_triangles(&triangles)?,
        );
        let sampler = surface
            .distance_sampler(SurfaceDistanceSamplerConfig {
                leaf_triangle_count,
                ..SurfaceDistanceSamplerConfig::default()
            })
            .map_err(|error| JsValue::from_str(&error.to_string()))?;

        Ok(Self { surface, sampler })
    }

    /// Samples the closest mesh surface point.
    ///
    /// The returned `Float32Array` layout is:
    /// `[hit, px, py, pz, nx, ny, nz, distance, triangle, nodes, leaves, triangles]`.
    #[must_use]
    pub fn sample(&self, x: f32, y: f32, z: f32) -> Float32Array {
        let Some(sample) = self.sampler.sample(Vec3::new(x, y, z)) else {
            return Float32Array::from(&[0.0_f32][..]);
        };
        Float32Array::from(
            &[
                1.0,
                sample.point.x,
                sample.point.y,
                sample.point.z,
                sample.normal.x,
                sample.normal.y,
                sample.normal.z,
                sample.distance,
                sample.triangle_index as f32,
                sample.diagnostics.node_tests as f32,
                sample.diagnostics.leaf_tests as f32,
                sample.diagnostics.triangle_tests as f32,
            ][..],
        )
    }

    /// Returns sampler build statistics.
    ///
    /// The returned `Uint32Array` layout is:
    /// `[vertices, triangles, bvh_nodes, bvh_leaves, max_depth, leaf_triangle_count]`.
    #[must_use]
    pub fn stats(&self) -> Uint32Array {
        let stats = self.sampler.stats();
        Uint32Array::from(
            &[
                usize_to_u32(self.surface.vertex_count()),
                usize_to_u32(stats.triangle_count),
                usize_to_u32(stats.node_count),
                usize_to_u32(stats.leaf_count),
                usize_to_u32(stats.max_depth),
                usize_to_u32(stats.leaf_triangle_count),
            ][..],
        )
    }
}

/// Matter-owned particle runtime exported to browser Wasm.
///
/// The browser owns controls, drawing, and visual trails. This runtime owns the
/// deterministic particle seed and fixed-step surface-attraction semantics.
#[wasm_bindgen]
pub struct HandMeshParticleRuntime {
    runtime: SurfaceParticleRuntime,
    last_distances: Vec<f32>,
    last_step: SurfaceParticleStepDiagnostics,
}

#[wasm_bindgen]
impl HandMeshParticleRuntime {
    /// Creates an empty particle runtime.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error only if the Matter runtime config is invalid.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let runtime = SurfaceParticleRuntime::new(
            "particles.handmesh_wasm",
            SurfaceParticleRuntimeConfig::default(),
        )
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
        Ok(Self {
            runtime,
            last_distances: Vec::new(),
            last_step: SurfaceParticleStepDiagnostics::default(),
        })
    }

    /// Resets particles to a deterministic random sphere.
    ///
    /// Arguments are center xyz, particle count, cloud radius, particle radius,
    /// source surface radius, and seed.
    #[allow(clippy::too_many_arguments)]
    pub fn reset_random_sphere(
        &mut self,
        center_x: f32,
        center_y: f32,
        center_z: f32,
        count: usize,
        cloud_radius: f32,
        particle_radius: f32,
        surface_radius: f32,
        seed: u32,
    ) -> Result<(), JsValue> {
        self.runtime
            .reset_random_sphere(
                Vec3::new(center_x, center_y, center_z),
                count,
                cloud_radius,
                particle_radius,
                surface_radius,
                seed,
            )
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.last_distances = vec![f32::NAN; self.runtime.particles().len()];
        self.last_step = SurfaceParticleStepDiagnostics {
            particle_count: self.runtime.particles().len(),
            ..SurfaceParticleStepDiagnostics::default()
        };
        Ok(())
    }

    /// Steps particles against a Matter mesh-distance runtime.
    ///
    /// The returned `Float32Array` layout is documented by `stats()`.
    #[allow(clippy::too_many_arguments)]
    pub fn step_against_surface(
        &mut self,
        distance_runtime: &HandMeshDistanceRuntime,
        sequence_center_x: f32,
        sequence_center_y: f32,
        sequence_center_z: f32,
        sequence_cloud_radius: f32,
        surface_radius: f32,
        delta_seconds: f32,
    ) -> Float32Array {
        let mut diagnostics = self.runtime.step_against_surface(
            &distance_runtime.sampler,
            surface_radius,
            Vec3::new(sequence_center_x, sequence_center_y, sequence_center_z),
            sequence_cloud_radius,
            delta_seconds,
        );
        let render_diagnostics = self.refresh_distances(&distance_runtime.sampler);
        diagnostics.closest_samples += render_diagnostics.closest_samples;
        diagnostics.surface_node_tests += render_diagnostics.surface_node_tests;
        diagnostics.surface_leaf_tests += render_diagnostics.surface_leaf_tests;
        diagnostics.surface_triangle_tests += render_diagnostics.surface_triangle_tests;
        self.last_step = diagnostics;
        self.stats()
    }

    /// Returns the latest particle snapshot.
    ///
    /// The returned `Float32Array` layout is 10 floats per particle:
    /// `[x, y, z, vx, vy, vz, radius, speed, age_seconds, last_distance]`.
    #[must_use]
    pub fn snapshot(&self) -> Float32Array {
        let mut values = Vec::with_capacity(self.runtime.particles().len() * 10);
        for (index, particle) in self.runtime.particles().particles.iter().enumerate() {
            values.extend_from_slice(&[
                particle.position.x,
                particle.position.y,
                particle.position.z,
                particle.velocity.x,
                particle.velocity.y,
                particle.velocity.z,
                particle.radius,
                particle.velocity.length(),
                particle.age_seconds,
                *self.last_distances.get(index).unwrap_or(&f32::NAN),
            ]);
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns the latest step diagnostics.
    ///
    /// The returned `Float32Array` layout is:
    /// `[particle_count, substeps, closest_samples, affected_particles,
    /// rejected_particles, clamped_particles, node_tests, leaf_tests,
    /// triangle_tests, max_speed]`.
    #[must_use]
    pub fn stats(&self) -> Float32Array {
        Float32Array::from(
            &[
                self.last_step.particle_count as f32,
                self.last_step.substeps as f32,
                self.last_step.closest_samples as f32,
                self.last_step.affected_particles as f32,
                self.last_step.rejected_particles as f32,
                self.last_step.clamped_particles as f32,
                self.last_step.surface_node_tests as f32,
                self.last_step.surface_leaf_tests as f32,
                self.last_step.surface_triangle_tests as f32,
                self.last_step.max_speed,
            ][..],
        )
    }

    /// Returns particle count.
    #[must_use]
    pub fn particle_count(&self) -> usize {
        self.runtime.particles().len()
    }
}

impl HandMeshParticleRuntime {
    fn refresh_distances(
        &mut self,
        sampler: &SurfaceDistanceSampler,
    ) -> SurfaceParticleStepDiagnostics {
        let mut diagnostics = SurfaceParticleStepDiagnostics {
            particle_count: self.runtime.particles().len(),
            ..SurfaceParticleStepDiagnostics::default()
        };
        self.last_distances.clear();
        self.last_distances
            .reserve(self.runtime.particles().particles.len());
        for particle in &self.runtime.particles().particles {
            if let Some(sample) = sampler.sample(particle.position) {
                self.last_distances.push(sample.distance);
                diagnostics.closest_samples += 1;
                diagnostics.surface_node_tests += sample.diagnostics.node_tests;
                diagnostics.surface_leaf_tests += sample.diagnostics.leaf_tests;
                diagnostics.surface_triangle_tests += sample.diagnostics.triangle_tests;
            } else {
                self.last_distances.push(f32::NAN);
                diagnostics.rejected_particles += 1;
            }
        }
        diagnostics
    }
}

fn decode_positions(values: &[f32]) -> Result<Vec<Vec3>, JsValue> {
    if values.is_empty() || values.len() % 3 != 0 {
        return Err(JsValue::from_str(
            "positions must contain a non-empty multiple of 3 values",
        ));
    }
    values
        .chunks_exact(3)
        .enumerate()
        .map(|(index, chunk)| {
            let position = Vec3::new(chunk[0], chunk[1], chunk[2]);
            if position.is_finite() {
                Ok(position)
            } else {
                Err(JsValue::from_str(&format!(
                    "position {index} contains non-finite values"
                )))
            }
        })
        .collect()
}

fn decode_triangles(values: &[u32]) -> Result<Vec<[u32; 3]>, JsValue> {
    if values.is_empty() || values.len() % 3 != 0 {
        return Err(JsValue::from_str(
            "triangles must contain a non-empty multiple of 3 values",
        ));
    }
    Ok(values
        .chunks_exact(3)
        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
        .collect())
}

fn usize_to_u32(value: usize) -> u32 {
    value.try_into().unwrap_or(u32::MAX)
}
