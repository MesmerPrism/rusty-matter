use crate::PARTICLE_DIAGNOSTICS_SCHEMA_ID;

/// Diagnostics emitted by a particle simulation step.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleSimulationDiagnostics {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable diagnostics identifier.
    pub diagnostics_id: String,
    /// Number of fixed steps applied.
    pub fixed_steps: u32,
    /// Number of fixed steps dropped when the frame exceeded its step budget.
    pub dropped_steps: u32,
    /// Number of particles in the set after stepping.
    pub particle_count: usize,
    /// Number of SDF sample attempts that produced a sample.
    pub sampled_particles: usize,
    /// Number of particles affected by SDF acceleration.
    pub affected_particles: usize,
    /// Number of particles skipped because the SDF could not be sampled.
    pub rejected_particles: usize,
    /// Number of particles whose speed was clamped.
    pub clamped_particles: usize,
    /// Candidate neighbor checks performed through the spatial hash.
    pub neighbor_checks: usize,
    /// Influence point samples that affected a particle.
    pub influence_samples: usize,
    /// One-shot impulses applied.
    pub impulses_applied: usize,
    /// Simple body collisions applied.
    pub body_collisions: usize,
    /// Maximum observed speed after stepping.
    pub max_speed: f32,
}

impl ParticleSimulationDiagnostics {
    /// Creates empty diagnostics.
    #[must_use]
    pub fn new(diagnostics_id: impl Into<String>, particle_count: usize) -> Self {
        Self {
            schema_id: PARTICLE_DIAGNOSTICS_SCHEMA_ID.to_owned(),
            diagnostics_id: diagnostics_id.into(),
            fixed_steps: 0,
            dropped_steps: 0,
            particle_count,
            sampled_particles: 0,
            affected_particles: 0,
            rejected_particles: 0,
            clamped_particles: 0,
            neighbor_checks: 0,
            influence_samples: 0,
            impulses_applied: 0,
            body_collisions: 0,
            max_speed: 0.0,
        }
    }

    pub(crate) fn merge_step(&mut self, step: &Self) {
        self.fixed_steps += step.fixed_steps;
        self.dropped_steps += step.dropped_steps;
        self.sampled_particles += step.sampled_particles;
        self.affected_particles += step.affected_particles;
        self.rejected_particles += step.rejected_particles;
        self.clamped_particles += step.clamped_particles;
        self.neighbor_checks += step.neighbor_checks;
        self.influence_samples += step.influence_samples;
        self.impulses_applied += step.impulses_applied;
        self.body_collisions += step.body_collisions;
        self.max_speed = self.max_speed.max(step.max_speed);
        self.particle_count = step.particle_count;
    }
}
