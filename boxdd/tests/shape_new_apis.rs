use boxdd::Filter;
use boxdd::prelude::*;
use boxdd::shapes;

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

fn approx_vec2(a: Vec2, b: Vec2, eps: f32) -> bool {
    approx_eq(a.x, b.x, eps) && approx_eq(a.y, b.y, eps)
}

fn approx_mass_data(a: MassData, b: MassData, eps: f32) -> bool {
    approx_eq(a.mass, b.mass, eps)
        && approx_vec2(a.center, b.center, eps)
        && approx_eq(a.rotational_inertia, b.rotational_inertia, eps)
}

fn same_chain_id(a: ChainId, b: ChainId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

#[test]
fn shape_closest_point_and_apply_wind_smoke() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let mut shape = world.create_polygon_shape_for_owned(body, &sdef, &poly);

    let target = Vec2::new(10.0, 0.0);
    let cp1 = shape.closest_point(target);
    let cp2 = world.shape_closest_point(shape.id(), target);
    assert!(approx_eq(cp1.x, cp2.x, 1e-6) && approx_eq(cp1.y, cp2.y, 1e-6));
    assert!(approx_eq(cp1.x, 0.5, 1e-3) && approx_eq(cp1.y, 0.0, 1e-3));

    let cp3 = shape.try_closest_point(target).unwrap();
    assert!(approx_eq(cp1.x, cp3.x, 1e-6) && approx_eq(cp1.y, cp3.y, 1e-6));

    world
        .try_shape_closest_point(shape.id(), target)
        .expect("try_shape_closest_point should succeed");

    shape.apply_wind(Vec2::new(5.0, 0.0), 1.0, 0.5, true);
    shape
        .try_apply_wind(Vec2::new(5.0, 0.0), 1.0, 0.5, true)
        .unwrap();
    world.shape_apply_wind(shape.id(), Vec2::new(5.0, 0.0), 1.0, 0.5, true);
    world
        .try_shape_apply_wind(shape.id(), Vec2::new(5.0, 0.0), 1.0, 0.5, true)
        .unwrap();
}

#[test]
fn shape_geometry_roundtrip_uses_safe_value_types() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder()
        .density(1.0)
        .filter(Filter::default())
        .build();

    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let mut circle_shape = world.create_circle_shape_for_owned(body, &sdef, &circle);
    assert_eq!(circle_shape.circle(), circle);

    let updated_circle = shapes::circle([0.0_f32, 0.0], 0.25);
    circle_shape.set_circle(&updated_circle);
    assert_eq!(circle_shape.circle(), updated_circle);

    let poly = shapes::box_polygon(0.5, 0.5);
    let poly_shape = world.create_polygon_shape_for_owned(body, &sdef, &poly);
    assert_eq!(poly_shape.polygon().count(), 4);
    assert!(approx_eq(
        poly_shape.polygon().radius(),
        poly.radius(),
        f32::EPSILON
    ));

    let wider = shapes::box_polygon(1.0, 0.25);
    world.shape_set_polygon(poly_shape.id(), &wider);
    let updated = poly_shape.polygon();
    assert_eq!(updated.count(), 4);
    assert!(approx_eq(updated.vertices()[0].x.abs(), 1.0, 1.0e-6));

    let chain = world.create_chain_for_owned(
        body,
        &boxdd::shapes::chain::ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build(),
    );
    let segment_shape_id = chain.segments()[0];
    let segment_shape = world
        .shape(segment_shape_id)
        .expect("chain segment shape should exist");
    assert_eq!(segment_shape.shape_type(), ShapeType::ChainSegment);
    assert!(
        segment_shape
            .parent_chain_id()
            .is_some_and(|parent| same_chain_id(parent, chain.id()))
    );

    let chain_segment = segment_shape.chain_segment();
    assert!(chain_segment.ghost1.x <= chain_segment.segment.point1.x);
    assert!(chain_segment.segment.point1.x < chain_segment.segment.point2.x);
    assert!(chain_segment.segment.point2.x <= chain_segment.ghost2.x);
}

#[test]
fn geometry_value_types_round_trip_through_explicit_raw_conversions() {
    let circle = shapes::circle([1.0_f32, -2.0], 0.5);
    assert_eq!(Circle::from_raw(circle.into_raw()), circle);

    let segment = shapes::segment([-1.0_f32, 2.0], [3.0, 4.0]);
    assert_eq!(Segment::from_raw(segment.into_raw()), segment);

    let chain_segment = shapes::chain_segment([-3.0_f32, 0.0], [-1.0, 0.0], [2.0, 0.0], [4.0, 0.0]);
    assert_eq!(
        ChainSegment::from_raw(chain_segment.into_raw()),
        chain_segment
    );

    let capsule = shapes::capsule([-1.0_f32, -1.0], [1.0, 1.0], 0.25);
    assert_eq!(Capsule::from_raw(capsule.into_raw()), capsule);

    let polygon = shapes::box_polygon(1.5, 0.75);
    let polygon_roundtrip = Polygon::from_raw(polygon.into_raw());
    assert_eq!(polygon_roundtrip.count(), polygon.count());
    assert!(approx_eq(
        polygon_roundtrip.radius(),
        polygon.radius(),
        f32::EPSILON
    ));
    assert!(approx_eq(
        polygon_roundtrip.centroid().x,
        polygon.centroid().x,
        f32::EPSILON
    ));
    assert!(approx_eq(
        polygon_roundtrip.centroid().y,
        polygon.centroid().y,
        f32::EPSILON
    ));
    for (lhs, rhs) in polygon_roundtrip.vertices().iter().zip(polygon.vertices()) {
        assert!(approx_eq(lhs.x, rhs.x, f32::EPSILON));
        assert!(approx_eq(lhs.y, rhs.y, f32::EPSILON));
    }
}

#[test]
fn shape_filters_use_safe_values_with_explicit_raw_escape_hatch() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let filter = Filter {
        category_bits: 0x0002,
        mask_bits: 0x0004,
        group_index: -3,
    };
    assert_eq!(Filter::from_raw(filter.into_raw()), filter);

    let sdef = ShapeDef::builder().density(1.0).filter(filter).build();
    let mut shape =
        world.create_circle_shape_for_owned(body, &sdef, &shapes::circle([0.0_f32, 0.0], 0.5));
    assert_eq!(shape.filter(), filter);
    assert_eq!(shape.try_filter().unwrap(), filter);

    let updated_filter = Filter {
        category_bits: 0x0010,
        mask_bits: 0x0020,
        group_index: 7,
    };
    shape.set_filter(updated_filter);
    assert_eq!(shape.filter(), updated_filter);

    shape.try_set_filter(filter).unwrap();
    assert_eq!(shape.filter(), filter);
}

#[test]
fn shape_runtime_queries_and_mass_data_match_safe_geometry_helpers() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder().density(2.0).build();
    let circle = shapes::circle([0.0_f32, 0.0], 1.0);
    let shape_id = world.create_circle_shape_for(body, &sdef, &circle);

    let expected_aabb = circle.aabb(Transform::IDENTITY);
    let expected_mass_data = circle.mass_data(2.0);
    let expected_cast = circle.ray_cast([-2.0_f32, 0.0], [4.0, 0.0]);

    {
        let shape = world.shape(shape_id).expect("shape should still be valid");

        let aabb = shape.aabb();
        assert!(aabb.lower.x <= expected_aabb.lower.x);
        assert!(aabb.lower.y <= expected_aabb.lower.y);
        assert!(aabb.upper.x >= expected_aabb.upper.x);
        assert!(aabb.upper.y >= expected_aabb.upper.y);
        assert_eq!(shape.try_aabb().unwrap(), aabb);

        assert!(shape.test_point([0.25_f32, 0.0]));
        assert!(shape.try_test_point([0.25_f32, 0.0]).unwrap());
        assert!(!shape.test_point([1.5_f32, 0.0]));

        let cast = shape.ray_cast([-2.0_f32, 0.0], [4.0, 0.0]);
        assert_eq!(cast.hit, expected_cast.hit);
        assert!(approx_eq(cast.fraction, expected_cast.fraction, 1.0e-6));
        assert!(approx_vec2(cast.point, expected_cast.point, 1.0e-6));
        assert!(approx_vec2(cast.normal, expected_cast.normal, 1.0e-6));
        let try_cast = shape.try_ray_cast([-2.0_f32, 0.0], [4.0, 0.0]).unwrap();
        assert_eq!(try_cast.hit, cast.hit);
        assert!(approx_eq(try_cast.fraction, cast.fraction, 1.0e-6));
        assert!(approx_vec2(try_cast.point, cast.point, 1.0e-6));
        assert!(approx_vec2(try_cast.normal, cast.normal, 1.0e-6));

        let mass_data = shape.mass_data();
        assert!(approx_mass_data(mass_data, expected_mass_data, 1.0e-5));
        assert!(approx_mass_data(
            shape.try_mass_data().unwrap(),
            expected_mass_data,
            1.0e-5
        ));
    }

    let world_aabb = world.shape_aabb(shape_id);
    assert_eq!(world_aabb, world.try_shape_aabb(shape_id).unwrap());
    assert!(world_aabb.lower.x <= expected_aabb.lower.x);
    assert!(world_aabb.lower.y <= expected_aabb.lower.y);
    assert!(world_aabb.upper.x >= expected_aabb.upper.x);
    assert!(world_aabb.upper.y >= expected_aabb.upper.y);
    assert!(world.shape_test_point(shape_id, [0.25_f32, 0.0]));
    assert!(!world.shape_test_point(shape_id, [1.5_f32, 0.0]));

    let world_cast = world.shape_ray_cast(shape_id, [-2.0_f32, 0.0], [4.0, 0.0]);
    assert_eq!(world_cast.hit, expected_cast.hit);
    assert!(approx_eq(
        world_cast.fraction,
        expected_cast.fraction,
        1.0e-6
    ));
    assert!(approx_vec2(world_cast.point, expected_cast.point, 1.0e-6));
    assert!(approx_vec2(world_cast.normal, expected_cast.normal, 1.0e-6));

    let world_mass_data = world.shape_mass_data(shape_id);
    assert!(approx_mass_data(
        world_mass_data,
        expected_mass_data,
        1.0e-5
    ));
}

#[test]
fn shape_runtime_event_toggles_are_visible_across_owned_scoped_and_world_apis() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());

    let sensor_shape_id = world.create_circle_shape_for(
        body,
        &ShapeDef::builder().sensor(true).build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let mut owned = world.create_polygon_shape_for_owned(
        body,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );
    let contact_shape_id = owned.id();

    {
        let mut sensor_shape = world
            .shape(sensor_shape_id)
            .expect("sensor shape should still be valid");
        assert!(!sensor_shape.sensor_events_enabled());
        sensor_shape.enable_sensor_events(true);
        assert!(sensor_shape.sensor_events_enabled());
    }
    assert!(world.shape_sensor_events_enabled(sensor_shape_id));
    world.shape_enable_sensor_events(sensor_shape_id, false);
    assert!(!world.shape_sensor_events_enabled(sensor_shape_id));

    assert!(!owned.contact_events_enabled());
    assert!(!owned.pre_solve_events_enabled());
    assert!(!owned.hit_events_enabled());

    owned.enable_contact_events(true);
    owned.enable_pre_solve_events(true);
    owned.enable_hit_events(true);
    assert!(owned.contact_events_enabled());
    assert!(owned.pre_solve_events_enabled());
    assert!(owned.hit_events_enabled());
    assert!(owned.try_contact_events_enabled().unwrap());
    assert!(owned.try_pre_solve_events_enabled().unwrap());
    assert!(owned.try_hit_events_enabled().unwrap());

    assert!(world.shape_contact_events_enabled(contact_shape_id));
    assert!(world.shape_pre_solve_events_enabled(contact_shape_id));
    assert!(world.shape_hit_events_enabled(contact_shape_id));

    world.shape_enable_contact_events(contact_shape_id, false);
    world.shape_enable_pre_solve_events(contact_shape_id, false);
    world.shape_enable_hit_events(contact_shape_id, false);

    assert!(!world.shape_contact_events_enabled(contact_shape_id));
    assert!(!world.shape_pre_solve_events_enabled(contact_shape_id));
    assert!(!world.shape_hit_events_enabled(contact_shape_id));
}
