use boxdd::{prelude::*, shapes};
use boxdd_sys::ffi;

fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() <= epsilon
}

fn approx_vec2(a: Vec2, b: Vec2, epsilon: f32) -> bool {
    approx_eq(a.x, b.x, epsilon) && approx_eq(a.y, b.y, epsilon)
}

fn approx_position(a: Position, b: Position, epsilon: WorldScalar) -> bool {
    (a.x - b.x).abs() <= epsilon && (a.y - b.y).abs() <= epsilon
}

fn initialize_foundation() {
    boxdd::Foundation::initialize_default().expect("default foundation should initialize");
}

#[test]
fn shape_spatial_queries_and_wind_use_one_borrow_scoped_capability() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([2.0_f32, 3.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape_id = world
        .body(body_id)
        .unwrap()
        .create_centered_circle(&ShapeDef::builder().density(1.0).build().unwrap(), 1.0)
        .unwrap();

    let mut shape = world.shape(shape_id).unwrap();
    assert_eq!(shape.body_id().unwrap(), body_id);
    assert_eq!(shape.shape_type().unwrap(), ShapeType::Circle);
    assert!(shape.test_point(Position::new(2.25, 3.0)).unwrap());
    assert!(!shape.test_point(Position::new(4.0, 3.0)).unwrap());

    let closest = shape.closest_point(Position::new(4.0, 3.0)).unwrap();
    assert!(approx_position(closest, Position::new(3.0, 3.0), 1.0e-5));

    let cast = shape
        .ray_cast(Position::new(0.0, 3.0), [4.0_f32, 0.0])
        .unwrap();
    assert!(cast.hit);
    assert!(approx_position(cast.point, Position::new(1.0, 3.0), 1.0e-5));
    assert!(approx_vec2(cast.normal, Vec2::new(-1.0, 0.0), 1.0e-5));

    shape.apply_wind([3.0_f32, 0.0], 1.0, 0.25, true).unwrap();
}

#[test]
fn typed_geometry_getters_reject_wrong_kinds_and_follow_every_setter_transition() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let shape_id = world
        .body(body)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    let mut shape = world.shape(shape_id).unwrap();

    assert_eq!(
        shape.segment(),
        Err(Error::WrongShapeType {
            expected: ShapeType::Segment,
            actual: ShapeType::Circle,
        })
    );

    let segment = shapes::segment([-1.0_f32, 0.0], [1.0, 0.0]).unwrap();
    shape.set_segment(&segment).unwrap();
    assert_eq!(shape.shape_type(), Ok(ShapeType::Segment));
    assert_eq!(shape.segment(), Ok(segment));
    assert_eq!(
        shape.chain_segment(),
        Err(Error::WrongShapeType {
            expected: ShapeType::ChainSegment,
            actual: ShapeType::Segment,
        })
    );

    let chain_segment =
        shapes::chain_segment([-2.0_f32, 0.0], [-1.0, 0.0], [1.0, 0.0], [2.0, 0.0]).unwrap();
    shape.set_chain_segment(&chain_segment).unwrap();
    assert_eq!(shape.shape_type(), Ok(ShapeType::ChainSegment));
    assert_eq!(shape.chain_segment(), Ok(chain_segment));
    assert_eq!(
        shape.capsule(),
        Err(Error::WrongShapeType {
            expected: ShapeType::Capsule,
            actual: ShapeType::ChainSegment,
        })
    );

    let capsule = shapes::capsule([-1.0_f32, 0.0], [1.0, 0.0], 0.25).unwrap();
    shape.set_capsule(&capsule).unwrap();
    assert_eq!(shape.shape_type(), Ok(ShapeType::Capsule));
    assert_eq!(shape.capsule(), Ok(capsule));
    assert!(matches!(
        shape.polygon(),
        Err(Error::WrongShapeType {
            expected: ShapeType::Polygon,
            actual: ShapeType::Capsule,
        })
    ));

    let polygon = shapes::square_polygon(0.75).unwrap();
    shape.set_polygon(&polygon).unwrap();
    assert_eq!(shape.shape_type(), Ok(ShapeType::Polygon));
    let actual_polygon = shape.polygon().unwrap();
    assert_eq!(actual_polygon.vertices(), polygon.vertices());
    assert_eq!(actual_polygon.normals(), polygon.normals());
    assert_eq!(actual_polygon.centroid(), polygon.centroid());
    assert_eq!(actual_polygon.radius(), polygon.radius());
    assert_eq!(
        shape.circle(),
        Err(Error::WrongShapeType {
            expected: ShapeType::Circle,
            actual: ShapeType::Polygon,
        })
    );

    let circle = shapes::circle([0.25_f32, 0.0], 0.5).unwrap();
    shape.set_circle(&circle).unwrap();
    assert_eq!(shape.shape_type(), Ok(ShapeType::Circle));
    assert_eq!(shape.circle(), Ok(circle));
}

#[test]
fn shape_spatial_queries_reject_invalid_world_inputs_before_ffi() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let shape_id = world
        .body(body_id)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 1.0)
        .unwrap();
    let mut shape = world.shape(shape_id).unwrap();

    assert_eq!(
        shape.closest_point(Position::new(WorldScalar::NAN, 0.0)),
        Err(Error::invalid_argument(
            "Shape::closest_point",
            "target",
            "an offset from the body representable by a finite local vector",
        ))
    );
    assert_eq!(
        shape.test_point(Position::new(0.0, WorldScalar::INFINITY)),
        Err(Error::invalid_argument(
            "Shape::test_point",
            "point",
            "an offset from the body representable by a finite local vector",
        ))
    );
    assert_eq!(
        shape.ray_cast(Position::ZERO, [f32::NAN, 0.0]),
        Err(Error::invalid_argument(
            "Shape::ray_cast",
            "translation",
            "a finite vector",
        ))
    );
    assert_eq!(
        shape.apply_wind([f32::NAN, 0.0], 1.0, 0.0, true),
        Err(Error::invalid_argument(
            "Shape::apply_wind",
            "wind",
            "a finite vector",
        ))
    );
    assert_eq!(
        shape.apply_wind(Vec2::ZERO, -1.0, 0.0, true),
        Err(Error::invalid_argument(
            "Shape::apply_wind",
            "drag",
            "a finite value greater than or equal to zero",
        ))
    );
}

#[test]
fn geometry_values_round_trip_through_explicit_raw_conversions() {
    initialize_foundation();

    let circle = shapes::circle([1.0_f32, -2.0], 0.75).unwrap();
    assert_eq!(shapes::Circle::from_raw(circle.into_raw()).unwrap(), circle);

    let segment = shapes::segment([-1.0_f32, 0.0], [2.0, 3.0]).unwrap();
    assert_eq!(
        shapes::Segment::from_raw(segment.into_raw()).unwrap(),
        segment
    );

    let capsule = shapes::capsule([-1.0_f32, 0.0], [1.0, 0.0], 0.4).unwrap();
    assert_eq!(
        shapes::Capsule::from_raw(capsule.into_raw()).unwrap(),
        capsule
    );

    let chain_segment =
        shapes::chain_segment([-2.0_f32, 0.0], [-1.0, 0.0], [1.0, 0.0], [2.0, 0.0]).unwrap();
    let copied_chain_segment = shapes::ChainSegment::from_raw(chain_segment.into_raw()).unwrap();
    assert_eq!(copied_chain_segment.ghost1(), chain_segment.ghost1());
    assert_eq!(copied_chain_segment.segment(), chain_segment.segment());
    assert_eq!(copied_chain_segment.ghost2(), chain_segment.ghost2());

    let polygon = shapes::box_polygon(1.5, 0.75).unwrap();
    let raw = polygon.into_raw();
    let copied_polygon = shapes::Polygon::from_raw(raw).unwrap();
    assert_eq!(copied_polygon.vertices(), polygon.vertices());
    assert_eq!(copied_polygon.normals(), polygon.normals());
    assert_eq!(copied_polygon.centroid(), polygon.centroid());
    assert_eq!(copied_polygon.radius(), polygon.radius());
}

#[test]
fn safe_geometry_helpers_compute_mass_bounds_and_casts() {
    initialize_foundation();

    let transform = WorldTransform::new(Position::new(3.0, -2.0), Rot::IDENTITY).unwrap();
    let circle = shapes::circle([0.0_f32, 0.0], 1.0).unwrap();
    let mass = circle.mass_data(2.0).unwrap();
    assert!(mass.mass() > 0.0);
    let bounds = circle.aabb(transform).unwrap();
    assert!(bounds.lower().x <= 2.0 && bounds.upper().x >= 4.0);
    assert!(bounds.lower().y <= -3.0 && bounds.upper().y >= -1.0);
    assert!(circle.contains_point([0.25_f32, 0.0]).unwrap());
    assert!(circle.ray_cast([-2.0_f32, 0.0], [4.0, 0.0]).unwrap().hit);

    let capsule = shapes::capsule([-1.0_f32, 0.0], [1.0, 0.0], 0.5).unwrap();
    assert!(capsule.mass_data(1.0).unwrap().mass() > 0.0);
    assert!(capsule.contains_point([0.0_f32, 0.25]).unwrap());

    let polygon = shapes::box_polygon(1.0, 0.5).unwrap();
    assert!(polygon.mass_data(1.0).unwrap().mass() > 0.0);
    assert!(polygon.contains_point([0.0_f32, 0.0]).unwrap());
}

#[test]
fn polygon_helpers_validate_dimensions_points_and_transforms() {
    initialize_foundation();

    let square = shapes::square_polygon(2.0).unwrap();
    assert_eq!(square.count(), 4);
    assert!(approx_eq(square.radius(), 0.0, 1.0e-6));

    let points = [[-1.0_f32, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
    assert!(shapes::polygon_hull_is_valid(points).unwrap());
    let hull = shapes::polygon_from_points(points, 0.1).unwrap();
    assert_eq!(hull.count(), 4);
    assert!(approx_eq(hull.radius(), 0.1, 1.0e-6));

    let offset = shapes::offset_box_polygon(
        1.0,
        0.5,
        Transform::from_pos_angle([2.0_f32, 3.0], 0.25).unwrap(),
    )
    .unwrap();
    assert!(approx_vec2(offset.centroid(), Vec2::new(2.0, 3.0), 1.0e-5));

    assert_eq!(
        shapes::box_polygon(0.0, 1.0).unwrap_err(),
        Error::invalid_argument(
            "Polygon::box_polygon",
            "half_width",
            "a finite value greater than zero",
        )
    );
    assert_eq!(
        shapes::rounded_box_polygon(1.0, 1.0, -0.1).unwrap_err(),
        Error::invalid_argument(
            "Polygon::rounded_box_polygon",
            "radius",
            "a finite value greater than or equal to zero",
        )
    );
    assert_eq!(
        shapes::polygon_from_points([[0.0_f32, 0.0], [1.0, 0.0]], 0.0).unwrap_err(),
        Error::invalid_argument(
            "Polygon::from_points",
            "points",
            "points that form a non-degenerate convex hull",
        )
    );
    assert_eq!(
        shapes::polygon_from_points(points, f32::NAN).unwrap_err(),
        Error::invalid_argument(
            "Polygon::from_points",
            "radius",
            "a finite value greater than or equal to zero",
        )
    );
}

#[test]
fn body_shape_constructors_return_typed_ids_or_recoverable_errors() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut body = world.body(body_id).unwrap();

    let circle = body
        .create_circle(
            &ShapeDef::default(),
            &shapes::circle([0.0_f32, 0.0], 0.5).unwrap(),
        )
        .unwrap();
    let box_shape = body.create_box(&ShapeDef::default(), 1.0, 0.5).unwrap();
    let capsule = body
        .create_capsule_between(&ShapeDef::default(), [-1.0_f32, 0.0], [1.0, 0.0], 0.25)
        .unwrap();
    assert_eq!(body.shapes().unwrap().len(), 3);
    assert!(body.shapes().unwrap().contains(&circle));
    assert!(body.shapes().unwrap().contains(&box_shape));
    assert!(body.shapes().unwrap().contains(&capsule));

    assert_eq!(
        body.create_polygon_from_points(&ShapeDef::default(), [[0.0_f32, 0.0], [1.0, 0.0]], 0.0,),
        Err(Error::invalid_argument(
            "Polygon::from_points",
            "points",
            "points that form a non-degenerate convex hull",
        ))
    );
    assert_eq!(
        body.create_capsule_between(&ShapeDef::default(), Vec2::ZERO, Vec2::ZERO, f32::NAN,),
        Err(Error::invalid_argument(
            "Capsule::new",
            "capsule",
            "finite geometry with endpoints separated by Box2D's minimum length and a non-negative radius",
        ))
    );
    assert_eq!(body.shape_count().unwrap(), 3);
}

#[test]
fn surface_material_and_shape_def_are_readable_value_types() {
    let material = SurfaceMaterial::default()
        .with_friction(0.25)
        .unwrap()
        .with_restitution(0.5)
        .unwrap()
        .with_rolling_resistance(0.125)
        .unwrap()
        .with_tangent_speed(2.0)
        .unwrap()
        .with_user_material_id(41)
        .with_custom_color(HexColor::from_rgb_u32(0x123456));
    assert_eq!(
        SurfaceMaterial::from_raw(material.into_raw()).unwrap(),
        material
    );
    assert_eq!(material.friction(), 0.25);
    assert_eq!(material.restitution(), 0.5);
    assert_eq!(material.rolling_resistance(), 0.125);
    assert_eq!(material.tangent_speed(), 2.0);
    assert_eq!(material.user_material_id(), 41);
    assert_eq!(material.custom_color().rgb_u32(), 0x123456);

    let filter = Filter {
        category_bits: 0x02,
        mask_bits: 0x04,
        group_index: -3,
    };
    let def = ShapeDef::builder()
        .material(material)
        .density(2.0)
        .filter(filter)
        .sensor(true)
        .enable_sensor_events(true)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .enable_pre_solve_events(true)
        .update_body_mass(true)
        .build()
        .unwrap();
    assert_eq!(def.material(), material);
    assert_eq!(def.density(), 2.0);
    assert_eq!(def.filter(), filter);
    assert!(def.is_sensor());
    assert!(def.sensor_events_enabled());
    assert!(def.contact_events_enabled());
    assert!(def.hit_events_enabled());
    assert!(def.pre_solve_events_enabled());
    assert!(def.updates_body_mass());
}

#[test]
fn definitions_reject_invalid_numeric_and_layout_inputs() {
    assert_eq!(
        SurfaceMaterial::default()
            .with_friction(f32::NAN)
            .unwrap_err(),
        Error::invalid_argument(
            "SurfaceMaterial::with_friction",
            "friction",
            "a finite value greater than or equal to zero",
        )
    );

    let mut raw_material = SurfaceMaterial::default().into_raw();
    raw_material.rollingResistance = -1.0;
    assert_eq!(
        SurfaceMaterial::from_raw(raw_material).unwrap_err(),
        Error::invalid_argument(
            "SurfaceMaterial::from_raw",
            "rolling_resistance",
            "a finite value greater than or equal to zero",
        )
    );

    let mut raw_material = SurfaceMaterial::default().into_raw();
    raw_material.customColor = u32::MAX;
    assert_eq!(
        SurfaceMaterial::from_raw(raw_material).unwrap_err(),
        Error::invalid_argument(
            "SurfaceMaterial::from_raw",
            "custom_color",
            "an RGB value in the inclusive range 0x000000..=0xFFFFFF",
        )
    );
    assert_eq!(
        ShapeDef::builder().density(-1.0).build().unwrap_err(),
        Error::invalid_argument(
            "ShapeDef::validate",
            "density",
            "a finite value greater than or equal to zero",
        )
    );
    assert_eq!(
        ChainDef::builder()
            .points([[0.0_f32, 0.0], [1.0, 0.0], [2.0, 0.0]])
            .build()
            .unwrap_err(),
        Error::InvalidChainDef
    );
}

#[test]
fn chain_definition_and_runtime_use_one_borrow_scoped_capability() {
    initialize_foundation();
    let initial = SurfaceMaterial::default()
        .with_friction(0.2)
        .unwrap()
        .with_user_material_id(7);
    let updated = SurfaceMaterial::default()
        .with_friction(0.8)
        .unwrap()
        .with_user_material_id(9);
    let points = [[-2.0_f32, 0.0], [-1.0, 0.0], [1.0, 0.0], [2.0, 0.0]];
    let def = ChainDef::builder()
        .points(points)
        .single_material(&initial)
        .enable_sensor_events(true)
        .build()
        .unwrap();
    assert_eq!(def.points(), points.map(Vec2::from).as_slice());
    assert!(def.sensor_events_enabled());
    assert_eq!(def.material_count(), 1);
    assert!(matches!(
        def.material_layout(),
        ChainDefMaterialLayout::Single(material) if material == initial
    ));

    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let chain_id = world.body(body_id).unwrap().create_chain(&def).unwrap();

    let segments = {
        let mut chain = world.chain(chain_id).unwrap();
        assert!(chain.segment_count().unwrap() > 0);
        assert_eq!(chain.surface_material_count().unwrap(), 1);
        assert_eq!(chain.surface_material(0).unwrap(), initial);
        chain.set_surface_material(0, &updated).unwrap();
        assert_eq!(chain.surface_material(0).unwrap(), updated);
        chain.segments().unwrap()
    };
    assert!(!segments.is_empty());
    assert_eq!(
        world.shape(segments[0]).unwrap().parent_chain_id().unwrap(),
        Some(chain_id)
    );
}

#[test]
fn shape_runtime_properties_filters_and_events_are_canonical_results() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let original_filter = Filter {
        category_bits: 0x02,
        mask_bits: 0x04,
        group_index: -3,
    };
    let material = SurfaceMaterial::default()
        .with_friction(0.25)
        .unwrap()
        .with_restitution(0.1)
        .unwrap()
        .with_user_material_id(41);
    let shape_id = world
        .body(body_id)
        .unwrap()
        .create_centered_circle(
            &ShapeDef::builder()
                .density(2.0)
                .filter(original_filter)
                .material(material)
                .build()
                .unwrap(),
            1.0,
        )
        .unwrap();

    let mut shape = world.shape(shape_id).unwrap();
    assert_eq!(shape.filter().unwrap(), original_filter);
    let updated_filter = Filter {
        category_bits: 0x10,
        mask_bits: 0x20,
        group_index: 7,
    };
    shape.set_filter(updated_filter).unwrap();
    assert_eq!(shape.filter().unwrap(), updated_filter);

    assert_eq!(shape.surface_material().unwrap(), material);
    assert_eq!(shape.density().unwrap(), 2.0);
    assert!(shape.mass_data().unwrap().mass() > 0.0);
    shape.set_density(3.0, true).unwrap();
    shape.set_friction(0.75).unwrap();
    shape.set_restitution(0.5).unwrap();
    shape.set_user_material(99).unwrap();
    assert_eq!(shape.density().unwrap(), 3.0);
    assert_eq!(shape.friction().unwrap(), 0.75);
    assert_eq!(shape.restitution().unwrap(), 0.5);
    assert_eq!(shape.user_material().unwrap(), 99);

    assert!(!shape.sensor_events_enabled().unwrap());
    assert!(!shape.contact_events_enabled().unwrap());
    assert!(!shape.pre_solve_events_enabled().unwrap());
    assert!(!shape.hit_events_enabled().unwrap());
    shape.enable_sensor_events(true).unwrap();
    shape.enable_contact_events(true).unwrap();
    shape.enable_pre_solve_events(true).unwrap();
    shape.enable_hit_events(true).unwrap();
    assert!(shape.sensor_events_enabled().unwrap());
    assert!(shape.contact_events_enabled().unwrap());
    assert!(shape.pre_solve_events_enabled().unwrap());
    assert!(shape.hit_events_enabled().unwrap());
}

#[test]
fn shape_capability_enforces_world_provenance_and_liveness() {
    let mut source = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body = source
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let shape_id = source
        .body(body)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    let mut target = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();

    assert_eq!(target.shape(shape_id).err().unwrap(), Error::WrongWorld);
    source.shape(shape_id).unwrap().destroy(true).unwrap();
    assert_eq!(source.shape(shape_id).err().unwrap(), Error::InvalidShapeId);
}

#[test]
fn shape_type_value_round_trips_through_the_sys_discriminant() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let shape_id = world
        .body(body)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    let shape = world.shape(shape_id).unwrap();

    assert_eq!(shape.shape_type().unwrap(), ShapeType::Circle);
    assert_eq!(
        ShapeType::from_raw(ffi::b2ShapeType_b2_circleShape),
        Some(ShapeType::Circle)
    );
    assert_eq!(
        ShapeType::Circle.into_raw(),
        ffi::b2ShapeType_b2_circleShape
    );
}
