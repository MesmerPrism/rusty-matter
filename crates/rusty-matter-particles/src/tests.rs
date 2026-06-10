use super::*;
use rusty_matter_mesh::{SurfaceDistanceSamplerConfig, TriangleMeshSurface};
use rusty_matter_model::{TriangleMeshSnapshot, Vec3};
use rusty_matter_sdf::{build_sdf_from_mesh, MeshSdfSignMode, MeshToSdfConfig, PackedSdfGrid};
use std::num::NonZeroUsize;

fn particle_set() -> ParticleSet {
    let mut particles = ParticleSet::new("particles.test");
    let mut particle = ParticleState::new("particle.0", Vec3::new(0.25, 0.25, 0.125), 0.01);
    particle.velocity = Vec3::ZERO;
    particles.push(particle);
    particles
}

fn triangle_sdf() -> PackedSdfGrid {
    let mesh = TriangleMeshSnapshot::new(
        "mesh.unit_triangle",
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        vec![[0, 1, 2]],
    );
    build_sdf_from_mesh(
        &mesh,
        MeshToSdfConfig {
            voxel_size: 0.25,
            padding_voxels: 2,
            max_voxels: 10_000,
            sign_mode: MeshSdfSignMode::TriangleNormal,
        },
    )
    .expect("SDF builds")
}

fn interaction_particle_set() -> ParticleSet {
    let mut particles = ParticleSet::new("particles.interaction_test");
    particles.push(ParticleState::new(
        "particle.0",
        Vec3::new(-0.05, 0.0, 0.0),
        0.02,
    ));
    particles.push(ParticleState::new(
        "particle.1",
        Vec3::new(0.05, 0.0, 0.0),
        0.02,
    ));
    particles
}

fn interaction_bundle() -> ParticleInteractions {
    let mut interactions = ParticleInteractions::default();
    interactions
        .influence_points
        .push(ParticleInfluencePoint::new(
            "influence.up_attract",
            Vec3::new(0.0, 0.2, 0.0),
            1.0,
            1.0,
            ParticleInfluenceMode::Attract,
        ));
    interactions.bodies.push(ParticleInteractionBody::sphere(
        "body.center_sphere",
        Vec3::ZERO,
        0.075,
    ));
    interactions
}

fn execution_config(
    backend: ParticleExecutionBackend,
    batch_size: usize,
    max_threads: Option<usize>,
) -> ParticleExecutionConfig {
    ParticleExecutionConfig {
        backend,
        batch_size: NonZeroUsize::new(batch_size).expect("test batch size is non-zero"),
        max_threads,
    }
}

fn interaction_simulator_with_execution(execution: ParticleExecutionConfig) -> ParticleSimulator {
    let mut simulator = ParticleSimulator::new_with_execution(
        interaction_particle_set(),
        ParticleFixedStepConfig {
            fixed_step_seconds: 1.0 / 30.0,
            max_steps_per_frame: 1,
            neighbor_radius: 0.2,
            neighbor_repulsion_strength: 1.0,
            ..ParticleFixedStepConfig::default()
        },
        SdfParticleInteractionConfig {
            mode: SdfParticleInteractionMode::Disabled,
            damping: 0.0,
            max_speed: 10.0,
            ..SdfParticleInteractionConfig::default()
        },
        execution,
    )
    .expect("simulator builds");
    simulator
        .set_interactions(interaction_bundle())
        .expect("interactions validate");
    simulator
        .push_impulse(ParticleImpulse::new(
            "impulse.up",
            Vec3::ZERO,
            0.25,
            Vec3::new(0.0, 0.1, 0.0),
        ))
        .expect("impulse validates");
    simulator
}

#[test]
fn particle_set_validates_particles() {
    let particles = particle_set();
    assert_eq!(particles.len(), 1);
    particles.validate().expect("set is valid");
}

#[test]
fn particle_render_payload_is_render_neutral() {
    let mut particles = particle_set();
    particles.particles[0].velocity = Vec3::new(0.0, 2.0, 0.0);
    particles.particles[0].flags = 7;

    let payload = ParticleRenderPayload::from_particle_set("particle.render.test", &particles)
        .expect("payload builds");

    assert_eq!(payload.source_set_id, particles.set_id);
    assert_eq!(payload.samples.len(), 1);
    assert_eq!(payload.samples[0].speed, 2.0);
    assert_eq!(payload.samples[0].flags, 7);
    assert_eq!(payload.bounds_min, Some(Vec3::new(0.24, 0.24, 0.115)));
    assert_eq!(payload.bounds_max, Some(Vec3::new(0.26, 0.26, 0.135)));
}

#[test]
fn particle_render_payload_rejects_empty_payload_id() {
    let particles = particle_set();
    assert!(matches!(
        ParticleRenderPayload::from_particle_set("", &particles),
        Err(ParticleError::EmptyRenderPayloadId)
    ));
}

#[test]
fn particle_rejects_non_finite_position() {
    let mut particle = ParticleState::new("particle.bad", Vec3::new(f32::NAN, 0.0, 0.0), 0.01);
    particle.velocity = Vec3::ZERO;
    assert!(matches!(
        particle.validate(),
        Err(ParticleError::NonFinitePosition { .. })
    ));
}

#[test]
fn fixed_step_simulator_applies_sdf_interaction() {
    let mut simulator = ParticleSimulator::new(
        particle_set(),
        ParticleFixedStepConfig {
            fixed_step_seconds: 1.0 / 30.0,
            max_steps_per_frame: 2,
            ..ParticleFixedStepConfig::default()
        },
        SdfParticleInteractionConfig {
            strength: 2.0,
            damping: 0.0,
            max_speed: 10.0,
            ..SdfParticleInteractionConfig::default()
        },
    )
    .expect("simulator builds");
    simulator.set_sdf(Some(triangle_sdf()));

    let diagnostics = simulator.step_frame(1.0 / 30.0);

    assert_eq!(diagnostics.fixed_steps, 1);
    assert_eq!(diagnostics.particle_count, 1);
    assert_eq!(diagnostics.sampled_particles, 1);
    assert_eq!(diagnostics.affected_particles, 1);
    assert!(diagnostics.max_speed > 0.0);
}

#[test]
fn spatial_hash_returns_neighbor_candidates() {
    let positions = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.05, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
    ];
    let mut grid = SpatialHashGrid::new(0.1);

    grid.build(&positions, 0.1).expect("grid builds");
    let candidates = grid.query_radius(Vec3::ZERO, 0.1);

    assert!(candidates.contains(&0));
    assert!(candidates.contains(&1));
    assert!(!candidates.contains(&2));
}

#[test]
fn particle_interactions_apply_to_step() {
    let mut simulator = interaction_simulator_with_execution(ParticleExecutionConfig::default());

    let diagnostics = simulator.step_frame(1.0 / 30.0);

    assert_eq!(diagnostics.fixed_steps, 1);
    assert!(diagnostics.neighbor_checks > 0);
    assert_eq!(diagnostics.influence_samples, 2);
    assert_eq!(diagnostics.impulses_applied, 2);
    assert_eq!(diagnostics.body_collisions, 2);
    assert!(diagnostics.max_speed > 0.0);
    assert_eq!(
        diagnostics.execution.backend,
        ParticleExecutionBackend::Serial
    );
    assert_eq!(diagnostics.execution.batch_size, 256);
    assert_eq!(diagnostics.execution.chunk_count, 1);
    assert_eq!(diagnostics.execution.worker_count, 1);
    assert_eq!(diagnostics.execution.particle_count, 2);
}

#[test]
fn particle_batch_size_does_not_change_serial_output() {
    let mut unit_batch = interaction_simulator_with_execution(execution_config(
        ParticleExecutionBackend::Serial,
        1,
        None,
    ));
    let mut full_batch = interaction_simulator_with_execution(execution_config(
        ParticleExecutionBackend::Serial,
        64,
        None,
    ));

    let unit_diagnostics = unit_batch.step_frame(1.0 / 30.0);
    let full_diagnostics = full_batch.step_frame(1.0 / 30.0);

    assert_eq!(unit_batch.particles(), full_batch.particles());
    assert_eq!(
        unit_diagnostics.neighbor_checks,
        full_diagnostics.neighbor_checks
    );
    assert_eq!(
        unit_diagnostics.influence_samples,
        full_diagnostics.influence_samples
    );
    assert_eq!(
        unit_diagnostics.impulses_applied,
        full_diagnostics.impulses_applied
    );
    assert_eq!(
        unit_diagnostics.body_collisions,
        full_diagnostics.body_collisions
    );
    assert_eq!(unit_diagnostics.execution.chunk_count, 2);
    assert_eq!(full_diagnostics.execution.chunk_count, 1);
}

#[cfg(feature = "parallel")]
#[test]
fn particle_parallel_execution_matches_serial_output() {
    let mut serial = interaction_simulator_with_execution(execution_config(
        ParticleExecutionBackend::Serial,
        1,
        None,
    ));
    let mut parallel = interaction_simulator_with_execution(execution_config(
        ParticleExecutionBackend::Parallel,
        1,
        Some(2),
    ));

    let serial_diagnostics = serial.step_frame(1.0 / 30.0);
    let parallel_diagnostics = parallel.step_frame(1.0 / 30.0);

    assert_eq!(parallel.particles(), serial.particles());
    assert_eq!(
        parallel_diagnostics.neighbor_checks,
        serial_diagnostics.neighbor_checks
    );
    assert_eq!(
        parallel_diagnostics.influence_samples,
        serial_diagnostics.influence_samples
    );
    assert_eq!(
        parallel_diagnostics.impulses_applied,
        serial_diagnostics.impulses_applied
    );
    assert_eq!(
        parallel_diagnostics.body_collisions,
        serial_diagnostics.body_collisions
    );
    assert_eq!(
        parallel_diagnostics.execution.backend,
        ParticleExecutionBackend::Parallel
    );
    assert_eq!(parallel_diagnostics.execution.batch_size, 1);
    assert_eq!(parallel_diagnostics.execution.chunk_count, 2);
    assert_eq!(parallel_diagnostics.execution.worker_count, 2);
}

#[test]
fn particle_interaction_validation_reports_specific_errors() {
    let influence =
        ParticleInfluencePoint::new("", Vec3::ZERO, 1.0, 1.0, ParticleInfluenceMode::Attract);
    assert!(matches!(
        influence.validate(),
        Err(ParticleError::EmptyInfluenceId)
    ));

    let body =
        ParticleInteractionBody::axis_aligned_box("body.bad", Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO);
    assert!(matches!(
        body.validate(),
        Err(ParticleError::InvalidBodyConfig(_))
    ));

    let config = ParticleFixedStepConfig {
        neighbor_radius: -1.0,
        ..ParticleFixedStepConfig::default()
    };
    assert!(matches!(
        config.validate(),
        Err(ParticleError::InvalidNeighborConfig(_))
    ));
}

#[test]
fn fixed_step_simulator_drops_excess_steps() {
    let mut simulator = ParticleSimulator::new(
        particle_set(),
        ParticleFixedStepConfig {
            fixed_step_seconds: 0.1,
            max_steps_per_frame: 1,
            ..ParticleFixedStepConfig::default()
        },
        SdfParticleInteractionConfig::default(),
    )
    .expect("simulator builds");

    let diagnostics = simulator.step_frame(0.35);

    assert_eq!(diagnostics.fixed_steps, 1);
    assert_eq!(diagnostics.dropped_steps, 2);
}

#[test]
fn surface_particle_reset_is_deterministic() {
    let mut left = SurfaceParticleRuntime::new(
        "particles.surface.left",
        SurfaceParticleRuntimeConfig::default(),
    )
    .expect("runtime builds");
    let mut right = SurfaceParticleRuntime::new(
        "particles.surface.right",
        SurfaceParticleRuntimeConfig::default(),
    )
    .expect("runtime builds");

    left.reset_random_sphere(Vec3::new(0.0, 0.0, 0.0), 8, 2.0, 0.01, 0.5, 17)
        .expect("left reset");
    right
        .reset_random_sphere(Vec3::new(0.0, 0.0, 0.0), 8, 2.0, 0.01, 0.5, 17)
        .expect("right reset");

    assert_eq!(
        left.particles().particles[0].position,
        right.particles().particles[0].position
    );
    assert_eq!(
        left.particles().particles[7].velocity,
        right.particles().particles[7].velocity
    );
}

#[test]
fn surface_particle_runtime_steps_against_accelerated_sampler() {
    let surface = TriangleMeshSurface::new(
        "mesh.surface_particle_test",
        vec![
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::new(0.5, -0.5, 0.0),
            Vec3::new(0.0, 0.5, 0.0),
        ],
        vec![[0, 1, 2]],
    );
    let sampler = surface
        .distance_sampler(SurfaceDistanceSamplerConfig::default())
        .expect("sampler builds");
    let mut runtime = SurfaceParticleRuntime::new(
        "particles.surface_step",
        SurfaceParticleRuntimeConfig {
            max_substep_seconds: 1.0 / 120.0,
            max_substeps_per_frame: 8,
            ..SurfaceParticleRuntimeConfig::default()
        },
    )
    .expect("runtime builds");
    runtime
        .reset_random_sphere(Vec3::new(0.0, 0.0, 0.2), 16, 0.8, 0.01, 0.5, 3)
        .expect("reset succeeds");
    let before = runtime.particles().particles[0].position;

    let diagnostics =
        runtime.step_against_surface(&sampler, 0.5, Vec3::new(0.0, 0.0, 0.0), 0.8, 1.0 / 30.0);

    assert_eq!(diagnostics.particle_count, 16);
    assert_eq!(diagnostics.substeps, 4);
    assert!(diagnostics.closest_samples >= 16);
    assert!(diagnostics.affected_particles >= 16);
    assert!(diagnostics.surface_triangle_tests >= diagnostics.closest_samples);
    assert!(diagnostics.max_speed.is_finite());
    assert_ne!(runtime.particles().particles[0].position, before);
}

#[test]
fn surface_particle_runtime_rejects_invalid_config() {
    let error = SurfaceParticleRuntime::new(
        "particles.surface_bad",
        SurfaceParticleRuntimeConfig {
            max_substep_seconds: 0.0,
            ..SurfaceParticleRuntimeConfig::default()
        },
    )
    .unwrap_err();

    assert_eq!(error, ParticleError::InvalidFixedStep);
}
