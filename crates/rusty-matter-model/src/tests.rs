use super::*;

#[test]
fn schema_id_accepts_matter_prefix() {
    let schema = MatterSchemaId::new(TRIANGLE_MESH_SCHEMA_ID).expect("schema is valid");
    assert_eq!(schema.as_str(), TRIANGLE_MESH_SCHEMA_ID);
}

#[test]
fn schema_id_rejects_non_matter_prefix() {
    assert!(MatterSchemaId::new("rusty.optics.mesh.triangle_mesh.v1").is_err());
}

#[test]
fn triangle_mesh_rejects_bad_indices() {
    let mesh = TriangleMeshSnapshot::new(
        "mesh.bad_index",
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        vec![[0, 1, 3]],
    );
    assert!(matches!(
        mesh.validate(),
        Err(MatterModelError::IndexOutOfRange { .. })
    ));
}

#[test]
fn bounds_from_points_computes_extents() {
    let bounds = Bounds3::from_points(&[
        Vec3::new(1.0, 2.0, -1.0),
        Vec3::new(-1.0, 3.0, 5.0),
        Vec3::new(0.0, -2.0, 2.0),
    ])
    .expect("bounds are valid");
    assert_eq!(bounds.min, Vec3::new(-1.0, -2.0, -1.0));
    assert_eq!(bounds.max, Vec3::new(1.0, 3.0, 5.0));
}
