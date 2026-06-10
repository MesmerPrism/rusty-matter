use rusty_matter_batch::{BatchExecutor, BatchReduce, BatchReport};
use rusty_matter_model::Vec3;
use rusty_matter_sdf::PackedSdfGrid;

use crate::{
    ParticleError, ParticleExecutionConfig, ParticleExecutionDiagnostics, ParticleFixedStepConfig,
    ParticleImpulse, ParticleInfluenceMode, ParticleInfluencePoint, ParticleInteractionBody,
    ParticleInteractionShape, ParticleInteractions, ParticleSet, ParticleSimulationDiagnostics,
    ParticleState, SdfParticleInteractionConfig, SdfParticleInteractionMode, SpatialHashGrid,
};

/// Fixed-step particle simulator.
#[derive(Clone, Debug)]
pub struct ParticleSimulator {
    particles: ParticleSet,
    fixed_step: ParticleFixedStepConfig,
    interaction: SdfParticleInteractionConfig,
    interactions: ParticleInteractions,
    pending_impulses: Vec<ParticleImpulse>,
    spatial_hash: SpatialHashGrid,
    execution: ParticleExecutionConfig,
    executor: BatchExecutor,
    sdf: Option<PackedSdfGrid>,
    accumulator_seconds: f32,
    tick: u64,
}

impl ParticleSimulator {
    /// Creates a simulator.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when inputs are invalid.
    pub fn new(
        particles: ParticleSet,
        fixed_step: ParticleFixedStepConfig,
        interaction: SdfParticleInteractionConfig,
    ) -> Result<Self, ParticleError> {
        Self::new_with_execution(
            particles,
            fixed_step,
            interaction,
            ParticleExecutionConfig::default(),
        )
    }

    /// Creates a simulator with explicit low-rate execution settings.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when inputs or execution settings are invalid.
    pub fn new_with_execution(
        particles: ParticleSet,
        fixed_step: ParticleFixedStepConfig,
        interaction: SdfParticleInteractionConfig,
        execution: ParticleExecutionConfig,
    ) -> Result<Self, ParticleError> {
        particles.validate()?;
        fixed_step.validate()?;
        interaction.validate()?;
        let executor = BatchExecutor::new(execution.batch_config()?)
            .map_err(|error| ParticleError::BatchExecution(error.to_string()))?;
        Ok(Self {
            spatial_hash: SpatialHashGrid::new(fixed_step.neighbor_radius.max(1.0e-5)),
            particles,
            fixed_step,
            interaction,
            interactions: ParticleInteractions::default(),
            pending_impulses: Vec::new(),
            execution,
            executor,
            sdf: None,
            accumulator_seconds: 0.0,
            tick: 0,
        })
    }

    /// Returns particle set state.
    #[must_use]
    pub fn particles(&self) -> &ParticleSet {
        &self.particles
    }

    /// Returns the configured non-SDF particle interactions.
    #[must_use]
    pub fn interactions(&self) -> &ParticleInteractions {
        &self.interactions
    }

    /// Returns the particle execution config.
    #[must_use]
    pub fn execution_config(&self) -> &ParticleExecutionConfig {
        &self.execution
    }

    /// Replaces the configured non-SDF particle interactions.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the interaction bundle is invalid.
    pub fn set_interactions(
        &mut self,
        interactions: ParticleInteractions,
    ) -> Result<(), ParticleError> {
        interactions.validate()?;
        self.interactions = interactions;
        Ok(())
    }

    /// Queues a one-shot impulse for the next fixed step.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the impulse is invalid.
    pub fn push_impulse(&mut self, impulse: ParticleImpulse) -> Result<(), ParticleError> {
        impulse.validate()?;
        self.pending_impulses.push(impulse);
        Ok(())
    }

    /// Sets the optional SDF field.
    pub fn set_sdf(&mut self, sdf: Option<PackedSdfGrid>) {
        self.sdf = sdf;
    }

    /// Advances by one variable frame using fixed steps.
    #[must_use]
    pub fn step_frame(&mut self, delta_seconds: f32) -> ParticleSimulationDiagnostics {
        let mut diagnostics = ParticleSimulationDiagnostics::new(
            format!("diagnostics.particle_frame.{}", self.tick),
            self.particles.len(),
        );
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return diagnostics;
        }

        self.accumulator_seconds += delta_seconds;
        let fixed_step = self.fixed_step.fixed_step_seconds;
        while self.accumulator_seconds >= fixed_step
            && diagnostics.fixed_steps < self.fixed_step.max_steps_per_frame
        {
            let step = self.step_fixed(fixed_step);
            diagnostics.merge_step(&step);
            diagnostics.fixed_steps += 1;
            self.particles.time_seconds += fixed_step;
            self.tick = self.tick.wrapping_add(1);
            self.accumulator_seconds -= fixed_step;
        }

        if self.accumulator_seconds >= fixed_step {
            diagnostics.dropped_steps = (self.accumulator_seconds / fixed_step).floor() as u32;
            self.accumulator_seconds %= fixed_step;
        }
        diagnostics
    }

    fn step_fixed(&mut self, delta_seconds: f32) -> ParticleSimulationDiagnostics {
        let mut diagnostics = ParticleSimulationDiagnostics::new(
            format!("diagnostics.particle_step.{}", self.tick),
            self.particles.len(),
        );
        if self.particles.is_empty() {
            return diagnostics;
        }

        let positions = self
            .particles
            .particles
            .iter()
            .map(|particle| particle.position)
            .collect::<Vec<_>>();
        let mut neighbor_enabled = self.fixed_step.neighbor_radius > 0.0
            && self.fixed_step.neighbor_repulsion_strength > 0.0;
        if neighbor_enabled
            && self
                .spatial_hash
                .build(&positions, self.fixed_step.neighbor_radius)
                .is_err()
        {
            neighbor_enabled = false;
            diagnostics.rejected_particles += self.particles.len();
        }
        let impulses = std::mem::take(&mut self.pending_impulses);
        let previous_particles = self.particles.particles.clone();
        let mut next_particles = previous_particles.clone();
        let snapshot = ParticleStepSnapshot {
            previous_particles: &previous_particles,
            positions: &positions,
            fixed_step: &self.fixed_step,
            interaction: &self.interaction,
            interactions: &self.interactions,
            impulses: &impulses,
            spatial_hash: &self.spatial_hash,
            sdf: self.sdf.as_ref(),
            neighbor_enabled,
            delta_seconds,
        };
        let report = self
            .executor
            .run_slice_chunks(&mut next_particles, |chunk, output| {
                step_particle_chunk(&snapshot, chunk.range.start, output)
            });
        diagnostics.merge_chunk_report(&report, self.execution.backend);
        self.particles.particles = next_particles;

        diagnostics.particle_count = self.particles.len();
        diagnostics
    }
}

struct ParticleStepSnapshot<'a> {
    previous_particles: &'a [ParticleState],
    positions: &'a [Vec3],
    fixed_step: &'a ParticleFixedStepConfig,
    interaction: &'a SdfParticleInteractionConfig,
    interactions: &'a ParticleInteractions,
    impulses: &'a [ParticleImpulse],
    spatial_hash: &'a SpatialHashGrid,
    sdf: Option<&'a PackedSdfGrid>,
    neighbor_enabled: bool,
    delta_seconds: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ParticleStepChunkDiagnostics {
    sampled_particles: usize,
    affected_particles: usize,
    rejected_particles: usize,
    clamped_particles: usize,
    neighbor_checks: usize,
    influence_samples: usize,
    impulses_applied: usize,
    body_collisions: usize,
    max_speed: f32,
}

impl BatchReduce for ParticleStepChunkDiagnostics {
    fn reduce(&mut self, other: Self) {
        self.sampled_particles += other.sampled_particles;
        self.affected_particles += other.affected_particles;
        self.rejected_particles += other.rejected_particles;
        self.clamped_particles += other.clamped_particles;
        self.neighbor_checks += other.neighbor_checks;
        self.influence_samples += other.influence_samples;
        self.impulses_applied += other.impulses_applied;
        self.body_collisions += other.body_collisions;
        self.max_speed = self.max_speed.max(other.max_speed);
    }
}

impl ParticleSimulationDiagnostics {
    fn merge_chunk_report(
        &mut self,
        report: &BatchReport<ParticleStepChunkDiagnostics>,
        backend: crate::ParticleExecutionBackend,
    ) {
        self.sampled_particles += report.diagnostics.sampled_particles;
        self.affected_particles += report.diagnostics.affected_particles;
        self.rejected_particles += report.diagnostics.rejected_particles;
        self.clamped_particles += report.diagnostics.clamped_particles;
        self.neighbor_checks += report.diagnostics.neighbor_checks;
        self.influence_samples += report.diagnostics.influence_samples;
        self.impulses_applied += report.diagnostics.impulses_applied;
        self.body_collisions += report.diagnostics.body_collisions;
        self.max_speed = self.max_speed.max(report.diagnostics.max_speed);
        self.execution = ParticleExecutionDiagnostics {
            backend,
            batch_size: report.batch_size,
            chunk_count: report.chunk_count,
            worker_count: report.worker_count,
            particle_count: report.len,
            elapsed_micros: report.elapsed.as_micros(),
        };
    }
}

fn step_particle_chunk(
    snapshot: &ParticleStepSnapshot<'_>,
    start_index: usize,
    output: &mut [ParticleState],
) -> ParticleStepChunkDiagnostics {
    let mut diagnostics = ParticleStepChunkDiagnostics::default();
    for (offset, particle) in output.iter_mut().enumerate() {
        let index = start_index + offset;
        step_one_particle(snapshot, index, particle, &mut diagnostics);
    }
    diagnostics
}

fn step_one_particle(
    snapshot: &ParticleStepSnapshot<'_>,
    index: usize,
    particle: &mut ParticleState,
    diagnostics: &mut ParticleStepChunkDiagnostics,
) {
    particle.age_seconds += snapshot.delta_seconds;
    if particle.inverse_mass == 0.0 {
        diagnostics.max_speed = diagnostics.max_speed.max(particle.velocity.length());
        return;
    }

    let position = snapshot.positions[index];
    let mut acceleration = Vec3::ZERO;
    if snapshot.neighbor_enabled {
        let (neighbor_acceleration, checks) = neighbor_acceleration(
            index,
            position,
            snapshot.positions,
            snapshot.spatial_hash,
            snapshot.fixed_step.neighbor_radius,
            snapshot.fixed_step.neighbor_repulsion_strength,
        );
        acceleration = acceleration + neighbor_acceleration;
        diagnostics.neighbor_checks += checks;
    }
    if let Some(sdf) = snapshot.sdf {
        match sdf.sample_nearest(position) {
            Some(sample) => {
                diagnostics.sampled_particles += 1;
                if let Some(sdf_acceleration) = sdf_acceleration(
                    sdf,
                    position,
                    sample.distance,
                    snapshot.interaction.mode,
                    snapshot.interaction.target_distance,
                    snapshot.interaction.strength,
                ) {
                    acceleration = acceleration + sdf_acceleration;
                    diagnostics.affected_particles += 1;
                }
            }
            None => diagnostics.rejected_particles += 1,
        }
    }

    let (influence_acceleration, influence_samples) =
        influence_acceleration(&snapshot.interactions.influence_points, position);
    acceleration = acceleration + influence_acceleration;
    diagnostics.influence_samples += influence_samples;

    let mut velocity =
        snapshot.previous_particles[index].velocity + acceleration * snapshot.delta_seconds;
    let (impulse_delta, impulses_applied) =
        impulse_velocity_delta(snapshot.impulses, position, particle.inverse_mass);
    velocity = velocity + impulse_delta;
    diagnostics.impulses_applied += impulses_applied;

    let damping = (1.0 - snapshot.interaction.damping * snapshot.delta_seconds).clamp(0.0, 1.0);
    velocity = velocity * damping;
    let (velocity, clamped) = clamp_speed(velocity, snapshot.interaction.max_speed);
    let mut velocity = velocity;
    if clamped {
        diagnostics.clamped_particles += 1;
    }

    let mut next_position = position + velocity * snapshot.delta_seconds;
    diagnostics.body_collisions += apply_interaction_bodies(
        &snapshot.interactions.bodies,
        particle.radius,
        &mut next_position,
        &mut velocity,
    );

    particle.velocity = velocity;
    particle.position = next_position;
    diagnostics.max_speed = diagnostics.max_speed.max(velocity.length());
}

fn neighbor_acceleration(
    index: usize,
    position: Vec3,
    positions: &[Vec3],
    grid: &SpatialHashGrid,
    radius: f32,
    strength: f32,
) -> (Vec3, usize) {
    if radius <= 0.0 || strength == 0.0 {
        return (Vec3::ZERO, 0);
    }

    let mut acceleration = Vec3::ZERO;
    let mut checks = 0;
    let radius_squared = radius * radius;
    grid.for_each_candidate(position, radius, |candidate| {
        if candidate == index {
            return;
        }
        let Some(candidate_position) = positions.get(candidate).copied() else {
            return;
        };
        let offset = position - candidate_position;
        let distance_squared = offset.length_squared();
        checks += 1;
        if distance_squared <= 1.0e-12 || distance_squared > radius_squared {
            return;
        }
        let distance = distance_squared.sqrt();
        let falloff = 1.0 - distance / radius;
        acceleration = acceleration + offset / distance * (falloff * strength);
    });

    (acceleration, checks)
}

fn influence_acceleration(points: &[ParticleInfluencePoint], position: Vec3) -> (Vec3, usize) {
    let mut acceleration = Vec3::ZERO;
    let mut samples = 0;

    for point in points {
        if point.radius <= 0.0 || point.strength == 0.0 {
            continue;
        }
        let direction_vector = match point.mode {
            ParticleInfluenceMode::Repel => position - point.position,
            ParticleInfluenceMode::Attract | ParticleInfluenceMode::GaussianAttract => {
                point.position - position
            }
        };
        let distance = direction_vector.length();
        if distance <= 1.0e-6 || distance > point.radius {
            continue;
        }
        let direction = direction_vector / distance;
        let t = distance / point.radius;
        let falloff = match point.mode {
            ParticleInfluenceMode::GaussianAttract => (-4.0 * t * t).exp(),
            ParticleInfluenceMode::Repel | ParticleInfluenceMode::Attract => 1.0 - t,
        };
        acceleration = acceleration + direction * (point.strength * falloff);
        samples += 1;
    }

    (acceleration, samples)
}

fn impulse_velocity_delta(
    impulses: &[ParticleImpulse],
    position: Vec3,
    inverse_mass: f32,
) -> (Vec3, usize) {
    if inverse_mass == 0.0 {
        return (Vec3::ZERO, 0);
    }

    let mut velocity_delta = Vec3::ZERO;
    let mut applied = 0;
    for impulse in impulses {
        let offset = position - impulse.position;
        let distance = offset.length();
        let falloff = if impulse.radius <= 0.0 {
            if distance <= 1.0e-6 {
                1.0
            } else {
                0.0
            }
        } else if distance <= impulse.radius {
            1.0 - distance / impulse.radius
        } else {
            0.0
        };
        if falloff <= 0.0 {
            continue;
        }
        velocity_delta = velocity_delta + impulse.velocity_delta * (falloff * inverse_mass);
        applied += 1;
    }

    (velocity_delta, applied)
}

fn apply_interaction_bodies(
    bodies: &[ParticleInteractionBody],
    particle_radius: f32,
    position: &mut Vec3,
    velocity: &mut Vec3,
) -> usize {
    let mut collisions = 0;
    for body in bodies {
        let collided = match body.shape {
            ParticleInteractionShape::Sphere { center, radius } => apply_sphere_body(
                center,
                radius + particle_radius,
                body.restitution,
                position,
                velocity,
            ),
            ParticleInteractionShape::AxisAlignedBox { min, max } => apply_aabb_body(
                min,
                max,
                particle_radius,
                body.restitution,
                position,
                velocity,
            ),
        };
        if collided {
            collisions += 1;
        }
    }
    collisions
}

fn apply_sphere_body(
    center: Vec3,
    radius: f32,
    restitution: f32,
    position: &mut Vec3,
    velocity: &mut Vec3,
) -> bool {
    let offset = *position - center;
    let distance = offset.length();
    if distance >= radius {
        return false;
    }

    let normal = if distance <= 1.0e-6 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        offset / distance
    };
    *position = center + normal * radius;
    reflect_if_inward(normal, restitution, velocity);
    true
}

fn apply_aabb_body(
    min: Vec3,
    max: Vec3,
    particle_radius: f32,
    restitution: f32,
    position: &mut Vec3,
    velocity: &mut Vec3,
) -> bool {
    let expanded_min = min - Vec3::new(particle_radius, particle_radius, particle_radius);
    let expanded_max = max + Vec3::new(particle_radius, particle_radius, particle_radius);
    if position.x < expanded_min.x
        || position.x > expanded_max.x
        || position.y < expanded_min.y
        || position.y > expanded_max.y
        || position.z < expanded_min.z
        || position.z > expanded_max.z
    {
        return false;
    }

    let candidates = [
        (
            position.x - expanded_min.x,
            Vec3::new(-1.0, 0.0, 0.0),
            0usize,
        ),
        (expanded_max.x - position.x, Vec3::new(1.0, 0.0, 0.0), 1),
        (position.y - expanded_min.y, Vec3::new(0.0, -1.0, 0.0), 2),
        (expanded_max.y - position.y, Vec3::new(0.0, 1.0, 0.0), 3),
        (position.z - expanded_min.z, Vec3::new(0.0, 0.0, -1.0), 4),
        (expanded_max.z - position.z, Vec3::new(0.0, 0.0, 1.0), 5),
    ];
    let (_, normal, side) = candidates
        .into_iter()
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .expect("candidate list is not empty");
    match side {
        0 => position.x = expanded_min.x,
        1 => position.x = expanded_max.x,
        2 => position.y = expanded_min.y,
        3 => position.y = expanded_max.y,
        4 => position.z = expanded_min.z,
        _ => position.z = expanded_max.z,
    }
    reflect_if_inward(normal, restitution, velocity);
    true
}

fn reflect_if_inward(normal: Vec3, restitution: f32, velocity: &mut Vec3) {
    let inward_speed = velocity.dot(normal);
    if inward_speed < 0.0 {
        *velocity = *velocity - normal * ((1.0 + restitution) * inward_speed);
    }
}

fn sdf_acceleration(
    sdf: &PackedSdfGrid,
    position: Vec3,
    distance: f32,
    mode: SdfParticleInteractionMode,
    target_distance: f32,
    strength: f32,
) -> Option<Vec3> {
    if mode == SdfParticleInteractionMode::Disabled || strength == 0.0 {
        return None;
    }
    let normal = sdf_gradient(sdf, position)?;
    match mode {
        SdfParticleInteractionMode::Disabled => None,
        SdfParticleInteractionMode::AttractToSurface => {
            let error = distance - target_distance;
            Some(normal * (-error * strength))
        }
        SdfParticleInteractionMode::RepelFromSurface => {
            let band = target_distance.abs().max(sdf.voxel_size);
            let penetration = band - distance.abs();
            if penetration <= 0.0 {
                None
            } else {
                let direction = if distance >= 0.0 {
                    normal
                } else {
                    normal * -1.0
                };
                Some(direction * (penetration / band * strength))
            }
        }
    }
}

fn sdf_gradient(sdf: &PackedSdfGrid, position: Vec3) -> Option<Vec3> {
    let h = sdf.voxel_size.max(1.0e-5);
    let dx = sample_distance(sdf, position + Vec3::new(h, 0.0, 0.0))?
        - sample_distance(sdf, position - Vec3::new(h, 0.0, 0.0))?;
    let dy = sample_distance(sdf, position + Vec3::new(0.0, h, 0.0))?
        - sample_distance(sdf, position - Vec3::new(0.0, h, 0.0))?;
    let dz = sample_distance(sdf, position + Vec3::new(0.0, 0.0, h))?
        - sample_distance(sdf, position - Vec3::new(0.0, 0.0, h))?;
    normalize_or(Vec3::new(dx, dy, dz), Vec3::new(0.0, 1.0, 0.0))
}

fn sample_distance(sdf: &PackedSdfGrid, position: Vec3) -> Option<f32> {
    sdf.sample_nearest(position).map(|sample| sample.distance)
}

fn normalize_or(vector: Vec3, fallback: Vec3) -> Option<Vec3> {
    if !vector.is_finite() {
        return None;
    }
    let length = vector.length();
    if length <= 1.0e-6 {
        Some(fallback)
    } else {
        Some(vector / length)
    }
}

fn clamp_speed(velocity: Vec3, max_speed: f32) -> (Vec3, bool) {
    if max_speed <= 0.0 {
        return (Vec3::ZERO, velocity.length() > 0.0);
    }
    let speed = velocity.length();
    if speed > max_speed {
        (velocity / speed * max_speed, true)
    } else {
        (velocity, false)
    }
}
