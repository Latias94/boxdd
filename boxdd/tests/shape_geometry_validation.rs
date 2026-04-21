use boxdd::{ApiError, BodyBuilder, Polygon, ShapeDef, World, WorldDef, shapes};

fn assert_cast_output_eq(actual: boxdd::CastOutput, expected: boxdd::CastOutput) {
    assert_eq!(actual.normal, expected.normal);
    assert_eq!(actual.point, expected.point);
    assert_eq!(actual.fraction, expected.fraction);
    assert_eq!(actual.iterations, expected.iterations);
    assert_eq!(actual.hit, expected.hit);
}

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

#[test]
fn standalone_geometry_try_helpers_reject_invalid_inputs() {
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let invalid_transform = boxdd::Transform::from_pos_angle([f32::NAN, 0.0], 0.0);

    assert_eq!(
        circle.try_mass_data(-1.0).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        circle.try_aabb(invalid_transform).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        circle.try_contains_point([f32::NAN, 0.0]).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        circle
            .try_ray_cast([0.0_f32, 0.0], [f32::NAN, 0.0])
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    assert_eq!(
        shapes::circle([f32::NAN, 0.0], 0.5)
            .try_mass_data(1.0)
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    let polygon = shapes::box_polygon(1.0, 1.0);
    assert_eq!(
        polygon.try_transformed(invalid_transform).unwrap_err(),
        ApiError::InvalidArgument
    );

    let mut raw_polygon = polygon.into_raw();
    raw_polygon.radius = -1.0;
    let invalid_polygon = Polygon::from_raw(raw_polygon);
    assert_eq!(
        invalid_polygon
            .try_aabb(boxdd::Transform::IDENTITY)
            .unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn safe_standalone_geometry_helpers_panic_on_invalid_inputs() {
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let invalid_transform = boxdd::Transform::from_pos_angle([f32::NAN, 0.0], 0.0);

    let mass_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        circle.mass_data(-1.0);
    }));
    assert!(mass_result.is_err());

    let aabb_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        circle.aabb(invalid_transform);
    }));
    assert!(aabb_result.is_err());

    let point_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        circle.contains_point([f32::NAN, 0.0]);
    }));
    assert!(point_result.is_err());

    let ray_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        circle.ray_cast([0.0_f32, 0.0], [f32::NAN, 0.0]);
    }));
    assert!(ray_result.is_err());

    let polygon = shapes::box_polygon(1.0, 1.0);
    let transform_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        polygon.transformed(invalid_transform);
    }));
    assert!(transform_result.is_err());
}

#[test]
fn degenerate_segment_and_capsule_helpers_remain_usable() {
    let segment = shapes::segment([0.0_f32, 0.0], [0.0_f32, 0.0]);
    assert_eq!(segment.validate().unwrap_err(), ApiError::InvalidArgument);
    assert_eq!(
        segment.try_aabb(boxdd::Transform::IDENTITY).unwrap(),
        segment.aabb(boxdd::Transform::IDENTITY)
    );
    assert_cast_output_eq(
        segment
            .try_ray_cast([-1.0_f32, 0.0], [2.0_f32, 0.0], false)
            .unwrap(),
        segment.ray_cast([-1.0_f32, 0.0], [2.0_f32, 0.0], false),
    );

    let capsule = shapes::capsule([0.0_f32, 0.0], [0.0_f32, 0.0], 0.5);
    assert_eq!(capsule.validate().unwrap_err(), ApiError::InvalidArgument);
    assert_eq!(capsule.try_mass_data(1.0).unwrap(), capsule.mass_data(1.0));
    assert_eq!(
        capsule.try_aabb(boxdd::Transform::IDENTITY).unwrap(),
        capsule.aabb(boxdd::Transform::IDENTITY)
    );
    assert_eq!(
        capsule.try_contains_point([0.0_f32, 0.0]).unwrap(),
        capsule.contains_point([0.0_f32, 0.0])
    );
    assert_cast_output_eq(
        capsule
            .try_ray_cast([-1.0_f32, 0.0], [2.0_f32, 0.0])
            .unwrap(),
        capsule.ray_cast([-1.0_f32, 0.0], [2.0_f32, 0.0]),
    );
}
