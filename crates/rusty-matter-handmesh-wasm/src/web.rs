use js_sys::{Float32Array, Uint32Array};
use rusty_matter_mesh::{
    SurfaceDistanceSampler, SurfaceDistanceSamplerConfig, TriangleMeshSurface,
};
use rusty_matter_model::Vec3;
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
