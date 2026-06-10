use rusty_matter_batch::{BatchExecutor, BatchReduce, BatchReport};
use rusty_matter_mesh::SurfaceDistanceSampler;
use rusty_matter_model::Vec3;

use crate::{
    ParticleError, ParticleExecutionConfig, ParticleExecutionDiagnostics, ParticleSet,
    ParticleState,
};

/// Configuration for particles attracted to an accelerated mesh surface.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceParticleRuntimeConfig {
    /// Target surface distance as a multiplier of particle radius.
    pub target_distance_radius_scale: f32,
    /// Minimum target distance in mesh units.
    pub minimum_target_distance: f32,
    /// Acceleration scale applied toward the target surface band.
    pub attraction_strength: f32,
    /// Velocity damping per second.
    pub damping: f32,
    /// Maximum particle speed as a multiplier of source surface radius.
    pub max_speed_radius_scale: f32,
    /// Radius multiplier before cloud confinement begins.
    pub cloud_confinement_radius_scale: f32,
    /// Acceleration scale applied back into the source cloud.
    pub cloud_confinement_strength: f32,
    /// Maximum fixed substep size.
    pub max_substep_seconds: f32,
    /// Maximum substeps consumed by one frame.
    pub max_substeps_per_frame: u32,
    /// Low-rate execution settings for particle batches.
    pub execution: ParticleExecutionConfig,
}

impl Default for SurfaceParticleRuntimeConfig {
    fn default() -> Self {
        Self {
            target_distance_radius_scale: 0.65,
            minimum_target_distance: 0.0008,
            attraction_strength: 19.0,
            damping: 1.55,
            max_speed_radius_scale: 1.9,
            cloud_confinement_radius_scale: 1.12,
            cloud_confinement_strength: 7.0,
            max_substep_seconds: 1.0 / 45.0,
            max_substeps_per_frame: 8,
            execution: ParticleExecutionConfig::default(),
        }
    }
}

impl SurfaceParticleRuntimeConfig {
    /// Validates the runtime configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when fields are non-finite or invalid.
    pub fn validate(&self) -> Result<(), ParticleError> {
        if !self.target_distance_radius_scale.is_finite() || self.target_distance_radius_scale < 0.0
        {
            return Err(ParticleError::InvalidInteractionConfig(
                "target_distance_radius_scale must be finite and non-negative",
            ));
        }
        if !self.minimum_target_distance.is_finite() || self.minimum_target_distance < 0.0 {
            return Err(ParticleError::InvalidInteractionConfig(
                "minimum_target_distance must be finite and non-negative",
            ));
        }
        if !self.attraction_strength.is_finite() || self.attraction_strength < 0.0 {
            return Err(ParticleError::InvalidInteractionConfig(
                "attraction_strength must be finite and non-negative",
            ));
        }
        if !self.damping.is_finite() || self.damping < 0.0 {
            return Err(ParticleError::InvalidInteractionConfig(
                "damping must be finite and non-negative",
            ));
        }
        if !self.max_speed_radius_scale.is_finite() || self.max_speed_radius_scale < 0.0 {
            return Err(ParticleError::InvalidInteractionConfig(
                "max_speed_radius_scale must be finite and non-negative",
            ));
        }
        if !self.cloud_confinement_radius_scale.is_finite()
            || self.cloud_confinement_radius_scale <= 0.0
        {
            return Err(ParticleError::InvalidInteractionConfig(
                "cloud_confinement_radius_scale must be finite and positive",
            ));
        }
        if !self.cloud_confinement_strength.is_finite() || self.cloud_confinement_strength < 0.0 {
            return Err(ParticleError::InvalidInteractionConfig(
                "cloud_confinement_strength must be finite and non-negative",
            ));
        }
        if !self.max_substep_seconds.is_finite() || self.max_substep_seconds <= 0.0 {
            return Err(ParticleError::InvalidFixedStep);
        }
        if self.max_substeps_per_frame == 0 {
            return Err(ParticleError::InvalidMaxSteps);
        }
        self.execution.validate()?;
        Ok(())
    }
}

/// Diagnostics from one surface-particle frame step.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceParticleStepDiagnostics {
    /// Number of particles in the runtime after the step.
    pub particle_count: usize,
    /// Number of fixed substeps consumed.
    pub substeps: u32,
    /// Surface closest-point samples performed by integration.
    pub closest_samples: usize,
    /// Particles receiving attraction acceleration.
    pub affected_particles: usize,
    /// Particles rejected because the surface could not be sampled.
    pub rejected_particles: usize,
    /// Particles whose speed was clamped.
    pub clamped_particles: usize,
    /// BVH node tests performed by integration samples.
    pub surface_node_tests: usize,
    /// BVH leaf tests performed by integration samples.
    pub surface_leaf_tests: usize,
    /// Exact triangle tests performed by integration samples.
    pub surface_triangle_tests: usize,
    /// Maximum observed speed after the step.
    pub max_speed: f32,
    /// Execution diagnostics accumulated across fixed substeps.
    pub execution: ParticleExecutionDiagnostics,
}

/// Matter-owned particle runtime for attraction to an accelerated mesh surface.
#[derive(Clone, Debug)]
pub struct SurfaceParticleRuntime {
    particles: ParticleSet,
    config: SurfaceParticleRuntimeConfig,
    executor: BatchExecutor,
}

impl PartialEq for SurfaceParticleRuntime {
    fn eq(&self, other: &Self) -> bool {
        self.particles == other.particles && self.config == other.config
    }
}

impl SurfaceParticleRuntime {
    /// Creates an empty runtime with the supplied particle-set identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the config is invalid.
    pub fn new(
        set_id: impl Into<String>,
        config: SurfaceParticleRuntimeConfig,
    ) -> Result<Self, ParticleError> {
        config.validate()?;
        let executor = BatchExecutor::new(config.execution.batch_config()?)
            .map_err(|error| ParticleError::BatchExecution(error.to_string()))?;
        Ok(Self {
            particles: ParticleSet::new(set_id),
            config,
            executor,
        })
    }

    /// Returns the current particle set.
    #[must_use]
    pub fn particles(&self) -> &ParticleSet {
        &self.particles
    }

    /// Returns the runtime config.
    #[must_use]
    pub fn config(&self) -> &SurfaceParticleRuntimeConfig {
        &self.config
    }

    /// Returns the particle execution config.
    #[must_use]
    pub fn execution_config(&self) -> &ParticleExecutionConfig {
        &self.config.execution
    }

    /// Resets particles to a deterministic random sphere around `center`.
    ///
    /// # Errors
    ///
    /// Returns [`ParticleError`] when the reset parameters are invalid.
    pub fn reset_random_sphere(
        &mut self,
        center: Vec3,
        count: usize,
        cloud_radius: f32,
        particle_radius: f32,
        surface_radius: f32,
        seed: u32,
    ) -> Result<(), ParticleError> {
        if !center.is_finite()
            || !cloud_radius.is_finite()
            || cloud_radius < 0.0
            || !particle_radius.is_finite()
            || particle_radius < 0.0
            || !surface_radius.is_finite()
            || surface_radius < 0.0
        {
            return Err(ParticleError::InvalidInteractionConfig(
                "surface particle reset parameters must be finite and non-negative",
            ));
        }

        let mut particles = ParticleSet::with_capacity(self.particles.set_id.clone(), count);
        let initial_speed = surface_radius * 0.025;
        for index in 0..count {
            let direction = random_unit_direction(index, seed);
            let radial = unit_hash(index, seed.wrapping_add(113)).cbrt() * cloud_radius;
            let position = center + direction * radial;
            let mut particle = ParticleState::new(
                format!("particle.surface.{index:04}"),
                position,
                particle_radius,
            );
            particle.velocity = direction * -initial_speed;
            particles.push(particle);
        }
        particles.validate()?;
        self.particles = particles;
        Ok(())
    }

    /// Advances particles against the current accelerated surface sampler.
    #[must_use]
    pub fn step_against_surface(
        &mut self,
        sampler: &SurfaceDistanceSampler,
        surface_radius: f32,
        sequence_center: Vec3,
        sequence_cloud_radius: f32,
        delta_seconds: f32,
    ) -> SurfaceParticleStepDiagnostics {
        let mut diagnostics = SurfaceParticleStepDiagnostics {
            particle_count: self.particles.len(),
            ..SurfaceParticleStepDiagnostics::default()
        };
        if self.particles.is_empty()
            || !delta_seconds.is_finite()
            || delta_seconds <= 0.0
            || !surface_radius.is_finite()
            || surface_radius <= 0.0
            || !sequence_center.is_finite()
            || !sequence_cloud_radius.is_finite()
            || sequence_cloud_radius <= 0.0
        {
            return diagnostics;
        }

        let substeps = (delta_seconds / self.config.max_substep_seconds)
            .ceil()
            .max(1.0) as u32;
        let substeps = substeps.clamp(1, self.config.max_substeps_per_frame);
        let sub_delta = delta_seconds / substeps as f32;
        let max_speed = surface_radius * self.config.max_speed_radius_scale;

        for _ in 0..substeps {
            let previous_particles = self.particles.particles.clone();
            let mut next_particles = previous_particles.clone();
            let snapshot = SurfaceParticleStepSnapshot {
                previous_particles: &previous_particles,
                sampler,
                config: &self.config,
                max_speed,
                sequence_center,
                sequence_cloud_radius,
                delta_seconds: sub_delta,
            };
            let report = self
                .executor
                .run_slice_chunks(&mut next_particles, |chunk, output| {
                    step_surface_particle_chunk(&snapshot, chunk.range.start, output)
                });
            diagnostics.merge_chunk_report(&report, self.config.execution.backend);
            self.particles.particles = next_particles;
            self.particles.time_seconds += sub_delta;
        }

        diagnostics.substeps = substeps;
        diagnostics.particle_count = self.particles.len();
        diagnostics
    }
}

struct SurfaceParticleStepSnapshot<'a> {
    previous_particles: &'a [ParticleState],
    sampler: &'a SurfaceDistanceSampler,
    config: &'a SurfaceParticleRuntimeConfig,
    max_speed: f32,
    sequence_center: Vec3,
    sequence_cloud_radius: f32,
    delta_seconds: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct SurfaceParticleStepChunkDiagnostics {
    closest_samples: usize,
    affected_particles: usize,
    rejected_particles: usize,
    clamped_particles: usize,
    surface_node_tests: usize,
    surface_leaf_tests: usize,
    surface_triangle_tests: usize,
    max_speed: f32,
}

impl BatchReduce for SurfaceParticleStepChunkDiagnostics {
    fn reduce(&mut self, other: Self) {
        self.closest_samples += other.closest_samples;
        self.affected_particles += other.affected_particles;
        self.rejected_particles += other.rejected_particles;
        self.clamped_particles += other.clamped_particles;
        self.surface_node_tests += other.surface_node_tests;
        self.surface_leaf_tests += other.surface_leaf_tests;
        self.surface_triangle_tests += other.surface_triangle_tests;
        self.max_speed = self.max_speed.max(other.max_speed);
    }
}

impl SurfaceParticleStepDiagnostics {
    fn merge_chunk_report(
        &mut self,
        report: &BatchReport<SurfaceParticleStepChunkDiagnostics>,
        backend: crate::ParticleExecutionBackend,
    ) {
        self.closest_samples += report.diagnostics.closest_samples;
        self.affected_particles += report.diagnostics.affected_particles;
        self.rejected_particles += report.diagnostics.rejected_particles;
        self.clamped_particles += report.diagnostics.clamped_particles;
        self.surface_node_tests += report.diagnostics.surface_node_tests;
        self.surface_leaf_tests += report.diagnostics.surface_leaf_tests;
        self.surface_triangle_tests += report.diagnostics.surface_triangle_tests;
        self.max_speed = self.max_speed.max(report.diagnostics.max_speed);
        self.execution.backend = backend;
        self.execution.batch_size = report.batch_size;
        self.execution.chunk_count += report.chunk_count;
        self.execution.worker_count = self.execution.worker_count.max(report.worker_count);
        self.execution.particle_count = report.len;
        self.execution.elapsed_micros += report.elapsed.as_micros();
    }
}

fn step_surface_particle_chunk(
    snapshot: &SurfaceParticleStepSnapshot<'_>,
    start_index: usize,
    output: &mut [ParticleState],
) -> SurfaceParticleStepChunkDiagnostics {
    let mut diagnostics = SurfaceParticleStepChunkDiagnostics::default();
    for (offset, particle) in output.iter_mut().enumerate() {
        let index = start_index + offset;
        step_one_surface_particle(snapshot, index, particle, &mut diagnostics);
    }
    diagnostics
}

fn step_one_surface_particle(
    snapshot: &SurfaceParticleStepSnapshot<'_>,
    index: usize,
    particle: &mut ParticleState,
    diagnostics: &mut SurfaceParticleStepChunkDiagnostics,
) {
    particle.age_seconds += snapshot.delta_seconds;
    if particle.inverse_mass == 0.0 {
        diagnostics.max_speed = diagnostics.max_speed.max(particle.velocity.length());
        return;
    }

    let position = snapshot.previous_particles[index].position;
    let Some(sample) = snapshot.sampler.sample(position) else {
        diagnostics.rejected_particles += 1;
        return;
    };
    diagnostics.closest_samples += 1;
    diagnostics.surface_node_tests += sample.diagnostics.node_tests;
    diagnostics.surface_leaf_tests += sample.diagnostics.leaf_tests;
    diagnostics.surface_triangle_tests += sample.diagnostics.triangle_tests;

    let outward = if sample.distance > 1.0e-7 {
        normalize_or(position - sample.point, sample.normal)
    } else {
        normalize_or(sample.normal, Vec3::new(0.0, 1.0, 0.0))
    };
    let target_distance = (particle.radius * snapshot.config.target_distance_radius_scale)
        .max(snapshot.config.minimum_target_distance);
    let error = sample.distance - target_distance;
    let mut acceleration = outward * (-error * snapshot.config.attraction_strength);

    let cloud_offset = position - snapshot.sequence_center;
    let cloud_distance = cloud_offset.length();
    let confinement_radius =
        snapshot.sequence_cloud_radius * snapshot.config.cloud_confinement_radius_scale;
    if cloud_distance > confinement_radius {
        acceleration = acceleration
            + normalize_or(cloud_offset, Vec3::new(0.0, 1.0, 0.0))
                * (-(cloud_distance - snapshot.sequence_cloud_radius)
                    * snapshot.config.cloud_confinement_strength);
    }
    diagnostics.affected_particles += 1;

    let mut velocity =
        snapshot.previous_particles[index].velocity + acceleration * snapshot.delta_seconds;
    let damping = (1.0 - snapshot.config.damping * snapshot.delta_seconds).clamp(0.0, 1.0);
    velocity = velocity * damping;
    let (velocity, clamped) = clamp_speed(velocity, snapshot.max_speed);
    if clamped {
        diagnostics.clamped_particles += 1;
    }
    particle.velocity = velocity;
    particle.position = position + velocity * snapshot.delta_seconds;
    diagnostics.max_speed = diagnostics.max_speed.max(velocity.length());
}

fn random_unit_direction(index: usize, seed: u32) -> Vec3 {
    let z = unit_hash(index, seed).mul_add(2.0, -1.0);
    let angle = unit_hash(index, seed.wrapping_add(41)) * core::f32::consts::TAU;
    let radius = (1.0 - z * z).max(0.0).sqrt();
    Vec3::new(angle.cos() * radius, angle.sin() * radius, z)
}

fn unit_hash(index: usize, seed: u32) -> f32 {
    let mut value = (index as u32).wrapping_add(1).wrapping_mul(2_654_435_761)
        ^ seed.wrapping_add(1).wrapping_mul(2_246_822_519);
    value ^= value >> 16;
    value = value.wrapping_mul(2_246_822_507);
    value ^= value >> 13;
    value = value.wrapping_mul(3_266_489_909);
    value ^= value >> 16;
    ((value >> 8) as f32) / 16_777_215.0
}

fn normalize_or(vector: Vec3, fallback: Vec3) -> Vec3 {
    if !vector.is_finite() {
        return fallback;
    }
    let length = vector.length();
    if length <= 1.0e-6 {
        fallback
    } else {
        vector / length
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
