use rusty_matter_batch::{BatchConfig, BatchExecutor, BatchReduce};
use rusty_matter_mesh::{
    DynamicMeshCollider, DynamicMeshColliderConfig, DynamicMeshColliderContact,
    DynamicMeshColliderUpdate, MeshSurfaceTopologyKey, SurfaceDistanceQueryDiagnostics,
    SurfaceDistanceSample, SurfaceDistanceSampler, SurfaceDistanceSamplerConfig,
    SurfaceDistanceSamplerStats, TriangleMeshSurface,
};
use rusty_matter_model::{TriangleMeshSnapshot, Vec3};
use rusty_matter_particles::{
    ParticleRenderPayload, SurfaceParticleRuntime, SurfaceParticleRuntimeConfig,
    SurfaceParticleStepDiagnostics,
};
use rusty_matter_sdf::{build_sdf_from_mesh, MeshToSdfConfig, PackedSdfGrid};

use crate::MatterSurfaceRuntimeError;

/// Schema ID for native Matter surface runtime update summaries.
pub const MATTER_SURFACE_RUNTIME_UPDATE_SCHEMA_ID: &str = "rusty.matter.surface_runtime.update.v1";
/// Schema ID for native Matter surface runtime stats.
pub const MATTER_SURFACE_RUNTIME_STATS_SCHEMA_ID: &str = "rusty.matter.surface_runtime.stats.v1";
/// Schema ID for native Matter surface contact probe batches.
pub const MATTER_SURFACE_CONTACT_PROBE_BATCH_SCHEMA_ID: &str =
    "rusty.matter.surface_runtime.contact_probe_batch.v1";
/// Schema ID for native Matter surface particle snapshots.
pub const MATTER_SURFACE_PARTICLE_SNAPSHOT_SCHEMA_ID: &str =
    "rusty.matter.surface_runtime.particle_snapshot.v1";
/// Browser-parity default particle count.
pub const DEFAULT_SURFACE_RUNTIME_PARTICLE_COUNT: usize = 1_000;
/// Browser-parity default particle seed.
pub const DEFAULT_SURFACE_RUNTIME_PARTICLE_SEED: u32 = 23;
/// Maximum particle count accepted by the deterministic native facade.
pub const MAX_SURFACE_RUNTIME_PARTICLE_COUNT: usize = 32_768;

/// Policy for refreshing per-particle surface-distance snapshot evidence.
///
/// Particle integration always samples the active surface as needed for
/// Matter-owned simulation. This policy only controls the extra snapshot
/// distances used by renderer-neutral visuals and debug evidence.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MatterSurfaceParticleDistanceRefreshPolicy {
    /// Refresh snapshot distances after each surface update and after each
    /// particle step. This preserves the original native facade behavior.
    #[default]
    SurfaceUpdateAndStep,
    /// Refresh snapshot distances after particle reset/step only. This avoids
    /// redundant pre-step refresh work for adapters that always step particles
    /// after installing a surface frame.
    StepOnly,
    /// Do not refresh snapshot distances automatically.
    ///
    /// Particle integration still samples the active surface as needed for
    /// Matter-owned simulation. This only disables the extra per-particle
    /// snapshot/debug distance pass.
    Disabled,
}

impl MatterSurfaceParticleDistanceRefreshPolicy {
    /// Stable marker token for compact runtime evidence.
    #[must_use]
    pub const fn marker_value(self) -> &'static str {
        match self {
            Self::SurfaceUpdateAndStep => "surface-update-and-step",
            Self::StepOnly => "step-only",
            Self::Disabled => "disabled",
        }
    }

    const fn refresh_after_surface_update(self) -> bool {
        matches!(self, Self::SurfaceUpdateAndStep)
    }

    const fn refresh_after_reset(self) -> bool {
        matches!(self, Self::SurfaceUpdateAndStep | Self::StepOnly)
    }

    const fn refresh_after_step(self) -> bool {
        matches!(self, Self::SurfaceUpdateAndStep | Self::StepOnly)
    }

    const fn clear_when_refresh_skipped(self) -> bool {
        matches!(self, Self::Disabled)
    }
}

/// Native Matter surface runtime configuration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceRuntimeConfig {
    /// Stable runtime identifier.
    pub runtime_id: String,
    /// Config for accelerated closest-surface queries.
    pub distance_sampler: SurfaceDistanceSamplerConfig,
    /// Config for dynamic collider payloads.
    pub collider: DynamicMeshColliderConfig,
    /// Config for Matter-owned particles attracted to the surface.
    pub particles: SurfaceParticleRuntimeConfig,
    /// Stable particle set identifier.
    pub particle_set_id: String,
    /// Policy for extra per-particle snapshot distance refreshes.
    pub particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy,
}

impl Default for MatterSurfaceRuntimeConfig {
    fn default() -> Self {
        Self {
            runtime_id: "matter.surface_runtime.default".to_owned(),
            distance_sampler: SurfaceDistanceSamplerConfig::default(),
            collider: DynamicMeshColliderConfig::default(),
            particles: SurfaceParticleRuntimeConfig::default(),
            particle_set_id: "particles.surface_runtime.default".to_owned(),
            particle_distance_refresh_policy:
                MatterSurfaceParticleDistanceRefreshPolicy::SurfaceUpdateAndStep,
        }
    }
}

impl MatterSurfaceRuntimeConfig {
    /// Validates the runtime configuration.
    ///
    /// # Errors
    ///
    /// Returns [`MatterSurfaceRuntimeError`] when identifiers or sub-configs
    /// are invalid.
    pub fn validate(&self) -> Result<(), MatterSurfaceRuntimeError> {
        if self.runtime_id.trim().is_empty() {
            return Err(MatterSurfaceRuntimeError::EmptyRuntimeId);
        }
        if self.particle_set_id.trim().is_empty() {
            return Err(MatterSurfaceRuntimeError::EmptyParticleSetId);
        }
        self.collider.validate()?;
        self.particles.validate()?;
        Ok(())
    }
}

/// Native animated-surface frame input.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceFrameInput {
    /// Current frame index in the source animation.
    pub frame_index: usize,
    /// Current source animation time in seconds.
    pub time_seconds: f32,
    /// Current animated triangle mesh surface.
    pub surface: TriangleMeshSurface,
}

impl MatterSurfaceFrameInput {
    /// Creates a frame input.
    #[must_use]
    pub fn new(frame_index: usize, time_seconds: f32, surface: TriangleMeshSurface) -> Self {
        Self {
            frame_index,
            time_seconds,
            surface,
        }
    }
}

/// Update summary after installing a new animated mesh surface.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceRuntimeUpdate {
    /// Schema identifier.
    pub schema_id: String,
    /// Runtime identifier.
    pub runtime_id: String,
    /// Current frame index, if known.
    pub frame_index: Option<usize>,
    /// Current source time, if known.
    pub time_seconds: Option<f32>,
    /// Current surface ID.
    pub surface_id: String,
    /// Current surface topology key.
    pub topology_key: MeshSurfaceTopologyKey,
    /// Current vertex count.
    pub vertex_count: usize,
    /// Current triangle count.
    pub triangle_count: usize,
    /// Distance sampler build stats.
    pub distance_sampler: SurfaceDistanceSamplerStats,
    /// Whether the distance sampler reused an existing topology tree and only
    /// refit triangle/node bounds for this update.
    pub distance_sampler_refit: bool,
    /// Dynamic collider update summary.
    pub collider_update: DynamicMeshColliderUpdate,
}

/// Runtime statistics for evidence and adapter decisions.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceRuntimeStats {
    /// Schema identifier.
    pub schema_id: String,
    /// Runtime identifier.
    pub runtime_id: String,
    /// Current surface ID.
    pub surface_id: Option<String>,
    /// Current frame index, if known.
    pub frame_index: Option<usize>,
    /// Current source time, if known.
    pub time_seconds: Option<f32>,
    /// Current surface topology key.
    pub topology_key: Option<MeshSurfaceTopologyKey>,
    /// Current vertex count.
    pub vertex_count: usize,
    /// Current triangle count.
    pub triangle_count: usize,
    /// Distance sampler stats.
    pub distance_sampler: Option<SurfaceDistanceSamplerStats>,
    /// Current particle count.
    pub particle_count: usize,
    /// Policy used for extra per-particle snapshot distance refreshes.
    pub particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy,
    /// Particle closest-distance samples recorded for the latest snapshot.
    pub particle_distance_samples: usize,
}

/// One dynamic contact probe request.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceContactProbe {
    /// Stable probe identifier.
    pub probe_id: String,
    /// Probe center.
    pub center: Vec3,
    /// Probe radius.
    pub radius: f32,
}

impl MatterSurfaceContactProbe {
    /// Creates a contact probe.
    #[must_use]
    pub fn sphere(probe_id: impl Into<String>, center: Vec3, radius: f32) -> Self {
        Self {
            probe_id: probe_id.into(),
            center,
            radius,
        }
    }
}

/// Result for one dynamic contact probe.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceContactProbeResult {
    /// Stable probe identifier.
    pub probe_id: String,
    /// Closest collider contact, if one was available.
    pub contact: Option<DynamicMeshColliderContact>,
    /// Whether the probe sphere overlapped the current collider surface.
    pub overlaps: bool,
}

/// Batched contact probe results.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceContactProbeBatch {
    /// Schema identifier.
    pub schema_id: String,
    /// Probe results.
    pub results: Vec<MatterSurfaceContactProbeResult>,
    /// Number of probes that returned a contact.
    pub contact_count: usize,
    /// Number of probes that overlapped the current collider.
    pub overlap_count: usize,
}

/// One particle snapshot row with latest surface-distance evidence.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceParticleSample {
    /// Source particle identifier.
    pub particle_id: String,
    /// Particle position.
    pub position: Vec3,
    /// Particle velocity.
    pub velocity: Vec3,
    /// Particle radius.
    pub radius: f32,
    /// Particle speed.
    pub speed: f32,
    /// Particle age in seconds.
    pub age_seconds: f32,
    /// Latest closest-surface distance, if sampled.
    pub last_surface_distance: Option<f32>,
}

/// Browser-parity typed particle snapshot.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceParticleSnapshot {
    /// Schema identifier.
    pub schema_id: String,
    /// Source particle set identifier.
    pub source_set_id: String,
    /// Source particle set time in seconds.
    pub time_seconds: f32,
    /// Particle rows.
    pub samples: Vec<MatterSurfaceParticleSample>,
    /// Distance-query diagnostics accumulated while refreshing distances.
    pub distance_diagnostics: SurfaceDistanceQueryDiagnostics,
}

/// Particle step result with refreshed distance evidence.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceStepDiagnostics {
    /// Matter-owned particle step diagnostics.
    pub particles: SurfaceParticleStepDiagnostics,
    /// Number of particle distance samples refreshed after the step.
    pub refreshed_distance_samples: usize,
    /// Distance-query diagnostics accumulated during refresh.
    pub refreshed_distance_diagnostics: SurfaceDistanceQueryDiagnostics,
}

/// Native Matter surface runtime facade.
#[derive(Clone, Debug, PartialEq)]
pub struct MatterSurfaceRuntime {
    config: MatterSurfaceRuntimeConfig,
    surface: Option<TriangleMeshSurface>,
    sampler: Option<SurfaceDistanceSampler>,
    collider: DynamicMeshCollider,
    particles: SurfaceParticleRuntime,
    last_particle_distances: Vec<Option<f32>>,
    last_particle_distance_diagnostics: SurfaceDistanceQueryDiagnostics,
    frame_index: Option<usize>,
    time_seconds: Option<f32>,
}

impl MatterSurfaceRuntime {
    /// Creates a runtime.
    ///
    /// # Errors
    ///
    /// Returns [`MatterSurfaceRuntimeError`] when the runtime config is
    /// invalid.
    pub fn new(config: MatterSurfaceRuntimeConfig) -> Result<Self, MatterSurfaceRuntimeError> {
        config.validate()?;
        let collider = DynamicMeshCollider::new(config.collider.clone());
        let particles =
            SurfaceParticleRuntime::new(config.particle_set_id.clone(), config.particles.clone())?;
        Ok(Self {
            config,
            surface: None,
            sampler: None,
            collider,
            particles,
            last_particle_distances: Vec::new(),
            last_particle_distance_diagnostics: SurfaceDistanceQueryDiagnostics::default(),
            frame_index: None,
            time_seconds: None,
        })
    }

    /// Returns the runtime configuration.
    #[must_use]
    pub fn config(&self) -> &MatterSurfaceRuntimeConfig {
        &self.config
    }

    /// Returns the current surface, if installed.
    #[must_use]
    pub fn surface(&self) -> Option<&TriangleMeshSurface> {
        self.surface.as_ref()
    }

    /// Returns the current distance sampler, if installed.
    #[must_use]
    pub fn distance_sampler(&self) -> Option<&SurfaceDistanceSampler> {
        self.sampler.as_ref()
    }

    /// Returns the dynamic collider.
    #[must_use]
    pub fn collider(&self) -> &DynamicMeshCollider {
        &self.collider
    }

    /// Returns the surface particle runtime.
    #[must_use]
    pub fn particle_runtime(&self) -> &SurfaceParticleRuntime {
        &self.particles
    }

    /// Installs a new animated frame.
    ///
    /// # Errors
    ///
    /// Returns [`MatterSurfaceRuntimeError`] when the frame time or surface is
    /// invalid.
    pub fn update_frame(
        &mut self,
        frame: MatterSurfaceFrameInput,
    ) -> Result<MatterSurfaceRuntimeUpdate, MatterSurfaceRuntimeError> {
        if !frame.time_seconds.is_finite() || frame.time_seconds < 0.0 {
            return Err(MatterSurfaceRuntimeError::InvalidFrameTime);
        }
        self.update_surface_internal(
            frame.surface,
            Some(frame.frame_index),
            Some(frame.time_seconds),
        )
    }

    /// Installs a new surface without source-frame metadata.
    ///
    /// # Errors
    ///
    /// Returns [`MatterSurfaceRuntimeError`] when the surface is invalid.
    pub fn update_surface(
        &mut self,
        surface: TriangleMeshSurface,
    ) -> Result<MatterSurfaceRuntimeUpdate, MatterSurfaceRuntimeError> {
        self.update_surface_internal(surface, None, None)
    }

    /// Samples the closest mesh point to `point`.
    #[must_use]
    pub fn sample_distance(&self, point: Vec3) -> Option<SurfaceDistanceSample> {
        self.sampler.as_ref()?.sample(point)
    }

    /// Runs a batch of dynamic collider contact probes.
    #[must_use]
    pub fn probe_contacts(
        &self,
        probes: &[MatterSurfaceContactProbe],
    ) -> MatterSurfaceContactProbeBatch {
        self.probe_contacts_with_batch_config(probes, BatchConfig::default())
            .expect("default contact probe batch config is valid")
    }

    /// Runs a batch of dynamic collider contact probes with an explicit Matter
    /// batch execution config.
    ///
    /// # Errors
    ///
    /// Returns [`MatterSurfaceRuntimeError`] when the batch executor cannot be
    /// created.
    pub fn probe_contacts_with_batch_config(
        &self,
        probes: &[MatterSurfaceContactProbe],
        batch_config: BatchConfig,
    ) -> Result<MatterSurfaceContactProbeBatch, MatterSurfaceRuntimeError> {
        let executor = BatchExecutor::new(batch_config)?;
        let mut results = vec![None; probes.len()];
        let report = executor.run_slice_chunks(&mut results, |chunk, output| {
            let mut diagnostics = ContactProbeChunkDiagnostics::default();
            for (probe, slot) in probes[chunk.range].iter().zip(output.iter_mut()) {
                let result = self.resolve_contact_probe(probe);
                if result.contact.is_some() {
                    diagnostics.contact_count += 1;
                }
                if result.overlaps {
                    diagnostics.overlap_count += 1;
                }
                *slot = Some(result);
            }
            diagnostics
        });

        let results = probes
            .iter()
            .zip(results)
            .map(|(probe, result)| {
                result.unwrap_or_else(|| MatterSurfaceContactProbeResult {
                    probe_id: probe.probe_id.clone(),
                    contact: None,
                    overlaps: false,
                })
            })
            .collect::<Vec<_>>();
        Ok(MatterSurfaceContactProbeBatch {
            schema_id: MATTER_SURFACE_CONTACT_PROBE_BATCH_SCHEMA_ID.to_owned(),
            results,
            contact_count: report.diagnostics.contact_count,
            overlap_count: report.diagnostics.overlap_count,
        })
    }

    /// Resets particles to a deterministic random sphere.
    ///
    /// # Errors
    ///
    /// Returns [`MatterSurfaceRuntimeError`] when reset parameters are invalid.
    pub fn reset_particles(
        &mut self,
        center: Vec3,
        count: usize,
        cloud_radius: f32,
        particle_radius: f32,
        surface_radius: f32,
        seed: u32,
    ) -> Result<MatterSurfaceParticleSnapshot, MatterSurfaceRuntimeError> {
        validate_particle_reset(count, cloud_radius, particle_radius, surface_radius)?;
        self.particles.reset_random_sphere(
            center,
            count,
            cloud_radius,
            particle_radius,
            surface_radius,
            seed,
        )?;
        if self
            .config
            .particle_distance_refresh_policy
            .refresh_after_reset()
        {
            self.refresh_particle_distances();
        } else {
            self.clear_particle_distances();
        }
        Ok(self.particle_snapshot())
    }

    /// Advances particles against the current surface sampler.
    ///
    /// # Errors
    ///
    /// Returns [`MatterSurfaceRuntimeError`] when no sampler is available.
    pub fn step_particles(
        &mut self,
        surface_radius: f32,
        sequence_center: Vec3,
        sequence_cloud_radius: f32,
        delta_seconds: f32,
    ) -> Result<MatterSurfaceStepDiagnostics, MatterSurfaceRuntimeError> {
        let sampler = self
            .sampler
            .as_ref()
            .ok_or(MatterSurfaceRuntimeError::DistanceSamplerUnavailable)?;
        let particles = self.particles.step_against_surface(
            sampler,
            surface_radius,
            sequence_center,
            sequence_cloud_radius,
            delta_seconds,
        );
        if self
            .config
            .particle_distance_refresh_policy
            .refresh_after_step()
        {
            self.refresh_particle_distances();
        } else {
            self.clear_particle_distances();
        }
        let refreshed_distance_samples = self
            .last_particle_distances
            .iter()
            .filter(|distance| distance.is_some())
            .count();
        Ok(MatterSurfaceStepDiagnostics {
            particles,
            refreshed_distance_samples,
            refreshed_distance_diagnostics: self.last_particle_distance_diagnostics,
        })
    }

    /// Returns a typed particle snapshot including last surface distance.
    #[must_use]
    pub fn particle_snapshot(&self) -> MatterSurfaceParticleSnapshot {
        let particles = self.particles.particles();
        let samples = particles
            .particles
            .iter()
            .enumerate()
            .map(|(index, particle)| MatterSurfaceParticleSample {
                particle_id: particle.particle_id.clone(),
                position: particle.position,
                velocity: particle.velocity,
                radius: particle.radius,
                speed: particle.velocity.length(),
                age_seconds: particle.age_seconds,
                last_surface_distance: self.last_particle_distances.get(index).copied().flatten(),
            })
            .collect::<Vec<_>>();
        MatterSurfaceParticleSnapshot {
            schema_id: MATTER_SURFACE_PARTICLE_SNAPSHOT_SCHEMA_ID.to_owned(),
            source_set_id: particles.set_id.clone(),
            time_seconds: particles.time_seconds,
            samples,
            distance_diagnostics: self.last_particle_distance_diagnostics,
        }
    }

    /// Builds a render-neutral Matter particle payload.
    ///
    /// # Errors
    ///
    /// Returns [`MatterSurfaceRuntimeError`] when the payload ID or particle
    /// set is invalid.
    pub fn particle_render_payload(
        &self,
        payload_id: impl Into<String>,
    ) -> Result<ParticleRenderPayload, MatterSurfaceRuntimeError> {
        let payload_id = payload_id.into();
        if payload_id.trim().is_empty() {
            return Err(MatterSurfaceRuntimeError::EmptyRenderPayloadId);
        }
        ParticleRenderPayload::from_particle_set(payload_id, self.particles.particles())
            .map_err(Into::into)
    }

    /// Builds a packed SDF grid from the current surface.
    ///
    /// # Errors
    ///
    /// Returns [`MatterSurfaceRuntimeError`] when no surface is available or
    /// the SDF build fails.
    pub fn build_sdf_grid(
        &self,
        config: MeshToSdfConfig,
    ) -> Result<PackedSdfGrid, MatterSurfaceRuntimeError> {
        let surface = self
            .surface
            .as_ref()
            .ok_or(MatterSurfaceRuntimeError::SurfaceUnavailable)?;
        let snapshot = TriangleMeshSnapshot::new(
            surface.surface_id.clone(),
            surface.positions.clone(),
            surface.triangles.clone(),
        );
        build_sdf_from_mesh(&snapshot, config).map_err(Into::into)
    }

    /// Returns current runtime statistics.
    #[must_use]
    pub fn stats(&self) -> MatterSurfaceRuntimeStats {
        MatterSurfaceRuntimeStats {
            schema_id: MATTER_SURFACE_RUNTIME_STATS_SCHEMA_ID.to_owned(),
            runtime_id: self.config.runtime_id.clone(),
            surface_id: self
                .surface
                .as_ref()
                .map(|surface| surface.surface_id.clone()),
            frame_index: self.frame_index,
            time_seconds: self.time_seconds,
            topology_key: self.surface.as_ref().map(TriangleMeshSurface::topology_key),
            vertex_count: self
                .surface
                .as_ref()
                .map_or(0, TriangleMeshSurface::vertex_count),
            triangle_count: self
                .surface
                .as_ref()
                .map_or(0, TriangleMeshSurface::triangle_count),
            distance_sampler: self.sampler.as_ref().map(|sampler| sampler.stats().clone()),
            particle_count: self.particles.particles().len(),
            particle_distance_refresh_policy: self.config.particle_distance_refresh_policy,
            particle_distance_samples: self
                .last_particle_distances
                .iter()
                .filter(|distance| distance.is_some())
                .count(),
        }
    }

    fn update_surface_internal(
        &mut self,
        surface: TriangleMeshSurface,
        frame_index: Option<usize>,
        time_seconds: Option<f32>,
    ) -> Result<MatterSurfaceRuntimeUpdate, MatterSurfaceRuntimeError> {
        surface.validate()?;
        let topology_key = surface.topology_key();
        let effective_sampler_config = SurfaceDistanceSamplerConfig {
            leaf_triangle_count: self.config.distance_sampler.effective_leaf_triangle_count(),
            max_depth: self.config.distance_sampler.effective_max_depth(),
        };
        let distance_sampler_refit = self.sampler.as_ref().is_some_and(|sampler| {
            sampler.topology_key() == &topology_key && sampler.config() == &effective_sampler_config
        });
        let distance_sampler = if distance_sampler_refit {
            self.sampler
                .as_mut()
                .expect("sampler exists when refit is selected")
                .refit_from_surface(&surface)?
        } else {
            let sampler = surface.distance_sampler(self.config.distance_sampler.clone())?;
            let distance_sampler = sampler.stats().clone();
            self.sampler = Some(sampler);
            distance_sampler
        };
        let collider_update = self.collider.update_from_surface(&surface);
        let update = MatterSurfaceRuntimeUpdate {
            schema_id: MATTER_SURFACE_RUNTIME_UPDATE_SCHEMA_ID.to_owned(),
            runtime_id: self.config.runtime_id.clone(),
            frame_index,
            time_seconds,
            surface_id: surface.surface_id.clone(),
            topology_key,
            vertex_count: surface.vertex_count(),
            triangle_count: surface.triangle_count(),
            distance_sampler,
            distance_sampler_refit,
            collider_update,
        };
        self.surface = Some(surface);
        self.frame_index = frame_index;
        self.time_seconds = time_seconds;
        if self
            .config
            .particle_distance_refresh_policy
            .refresh_after_surface_update()
        {
            self.refresh_particle_distances();
        } else if self
            .config
            .particle_distance_refresh_policy
            .clear_when_refresh_skipped()
        {
            self.clear_particle_distances();
        }
        Ok(update)
    }

    fn clear_particle_distances(&mut self) {
        let len = self.particles.particles().len();
        self.last_particle_distances.clear();
        self.last_particle_distances.resize(len, None);
        self.last_particle_distance_diagnostics = SurfaceDistanceQueryDiagnostics::default();
    }

    fn refresh_particle_distances(&mut self) {
        self.clear_particle_distances();
        let particles = self.particles.particles();
        let Some(sampler) = self.sampler.as_ref() else {
            return;
        };
        for (index, particle) in particles.particles.iter().enumerate() {
            let Some(sample) = sampler.sample(particle.position) else {
                continue;
            };
            self.last_particle_distances[index] = Some(sample.distance);
            self.last_particle_distance_diagnostics.node_tests += sample.diagnostics.node_tests;
            self.last_particle_distance_diagnostics.leaf_tests += sample.diagnostics.leaf_tests;
            self.last_particle_distance_diagnostics.triangle_tests +=
                sample.diagnostics.triangle_tests;
        }
    }

    fn resolve_contact_probe(
        &self,
        probe: &MatterSurfaceContactProbe,
    ) -> MatterSurfaceContactProbeResult {
        let contact = if probe.center.is_finite() && probe.radius.is_finite() && probe.radius >= 0.0
        {
            self.collider.closest_point(probe.center)
        } else {
            None
        };
        let overlaps = contact
            .as_ref()
            .is_some_and(|contact| contact.distance <= probe.radius.max(0.0));
        MatterSurfaceContactProbeResult {
            probe_id: probe.probe_id.clone(),
            contact,
            overlaps,
        }
    }
}

impl Default for MatterSurfaceRuntime {
    fn default() -> Self {
        Self::new(MatterSurfaceRuntimeConfig::default()).expect("default config is valid")
    }
}

fn validate_particle_reset(
    count: usize,
    cloud_radius: f32,
    particle_radius: f32,
    surface_radius: f32,
) -> Result<(), MatterSurfaceRuntimeError> {
    if count > MAX_SURFACE_RUNTIME_PARTICLE_COUNT {
        return Err(MatterSurfaceRuntimeError::InvalidParticleCount);
    }
    if !cloud_radius.is_finite() || cloud_radius < 0.0 {
        return Err(MatterSurfaceRuntimeError::InvalidParticleReset(
            "cloud_radius must be finite and non-negative",
        ));
    }
    if !particle_radius.is_finite() || particle_radius < 0.0 {
        return Err(MatterSurfaceRuntimeError::InvalidParticleReset(
            "particle_radius must be finite and non-negative",
        ));
    }
    if !surface_radius.is_finite() || surface_radius < 0.0 {
        return Err(MatterSurfaceRuntimeError::InvalidParticleReset(
            "surface_radius must be finite and non-negative",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ContactProbeChunkDiagnostics {
    contact_count: usize,
    overlap_count: usize,
}

impl BatchReduce for ContactProbeChunkDiagnostics {
    fn reduce(&mut self, other: Self) {
        self.contact_count += other.contact_count;
        self.overlap_count += other.overlap_count;
    }
}
