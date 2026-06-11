use super::*;
use rusty_matter_model::Vec3;

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

fn grid_surface(rows: usize, cols: usize) -> TriangleMeshSurface {
    let mut positions = Vec::with_capacity((rows + 1) * (cols + 1));
    for row in 0..=rows {
        for col in 0..=cols {
            positions.push(Vec3::new(
                col as f32 / cols as f32,
                row as f32 / rows as f32,
                0.0,
            ));
        }
    }

    let mut triangles = Vec::with_capacity(rows * cols * 2);
    let stride = cols + 1;
    for row in 0..rows {
        for col in 0..cols {
            let a = (row * stride + col) as u32;
            let b = a + 1;
            let c = ((row + 1) * stride + col) as u32;
            let d = c + 1;
            triangles.push([a, b, d]);
            triangles.push([a, d, c]);
        }
    }

    TriangleMeshSurface::new("mesh.grid", positions, triangles)
}

#[test]
fn surface_validates_and_hashes_topology() {
    let surface = unit_square_surface();

    surface.validate().expect("surface validates");
    assert_eq!(surface.vertex_count(), 4);
    assert_eq!(surface.triangle_count(), 2);
    assert_eq!(surface.topology_key().triangle_count, 2);
}

#[test]
fn sampler_is_deterministic() {
    let surface = unit_square_surface();
    let config = MeshSurfaceSampleConfig {
        sample_set_id: "samples.unit_square".to_owned(),
        point_count: 8,
        first_tier_neighbor_count: 2,
        second_tier_neighbor_count: 2,
        seed: 42,
        ..MeshSurfaceSampleConfig::default()
    };

    let first = sample_mesh_surface_points(&surface, &config).expect("samples build");
    let second = sample_mesh_surface_points(&surface, &config).expect("samples build");

    assert_eq!(first, second);
    assert_eq!(first.len(), 8);
    assert!(first.is_valid());
}

#[test]
fn high_quality_coordinate_map_builds_local_frames() {
    let surface = unit_square_surface();
    let mut sample_config = MeshSurfaceSampleConfig::high_quality_surface_points(12);
    sample_config.sample_set_id = "samples.coordinate_map".to_owned();
    let frame_config = MeshCoordinateFrameConfig {
        frame_config_id: "mesh.coordinate_frame.test".to_owned(),
        max_displacement: Vec3::new(0.02, 0.03, 0.04),
        clamp_mode: MeshLocalDisplacementClampMode::Ellipsoid,
        ..MeshCoordinateFrameConfig::default()
    };

    let coordinate_map = MeshCoordinateMap::from_surface(
        "mesh.coordinate_map.unit_square",
        &surface,
        sample_config,
        frame_config,
    )
    .expect("coordinate map builds");
    let displaced = coordinate_map.frames.frames[0]
        .displace(Vec3::new(2.0, 0.0, 0.0), coordinate_map.frames.clamp_mode);

    assert!(coordinate_map.is_valid_for_surface(&surface));
    assert_eq!(coordinate_map.samples.len(), 12);
    assert_eq!(coordinate_map.frames.frames.len(), 12);
    assert!(
        (displaced - coordinate_map.frames.frames[0].anchor).length()
            <= coordinate_map.frames.frames[0].max_displacement.x + 1.0e-5
    );
}

#[test]
fn live_sampler_updates_deformed_surface() {
    let surface = unit_square_surface();
    let mut deformed = surface.clone();
    for position in &mut deformed.positions {
        position.z += 0.125;
    }
    let config = MeshSurfaceSampleConfig {
        sample_set_id: "samples.deformed".to_owned(),
        point_count: 4,
        ..MeshSurfaceSampleConfig::default()
    };
    let mut sampler = LiveMeshSurfaceSampler::new(config);

    let first = sampler.update_from_surface(&surface);
    let old_positions = sampler.samples().expect("samples exist").positions();
    let second = sampler.update_from_surface(&deformed);
    let samples = sampler.samples().expect("samples exist");

    assert_eq!(first.status, LiveMeshSurfaceUpdateStatus::Initialized);
    assert_eq!(second.status, LiveMeshSurfaceUpdateStatus::Updated);
    for (old, sample) in old_positions.iter().copied().zip(samples.samples.iter()) {
        assert!((sample.position - (old + Vec3::new(0.0, 0.0, 0.125))).length() < 1.0e-5);
    }
}

#[test]
fn live_sampler_resamples_changed_topology() {
    let surface = unit_square_surface();
    let mut changed = surface.clone();
    changed.triangles.push([0, 2, 1]);
    let config = MeshSurfaceSampleConfig {
        sample_set_id: "samples.changed".to_owned(),
        point_count: 4,
        ..MeshSurfaceSampleConfig::default()
    };
    let mut sampler = LiveMeshSurfaceSampler::new(config);

    let first = sampler.update_from_surface(&surface);
    let second = sampler.update_from_surface(&changed);

    assert_eq!(first.status, LiveMeshSurfaceUpdateStatus::Initialized);
    assert_eq!(
        second.status,
        LiveMeshSurfaceUpdateStatus::ResampledTopology
    );
    assert_ne!(first.topology_key, second.topology_key);
}

#[test]
fn distance_sampler_returns_closest_surface_point() {
    let surface = unit_square_surface();
    let sampler = surface
        .distance_sampler(SurfaceDistanceSamplerConfig::default())
        .expect("distance sampler builds");
    let sample = sampler
        .sample(Vec3::new(0.25, 0.25, 0.2))
        .expect("sample exists");

    assert!((sample.point - Vec3::new(0.25, 0.25, 0.0)).length() < 1.0e-5);
    assert!((sample.distance - 0.2).abs() < 1.0e-5);
    assert_eq!(sample.diagnostics.triangle_tests, surface.triangle_count());
    assert_eq!(sampler.topology_key(), &surface.topology_key());
}

#[test]
fn distance_sampler_refits_deformed_surface_without_rebuilding_tree() {
    let surface = grid_surface(6, 6);
    let mut deformed = surface.clone();
    for position in &mut deformed.positions {
        position.z += 0.125;
    }
    let mut sampler = surface
        .distance_sampler(SurfaceDistanceSamplerConfig {
            leaf_triangle_count: 4,
            ..SurfaceDistanceSamplerConfig::default()
        })
        .expect("distance sampler builds");
    let original_stats = sampler.stats().clone();

    let refit_stats = sampler
        .refit_from_surface(&deformed)
        .expect("same-topology refit succeeds");
    let sample = sampler
        .sample(Vec3::new(0.25, 0.25, 0.25))
        .expect("sample exists after refit");

    assert_eq!(refit_stats, original_stats);
    assert_eq!(sampler.stats(), &original_stats);
    assert_eq!(sampler.topology_key(), &deformed.topology_key());
    assert!((sample.distance - 0.125).abs() < 1.0e-4);
}

#[test]
fn distance_sampler_refit_rejects_changed_topology() {
    let surface = unit_square_surface();
    let mut changed = surface.clone();
    changed.triangles.push([0, 2, 1]);
    let mut sampler = surface
        .distance_sampler(SurfaceDistanceSamplerConfig::default())
        .expect("distance sampler builds");

    let error = sampler.refit_from_surface(&changed).unwrap_err();

    assert_eq!(error, MatterMeshError::ChangedTopology);
}

#[test]
fn distance_sampler_prunes_dense_surface_queries() {
    let surface = grid_surface(24, 24);
    let sampler = surface
        .distance_sampler(SurfaceDistanceSamplerConfig {
            leaf_triangle_count: 6,
            ..SurfaceDistanceSamplerConfig::default()
        })
        .expect("distance sampler builds");
    let sample = sampler
        .sample(Vec3::new(0.36, 0.72, 0.15))
        .expect("sample exists");

    assert_eq!(surface.triangle_count(), 1_152);
    assert!(sampler.stats().node_count > 1);
    assert!(sample.diagnostics.node_tests < sampler.stats().node_count);
    assert!(
        sample.diagnostics.triangle_tests < surface.triangle_count() / 8,
        "expected BVH to test far fewer triangles, tested {} of {}",
        sample.diagnostics.triangle_tests,
        surface.triangle_count()
    );
    assert!((sample.distance - 0.15).abs() < 1.0e-4);
}

#[test]
fn hand_validation_mesh_frame_reuses_generic_surface() {
    let surface = unit_square_surface();
    let mut frame = HandValidationMeshFrame::from_surface(
        "hand.validation_mesh.left.0001",
        Handedness::Left,
        "local_floor",
        "meta.hand_tracking_mesh",
        0.5,
        surface.clone(),
    );
    frame.normals = vec![Vec3::new(0.0, 0.0, 1.0); surface.vertex_count()];

    let config = MeshSurfaceSampleConfig {
        sample_set_id: "samples.hand_validation_mesh".to_owned(),
        point_count: 6,
        ..MeshSurfaceSampleConfig::default()
    };
    let samples = frame
        .surface()
        .sample_points(&config)
        .expect("hand mesh samples through generic surface");
    let mut collider = DynamicMeshCollider::default();
    let update = collider.update_from_surface(frame.surface());

    frame.validate().expect("hand frame validates");
    assert_eq!(frame.topology_key, surface.topology_key());
    assert_eq!(samples.topology_key, surface.topology_key());
    assert_eq!(update.status, DynamicMeshColliderUpdateStatus::Initialized);
}

#[test]
fn hand_rig_and_joint_frame_validate_recording_payloads() {
    let surface = unit_square_surface();
    let mut rig = HandRigCapture::from_bind_surface(
        "hand.rig_capture.left",
        Handedness::Left,
        "local_floor",
        "meta.hand_tracking_mesh",
        surface,
    );
    rig.joint_parent_indices = vec![-1, 0];
    rig.joint_radii_m = vec![0.01, 0.008];
    rig.joint_bind_poses = vec![
        HandJointPose {
            position: Vec3::ZERO,
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            radius_m: 0.01,
        },
        HandJointPose {
            position: Vec3::new(0.1, 0.0, 0.0),
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            radius_m: 0.008,
        },
    ];
    rig.vertex_joint_indices = vec![[0, 1, 0, 0]; rig.bind_surface.vertex_count()];
    rig.vertex_joint_weights = vec![[0.75, 0.25, 0.0, 0.0]; rig.bind_surface.vertex_count()];

    let joint_frame = HandJointFrame {
        schema_id: HAND_JOINT_FRAME_SCHEMA_ID.to_owned(),
        frame_id: "hand.joint_frame.left.0001".to_owned(),
        handedness: Handedness::Left,
        reference_space: "local_floor".to_owned(),
        source: "meta.hand_tracking_mesh".to_owned(),
        time_seconds: 0.25,
        poses: vec![
            HandJointPose {
                position: Vec3::ZERO,
                orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
                radius_m: 0.01,
            },
            HandJointPose {
                position: Vec3::new(0.1, 0.0, 0.0),
                orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
                radius_m: 0.008,
            },
        ],
        confidence: vec![1.0, 0.9],
    };

    rig.validate().expect("rig validates");
    joint_frame.validate().expect("joint frame validates");
}

#[test]
fn hand_rig_skins_joint_frame_to_validation_mesh() {
    let surface = unit_square_surface();
    let mut rig = HandRigCapture::from_bind_surface(
        "hand.rig_capture.skinning",
        Handedness::Left,
        "local_floor",
        "meta.hand_tracking_mesh",
        surface,
    );
    rig.bind_normals = vec![Vec3::new(0.0, 0.0, 1.0); rig.bind_surface.vertex_count()];
    rig.joint_parent_indices = vec![-1, 0];
    rig.joint_radii_m = vec![0.01, 0.008];
    rig.joint_bind_poses = vec![
        HandJointPose {
            position: Vec3::ZERO,
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            radius_m: 0.01,
        },
        HandJointPose {
            position: Vec3::new(1.0, 0.0, 0.0),
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            radius_m: 0.008,
        },
    ];
    rig.vertex_joint_indices = vec![[0, 0, 0, 0], [1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 0, 0]];
    rig.vertex_joint_weights = vec![
        [1.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.5, 0.5, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
    ];
    let joint_frame = HandJointFrame {
        schema_id: HAND_JOINT_FRAME_SCHEMA_ID.to_owned(),
        frame_id: "hand.joint_frame.skinning.0001".to_owned(),
        handedness: Handedness::Left,
        reference_space: "local_floor".to_owned(),
        source: "meta.hand_tracking_mesh".to_owned(),
        time_seconds: 0.25,
        poses: vec![
            HandJointPose {
                position: Vec3::new(0.0, 0.0, 0.25),
                orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
                radius_m: 0.01,
            },
            HandJointPose {
                position: Vec3::new(1.0, 0.0, 0.5),
                orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
                radius_m: 0.008,
            },
        ],
        confidence: vec![1.0, 1.0],
    };

    let actual = rig
        .skin_to_validation_frame(&joint_frame, "hand.validation_mesh.skinning.0001")
        .expect("rig skins");
    let expected_surface = TriangleMeshSurface::new(
        "hand.validation_mesh.skinning.0001.surface",
        vec![
            Vec3::new(0.0, 0.0, 0.25),
            Vec3::new(1.0, 0.0, 0.5),
            Vec3::new(1.0, 1.0, 0.375),
            Vec3::new(0.0, 1.0, 0.25),
        ],
        rig.bind_surface.triangles.clone(),
    );
    let mut expected = HandValidationMeshFrame::from_surface(
        "hand.validation_mesh.skinning.0001",
        Handedness::Left,
        "local_floor",
        "meta.hand_tracking_mesh",
        0.25,
        expected_surface,
    );
    expected.normals = vec![Vec3::new(0.0, 0.0, 1.0); 4];

    let comparison = expected
        .compare_with(&actual, HandValidationMeshTolerance::default())
        .expect("comparison builds");

    actual.validate().expect("skinned frame validates");
    assert!(comparison.passed, "{comparison:?}");
    assert_eq!(comparison.position_mismatch_count, 0);
    assert_eq!(comparison.normal_mismatch_count, 0);
    assert_eq!(actual.topology_key, rig.bind_surface.topology_key());

    let samples = rig
        .skinning_matrix_samples(&joint_frame, 4)
        .expect("matrix samples build");
    assert_eq!(samples.len(), 4);
    let blended = samples[2];
    assert_eq!(blended.vertex_index, 2);
    assert_eq!(blended.bind_position, [1.0, 1.0, 0.0, 1.0]);
    assert_eq!(blended.joint_indices, [0, 1, 0, 0]);
    assert_eq!(blended.joint_weights, [0.5, 0.5, 0.0, 0.0]);
    assert_eq!(blended.joint_matrices[0][2][3], 0.25);
    assert_eq!(blended.joint_matrices[1][2][3], 0.5);
    assert_eq!(blended.expected_position, [1.0, 1.0, 0.375, 1.0]);
}

#[test]
fn hand_validation_comparison_reports_position_mismatch() {
    let surface = unit_square_surface();
    let mut shifted = surface.clone();
    shifted.positions[0].z = 0.01;
    let expected = HandValidationMeshFrame::from_surface(
        "hand.validation_mesh.expected",
        Handedness::Left,
        "local_floor",
        "meta.hand_tracking_mesh",
        0.25,
        surface,
    );
    let actual = HandValidationMeshFrame::from_surface(
        "hand.validation_mesh.actual",
        Handedness::Left,
        "local_floor",
        "meta.hand_tracking_mesh",
        0.25,
        shifted,
    );

    let comparison = expected
        .compare_with(
            &actual,
            HandValidationMeshTolerance {
                max_position_error_m: 0.001,
                max_normal_error: 0.001,
            },
        )
        .expect("comparison builds");

    assert!(!comparison.passed);
    assert_eq!(comparison.position_mismatch_count, 1);
    assert!(comparison.max_position_error_m > 0.009);
}

#[test]
fn dynamic_collider_inflates_and_queries_surface() {
    let surface = unit_square_surface();
    let mut collider = DynamicMeshCollider::new(DynamicMeshColliderConfig {
        surface_inflation: 0.1,
        contact_padding: 0.01,
        prefer_convex: true,
        ..DynamicMeshColliderConfig::default()
    });

    let update = collider.update_from_surface(&surface);
    let contact = collider
        .closest_point(Vec3::new(0.25, 0.25, 0.2))
        .expect("contact is available");

    assert_eq!(update.status, DynamicMeshColliderUpdateStatus::Initialized);
    assert_eq!(update.vertex_count, 4);
    assert_eq!(update.triangle_count, 2);
    assert!(update.convex_eligible);
    assert!(collider.diagnostic_shell().is_some());
    assert!(collider.distance_sampler().is_some());
    assert!(contact.distance < 0.11);
    assert!(collider.overlaps_sphere(Vec3::new(0.25, 0.25, 0.2), 0.11));
}

#[test]
fn invalid_surface_rejects_bad_indices() {
    let surface = TriangleMeshSurface::new(
        "mesh.bad",
        vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)],
        vec![[0, 1, 2]],
    );

    assert!(matches!(
        surface.validate(),
        Err(MatterMeshError::IndexOutOfRange { .. })
    ));
}
