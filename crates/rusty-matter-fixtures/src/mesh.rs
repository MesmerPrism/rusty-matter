use rusty_matter_mesh::{
    DynamicMeshCollider, DynamicMeshColliderConfig, DynamicMeshColliderUpdateStatus,
    HandValidationMeshFrame, Handedness, MeshCoordinateFrameConfig, MeshCoordinateMap,
    MeshCoordinateMapPackage, MeshLocalDisplacementClampMode, MeshSourceDescriptor,
    MeshSurfaceSampleConfig, MeshSurfaceSamplePattern, TriangleMeshSurface,
};
use rusty_matter_model::Vec3;

use crate::error::CliError;
use crate::summary::{
    DynamicColliderSummary, HandValidationMeshSummary, MeshCoordinateMapPackageSummary,
    MeshCoordinateMapSummary, MeshSurfaceSampleSummary,
};

pub(crate) fn unit_square_surface() -> TriangleMeshSurface {
    TriangleMeshSurface::new(
        "mesh.unit_square_surface",
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    )
}

pub(crate) fn mesh_surface_sample_summary(
    surface: &TriangleMeshSurface,
) -> Result<MeshSurfaceSampleSummary, CliError> {
    let config = MeshSurfaceSampleConfig {
        sample_config_id: "mesh.surface_sample.unit_square_fixture".to_owned(),
        sample_set_id: "mesh.surface_samples.unit_square_fixture".to_owned(),
        point_count: 16,
        first_tier_neighbor_count: 3,
        second_tier_neighbor_count: 4,
        seed: 12_345,
        pattern: MeshSurfaceSamplePattern::LowDiscrepancy,
        ..MeshSurfaceSampleConfig::default()
    };
    let samples = surface.sample_points(&config).map_err(CliError::Mesh)?;
    let first_sample = samples
        .samples
        .first()
        .expect("fixture config requests samples");
    let first_counts = neighbor_count_range(&samples.first_tier_neighbors);
    let second_counts = neighbor_count_range(&samples.second_tier_neighbors);
    Ok(MeshSurfaceSampleSummary {
        schema_id: "rusty.matter.fixture.mesh_surface_sample_summary.v1".to_owned(),
        fixture_id: "fixture.mesh.unit_square_samples.v1".to_owned(),
        surface_id: surface.surface_id.clone(),
        topology_index_hash: surface.topology_key().index_hash,
        vertex_count: surface.vertex_count(),
        triangle_count: surface.triangle_count(),
        sample_count: samples.len(),
        pattern: sample_pattern_label(config.pattern).to_owned(),
        first_position: first_sample.position,
        first_normal: first_sample.normal,
        first_tier_min: first_counts.0,
        first_tier_max: first_counts.1,
        second_tier_min: second_counts.0,
        second_tier_max: second_counts.1,
    })
}

pub(crate) fn mesh_coordinate_map_summary(
    surface: &TriangleMeshSurface,
) -> Result<MeshCoordinateMapSummary, CliError> {
    let mut sample_config = MeshSurfaceSampleConfig::high_quality_surface_points(12);
    sample_config.sample_config_id = "mesh.surface_sample.coordinate_map_fixture".to_owned();
    sample_config.sample_set_id = "mesh.surface_samples.coordinate_map_fixture".to_owned();
    let frame_config = MeshCoordinateFrameConfig {
        frame_config_id: "mesh.coordinate_frame.coordinate_map_fixture".to_owned(),
        max_displacement: Vec3::new(0.02, 0.03, 0.04),
        clamp_mode: MeshLocalDisplacementClampMode::Ellipsoid,
        ..MeshCoordinateFrameConfig::default()
    };
    let coordinate_map = MeshCoordinateMap::from_surface(
        "mesh.coordinate_map.unit_square_fixture",
        surface,
        sample_config,
        frame_config,
    )
    .map_err(CliError::Mesh)?;
    let first_frame = coordinate_map
        .frames
        .frames
        .first()
        .expect("fixture config requests frames");
    Ok(MeshCoordinateMapSummary {
        schema_id: "rusty.matter.fixture.mesh_coordinate_map_summary.v1".to_owned(),
        fixture_id: "fixture.mesh.unit_square_coordinate_map.v1".to_owned(),
        coordinate_map_id: coordinate_map.coordinate_map_id,
        surface_id: coordinate_map.samples.surface_id.clone(),
        topology_index_hash: coordinate_map.topology_key.index_hash,
        sample_count: coordinate_map.samples.len(),
        frame_count: coordinate_map.frames.frames.len(),
        clamp_mode: clamp_mode_label(coordinate_map.frames.clamp_mode).to_owned(),
        first_anchor: first_frame.anchor,
        first_axis_z: first_frame.axis_z,
        first_displaced_point: first_frame
            .displace(Vec3::new(0.5, -0.25, 2.0), coordinate_map.frames.clamp_mode),
    })
}

pub(crate) fn mesh_coordinate_map_package_summary(
    surface: &TriangleMeshSurface,
) -> Result<MeshCoordinateMapPackageSummary, CliError> {
    let mut sample_config =
        MeshSurfaceSampleConfig::high_quality_surface_points(10).without_neighbors();
    sample_config.sample_config_id = "mesh.surface_sample.package_fixture".to_owned();
    sample_config.sample_set_id = "mesh.surface_samples.package_fixture".to_owned();
    let coordinate_map = MeshCoordinateMap::from_surface(
        "mesh.coordinate_map.package_fixture",
        surface,
        sample_config,
        MeshCoordinateFrameConfig::default(),
    )
    .map_err(CliError::Mesh)?;
    let source = MeshSourceDescriptor::new(
        "mesh.source.unit_square_procedural",
        "procedural:unit_square",
        "procedural",
        "procedural.unit_square.v1",
        "AGPL-3.0-or-later",
        "Rusty Matter procedural fixture",
    );
    let package = MeshCoordinateMapPackage::new(
        "mesh.coordinate_map_package.unit_square_fixture",
        source,
        surface.clone(),
        coordinate_map,
    );
    package.validate().map_err(CliError::Mesh)?;
    let first_sample = package
        .coordinate_map
        .samples
        .samples
        .first()
        .expect("fixture config requests samples");
    let first_anchor = first_sample.position;
    let first_normal = first_sample.normal;
    Ok(MeshCoordinateMapPackageSummary {
        schema_id: "rusty.matter.fixture.mesh_coordinate_map_package_summary.v1".to_owned(),
        fixture_id: "fixture.mesh.unit_square_coordinate_map_package.v1".to_owned(),
        package_id: package.package_id,
        source_id: package.source.source_id,
        source_format: package.source.source_format,
        source_hash: package.source.source_hash,
        surface_id: package.surface.surface_id,
        coordinate_map_id: package.coordinate_map.coordinate_map_id,
        topology_index_hash: package.coordinate_map.topology_key.index_hash,
        sample_count: package.coordinate_map.samples.len(),
        has_same_surface_neighbors: package
            .coordinate_map
            .samples
            .first_tier_neighbors
            .iter()
            .chain(package.coordinate_map.samples.second_tier_neighbors.iter())
            .any(|neighbors| !neighbors.is_empty()),
        first_anchor,
        first_normal,
    })
}

pub(crate) fn dynamic_collider_summary(
    surface: &TriangleMeshSurface,
) -> Result<DynamicColliderSummary, CliError> {
    let mut collider = DynamicMeshCollider::new(DynamicMeshColliderConfig {
        collider_config_id: "mesh.dynamic_collider.unit_square_fixture".to_owned(),
        surface_inflation: 0.05,
        contact_padding: 0.01,
        prefer_convex: true,
        diagnostic_shell_inflation: 0.01,
        ..DynamicMeshColliderConfig::default()
    });
    let update = collider.update_from_surface(surface);
    let contact = collider
        .closest_point(Vec3::new(0.25, 0.25, 0.10))
        .ok_or(CliError::MissingColliderContact)?;
    Ok(DynamicColliderSummary {
        schema_id: "rusty.matter.fixture.dynamic_collider_summary.v1".to_owned(),
        fixture_id: "fixture.mesh.unit_square_dynamic_collider.v1".to_owned(),
        surface_id: surface.surface_id.clone(),
        status: collider_status_label(update.status).to_owned(),
        vertex_count: update.vertex_count,
        triangle_count: update.triangle_count,
        diagnostic_shell_vertex_count: update.diagnostic_shell_vertex_count,
        diagnostic_shell_triangle_count: update.diagnostic_shell_triangle_count,
        closest_point: contact.point,
        closest_distance: contact.distance,
        overlaps_probe_sphere: collider.overlaps_sphere(Vec3::new(0.25, 0.25, 0.10), 0.06),
    })
}

pub(crate) fn synthetic_hand_validation_mesh_frame() -> HandValidationMeshFrame {
    let mut surface = unit_square_surface();
    surface.surface_id = "mesh.synthetic_meta_hand_patch.left".to_owned();
    for (index, position) in surface.positions.iter_mut().enumerate() {
        position.z = index as f32 * 0.01;
    }
    let mut frame = HandValidationMeshFrame::from_surface(
        "hand.validation_mesh.synthetic_left.0001",
        Handedness::Left,
        "local_floor",
        "meta.hand_tracking_mesh",
        0.125,
        surface,
    );
    frame.normals = vec![Vec3::new(0.0, 0.0, 1.0); frame.surface.vertex_count()];
    frame
}

pub(crate) fn hand_validation_mesh_summary(
    frame: &HandValidationMeshFrame,
) -> Result<HandValidationMeshSummary, CliError> {
    frame.validate().map_err(CliError::Mesh)?;
    Ok(HandValidationMeshSummary {
        schema_id: "rusty.matter.fixture.hand_validation_mesh_summary.v1".to_owned(),
        fixture_id: "fixture.hand.synthetic_validation_mesh.v1".to_owned(),
        frame_id: frame.frame_id.clone(),
        handedness: handedness_label(frame.handedness).to_owned(),
        source: frame.source.clone(),
        surface_id: frame.surface.surface_id.clone(),
        topology_index_hash: frame.topology_key.index_hash,
        vertex_count: frame.surface.vertex_count(),
        triangle_count: frame.surface.triangle_count(),
    })
}

fn neighbor_count_range(neighbors: &[Vec<usize>]) -> (usize, usize) {
    let min = neighbors
        .iter()
        .map(Vec::len)
        .min()
        .expect("fixture has neighbors");
    let max = neighbors
        .iter()
        .map(Vec::len)
        .max()
        .expect("fixture has neighbors");
    (min, max)
}

fn sample_pattern_label(pattern: MeshSurfaceSamplePattern) -> &'static str {
    match pattern {
        MeshSurfaceSamplePattern::AreaStratified => "area_stratified",
        MeshSurfaceSamplePattern::LowDiscrepancy => "low_discrepancy",
    }
}

fn clamp_mode_label(mode: MeshLocalDisplacementClampMode) -> &'static str {
    match mode {
        MeshLocalDisplacementClampMode::PerAxis => "per_axis",
        MeshLocalDisplacementClampMode::Ellipsoid => "ellipsoid",
    }
}

fn collider_status_label(status: DynamicMeshColliderUpdateStatus) -> &'static str {
    match status {
        DynamicMeshColliderUpdateStatus::Disabled => "disabled",
        DynamicMeshColliderUpdateStatus::Initialized => "initialized",
        DynamicMeshColliderUpdateStatus::Updated => "updated",
        DynamicMeshColliderUpdateStatus::ChangedTopology => "changed_topology",
        DynamicMeshColliderUpdateStatus::InvalidSurface => "invalid_surface",
    }
}

fn handedness_label(handedness: Handedness) -> &'static str {
    match handedness {
        Handedness::Unknown => "unknown",
        Handedness::Left => "left",
        Handedness::Right => "right",
    }
}
