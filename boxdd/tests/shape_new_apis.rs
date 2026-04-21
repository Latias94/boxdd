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

fn approx_polygon(a: &Polygon, b: &Polygon, eps: f32) -> bool {
    a.count() == b.count()
        && approx_vec2(a.centroid(), b.centroid(), eps)
        && approx_eq(a.radius(), b.radius(), eps)
        && a.vertices()
            .iter()
            .zip(b.vertices())
            .all(|(lhs, rhs)| approx_vec2(*lhs, *rhs, eps))
        && a.normals()
            .iter()
            .zip(b.normals())
            .all(|(lhs, rhs)| approx_vec2(*lhs, *rhs, eps))
}

fn same_chain_id(a: ChainId, b: ChainId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

fn same_shape_id(a: ShapeId, b: ShapeId) -> bool {
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

    let rounded = shapes::rounded_box_polygon(1.5, 0.75, 0.2);
    let rounded_roundtrip = Polygon::from_raw(rounded.into_raw());
    assert_eq!(rounded_roundtrip.count(), 4);
    assert!(approx_eq(rounded_roundtrip.radius(), 0.2, f32::EPSILON));
    for (lhs, rhs) in rounded_roundtrip.vertices().iter().zip(rounded.vertices()) {
        assert!(approx_eq(lhs.x, rhs.x, f32::EPSILON));
        assert!(approx_eq(lhs.y, rhs.y, f32::EPSILON));
    }
}

#[test]
fn polygon_helpers_cover_square_offset_and_hull_workflows() {
    let square = shapes::square_polygon(1.25);
    let square_by_method = Polygon::square_polygon(1.25);
    let same_box = shapes::box_polygon(1.25, 1.25);
    assert!(approx_polygon(&square, &square_by_method, 1.0e-6));
    assert!(approx_polygon(&square, &same_box, 1.0e-6));

    let transform = Transform::from_pos_angle([2.5_f32, -1.25], 0.35);
    let offset_box = shapes::offset_box_polygon(1.5, 0.75, transform);
    let expected_box = shapes::box_polygon(1.5, 0.75).transformed(transform);
    assert!(approx_polygon(&offset_box, &expected_box, 1.0e-5));

    let offset_rounded = shapes::offset_rounded_box_polygon(1.5, 0.75, 0.2, transform);
    let expected_rounded = shapes::rounded_box_polygon(1.5, 0.75, 0.2).transformed(transform);
    assert!(approx_polygon(&offset_rounded, &expected_rounded, 1.0e-5));

    let hull_points = [
        Vec2::new(-1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, -1.0),
    ];
    assert!(shapes::polygon_hull_is_valid(hull_points));

    let offset_hull = shapes::offset_polygon_from_points(hull_points, 0.15, transform)
        .expect("valid hull should build an offset polygon");
    let expected_hull = shapes::polygon_from_points(hull_points, 0.15)
        .expect("valid hull should build a polygon")
        .transformed(transform);
    assert!(approx_polygon(&offset_hull, &expected_hull, 1.0e-5));

    let collinear = [
        Vec2::new(-1.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
    ];
    assert!(!shapes::polygon_hull_is_valid(collinear));
    assert!(shapes::polygon_from_points(collinear, 0.0).is_none());
    assert!(shapes::offset_polygon_from_points(collinear, 0.0, transform).is_none());

    let too_many_points: Vec<Vec2> = (0..=MAX_POLYGON_VERTICES)
        .map(|i| Vec2::new(i as f32, (i % 2) as f32))
        .collect();
    assert!(!shapes::polygon_hull_is_valid(
        too_many_points.iter().copied()
    ));
    assert!(
        shapes::offset_polygon_from_points(too_many_points.iter().copied(), 0.0, transform)
            .is_none()
    );
}

#[test]
fn surface_material_is_a_readable_value_type_with_explicit_raw_conversions() {
    let material = SurfaceMaterial::default()
        .with_friction(0.35)
        .with_restitution(0.6)
        .with_rolling_resistance(0.15)
        .with_tangent_speed(-2.5)
        .with_user_material_id(99)
        .with_custom_color(HexColor::from_rgb(0xAA, 0xBB, 0xCC));

    let copy = material;
    assert_eq!(copy, material);
    assert!(approx_eq(material.friction(), 0.35, f32::EPSILON));
    assert!(approx_eq(material.restitution(), 0.6, f32::EPSILON));
    assert!(approx_eq(material.rolling_resistance(), 0.15, f32::EPSILON));
    assert!(approx_eq(material.tangent_speed(), -2.5, f32::EPSILON));
    assert_eq!(material.user_material_id(), 99);
    assert_eq!(
        material.custom_color(),
        HexColor::from_rgb(0xAA, 0xBB, 0xCC)
    );
    assert_eq!(SurfaceMaterial::from_raw(material.into_raw()), material);
}

#[test]
fn shape_def_is_a_readable_value_type_and_can_seed_a_builder() {
    let material = SurfaceMaterial::default()
        .with_friction(0.45)
        .with_restitution(0.2)
        .with_user_material_id(7);
    let filter = Filter {
        category_bits: 0x0010,
        mask_bits: 0x0020,
        group_index: -5,
    };

    let sdef = ShapeDef::builder()
        .material(material)
        .density(2.5)
        .filter(filter)
        .enable_custom_filtering(true)
        .sensor(true)
        .enable_sensor_events(true)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .enable_pre_solve_events(true)
        .invoke_contact_creation(true)
        .update_body_mass(false)
        .build();

    assert_eq!(sdef.material(), material);
    assert!(approx_eq(sdef.density(), 2.5, f32::EPSILON));
    assert_eq!(sdef.filter(), filter);
    assert!(sdef.is_sensor());
    assert!(sdef.custom_filtering_enabled());
    assert!(sdef.sensor_events_enabled());
    assert!(sdef.contact_events_enabled());
    assert!(sdef.hit_events_enabled());
    assert!(sdef.pre_solve_events_enabled());
    assert!(sdef.invokes_contact_creation());
    assert!(!sdef.updates_body_mass());

    let rebuilt = ShapeDefBuilder::from(sdef.clone())
        .density(4.0)
        .sensor(false)
        .build();
    assert_eq!(rebuilt.material(), material);
    assert!(approx_eq(rebuilt.density(), 4.0, f32::EPSILON));
    assert!(!rebuilt.is_sensor());
    assert_eq!(rebuilt.filter(), filter);

    let roundtrip = ShapeDef::from_raw(sdef.into_raw());
    assert_eq!(roundtrip.material(), material);
    assert!(approx_eq(roundtrip.density(), 2.5, f32::EPSILON));
    assert_eq!(roundtrip.filter(), filter);
    assert!(roundtrip.is_sensor());
}

#[test]
fn chain_def_exposes_points_flags_and_material_layout() {
    let points = [
        Vec2::new(-2.0, 0.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(2.0, 0.0),
    ];
    let filter = Filter {
        category_bits: 0x0008,
        mask_bits: 0x0010,
        group_index: 3,
    };

    let default_def = boxdd::shapes::chain::ChainDef::builder()
        .points(points)
        .filter(filter)
        .enable_sensor_events(true)
        .build();
    assert_eq!(default_def.points(), points.as_slice());
    assert!(!default_def.is_loop());
    assert_eq!(default_def.filter(), filter);
    assert!(default_def.sensor_events_enabled());
    assert_eq!(default_def.material_count(), 1);
    match default_def.material_layout() {
        ChainDefMaterialLayout::Default(material) => {
            assert_eq!(material, SurfaceMaterial::default());
        }
        other => panic!("expected default material layout, got {other:?}"),
    }

    let single_material = SurfaceMaterial::default()
        .with_friction(0.3)
        .with_restitution(0.1);
    let single_def = boxdd::shapes::chain::ChainDef::builder()
        .points(points)
        .single_material(&single_material)
        .build();
    assert_eq!(single_def.material_count(), 1);
    match single_def.material_layout() {
        ChainDefMaterialLayout::Single(material) => {
            assert_eq!(material, single_material);
        }
        other => panic!("expected single material layout, got {other:?}"),
    }

    let multiple_materials = [
        SurfaceMaterial::default().with_friction(0.1),
        SurfaceMaterial::default().with_friction(0.2),
        SurfaceMaterial::default().with_friction(0.3),
        SurfaceMaterial::default().with_friction(0.4),
    ];
    let multiple_def = ChainDefBuilder::from(
        boxdd::shapes::chain::ChainDef::builder()
            .points(points)
            .materials(&multiple_materials)
            .build(),
    )
    .is_loop(true)
    .build();
    assert!(multiple_def.is_loop());
    assert_eq!(multiple_def.points(), points.as_slice());
    assert_eq!(multiple_def.material_count(), multiple_materials.len());
    match multiple_def.material_layout() {
        ChainDefMaterialLayout::Multiple(materials) => {
            assert_eq!(materials, multiple_materials.as_slice());
        }
        other => panic!("expected multiple material layout, got {other:?}"),
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
fn world_handle_shape_runtime_queries_match_world_queries() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    let sensor_body = world.create_body_id(BodyBuilder::new().position([0.0_f32, 1.5]).build());
    let sensor_material = SurfaceMaterial::default()
        .with_friction(0.25)
        .with_restitution(0.1)
        .with_user_material_id(41);
    let sensor_shape_id = world.create_polygon_shape_for(
        sensor_body,
        &ShapeDef::builder()
            .density(1.0)
            .sensor(true)
            .enable_sensor_events(true)
            .material(sensor_material)
            .build(),
        &shapes::box_polygon(2.0, 0.3),
    );

    let visitor_body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.0])
            .build(),
    );
    let visitor_shape_id = world.create_circle_shape_for(
        visitor_body,
        &ShapeDef::builder()
            .density(1.0)
            .enable_sensor_events(true)
            .build(),
        &shapes::circle([0.0_f32, 0.0], 0.25),
    );

    world.shape_enable_contact_events(sensor_shape_id, true);
    world.shape_enable_pre_solve_events(sensor_shape_id, true);
    world.shape_enable_hit_events(sensor_shape_id, true);

    for _ in 0..240 {
        world.step(1.0 / 120.0, 8);
        if !world.shape_sensor_overlaps(sensor_shape_id).is_empty() {
            break;
        }
    }

    let handle = world.handle();

    assert_eq!(
        handle.shape_surface_material(sensor_shape_id),
        sensor_material
    );
    assert_eq!(
        handle.try_shape_surface_material(sensor_shape_id).unwrap(),
        sensor_material
    );
    assert_eq!(handle.shape_body_id(sensor_shape_id), sensor_body);
    assert_eq!(
        handle.try_shape_body_id(sensor_shape_id).unwrap(),
        sensor_body
    );

    let world_aabb = world.shape_aabb(sensor_shape_id);
    assert_eq!(handle.shape_aabb(sensor_shape_id), world_aabb);
    assert_eq!(handle.try_shape_aabb(sensor_shape_id).unwrap(), world_aabb);

    assert_eq!(
        handle.shape_test_point(sensor_shape_id, [0.0_f32, 1.5]),
        world.shape_test_point(sensor_shape_id, [0.0_f32, 1.5])
    );
    assert_eq!(
        handle
            .try_shape_test_point(sensor_shape_id, [3.0_f32, 1.5])
            .unwrap(),
        world.shape_test_point(sensor_shape_id, [3.0_f32, 1.5])
    );

    let world_cast = world.shape_ray_cast(sensor_shape_id, [-3.0_f32, 1.5], [6.0_f32, 0.0]);
    let handle_cast = handle.shape_ray_cast(sensor_shape_id, [-3.0_f32, 1.5], [6.0_f32, 0.0]);
    assert_eq!(handle_cast.hit, world_cast.hit);
    assert!(approx_eq(handle_cast.fraction, world_cast.fraction, 1.0e-6));
    assert!(approx_vec2(handle_cast.point, world_cast.point, 1.0e-6));
    assert!(approx_vec2(handle_cast.normal, world_cast.normal, 1.0e-6));
    let handle_try_cast = handle
        .try_shape_ray_cast(sensor_shape_id, [-3.0_f32, 1.5], [6.0_f32, 0.0])
        .unwrap();
    assert_eq!(handle_try_cast.hit, world_cast.hit);
    assert!(approx_eq(
        handle_try_cast.fraction,
        world_cast.fraction,
        1.0e-6
    ));
    assert!(approx_vec2(handle_try_cast.point, world_cast.point, 1.0e-6));
    assert!(approx_vec2(
        handle_try_cast.normal,
        world_cast.normal,
        1.0e-6
    ));

    let world_closest_point = world.shape_closest_point(sensor_shape_id, [3.5_f32, 1.5]);
    assert_eq!(
        handle.shape_closest_point(sensor_shape_id, [3.5_f32, 1.5]),
        world_closest_point
    );
    assert_eq!(
        handle
            .try_shape_closest_point(sensor_shape_id, [3.5_f32, 1.5])
            .unwrap(),
        world_closest_point
    );

    assert!(approx_mass_data(
        handle.shape_mass_data(sensor_shape_id),
        world.shape_mass_data(sensor_shape_id),
        1.0e-6
    ));
    assert!(approx_mass_data(
        handle.try_shape_mass_data(sensor_shape_id).unwrap(),
        world.shape_mass_data(sensor_shape_id),
        1.0e-6
    ));

    assert!(handle.shape_sensor_events_enabled(sensor_shape_id));
    assert!(
        handle
            .try_shape_sensor_events_enabled(sensor_shape_id)
            .unwrap()
    );
    assert!(handle.shape_contact_events_enabled(sensor_shape_id));
    assert!(
        handle
            .try_shape_contact_events_enabled(sensor_shape_id)
            .unwrap()
    );
    assert!(handle.shape_pre_solve_events_enabled(sensor_shape_id));
    assert!(
        handle
            .try_shape_pre_solve_events_enabled(sensor_shape_id)
            .unwrap()
    );
    assert!(handle.shape_hit_events_enabled(sensor_shape_id));
    assert!(
        handle
            .try_shape_hit_events_enabled(sensor_shape_id)
            .unwrap()
    );

    let sensor_capacity = world.shape_sensor_capacity(sensor_shape_id);
    assert_eq!(
        handle.shape_sensor_capacity(sensor_shape_id),
        sensor_capacity
    );
    assert_eq!(
        handle.try_shape_sensor_capacity(sensor_shape_id).unwrap(),
        sensor_capacity
    );

    let world_overlaps = world.shape_sensor_overlaps(sensor_shape_id);
    assert!(!world_overlaps.is_empty());
    assert!(
        world_overlaps
            .iter()
            .copied()
            .any(|id| same_shape_id(id, visitor_shape_id))
    );
    let handle_overlaps = handle.shape_sensor_overlaps(sensor_shape_id);
    assert_eq!(handle_overlaps.len(), world_overlaps.len());
    assert!(
        handle_overlaps
            .iter()
            .copied()
            .any(|id| same_shape_id(id, visitor_shape_id))
    );
    let mut overlap_buf = Vec::with_capacity(8);
    let overlap_buf_ptr = overlap_buf.as_ptr();
    handle.shape_sensor_overlaps_into(sensor_shape_id, &mut overlap_buf);
    assert_eq!(overlap_buf.as_ptr(), overlap_buf_ptr);
    assert_eq!(overlap_buf.len(), world_overlaps.len());
    handle
        .try_shape_sensor_overlaps_into(sensor_shape_id, &mut overlap_buf)
        .unwrap();
    assert_eq!(overlap_buf.as_ptr(), overlap_buf_ptr);
    assert_eq!(overlap_buf.len(), world_overlaps.len());

    let world_overlaps_valid = world.shape_sensor_overlaps_valid(sensor_shape_id);
    assert!(!world_overlaps_valid.is_empty());
    assert!(
        world_overlaps_valid
            .iter()
            .copied()
            .any(|id| same_shape_id(id, visitor_shape_id))
    );
    let handle_overlaps_valid = handle.shape_sensor_overlaps_valid(sensor_shape_id);
    assert_eq!(handle_overlaps_valid.len(), world_overlaps_valid.len());
    assert!(
        handle_overlaps_valid
            .iter()
            .copied()
            .any(|id| same_shape_id(id, visitor_shape_id))
    );
    let mut overlap_valid_buf = Vec::with_capacity(8);
    let overlap_valid_buf_ptr = overlap_valid_buf.as_ptr();
    handle.shape_sensor_overlaps_valid_into(sensor_shape_id, &mut overlap_valid_buf);
    assert_eq!(overlap_valid_buf.as_ptr(), overlap_valid_buf_ptr);
    assert_eq!(overlap_valid_buf.len(), world_overlaps_valid.len());
    handle
        .try_shape_sensor_overlaps_valid_into(sensor_shape_id, &mut overlap_valid_buf)
        .unwrap();
    assert_eq!(overlap_valid_buf.as_ptr(), overlap_valid_buf_ptr);
    assert_eq!(overlap_valid_buf.len(), world_overlaps_valid.len());
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
