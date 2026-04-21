use boxdd::{ApiError, BodyBuilder, Polygon, ShapeDef, World, WorldDef, shapes};

#[test]
fn world_try_shape_set_geometry_rejects_invalid_values() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().build());
    let def = ShapeDef::default();
    let shape_id =
        world.create_circle_shape_for(body_id, &def, &shapes::circle([0.0_f32, 0.0], 0.5));

    assert_eq!(
        world
            .try_shape_set_segment(shape_id, &shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]))
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    let mut raw_polygon = shapes::box_polygon(0.5, 0.5).into_raw();
    raw_polygon.radius = -1.0;
    assert_eq!(
        world
            .try_shape_set_polygon(shape_id, &Polygon::from_raw(raw_polygon))
            .unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn owned_shape_try_set_geometry_rejects_invalid_values() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyBuilder::new().build());
    let def = ShapeDef::default();
    let mut shape =
        world.create_circle_shape_for_owned(body_id, &def, &shapes::circle([0.0_f32, 0.0], 0.5));

    assert_eq!(
        shape
            .try_set_circle(&shapes::circle([f32::NAN, 0.0], 0.5))
            .unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        shape
            .try_set_capsule(&shapes::capsule([0.0_f32, 0.0], [0.0_f32, 0.0], 0.25))
            .unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn safe_shape_creation_panics_on_invalid_geometry() {
    let world_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut world = World::new(WorldDef::default()).unwrap();
        let body_id = world.create_body_id(BodyBuilder::new().build());
        world.create_circle_shape_for(
            body_id,
            &ShapeDef::default(),
            &shapes::circle([f32::NAN, 0.0], 0.5),
        );
    }));
    assert!(world_result.is_err());

    let body_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut world = World::new(WorldDef::default()).unwrap();
        let mut body = world.create_body(BodyBuilder::new().build());
        body.create_segment_shape(
            &ShapeDef::default(),
            &shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]),
        );
    }));
    assert!(body_result.is_err());
}
