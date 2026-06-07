use super::*;
use rusty_matter_model::{TriangleMeshSnapshot, Vec3};
use rusty_matter_sdf::{build_sdf_from_mesh, MeshSdfSignMode, MeshToSdfConfig, PackedSdfGrid};

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
    let mut simulator = ParticleSimulator::new(
        particles,
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
    )
    .expect("simulator builds");
    simulator
        .set_interactions(interactions)
        .expect("interactions validate");
    simulator
        .push_impulse(ParticleImpulse::new(
            "impulse.up",
            Vec3::ZERO,
            0.25,
            Vec3::new(0.0, 0.1, 0.0),
        ))
        .expect("impulse validates");

    let diagnostics = simulator.step_frame(1.0 / 30.0);

    assert_eq!(diagnostics.fixed_steps, 1);
    assert!(diagnostics.neighbor_checks > 0);
    assert_eq!(diagnostics.influence_samples, 2);
    assert_eq!(diagnostics.impulses_applied, 2);
    assert_eq!(diagnostics.body_collisions, 2);
    assert!(diagnostics.max_speed > 0.0);
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
