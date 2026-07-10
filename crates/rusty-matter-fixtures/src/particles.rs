use rusty_matter_model::Vec3;
use rusty_matter_particles::{
    ParticleFixedStepConfig, ParticleImpulse, ParticleInfluenceMode, ParticleInfluencePoint,
    ParticleInteractionBody, ParticleInteractions, ParticleRenderPayload, ParticleSet,
    ParticleSimulationDiagnostics, ParticleSimulator, ParticleState, SdfParticleInteractionConfig,
    SdfParticleInteractionMode,
};
use rusty_matter_sdf::{build_sdf_from_mesh, MeshSdfSignMode, MeshToSdfConfig};
use rusty_matter_surface_runtime::MatterSurfaceRuntime;

use crate::error::CliError;
use crate::sdf::unit_triangle_mesh;
use crate::summary::{
    ParticleContractConformance, ParticleRenderPayloadSummary, ParticleStepSummary,
};

pub(crate) fn particle_contract_conformance() -> Result<ParticleContractConformance, CliError> {
    let mut particles = ParticleSet::new("particles.contract_conformance");
    let mut first = ParticleState::new("particle.contract.0", Vec3::new(-0.05, 0.0, 0.0), 0.015);
    first.velocity = Vec3::new(0.0, 0.20, 0.0);
    particles.push(first);
    particles.push(ParticleState::new(
        "particle.contract.1",
        Vec3::new(0.05, 0.01, 0.0),
        0.020,
    ));

    let fixed_step = ParticleFixedStepConfig {
        fixed_step_seconds: 1.0 / 60.0,
        max_steps_per_frame: 2,
        ..ParticleFixedStepConfig::default()
    };
    let mut simulator = ParticleSimulator::new(
        particles,
        fixed_step.clone(),
        SdfParticleInteractionConfig {
            mode: SdfParticleInteractionMode::Disabled,
            ..SdfParticleInteractionConfig::default()
        },
    )
    .map_err(CliError::Particle)?;
    let mut diagnostics = simulator.step_frame(1.0 / 60.0);
    diagnostics.execution.elapsed_micros = 0;
    let particle_set = simulator.particles().clone();
    let render_payload =
        ParticleRenderPayload::from_particle_set("particle.payload.contract", &particle_set)
            .map_err(CliError::Particle)?;
    let surface_snapshot = MatterSurfaceRuntime::default().particle_snapshot();

    Ok(ParticleContractConformance {
        schema_id: "rusty.matter.fixture.particle_contract_conformance.v1".to_owned(),
        fixture_id: "fixture.particle.contract_conformance.v1".to_owned(),
        particle_set,
        fixed_step,
        diagnostics,
        render_payload,
        surface_snapshot,
    })
}

pub(crate) fn particle_contract_leak_rejection() -> Result<(), serde_json::Error> {
    let mut value = serde_json::to_value(
        particle_contract_conformance().expect("particle conformance fixture must build"),
    )?;
    let object = value
        .as_object_mut()
        .expect("serialized particle conformance fixture is an object");
    object.insert(
        "application_scene".to_owned(),
        serde_json::json!("spatial-panel"),
    );
    object.insert("platform_handle".to_owned(), serde_json::json!(42));
    object.insert(
        "renderer_resource".to_owned(),
        serde_json::json!("vk-buffer"),
    );
    object.insert(
        "private_driver".to_owned(),
        serde_json::json!("vendor-secret"),
    );
    object.insert("control_rate_hz".to_owned(), serde_json::json!(240));
    serde_json::from_value::<ParticleContractConformance>(value).map(|_| ())
}

pub(crate) fn particle_sdf_attraction_step_summary() -> Result<ParticleStepSummary, CliError> {
    let mut particles = ParticleSet::new("particles.sdf_attraction_fixture");
    particles.push(ParticleState::new(
        "particle.0",
        Vec3::new(0.25, 0.25, 0.125),
        0.01,
    ));
    let sdf = build_sdf_from_mesh(
        &unit_triangle_mesh(),
        MeshToSdfConfig {
            voxel_size: 0.25,
            padding_voxels: 2,
            max_voxels: 10_000,
            sign_mode: MeshSdfSignMode::TriangleNormal,
        },
    )
    .map_err(CliError::Sdf)?;
    let mut simulator = ParticleSimulator::new(
        particles,
        ParticleFixedStepConfig {
            fixed_step_seconds: 1.0 / 30.0,
            max_steps_per_frame: 1,
            ..ParticleFixedStepConfig::default()
        },
        SdfParticleInteractionConfig {
            strength: 2.0,
            damping: 0.0,
            max_speed: 10.0,
            ..SdfParticleInteractionConfig::default()
        },
    )
    .map_err(CliError::Particle)?;
    simulator.set_sdf(Some(sdf));
    let diagnostics = simulator.step_frame(1.0 / 30.0);
    Ok(summarize_particle_step(
        "fixture.particle.sdf_attraction_step.v1",
        simulator.particles(),
        &diagnostics,
    ))
}

pub(crate) fn particle_interaction_step_summary() -> Result<ParticleStepSummary, CliError> {
    let mut particles = ParticleSet::new("particles.interaction_fixture");
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
    interactions.interactions_id = "interactions.interaction_fixture".to_owned();
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
    .map_err(CliError::Particle)?;
    simulator
        .set_interactions(interactions)
        .map_err(CliError::Particle)?;
    simulator
        .push_impulse(ParticleImpulse::new(
            "impulse.up",
            Vec3::ZERO,
            0.25,
            Vec3::new(0.0, 0.1, 0.0),
        ))
        .map_err(CliError::Particle)?;

    let diagnostics = simulator.step_frame(1.0 / 30.0);
    Ok(summarize_particle_step(
        "fixture.particle.interaction_step.v1",
        simulator.particles(),
        &diagnostics,
    ))
}

pub(crate) fn particle_render_payload_summary() -> Result<ParticleRenderPayloadSummary, CliError> {
    let mut particles = ParticleSet::new("particles.render_fixture");
    let mut first = ParticleState::new("particle.render.0", Vec3::new(-0.05, 0.0, 0.0), 0.015);
    first.velocity = Vec3::new(0.0, 0.20, 0.0);
    first.age_seconds = 0.5;
    first.flags = 1;
    particles.push(first);
    let mut second = ParticleState::new("particle.render.1", Vec3::new(0.05, 0.01, 0.0), 0.020);
    second.velocity = Vec3::new(0.0, 0.10, 0.0);
    second.age_seconds = 0.25;
    second.flags = 2;
    particles.push(second);
    particles.time_seconds = 1.25;

    let payload =
        ParticleRenderPayload::from_particle_set("particle.render_payload.fixture", &particles)
            .map_err(CliError::Particle)?;
    let first_sample = payload
        .samples
        .first()
        .expect("fixture contains render samples");

    Ok(ParticleRenderPayloadSummary {
        schema_id: "rusty.matter.fixture.particle_render_payload_summary.v1".to_owned(),
        fixture_id: "fixture.particle.render_payload.v1".to_owned(),
        payload_id: payload.payload_id,
        source_set_id: payload.source_set_id,
        sample_count: payload.samples.len(),
        first_particle_id: first_sample.particle_id.clone(),
        first_position: first_sample.position,
        first_radius: first_sample.radius,
        first_speed: first_sample.speed,
        bounds_min: payload.bounds_min,
        bounds_max: payload.bounds_max,
    })
}

fn summarize_particle_step(
    fixture_id: impl Into<String>,
    particles: &ParticleSet,
    diagnostics: &ParticleSimulationDiagnostics,
) -> ParticleStepSummary {
    let first = particles
        .particles
        .first()
        .expect("fixture has one particle after stepping");
    ParticleStepSummary {
        schema_id: "rusty.matter.fixture.particle_step_summary.v1".to_owned(),
        fixture_id: fixture_id.into(),
        set_id: particles.set_id.clone(),
        particle_count: diagnostics.particle_count,
        fixed_steps: diagnostics.fixed_steps,
        sampled_particles: diagnostics.sampled_particles,
        affected_particles: diagnostics.affected_particles,
        rejected_particles: diagnostics.rejected_particles,
        clamped_particles: diagnostics.clamped_particles,
        neighbor_checks: diagnostics.neighbor_checks,
        influence_samples: diagnostics.influence_samples,
        impulses_applied: diagnostics.impulses_applied,
        body_collisions: diagnostics.body_collisions,
        max_speed: diagnostics.max_speed,
        first_position: first.position,
        first_velocity: first.velocity,
    }
}
