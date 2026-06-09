use rusty_matter_model::Vec3;

use crate::{
    MatterFieldError, SurfaceFieldDebugFrame, SurfaceFieldDebugFrameSequence,
    SurfaceFieldPerturbation, SurfaceFieldPerturbationEffect, SurfaceFieldRunSummary,
    SurfaceFieldRuntime, SurfaceFieldState, SurfaceFieldStepDiagnostics, SurfaceFieldSubstrate,
    SurfaceScalarFieldKind, SurfaceVectorFieldKind,
};

/// One precomputed sparse neighbor link used by surface-field dynamics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceFieldNeighborLink {
    /// Neighbor node index.
    pub target: usize,
    /// Neighbor tier, starting at 1.
    pub tier: u8,
    /// Relative link weight.
    pub weight: f32,
}

/// Sparse surface-neighbor plan reused by fixed-step field dynamics.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldDynamicsPlan {
    /// Number of substrate nodes.
    pub node_count: usize,
    /// Enabled neighbor tier count.
    pub enabled_neighbor_tiers: u8,
    /// Directed sparse link count.
    pub directed_link_count: usize,
    /// Sparse links grouped by source node.
    pub links: Vec<Vec<SurfaceFieldNeighborLink>>,
}

impl SurfaceFieldDynamicsPlan {
    /// Builds a sparse dynamics plan from a validated substrate and runtime
    /// config.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when substrate or config contracts are
    /// invalid.
    pub fn from_substrate(
        substrate: &SurfaceFieldSubstrate,
        config: &crate::SurfaceFieldRuntimeConfig,
    ) -> Result<Self, MatterFieldError> {
        substrate.validate()?;
        config.validate()?;

        let mut links = Vec::with_capacity(substrate.node_count());
        let mut directed_link_count = 0_usize;
        for node in &substrate.nodes {
            let mut node_links = Vec::with_capacity(
                node.first_tier_neighbors.len() + node.second_tier_neighbors.len(),
            );
            node_links.extend(node.first_tier_neighbors.iter().copied().map(|target| {
                SurfaceFieldNeighborLink {
                    target,
                    tier: 1,
                    weight: 1.0,
                }
            }));
            if config.enabled_neighbor_tiers >= 2 && config.second_tier_coupling_weight > 0.0 {
                node_links.extend(node.second_tier_neighbors.iter().copied().map(|target| {
                    SurfaceFieldNeighborLink {
                        target,
                        tier: 2,
                        weight: config.second_tier_coupling_weight,
                    }
                }));
            }
            directed_link_count += node_links.len();
            links.push(node_links);
        }

        let plan = Self {
            node_count: substrate.node_count(),
            enabled_neighbor_tiers: config.enabled_neighbor_tiers,
            directed_link_count,
            links,
        };
        plan.validate()?;
        Ok(plan)
    }

    /// Validates sparse link targets and weights.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the plan is internally inconsistent.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.node_count == 0 || self.links.len() != self.node_count {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "dynamics plan node count must match links",
            ));
        }
        if !(1..=2).contains(&self.enabled_neighbor_tiers) {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "dynamics plan neighbor tiers must be 1 or 2",
            ));
        }
        let mut counted_links = 0_usize;
        for (source, links) in self.links.iter().enumerate() {
            for link in links {
                if link.target >= self.node_count {
                    return Err(MatterFieldError::InvalidNeighbor {
                        node_index: source,
                        neighbor_index: link.target,
                    });
                }
                if link.target == source {
                    return Err(MatterFieldError::SelfNeighbor { node_index: source });
                }
                if !(1..=self.enabled_neighbor_tiers).contains(&link.tier) {
                    return Err(MatterFieldError::InvalidRuntimeConfig(
                        "dynamics plan link tier exceeds enabled tiers",
                    ));
                }
                if !link.weight.is_finite() || link.weight <= 0.0 {
                    return Err(MatterFieldError::InvalidRuntimeConfig(
                        "dynamics plan link weight must be finite and positive",
                    ));
                }
                counted_links += 1;
            }
        }
        if counted_links != self.directed_link_count {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "dynamics plan directed link count must match links",
            ));
        }
        Ok(())
    }
}

impl SurfaceFieldRuntime {
    /// Builds a reusable sparse dynamics plan for a substrate.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the substrate or runtime config is
    /// invalid.
    pub fn dynamics_plan(
        &self,
        substrate: &SurfaceFieldSubstrate,
    ) -> Result<SurfaceFieldDynamicsPlan, MatterFieldError> {
        SurfaceFieldDynamicsPlan::from_substrate(substrate, self.config())
    }

    /// Advances one fixed step in place.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when contracts, perturbations, or generated
    /// values are invalid.
    pub fn step_fixed(
        &self,
        substrate: &SurfaceFieldSubstrate,
        state: &mut SurfaceFieldState,
        perturbations: &[SurfaceFieldPerturbation],
        step_index: u32,
    ) -> Result<SurfaceFieldStepDiagnostics, MatterFieldError> {
        let plan = self.dynamics_plan(substrate)?;
        self.step_with_plan(&plan, substrate, state, perturbations, step_index)
    }

    /// Runs fixed-step dynamics and emits a Matter-owned debug frame sequence.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when contracts, run bounds, perturbations,
    /// or generated frames are invalid.
    pub fn run_debug_sequence(
        &self,
        sequence_id: impl Into<String>,
        substrate: &SurfaceFieldSubstrate,
        initial_state: &SurfaceFieldState,
        perturbations: &[SurfaceFieldPerturbation],
        step_count: u32,
        frame_stride: u32,
    ) -> Result<SurfaceFieldDebugFrameSequence, MatterFieldError> {
        if step_count == 0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "step_count must be non-zero for dynamic runs",
            ));
        }
        if step_count > self.config().max_steps_per_run {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "step_count exceeds max_steps_per_run",
            ));
        }
        if frame_stride == 0 {
            return Err(MatterFieldError::InvalidRunSummary(
                "frame_stride must be non-zero",
            ));
        }
        validate_state_for_substrate(substrate, initial_state)?;
        for perturbation in perturbations {
            perturbation.validate(substrate.node_count())?;
        }

        let sequence_id = sequence_id.into();
        let plan = self.dynamics_plan(substrate)?;
        let mut state = initial_state.clone();
        let initial_time_seconds = state.time_seconds;
        let mut diagnostics = Vec::with_capacity(step_count as usize);
        let mut frames = Vec::new();
        frames.push(SurfaceFieldDebugFrame::from_state_at_step(
            format!("{sequence_id}.frame.0000"),
            substrate,
            &state,
            perturbations,
            0,
        )?);

        for step in 0..step_count {
            let diagnostic =
                self.step_with_plan(&plan, substrate, &mut state, perturbations, step)?;
            state.time_seconds =
                initial_time_seconds + (step + 1) as f32 * self.config().fixed_step_seconds;
            state.validate()?;
            diagnostics.push(diagnostic);

            let emitted_step = step + 1;
            if emitted_step % frame_stride == 0 || emitted_step == step_count {
                frames.push(SurfaceFieldDebugFrame::from_state_at_step(
                    format!("{sequence_id}.frame.{emitted_step:04}"),
                    substrate,
                    &state,
                    perturbations,
                    emitted_step,
                )?);
            }
        }

        let summary = SurfaceFieldRunSummary::from_run(
            format!("{sequence_id}.summary"),
            substrate,
            &state,
            self.config(),
            perturbations,
            step_count,
        )?;
        SurfaceFieldDebugFrameSequence::new(
            sequence_id,
            self.config().fixed_step_seconds,
            step_count,
            frame_stride,
            diagnostics,
            summary,
            frames,
        )
    }

    fn step_with_plan(
        &self,
        plan: &SurfaceFieldDynamicsPlan,
        substrate: &SurfaceFieldSubstrate,
        state: &mut SurfaceFieldState,
        perturbations: &[SurfaceFieldPerturbation],
        step_index: u32,
    ) -> Result<SurfaceFieldStepDiagnostics, MatterFieldError> {
        plan.validate()?;
        validate_state_for_substrate(substrate, state)?;
        if plan.node_count != substrate.node_count() {
            return Err(MatterFieldError::InvalidRuntimeConfig(
                "dynamics plan node count must match substrate",
            ));
        }
        for perturbation in perturbations {
            perturbation.validate(substrate.node_count())?;
        }

        let mut diagnostic = SurfaceFieldStepDiagnostics::empty(step_index + 1);
        diagnostic.scalar_field_count = state.scalar_fields.len();
        diagnostic.vector_field_count = state.vector_fields.len();
        diagnostic.updated_nodes = substrate.node_count();

        let (active_perturbations, coupling_multipliers) =
            apply_perturbations(substrate, state, perturbations, step_index, self.config())?;
        diagnostic.active_perturbations = active_perturbations;

        for field in &mut state.scalar_fields {
            update_scalar_values(
                &mut field.values,
                plan,
                self.config(),
                &coupling_multipliers,
                &mut diagnostic.clamped_scalars,
                &mut diagnostic.neighbor_links_visited,
            );
        }

        let gradient_values = active_gradient_values(state);
        for field in &mut state.vector_fields {
            update_vector_values(
                &mut field.vectors,
                gradient_values.as_deref(),
                plan,
                substrate,
                self.config(),
                &coupling_multipliers,
                &mut diagnostic.clamped_vectors,
                &mut diagnostic.neighbor_links_visited,
            );
        }

        state.validate()?;
        diagnostic.validate(substrate.node_count())?;
        Ok(diagnostic)
    }
}

fn validate_state_for_substrate(
    substrate: &SurfaceFieldSubstrate,
    state: &SurfaceFieldState,
) -> Result<(), MatterFieldError> {
    substrate.validate()?;
    state.validate()?;
    if state.substrate_id != substrate.substrate_id {
        return Err(MatterFieldError::InvalidRunSummary(
            "state substrate id must match substrate",
        ));
    }
    if state.node_count != substrate.node_count() {
        return Err(MatterFieldError::NodeCountMismatch {
            expected: substrate.node_count(),
            actual: state.node_count,
        });
    }
    Ok(())
}

fn apply_perturbations(
    substrate: &SurfaceFieldSubstrate,
    state: &mut SurfaceFieldState,
    perturbations: &[SurfaceFieldPerturbation],
    step_index: u32,
    config: &crate::SurfaceFieldRuntimeConfig,
) -> Result<(usize, Vec<f32>), MatterFieldError> {
    let mut active_count = 0_usize;
    let mut coupling = vec![1.0; substrate.node_count()];

    for perturbation in perturbations {
        if !is_active(perturbation, step_index) {
            continue;
        }
        active_count += 1;
        match &perturbation.effect {
            SurfaceFieldPerturbationEffect::WoundRegion { signal_value } => {
                let clamped_value =
                    signal_value.clamp(config.scalar_clamp_min, config.scalar_clamp_max);
                let applied = apply_scalar_perturbation(
                    state,
                    perturbation,
                    SurfaceScalarFieldKind::WoundSignal,
                    |value| *value = clamped_value,
                );
                if !applied {
                    return Err(MatterFieldError::InvalidPerturbation(
                        "wound perturbation target field must exist",
                    ));
                }
            }
            SurfaceFieldPerturbationEffect::DepolarizeRegion { delta } => {
                let applied = apply_scalar_perturbation(
                    state,
                    perturbation,
                    SurfaceScalarFieldKind::VmemLike,
                    |value| *value += *delta,
                );
                if !applied {
                    return Err(MatterFieldError::InvalidPerturbation(
                        "depolarization perturbation target field must exist",
                    ));
                }
            }
            SurfaceFieldPerturbationEffect::NormalPolarity { vector } => {
                let applied = apply_vector_perturbation(
                    substrate,
                    state,
                    perturbation,
                    SurfaceVectorFieldKind::Polarity,
                    |node_index, value| {
                        let tangent =
                            project_to_tangent(*vector, substrate.nodes[node_index].normal);
                        *value = normalize_or_zero(tangent) * config.vector_clamp_length;
                    },
                );
                if !applied {
                    return Err(MatterFieldError::InvalidPerturbation(
                        "normal polarity perturbation target field must exist",
                    ));
                }
            }
            SurfaceFieldPerturbationEffect::PolarityInversion => {
                let applied = apply_vector_perturbation(
                    substrate,
                    state,
                    perturbation,
                    SurfaceVectorFieldKind::Polarity,
                    |_node_index, value| *value = *value * -1.0,
                );
                if !applied {
                    return Err(MatterFieldError::InvalidPerturbation(
                        "polarity inversion perturbation target field must exist",
                    ));
                }
            }
            SurfaceFieldPerturbationEffect::CouplingMultiplierChange { multiplier } => {
                for &node_index in &perturbation.node_indices {
                    coupling[node_index] *= *multiplier;
                }
            }
        }
    }

    Ok((active_count, coupling))
}

fn apply_scalar_perturbation(
    state: &mut SurfaceFieldState,
    perturbation: &SurfaceFieldPerturbation,
    fallback_kind: SurfaceScalarFieldKind,
    mut update: impl FnMut(&mut f32),
) -> bool {
    let mut applied = false;
    for field in &mut state.scalar_fields {
        if !scalar_field_matches(
            field.kind,
            &field.field_id,
            &perturbation.target_field_id,
            fallback_kind,
        ) {
            continue;
        }
        for &node_index in &perturbation.node_indices {
            update(&mut field.values[node_index]);
        }
        applied = true;
    }
    applied
}

fn apply_vector_perturbation(
    substrate: &SurfaceFieldSubstrate,
    state: &mut SurfaceFieldState,
    perturbation: &SurfaceFieldPerturbation,
    fallback_kind: SurfaceVectorFieldKind,
    mut update: impl FnMut(usize, &mut Vec3),
) -> bool {
    let mut applied = false;
    for field in &mut state.vector_fields {
        if !vector_field_matches(
            field.kind,
            &field.field_id,
            &perturbation.target_field_id,
            fallback_kind,
        ) {
            continue;
        }
        for &node_index in &perturbation.node_indices {
            if node_index < substrate.node_count() {
                update(node_index, &mut field.vectors[node_index]);
            }
        }
        applied = true;
    }
    applied
}

fn scalar_field_matches(
    kind: SurfaceScalarFieldKind,
    field_id: &str,
    target_field_id: &Option<String>,
    fallback_kind: SurfaceScalarFieldKind,
) -> bool {
    target_field_id
        .as_ref()
        .map_or(kind == fallback_kind, |target| target == field_id)
}

fn vector_field_matches(
    kind: SurfaceVectorFieldKind,
    field_id: &str,
    target_field_id: &Option<String>,
    fallback_kind: SurfaceVectorFieldKind,
) -> bool {
    target_field_id
        .as_ref()
        .map_or(kind == fallback_kind, |target| target == field_id)
}

fn update_scalar_values(
    values: &mut [f32],
    plan: &SurfaceFieldDynamicsPlan,
    config: &crate::SurfaceFieldRuntimeConfig,
    coupling_multipliers: &[f32],
    clamped_scalars: &mut usize,
    neighbor_links_visited: &mut usize,
) {
    let source = values.to_vec();
    for (node_index, value) in values.iter_mut().enumerate() {
        let current = source[node_index];
        let (neighbor_mean, visited_links) =
            weighted_scalar_mean(current, &source, &plan.links[node_index]);
        *neighbor_links_visited += visited_links;
        let local_coupling = coupling_multipliers[node_index];
        let diffusion = config.scalar_diffusion_rate * local_coupling * (neighbor_mean - current);
        let decay = config.scalar_decay_rate * current;
        let next = current + config.fixed_step_seconds * (diffusion - decay);
        let clamped = next.clamp(config.scalar_clamp_min, config.scalar_clamp_max);
        if clamped != next {
            *clamped_scalars += 1;
        }
        *value = clamped;
    }
}

fn update_vector_values(
    vectors: &mut [Vec3],
    gradient_values: Option<&[f32]>,
    plan: &SurfaceFieldDynamicsPlan,
    substrate: &SurfaceFieldSubstrate,
    config: &crate::SurfaceFieldRuntimeConfig,
    coupling_multipliers: &[f32],
    clamped_vectors: &mut usize,
    neighbor_links_visited: &mut usize,
) {
    let source = vectors.to_vec();
    for (node_index, value) in vectors.iter_mut().enumerate() {
        let normal = substrate.nodes[node_index].normal;
        let current = project_to_tangent(source[node_index], normal);
        let (neighbor_mean, gradient, visited_links) =
            weighted_vector_targets(node_index, &source, gradient_values, plan, substrate);
        *neighbor_links_visited += visited_links;
        let local_coupling = coupling_multipliers[node_index];
        let alignment = (neighbor_mean - current)
            * (config.vector_alignment_rate * local_coupling * config.fixed_step_seconds);
        let gradient_response =
            gradient * (config.vector_gradient_rate * local_coupling * config.fixed_step_seconds);
        let next = project_to_tangent(current + alignment + gradient_response, normal);
        let (clamped, was_clamped) = clamp_vector_length(next, config.vector_clamp_length);
        if was_clamped {
            *clamped_vectors += 1;
        }
        *value = clamped;
    }
}

fn active_gradient_values(state: &SurfaceFieldState) -> Option<Vec<f32>> {
    state
        .scalar_fields
        .iter()
        .find(|field| field.kind == SurfaceScalarFieldKind::WoundSignal)
        .or_else(|| state.scalar_fields.first())
        .map(|field| field.values.clone())
}

fn weighted_scalar_mean(
    fallback: f32,
    values: &[f32],
    links: &[SurfaceFieldNeighborLink],
) -> (f32, usize) {
    let mut weight_sum = 0.0_f32;
    let mut value_sum = 0.0_f32;
    for link in links {
        weight_sum += link.weight;
        value_sum += values[link.target] * link.weight;
    }
    if weight_sum > 0.0 {
        (value_sum / weight_sum, links.len())
    } else {
        (fallback, 0)
    }
}

fn weighted_vector_targets(
    node_index: usize,
    vectors: &[Vec3],
    gradient_values: Option<&[f32]>,
    plan: &SurfaceFieldDynamicsPlan,
    substrate: &SurfaceFieldSubstrate,
) -> (Vec3, Vec3, usize) {
    let normal = substrate.nodes[node_index].normal;
    let mut weight_sum = 0.0_f32;
    let mut vector_sum = Vec3::ZERO;
    let mut gradient_sum = Vec3::ZERO;
    for link in &plan.links[node_index] {
        weight_sum += link.weight;
        vector_sum = vector_sum + project_to_tangent(vectors[link.target], normal) * link.weight;
        if let Some(values) = gradient_values {
            let direction =
                substrate.nodes[link.target].position - substrate.nodes[node_index].position;
            let direction = normalize_or_zero(project_to_tangent(direction, normal));
            let scalar_delta = values[link.target] - values[node_index];
            gradient_sum = gradient_sum + direction * (scalar_delta * link.weight);
        }
    }
    if weight_sum > 0.0 {
        (
            project_to_tangent(vector_sum / weight_sum, normal),
            project_to_tangent(gradient_sum / weight_sum, normal),
            plan.links[node_index].len(),
        )
    } else {
        (vectors[node_index], Vec3::ZERO, 0)
    }
}

fn clamp_vector_length(vector: Vec3, max_length: f32) -> (Vec3, bool) {
    let length = vector.length();
    if length > max_length {
        (vector / length * max_length, true)
    } else {
        (vector, false)
    }
}

fn project_to_tangent(vector: Vec3, normal: Vec3) -> Vec3 {
    let normal_length = normal.length();
    if normal_length <= 1.0e-6 {
        return vector;
    }
    let unit_normal = normal / normal_length;
    vector - unit_normal * vector.dot(unit_normal)
}

fn normalize_or_zero(vector: Vec3) -> Vec3 {
    let length = vector.length();
    if length > 1.0e-6 {
        vector / length
    } else {
        Vec3::ZERO
    }
}

fn is_active(perturbation: &SurfaceFieldPerturbation, step_index: u32) -> bool {
    step_index >= perturbation.start_step
        && step_index - perturbation.start_step < perturbation.duration_steps
}
