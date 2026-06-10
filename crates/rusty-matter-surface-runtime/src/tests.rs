use super::*;
use rusty_matter_mesh::{DynamicMeshColliderUpdateStatus, TriangleMeshSurface};
use rusty_matter_model::Vec3;
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
    assert_eq!(diagnostics.refreshed_distance_samples, 16);
    assert_eq!(snapshot.samples.len(), 16);
    assert!(snapshot
        .samples
        .iter()
        .all(|sample| sample.last_surface_distance.is_some()));
    assert_eq!(payload.samples.len(), 16);
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
