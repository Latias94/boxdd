use boxdd::{Sweep, atan2, is_valid_float, length_units_per_meter, prelude::*, shapes};

fn approx_eq(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

fn approx_world_eq(actual: WorldScalar, expected: WorldScalar) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

fn average_coefficient(a: MaterialMixInput, b: MaterialMixInput) -> f32 {
    0.5 * (a.coefficient + b.coefficient)
}

#[test]
fn body_runtime_state_forces_mass_and_lifecycle_succeed() {
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let world_is_valid = boxdd::World::is_valid(&world);
    assert!(world_is_valid);
    let builder = BodyBuilder::default();
    let body_id = world.create_body_id(
        builder
            .body_type(BodyType::Dynamic)
            .position([0.5_f32, 1.0])
            .linear_damping(0.2)
            .angular_damping(0.3)
            .gravity_scale(0.8)
            .build(),
    );
    let shape_def = ShapeDef::builder().density(2.0).build();
    let shape_id =
        world.create_polygon_shape_for(body_id, &shape_def, &shapes::box_polygon(0.5_f32, 0.25));
    let shape_exists = world.shape(shape_id).is_some();
    assert!(shape_exists);

    world.body_wake_touching(body_id);

    {
        let mut body = world.body(body_id).expect("body should remain valid");

        let valid = body.is_valid();
        assert!(valid);
        let world_id = body.try_world_id_raw();
        assert!(world_id.is_ok());

        let position = body.position();
        assert!(approx_world_eq(position.x, 0.5));
        assert!(approx_world_eq(position.y, 1.0));
        let transform = body.transform();
        assert!(approx_world_eq(transform.position().x, position.x));
        assert!(approx_world_eq(transform.position().y, position.y));

        let local_point = body.local_point([1.0_f32, 1.5]);
        assert!(local_point.x.is_finite() && local_point.y.is_finite());
        let world_point = body.world_point(local_point);
        assert!(approx_world_eq(world_point.x, 1.0));
        assert!(approx_world_eq(world_point.y, 1.5));
        let local_vector = body.local_vector([1.0_f32, -0.5]);
        assert!(local_vector.x.is_finite() && local_vector.y.is_finite());
        let world_vector = body.world_vector(local_vector);
        assert!(approx_eq(world_vector.x, 1.0));
        assert!(approx_eq(world_vector.y, -0.5));

        body.set_angular_velocity(0.75);
        let local_point_velocity = body.local_point_velocity([0.25_f32, 0.0]);
        assert!(local_point_velocity.x.is_finite() && local_point_velocity.y.is_finite());
        let world_point_velocity = body.try_world_point_velocity([0.75_f32, 1.0]);
        assert!(world_point_velocity.is_ok());
        let world_point_velocity =
            world_point_velocity.expect("world-point velocity query should succeed");
        assert!(world_point_velocity.x.is_finite() && world_point_velocity.y.is_finite());

        let mass = body.mass();
        assert!(mass > 0.0);
        let inertia = body.rotational_inertia();
        assert!(inertia > 0.0);
        let local_center = body.local_center_of_mass();
        assert!(local_center.x.is_finite() && local_center.y.is_finite());
        let world_center = body.try_world_center_of_mass();
        assert!(world_center.is_ok());
        let mass_data = body.mass_data();
        assert!(mass_data.mass > 0.0);
        body.set_mass_data(mass_data);
        body.apply_mass_from_shapes();

        let body_type = body.body_type();
        assert_eq!(body_type, BodyType::Dynamic);
        body.set_body_type(BodyType::Kinematic);
        body.set_body_type(BodyType::Dynamic);

        let gravity_scale = body.gravity_scale();
        assert!(approx_eq(gravity_scale, 0.8));
        let linear_damping = body.linear_damping();
        assert!(approx_eq(linear_damping, 0.2));
        let angular_damping = body.angular_damping();
        assert!(approx_eq(angular_damping, 0.3));

        body.disable();
        body.enable();
        body.set_target_transform(
            WorldTransform::from_pos_angle([0.75_f32, 1.25], 0.1),
            1.0 / 60.0,
            true,
        );
        body.set_position_and_rotation([0.5_f32, 1.0], 0.0);
        body.try_set_position_and_rotation([0.5_f32, 1.0], 0.0)
            .expect("validated position update should succeed");

        body.apply_force([2.0_f32, -1.0], [0.5_f32, 1.0], true);
        body.apply_force_to_center([1.0_f32, 0.5], true);
        body.apply_torque(0.25, true);
        body.apply_linear_impulse([0.2_f32, 0.1], [0.5_f32, 1.0], true);
        body.apply_linear_impulse_to_center([-0.1_f32, 0.2], true);
        body.apply_angular_impulse(0.15, true);

        body.set_user_data(17_u32);
        let user_data = body.try_user_data_ptr_raw();
        assert!(user_data.is_ok());
        let user_data = user_data.expect("user-data query should succeed");
        assert!(!user_data.is_null());

        body.clear_forces();
        let user_data_cleared = body.clear_user_data();
        assert!(user_data_cleared);
    }

    let owned = world.create_body_owned(BodyBuilder::new().build());
    let owned_valid = owned.is_valid();
    assert!(owned_valid);
    owned.destroy();
    std::mem::drop(world);
}

#[cfg(feature = "double-precision")]
#[test]
fn body_world_coordinate_apis_preserve_double_precision() {
    let origin = Position::new(1_000_000_000_000.25, -1_000_000_000_000.5);
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let body_id = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position(origin)
            .build(),
    );

    let mut body = world.body(body_id).expect("body should remain valid");
    assert_eq!(body.position(), origin);
    assert_eq!(body.transform().position(), origin);
    assert_eq!(body.world_center_of_mass(), origin);

    let local_point = Vec2::new(0.5, -0.25);
    let world_point = body.world_point(local_point);
    assert_eq!(world_point, Position::new(origin.x + 0.5, origin.y - 0.25));
    assert_eq!(body.local_point(world_point), local_point);
    assert!(body.world_point_velocity(world_point).is_valid());

    let moved = Position::new(origin.x + 4.0, origin.y + 8.0);
    body.set_position_and_rotation(moved, 0.0);
    assert_eq!(body.position(), moved);
    body.apply_force([1.0_f32, 0.0], moved, true);
    body.apply_linear_impulse([0.25_f32, 0.0], moved, true);
    body.set_target_transform(WorldTransform::new(moved, Rot::IDENTITY), 1.0 / 60.0, true);
}

#[test]
fn live_body_contact_and_contact_snapshot_succeed() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build())
        .expect("world creation should succeed");
    let ground = world.create_body_id(BodyBuilder::new().build());
    let ground_shape = world.create_polygon_shape_for(
        ground,
        &ShapeDef::default(),
        &shapes::box_polygon(10.0_f32, 0.5),
    );
    let ground_shape_exists = world.shape(ground_shape).is_some();
    assert!(ground_shape_exists);

    let dynamic = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.0])
            .build(),
    );
    let dynamic_shape = world.create_polygon_shape_for(
        dynamic,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5_f32, 0.5),
    );
    let dynamic_shape_exists = world.shape(dynamic_shape).is_some();
    assert!(dynamic_shape_exists);

    for _ in 0..180 {
        world.step(1.0 / 60.0, 4);
    }

    let contacts = {
        let body = world
            .body(dynamic)
            .expect("dynamic body should remain valid");
        body.contact_data()
    };
    assert!(!contacts.is_empty());
    let contact_id = contacts
        .first()
        .expect("resting body should have a live contact")
        .contact_id;
    let contact_valid = contact_id.is_valid();
    assert!(contact_valid);
    let snapshot = boxdd::ContactId::data(contact_id);
    assert_eq!(snapshot.contact_id.index1, contact_id.index1);
    assert_eq!(snapshot.contact_id.generation, contact_id.generation);
}

#[test]
fn chain_runtime_creation_queries_and_destroy_succeed() {
    let default_def = ChainDef::default();
    let default_points = default_def.points();
    assert!(default_points.is_empty());

    let chain_def = ChainDef::builder()
        .points([
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ])
        .build();
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let body_id = world.create_body_id(BodyBuilder::new().build());

    let mut body = world.body(body_id).expect("chain body should remain valid");
    let chain = body.create_chain(&chain_def);
    let chain_valid = chain.is_valid();
    assert!(chain_valid);
    let segments = chain.segments();
    assert!(!segments.is_empty());
    let material_count = chain.surface_material_count();
    assert!(material_count > 0);
    chain.destroy();
}

#[test]
fn foundation_collision_and_math_runtime_paths_succeed() {
    let valid_float = is_valid_float(1.25);
    assert!(valid_float);
    let units_per_meter = length_units_per_meter();
    assert!(units_per_meter.is_finite() && units_per_meter > 0.0);
    let angle = atan2(1.0, 0.0);
    assert!(approx_eq(angle, core::f32::consts::FRAC_PI_2));

    let sweep = Sweep::new(
        [0.0_f32, 0.0],
        [0.0_f32, 0.0],
        [2.0_f32, 0.0],
        Rot::IDENTITY,
        Rot::IDENTITY,
    );
    let sweep_transform = sweep.transform_at(0.5);
    assert!(approx_eq(sweep_transform.position().x, 1.0));
    assert!(approx_eq(sweep_transform.position().y, 0.0));

    let explosion = ExplosionDef::default();
    let blast_radius = explosion.blast_radius();
    assert!(blast_radius.is_finite() && blast_radius >= 0.0);

    let points = [
        Vec2::new(-0.75, -0.5),
        Vec2::new(0.75, -0.5),
        Vec2::new(0.75, 0.5),
        Vec2::new(-0.75, 0.5),
    ];
    let hull_valid = Polygon::hull_is_valid(points);
    assert!(hull_valid);

    let box_polygon = Polygon::box_polygon(0.5, 0.25);
    assert_eq!(box_polygon.count(), 4);
    let rounded_box = Polygon::rounded_box_polygon(0.5, 0.25, 0.05);
    assert_eq!(rounded_box.count(), 4);
    let offset = Transform::from_pos_angle([1.0_f32, -0.5], 0.2);
    let offset_box = Polygon::offset_box_polygon(0.5, 0.25, offset);
    assert_eq!(offset_box.count(), 4);
    let offset_rounded_box = Polygon::offset_rounded_box_polygon(0.5, 0.25, 0.05, offset);
    assert_eq!(offset_rounded_box.count(), 4);
    let offset_polygon = Polygon::offset_from_points(points, 0.0, offset);
    assert!(offset_polygon.is_some());
    let offset_rounded_polygon = Polygon::offset_from_points(points, 0.05, offset);
    assert!(offset_rounded_polygon.is_some());

    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let body_id = world.create_body_id(BodyBuilder::new().build());
    let mut body = world
        .body(body_id)
        .expect("polygon body should remain valid");
    let shape = body.create_polygon_from_points(&ShapeDef::default(), points, 0.0);
    assert!(shape.is_some());
}

#[test]
fn world_queries_events_user_data_and_callback_cleanup_succeed() {
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let ground = world.create_body_id(BodyBuilder::new().build());
    let ground_shape = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().enable_contact_events(true).build(),
        &shapes::box_polygon(10.0_f32, 0.5),
    );
    let ground_shape_exists = world.shape(ground_shape).is_some();
    assert!(ground_shape_exists);

    let wall = world.create_body_id(BodyBuilder::new().position([1.0_f32, 1.0]).build());
    let wall_shape = world.create_polygon_shape_for(
        wall,
        &ShapeDef::default(),
        &shapes::box_polygon(0.25_f32, 1.0),
    );
    let wall_shape_exists = world.shape(wall_shape).is_some();
    assert!(wall_shape_exists);

    let dynamic = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 0.8])
            .linear_velocity([0.1_f32, 0.0])
            .build(),
    );
    let dynamic_shape = world.create_polygon_shape_for(
        dynamic,
        &ShapeDef::builder()
            .density(1.0)
            .enable_contact_events(true)
            .build(),
        &shapes::box_polygon(0.4_f32, 0.4),
    );
    let dynamic_shape_exists = world.shape(dynamic_shape).is_some();
    assert!(dynamic_shape_exists);

    let sensor_body = world.create_body_id(BodyBuilder::new().position([3.0_f32, 1.0]).build());
    let sensor_shape = world.create_polygon_shape_for(
        sensor_body,
        &ShapeDef::builder()
            .sensor(true)
            .enable_sensor_events(true)
            .build(),
        &shapes::box_polygon(0.75_f32, 0.75),
    );
    let sensor_shape_exists = world.shape(sensor_shape).is_some();
    assert!(sensor_shape_exists);
    let visitor = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([3.0_f32, 1.0])
            .build(),
    );
    let visitor_shape = world.create_polygon_shape_for(
        visitor,
        &ShapeDef::builder()
            .density(1.0)
            .enable_sensor_events(true)
            .build(),
        &shapes::box_polygon(0.25_f32, 0.25),
    );
    let visitor_shape_exists = world.shape(visitor_shape).is_some();
    assert!(visitor_shape_exists);

    world.step(1.0 / 60.0, 4);

    let ray_hits = world.cast_ray_all(
        Position::new(0.0, 5.0),
        [0.0_f32, -10.0],
        QueryFilter::default(),
    );
    assert!(!ray_hits.is_empty());
    let cast_points = [
        Vec2::new(-0.25, 2.0),
        Vec2::new(0.25, 2.0),
        Vec2::new(0.25, 2.5),
        Vec2::new(-0.25, 2.5),
    ];
    let shape_hits = world.cast_shape_points(
        Position::ZERO,
        cast_points,
        0.0,
        [0.0_f32, -4.0],
        QueryFilter::default(),
    );
    assert!(!shape_hits.is_empty());
    let local_cast_points = [
        Vec2::new(-0.25, -0.25),
        Vec2::new(0.25, -0.25),
        Vec2::new(0.25, 0.25),
        Vec2::new(-0.25, 0.25),
    ];
    let offset_shape_hits = world.cast_shape_points_with_offset(
        Position::ZERO,
        local_cast_points,
        0.0,
        [0.0_f32, 2.0],
        0.0_f32,
        [0.0_f32, -4.0],
        QueryFilter::default(),
    );
    assert!(!offset_shape_hits.is_empty());
    let mover_planes = world.collide_mover(
        Position::ZERO,
        [5.0_f32, 0.7],
        [5.0_f32, 1.5],
        0.25,
        QueryFilter::default(),
    );
    assert!(!mover_planes.is_empty());

    let body_events = world.body_events();
    assert!(!body_events.is_empty());
    let contact_events = world.contact_events();
    assert!(!contact_events.begin.is_empty());
    let joint_events = world.joint_events();
    assert!(joint_events.is_empty());
    let sensor_events = world.sensor_events();
    assert!(!sensor_events.begin.is_empty());

    world.set_user_data(29_u32);
    let has_user_data = boxdd::World::has_user_data(&world);
    assert!(has_user_data);
    let user_data_cleared = world.clear_user_data();
    assert!(user_data_cleared);

    world.set_friction_callback(average_coefficient);
    world.set_restitution_callback(average_coefficient);
    world.clear_friction_callback();
    world.clear_restitution_callback();
}
