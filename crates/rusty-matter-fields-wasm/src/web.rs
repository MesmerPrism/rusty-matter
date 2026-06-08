use js_sys::{Float32Array, Uint32Array};
use rusty_matter_fields::{
    SurfaceFieldPerturbation, SurfaceFieldPerturbationEffect, SurfaceFieldRuntime,
    SurfaceFieldRuntimeConfig, SurfaceFieldState, SurfaceFieldStepDiagnostics,
    SurfaceFieldSubstrate, SurfaceScalarField, SurfaceScalarFieldKind, SurfaceVectorField,
    SurfaceVectorFieldKind,
};
use rusty_matter_mesh::{MeshSurfaceSampleConfig, MeshSurfaceSamplePattern, TriangleMeshSurface};
use rusty_matter_model::Vec3;
use wasm_bindgen::prelude::*;

/// Realtime Matter surface-field runtime exported to browser Wasm.
///
/// The browser owns controls and drawing. This runtime owns the substrate,
/// state, perturbation schedule, sparse neighbor plan, and fixed-step updates.
#[wasm_bindgen]
pub struct SurfaceFieldRealtimeRuntime {
    runtime: SurfaceFieldRuntime,
    substrate: SurfaceFieldSubstrate,
    initial_state: SurfaceFieldState,
    state: SurfaceFieldState,
    perturbations: Vec<SurfaceFieldPerturbation>,
    step_index: u32,
    last_step: SurfaceFieldStepDiagnostics,
}

#[wasm_bindgen]
impl SurfaceFieldRealtimeRuntime {
    /// Creates the deterministic unit-square realtime demo runtime.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when the Matter substrate, fields, runtime
    /// config, or perturbation contracts fail validation.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let (substrate, state, perturbations) = dynamic_contracts()?;
        let runtime = SurfaceFieldRuntime::new(SurfaceFieldRuntimeConfig {
            config_id: "fields.runtime.wasm_dynamic".to_owned(),
            fixed_step_seconds: 1.0 / 30.0,
            max_steps_per_run: 240,
            scalar_diffusion_rate: 2.8,
            scalar_decay_rate: 0.18,
            second_tier_coupling_weight: 0.42,
            vector_alignment_rate: 3.2,
            vector_gradient_rate: 1.9,
            ..SurfaceFieldRuntimeConfig::default()
        })
        .map_err(to_js_error)?;
        Ok(Self {
            runtime,
            substrate,
            initial_state: state.clone(),
            state,
            perturbations,
            step_index: 0,
            last_step: SurfaceFieldStepDiagnostics::empty(0),
        })
    }

    /// Resets the state and timestep to the deterministic initial condition.
    pub fn reset(&mut self) {
        self.state = self.initial_state.clone();
        self.step_index = 0;
        self.last_step = SurfaceFieldStepDiagnostics::empty(0);
    }

    /// Advances one or more Matter fixed steps and returns runtime stats.
    ///
    /// The returned `Float32Array` layout matches `stats()`. Each call is
    /// bounded to at most eight fixed steps so a browser frame cannot request an
    /// unbounded simulation burst.
    pub fn step(&mut self, requested_steps: u32) -> Result<Float32Array, JsValue> {
        let steps = requested_steps.clamp(1, 8);
        for _ in 0..steps {
            self.last_step = self
                .runtime
                .step_fixed(
                    &self.substrate,
                    &mut self.state,
                    &self.perturbations,
                    self.step_index,
                )
                .map_err(to_js_error)?;
            self.step_index += 1;
            self.state.time_seconds =
                self.step_index as f32 * self.runtime.config().fixed_step_seconds;
        }
        Ok(self.stats())
    }

    /// Returns static node topology.
    ///
    /// The returned `Float32Array` layout is six floats per node:
    /// `[x, y, z, nx, ny, nz]`.
    #[must_use]
    pub fn nodes(&self) -> Float32Array {
        let mut values = Vec::with_capacity(self.substrate.node_count() * 6);
        for node in &self.substrate.nodes {
            values.extend_from_slice(&[
                node.position.x,
                node.position.y,
                node.position.z,
                node.normal.x,
                node.normal.y,
                node.normal.z,
            ]);
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns static sparse neighbor edges.
    ///
    /// The returned `Uint32Array` layout is three unsigned integers per edge:
    /// `[from, to, tier]`.
    #[must_use]
    pub fn edges(&self) -> Uint32Array {
        let mut values = Vec::with_capacity(
            self.substrate.first_tier_edge_count() + self.substrate.second_tier_edge_count(),
        );
        for node in &self.substrate.nodes {
            values.extend(
                node.first_tier_neighbors
                    .iter()
                    .copied()
                    .flat_map(|to| [usize_to_u32(node.node_index), usize_to_u32(to), 1]),
            );
            values.extend(
                node.second_tier_neighbors
                    .iter()
                    .copied()
                    .flat_map(|to| [usize_to_u32(node.node_index), usize_to_u32(to), 2]),
            );
        }
        Uint32Array::from(values.as_slice())
    }

    /// Returns perturbation region metadata.
    ///
    /// The returned `Uint32Array` layout is four unsigned integers per region:
    /// `[effect_code, target_code, node_offset, node_count]`.
    #[must_use]
    pub fn region_metadata(&self) -> Uint32Array {
        let mut offset = 0_u32;
        let mut values = Vec::with_capacity(self.perturbations.len() * 4);
        for perturbation in &self.perturbations {
            let len = usize_to_u32(perturbation.node_indices.len());
            values.extend_from_slice(&[
                effect_code(&perturbation.effect),
                target_code(perturbation.target_field_id.as_deref()),
                offset,
                len,
            ]);
            offset = offset.saturating_add(len);
        }
        Uint32Array::from(values.as_slice())
    }

    /// Returns flattened perturbation region node indices.
    #[must_use]
    pub fn region_nodes(&self) -> Uint32Array {
        let values = self
            .perturbations
            .iter()
            .flat_map(|perturbation| perturbation.node_indices.iter().copied())
            .map(usize_to_u32)
            .collect::<Vec<_>>();
        Uint32Array::from(values.as_slice())
    }

    /// Returns current scalar and vector values.
    ///
    /// The returned `Float32Array` layout is six floats per node:
    /// `[vmem_like, wound_signal, morphogen, polarity_x, polarity_y, polarity_z]`.
    #[must_use]
    pub fn snapshot(&self) -> Float32Array {
        let vmem = scalar_values(&self.state, "field.vmem_like");
        let wound = scalar_values(&self.state, "field.wound_signal");
        let morphogen = scalar_values(&self.state, "field.morphogen");
        let polarity = vector_values(&self.state, "field.polarity");
        let mut values = Vec::with_capacity(self.substrate.node_count() * 6);
        for node_index in 0..self.substrate.node_count() {
            let vector = polarity[node_index];
            values.extend_from_slice(&[
                vmem[node_index],
                wound[node_index],
                morphogen[node_index],
                vector.x,
                vector.y,
                vector.z,
            ]);
        }
        Float32Array::from(values.as_slice())
    }

    /// Returns the latest runtime stats.
    ///
    /// The returned `Float32Array` layout is:
    /// `[step, time_seconds, node_count, edge_count, scalar_fields,
    /// vector_fields, active_perturbations, neighbor_links_visited,
    /// clamped_scalars, clamped_vectors, fixed_step_seconds]`.
    #[must_use]
    pub fn stats(&self) -> Float32Array {
        Float32Array::from(
            &[
                self.step_index as f32,
                self.state.time_seconds,
                self.substrate.node_count() as f32,
                (self.substrate.first_tier_edge_count() + self.substrate.second_tier_edge_count())
                    as f32,
                self.state.scalar_fields.len() as f32,
                self.state.vector_fields.len() as f32,
                self.last_step.active_perturbations as f32,
                self.last_step.neighbor_links_visited as f32,
                self.last_step.clamped_scalars as f32,
                self.last_step.clamped_vectors as f32,
                self.runtime.config().fixed_step_seconds,
            ][..],
        )
    }

    /// Returns substrate node count.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.substrate.node_count()
    }
}

fn dynamic_contracts() -> Result<
    (
        SurfaceFieldSubstrate,
        SurfaceFieldState,
        Vec<SurfaceFieldPerturbation>,
    ),
    JsValue,
> {
    let surface = unit_square_surface();
    let samples = surface
        .sample_points(&MeshSurfaceSampleConfig {
            sample_config_id: "mesh.surface_sample.field_wasm_dynamic".to_owned(),
            sample_set_id: "mesh.surface_samples.field_wasm_dynamic".to_owned(),
            point_count: 64,
            first_tier_neighbor_count: 4,
            second_tier_neighbor_count: 4,
            seed: 65_537,
            pattern: MeshSurfaceSamplePattern::LowDiscrepancy,
            ..MeshSurfaceSampleConfig::default()
        })
        .map_err(to_js_error)?;
    let substrate = SurfaceFieldSubstrate::from_sample_set(
        "fields.substrate.wasm_unit_square_dynamic",
        &samples,
    )
    .map_err(to_js_error)?;
    let node_count = substrate.node_count();
    let vmem_values = substrate
        .nodes
        .iter()
        .map(|node| 0.16 + (node.position.y - 0.5) * 0.18)
        .collect::<Vec<_>>();
    let morphogen_values = substrate
        .nodes
        .iter()
        .map(|node| node.position.x.clamp(0.0, 1.0))
        .collect::<Vec<_>>();
    let polarity_vectors = substrate
        .nodes
        .iter()
        .map(|node| normalize(Vec3::new(1.0, (node.position.y - 0.5) * 0.45, 0.0)))
        .collect::<Vec<_>>();
    let state = SurfaceFieldState::new(
        "fields.state.wasm_unit_square_dynamic",
        &substrate,
        vec![
            SurfaceScalarField::new(
                "field.vmem_like",
                SurfaceScalarFieldKind::VmemLike,
                vmem_values,
            ),
            SurfaceScalarField::constant(
                "field.wound_signal",
                SurfaceScalarFieldKind::WoundSignal,
                node_count,
                0.0,
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
    .map_err(to_js_error)?;

    let mut wound = SurfaceFieldPerturbation::new(
        "perturbation.wound.dynamic_center",
        Some("field.wound_signal".to_owned()),
        nearest_nodes(&substrate, Vec3::new(0.28, 0.64, 0.0), 6),
        SurfaceFieldPerturbationEffect::WoundRegion { signal_value: 1.0 },
    );
    wound.duration_steps = 30;
    let mut vmem = SurfaceFieldPerturbation::new(
        "perturbation.vmem.dynamic_offset",
        Some("field.vmem_like".to_owned()),
        nearest_nodes(&substrate, Vec3::new(0.50, 0.48, 0.0), 10),
        SurfaceFieldPerturbationEffect::DepolarizeRegion { delta: 0.12 },
    );
    vmem.start_step = 10;
    vmem.duration_steps = 36;
    let mut polarity = SurfaceFieldPerturbation::new(
        "perturbation.polarity.dynamic_inversion",
        Some("field.polarity".to_owned()),
        nearest_nodes(&substrate, Vec3::new(0.72, 0.34, 0.0), 8),
        SurfaceFieldPerturbationEffect::PolarityInversion,
    );
    polarity.start_step = 18;
    let mut coupling = SurfaceFieldPerturbation::new(
        "perturbation.coupling.dynamic_wound_shell",
        None,
        nearest_nodes(&substrate, Vec3::new(0.36, 0.58, 0.0), 14),
        SurfaceFieldPerturbationEffect::CouplingMultiplierChange { multiplier: 1.45 },
    );
    coupling.duration_steps = 90;

    Ok((substrate, state, vec![wound, vmem, polarity, coupling]))
}

fn scalar_values<'a>(state: &'a SurfaceFieldState, field_id: &str) -> &'a [f32] {
    state
        .scalar_field(field_id)
        .map_or(&[], |field| field.values.as_slice())
}

fn vector_values<'a>(state: &'a SurfaceFieldState, field_id: &str) -> &'a [Vec3] {
    state
        .vector_field(field_id)
        .map_or(&[], |field| field.vectors.as_slice())
}

fn unit_square_surface() -> TriangleMeshSurface {
    TriangleMeshSurface::new(
        "mesh.unit_square_surface",
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    )
}

fn nearest_nodes(substrate: &SurfaceFieldSubstrate, center: Vec3, count: usize) -> Vec<usize> {
    let mut nodes = substrate
        .nodes
        .iter()
        .map(|node| (node.node_index, node.position.distance_squared(center)))
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.1.total_cmp(&right.1));
    nodes
        .into_iter()
        .take(count.min(substrate.node_count()))
        .map(|(node_index, _)| node_index)
        .collect()
}

fn normalize(vector: Vec3) -> Vec3 {
    let length = vector.length();
    if length > 1.0e-6 {
        vector / length
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    }
}

fn effect_code(effect: &SurfaceFieldPerturbationEffect) -> u32 {
    match effect {
        SurfaceFieldPerturbationEffect::WoundRegion { .. } => 1,
        SurfaceFieldPerturbationEffect::DepolarizeRegion { .. } => 2,
        SurfaceFieldPerturbationEffect::PolarityInversion => 3,
        SurfaceFieldPerturbationEffect::CouplingMultiplierChange { .. } => 4,
        SurfaceFieldPerturbationEffect::NormalPolarity { .. } => 5,
    }
}

fn target_code(target_field_id: Option<&str>) -> u32 {
    match target_field_id {
        Some("field.wound_signal") => 1,
        Some("field.vmem_like") => 2,
        Some("field.polarity") => 3,
        Some("field.morphogen") => 4,
        Some(_) => 99,
        None => 0,
    }
}

fn usize_to_u32(value: usize) -> u32 {
    value.try_into().unwrap_or(u32::MAX)
}

fn to_js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
