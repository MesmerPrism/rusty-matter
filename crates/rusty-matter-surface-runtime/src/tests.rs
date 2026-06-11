use super::*;
use std::num::NonZeroUsize;

use rusty_matter_mesh::{
    DynamicMeshColliderUpdateStatus, SurfaceDistanceQueryDiagnostics, TriangleMeshSurface,
};
use rusty_matter_model::Vec3;
#[cfg(feature = "parallel")]
use rusty_matter_particles::SurfaceParticleRuntimeConfig;
use rusty_matter_sdf::{MeshSdfSignMode, MeshToSdfConfig};

fn unit_square_surface() -> TriangleMeshSurface {
    TriangleMeshSurface::new(
        "mesh.unit_square",
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    )
}

#[test]
fn runtime_updates_surface_and_exposes_sampler_stats() {
    let mut runtime = MatterSurfaceRuntime::new(MatterSurfaceRuntimeConfig {
        runtime_id: "matter.surface_runtime.test".to_owned(),
        ..MatterSurfaceRuntimeConfig::default()
    })
    .expect("runtime builds");

    let update = runtime
        .update_frame(MatterSurfaceFrameInput::new(7, 0.25, unit_square_surface()))
        .expect("surface update succeeds");
    let sample = runtime
        .sample_distance(Vec3::new(0.25, 0.25, 0.2))
        .expect("distance sample exists");
    let stats = runtime.stats();

    assert_eq!(update.schema_id, MATTER_SURFACE_RUNTIME_UPDATE_SCHEMA_ID);
    assert_eq!(update.frame_index, Some(7));
    assert_eq!(update.time_seconds, Some(0.25));
    assert_eq!(update.vertex_count, 4);
    assert_eq!(update.triangle_count, 2);
    assert_eq!(
        update.collider_update.status,
        DynamicMeshColliderUpdateStatus::Initialized
    );
    assert_eq!(update.distance_sampler.triangle_count, 2);
    assert!((sample.distance - 0.2).abs() < 1.0e-5);
    assert_eq!(stats.frame_index, Some(7));
    assert_eq!(stats.vertex_count, 4);
    assert_eq!(stats.triangle_count, 2);
    assert_eq!(
        stats.particle_distance_refresh_policy,
        MatterSurfaceParticleDistanceRefreshPolicy::SurfaceUpdateAndStep
    );
}

#[test]
fn runtime_refits_distance_sampler_for_matching_topology_frames() {
    let mut runtime = MatterSurfaceRuntime::default();
    let surface = unit_square_surface();
    let mut deformed = surface.clone();
    for position in &mut deformed.positions {
        position.z += 0.125;
    }

    let first = runtime
        .update_frame(MatterSurfaceFrameInput::new(1, 0.0, surface))
        .expect("initial surface update succeeds");
    let second = runtime
        .update_frame(MatterSurfaceFrameInput::new(2, 0.1, deformed))
        .expect("matching-topology surface update refits");
    let sample = runtime
        .sample_distance(Vec3::new(0.25, 0.25, 0.25))
        .expect("distance sample exists");

    assert!(!first.distance_sampler_refit);
    assert!(second.distance_sampler_refit);
    assert_eq!(first.distance_sampler, second.distance_sampler);
    assert!((sample.distance - 0.125).abs() < 1.0e-5);
}

#[test]
fn runtime_probes_dynamic_collider_contacts() {
    let mut runtime = MatterSurfaceRuntime::default();
    runtime
        .update_surface(unit_square_surface())
        .expect("surface update succeeds");

    let batch = runtime.probe_contacts(&[
        MatterSurfaceContactProbe::sphere("probe.near", Vec3::new(0.25, 0.25, 0.05), 0.1),
        MatterSurfaceContactProbe::sphere("probe.far", Vec3::new(0.25, 0.25, 1.0), 0.1),
    ]);

    assert_eq!(
        batch.schema_id,
        MATTER_SURFACE_CONTACT_PROBE_BATCH_SCHEMA_ID
    );
    assert_eq!(batch.results.len(), 2);
    assert_eq!(batch.contact_count, 2);
    assert_eq!(batch.overlap_count, 1);
    assert!(batch.results[0].overlaps);
    assert!(!batch.results[1].overlaps);
}

#[test]
fn contact_probe_batch_preserves_input_order_across_chunks() {
    let mut runtime = MatterSurfaceRuntime::default();
    runtime
        .update_surface(unit_square_surface())
        .expect("surface update succeeds");
    let probes = (0..7)
        .map(|index| {
            MatterSurfaceContactProbe::sphere(
                format!("probe.{index}"),
                Vec3::new(
                    index as f32 * 0.1,
                    0.25,
                    if index % 2 == 0 { 0.03 } else { 0.6 },
                ),
                0.08,
            )
        })
        .collect::<Vec<_>>();

    let batch = runtime
        .probe_contacts_with_batch_config(
            &probes,
            BatchConfig {
                batch_size: NonZeroUsize::new(2).expect("test batch size is non-zero"),
                ..BatchConfig::default()
            },
        )
        .expect("contact probe batch succeeds");

    assert_eq!(batch.results.len(), probes.len());
    assert_eq!(batch.contact_count, probes.len());
    assert_eq!(batch.overlap_count, 4);
    for (probe, result) in probes.iter().zip(batch.results.iter()) {
        assert_eq!(result.probe_id, probe.probe_id);
    }
}

#[test]
fn contact_probe_batch_accepts_reusable_executor() {
    let mut runtime = MatterSurfaceRuntime::default();
    runtime
        .update_surface(unit_square_surface())
        .expect("surface update succeeds");
    let probes = [
        MatterSurfaceContactProbe::sphere("probe.near", Vec3::new(0.25, 0.25, 0.05), 0.1),
        MatterSurfaceContactProbe::sphere("probe.far", Vec3::new(0.25, 0.25, 1.0), 0.1),
    ];
    let executor = BatchExecutor::new(BatchConfig {
        batch_size: NonZeroUsize::new(1).expect("test batch size is non-zero"),
        ..BatchConfig::default()
    })
    .expect("executor builds");

    let batch = runtime.probe_contacts_with_executor(&probes, &executor);

    assert_eq!(batch.results.len(), probes.len());
    assert_eq!(batch.contact_count, 2);
    assert_eq!(batch.overlap_count, 1);
    assert_eq!(batch.results[0].probe_id, "probe.near");
    assert_eq!(batch.results[1].probe_id, "probe.far");
}

#[test]
fn contact_probe_batch_rejects_invalid_worker_cap() {
    let runtime = MatterSurfaceRuntime::default();
    let error = runtime
        .probe_contacts_with_batch_config(
            &[],
            BatchConfig {
                max_threads: Some(0),
                ..BatchConfig::default()
            },
        )
        .unwrap_err();

    assert!(matches!(error, MatterSurfaceRuntimeError::Batch(_)));
}

#[cfg(feature = "parallel")]
#[test]
fn contact_probe_parallel_execution_matches_serial_output() {
    let mut runtime = MatterSurfaceRuntime::default();
    runtime
        .update_surface(unit_square_surface())
        .expect("surface update succeeds");
    let probes = (0..17)
        .map(|index| {
            let x = (index % 5) as f32 * 0.22;
            let y = (index / 5) as f32 * 0.22;
            let z = if index % 3 == 0 { 0.04 } else { 0.45 };
            MatterSurfaceContactProbe::sphere(format!("probe.{index}"), Vec3::new(x, y, z), 0.09)
        })
        .collect::<Vec<_>>();

    let serial = runtime
        .probe_contacts_with_batch_config(
            &probes,
            BatchConfig {
                batch_size: NonZeroUsize::new(3).expect("test batch size is non-zero"),
                ..BatchConfig::default()
            },
        )
        .expect("serial batch succeeds");
    let parallel = runtime
        .probe_contacts_with_batch_config(
            &probes,
            BatchConfig {
                backend: BatchBackendKind::Rayon,
                batch_size: NonZeroUsize::new(3).expect("test batch size is non-zero"),
                max_threads: Some(2),
            },
        )
        .expect("parallel batch succeeds");

    assert_eq!(parallel, serial);
}

#[test]
fn runtime_steps_particles_and_refreshes_last_distances() {
    let mut runtime = MatterSurfaceRuntime::default();
    runtime
        .update_surface(unit_square_surface())
        .expect("surface update succeeds");
    let reset = runtime
        .reset_particles(Vec3::new(0.5, 0.5, 0.25), 16, 0.6, 0.01, 0.5, 11)
        .expect("reset succeeds");

    let diagnostics = runtime
        .step_particles(0.5, Vec3::new(0.5, 0.5, 0.0), 0.8, 1.0 / 30.0)
        .expect("step succeeds");
    let snapshot = runtime.particle_snapshot();
    let payload = runtime
        .particle_render_payload("particles.render.test")
        .expect("render payload builds");

    assert_eq!(reset.samples.len(), 16);
    assert_eq!(diagnostics.particles.particle_count, 16);
    assert!(diagnostics.particles.closest_samples >= 16);
    assert_eq!(diagnostics.particles.execution.batch_size, 256);
    assert_eq!(diagnostics.particles.execution.worker_count, 1);
    assert_eq!(diagnostics.particles.execution.particle_count, 16);
    assert_eq!(diagnostics.refreshed_distance_samples, 16);
    assert_eq!(
        diagnostics.refreshed_distance_execution.backend,
        ParticleExecutionBackend::Serial
    );
    assert_eq!(diagnostics.refreshed_distance_execution.batch_size, 256);
    assert_eq!(diagnostics.refreshed_distance_execution.worker_count, 1);
    assert_eq!(diagnostics.refreshed_distance_execution.particle_count, 16);
    assert_eq!(snapshot.samples.len(), 16);
    assert!(snapshot
        .samples
        .iter()
        .all(|sample| sample.last_surface_distance.is_some()));
    assert_eq!(payload.samples.len(), 16);
}

#[cfg(feature = "parallel")]
#[test]
fn particle_distance_refresh_parallel_execution_matches_serial_output() {
    let mut serial = runtime_with_particle_execution(ParticleExecutionBackend::Serial);
    let mut parallel = runtime_with_particle_execution(ParticleExecutionBackend::Parallel);

    for runtime in [&mut serial, &mut parallel] {
        runtime
            .update_surface(unit_square_surface())
            .expect("surface update succeeds");
        runtime
            .reset_particles(Vec3::new(0.5, 0.5, 0.25), 17, 0.4, 0.01, 0.5, 31)
            .expect("reset succeeds");
    }

    let serial_step = serial
        .step_particles(0.5, Vec3::new(0.5, 0.5, 0.0), 0.8, 0.0)
        .expect("serial step succeeds");
    let parallel_step = parallel
        .step_particles(0.5, Vec3::new(0.5, 0.5, 0.0), 0.8, 0.0)
        .expect("parallel step succeeds");
    let serial_distances = serial
        .particle_snapshot()
        .samples
        .iter()
        .map(|sample| sample.last_surface_distance)
        .collect::<Vec<_>>();
    let parallel_distances = parallel
        .particle_snapshot()
        .samples
        .iter()
        .map(|sample| sample.last_surface_distance)
        .collect::<Vec<_>>();

    assert_eq!(parallel_distances, serial_distances);
    assert_eq!(
        parallel_step.refreshed_distance_diagnostics,
        serial_step.refreshed_distance_diagnostics
    );
    assert_eq!(
        parallel_step.refreshed_distance_execution.backend,
        ParticleExecutionBackend::Parallel
    );
    assert_eq!(parallel_step.refreshed_distance_execution.batch_size, 3);
    assert_eq!(parallel_step.refreshed_distance_execution.chunk_count, 6);
    assert_eq!(parallel_step.refreshed_distance_execution.worker_count, 2);
    assert_eq!(
        parallel_step.refreshed_distance_execution.particle_count,
        17
    );
}

#[test]
fn step_only_particle_distance_refresh_skips_surface_update_refresh() {
    let mut runtime = MatterSurfaceRuntime::new(MatterSurfaceRuntimeConfig {
        particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::StepOnly,
        ..MatterSurfaceRuntimeConfig::default()
    })
    .expect("runtime builds");
    let surface = unit_square_surface();
    runtime
        .update_surface(surface.clone())
        .expect("surface update succeeds");
    runtime
        .reset_particles(Vec3::new(0.5, 0.5, 0.25), 8, 0.1, 0.01, 0.5, 11)
        .expect("reset succeeds");
    let before_update = runtime
        .particle_snapshot()
        .samples
        .iter()
        .map(|sample| sample.last_surface_distance)
        .collect::<Vec<_>>();

    let mut raised = surface;
    for position in &mut raised.positions {
        position.z += 1.0;
    }
    runtime
        .update_surface(raised)
        .expect("matching-topology update succeeds");
    let after_update = runtime
        .particle_snapshot()
        .samples
        .iter()
        .map(|sample| sample.last_surface_distance)
        .collect::<Vec<_>>();

    let step = runtime
        .step_particles(0.5, Vec3::new(0.5, 0.5, 0.0), 0.8, 0.0)
        .expect("step refresh succeeds");
    let after_step = runtime
        .particle_snapshot()
        .samples
        .iter()
        .map(|sample| sample.last_surface_distance)
        .collect::<Vec<_>>();

    assert_eq!(before_update, after_update);
    assert_ne!(after_update, after_step);
    assert_eq!(step.refreshed_distance_samples, 8);
    assert_eq!(
        runtime.stats().particle_distance_refresh_policy,
        MatterSurfaceParticleDistanceRefreshPolicy::StepOnly
    );
}

#[cfg(feature = "parallel")]
fn runtime_with_particle_execution(backend: ParticleExecutionBackend) -> MatterSurfaceRuntime {
    MatterSurfaceRuntime::new(MatterSurfaceRuntimeConfig {
        particles: SurfaceParticleRuntimeConfig {
            execution: ParticleExecutionConfig {
                backend,
                batch_size: NonZeroUsize::new(3).expect("test batch size is non-zero"),
                max_threads: Some(2),
            },
            ..SurfaceParticleRuntimeConfig::default()
        },
        particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::StepOnly,
        ..MatterSurfaceRuntimeConfig::default()
    })
    .expect("runtime builds")
}

#[test]
fn disabled_particle_distance_refresh_skips_snapshot_sampling() {
    let mut runtime = MatterSurfaceRuntime::new(MatterSurfaceRuntimeConfig {
        particle_distance_refresh_policy: MatterSurfaceParticleDistanceRefreshPolicy::Disabled,
        ..MatterSurfaceRuntimeConfig::default()
    })
    .expect("runtime builds");
    runtime
        .update_surface(unit_square_surface())
        .expect("surface update succeeds");
    let reset = runtime
        .reset_particles(Vec3::new(0.5, 0.5, 0.25), 8, 0.1, 0.01, 0.5, 11)
        .expect("reset succeeds");

    assert!(reset
        .samples
        .iter()
        .all(|sample| sample.last_surface_distance.is_none()));
    assert_eq!(runtime.stats().particle_distance_samples, 0);

    let diagnostics = runtime
        .step_particles(0.5, Vec3::new(0.5, 0.5, 0.0), 0.8, 1.0 / 90.0)
        .expect("step succeeds");
    let snapshot = runtime.particle_snapshot();

    assert_eq!(diagnostics.particles.particle_count, 8);
    assert!(diagnostics.particles.closest_samples >= 8);
    assert_eq!(diagnostics.refreshed_distance_samples, 0);
    assert_eq!(
        diagnostics.refreshed_distance_diagnostics,
        SurfaceDistanceQueryDiagnostics::default()
    );
    assert_eq!(runtime.stats().particle_distance_samples, 0);
    assert!(snapshot
        .samples
        .iter()
        .all(|sample| sample.last_surface_distance.is_none()));
}

#[test]
fn runtime_builds_sdf_grid_from_current_surface() {
    let mut runtime = MatterSurfaceRuntime::default();
    runtime
        .update_surface(unit_square_surface())
        .expect("surface update succeeds");

    let grid = runtime
        .build_sdf_grid(MeshToSdfConfig {
            voxel_size: 0.5,
            padding_voxels: 1,
            max_voxels: 1_000,
            sign_mode: MeshSdfSignMode::UnsignedOnly,
        })
        .expect("grid builds");

    assert_eq!(grid.sample_count(), 32);
    assert!(grid.distances.iter().all(|distance| distance.is_finite()));
}

#[test]
fn runtime_rejects_particle_reset_over_budget() {
    let mut runtime = MatterSurfaceRuntime::default();
    let error = runtime
        .reset_particles(
            Vec3::ZERO,
            MAX_SURFACE_RUNTIME_PARTICLE_COUNT + 1,
            1.0,
            0.01,
            1.0,
            DEFAULT_SURFACE_RUNTIME_PARTICLE_SEED,
        )
        .unwrap_err();

    assert_eq!(error, MatterSurfaceRuntimeError::InvalidParticleCount);
}
