use boxdd::{
    BodyBuilder, BodyId, BodyType, ChainDef, CollisionPlane, MaterialMixInput, Position,
    QueryFilter, Rot, ShapeDef, Transform, Vec2, World, WorldDef, WorldScalar, WorldTransform,
    is_valid_float, length_units_per_meter, shapes,
};

fn approx_eq(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

fn approx_world_eq(actual: WorldScalar, expected: WorldScalar) -> bool {
    (actual - expected).abs() <= 1.0e-5
}

fn average_coefficient(a: MaterialMixInput, b: MaterialMixInput) -> f32 {
    0.5 * (a.coefficient + b.coefficient)
}

fn accept_contact_pair(_: boxdd::ShapeId, _: boxdd::ShapeId) -> bool {
    true
}

fn accept_pre_solve(_: boxdd::ShapeId, _: boxdd::ShapeId, _: Position, _: Vec2) -> bool {
    true
}

struct NoopDebugDraw;

impl boxdd::DebugDraw for NoopDebugDraw {}

#[test]
fn body_runtime_state_forces_mass_and_lifecycle_succeed() {
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let world_is_valid = boxdd::World::is_valid(&world);
    let builder = BodyBuilder::default();
    let builder = BodyBuilder::body_type(builder, BodyType::Dynamic);
    let builder = BodyBuilder::position(builder, [0.5_f32, 1.0]);
    let builder = BodyBuilder::linear_damping(builder, 0.2);
    let builder = BodyBuilder::angular_damping(builder, 0.3);
    let builder = BodyBuilder::gravity_scale(builder, 0.8);
    let body_id = boxdd::World::create_body_id(&mut world, BodyBuilder::build(builder));

    let shape_builder = ShapeDef::builder();
    let shape_builder = boxdd::ShapeDefBuilder::density(shape_builder, 2.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_builder);
    let polygon = shapes::box_polygon(0.5_f32, 0.25);
    let shape_id =
        boxdd::World::create_polygon_shape_for(&mut world, body_id, &shape_def, &polygon);
    let shape_exists = boxdd::World::try_shape(&mut world, shape_id).is_ok();

    boxdd::World::body_wake_touching(&mut world, body_id);

    let mut body: boxdd::Body<'_> =
        boxdd::World::try_body(&mut world, body_id).expect("body should remain valid");
    let world_id = boxdd::Body::try_world_id_raw(&body);
    let position = boxdd::Body::position(&body);
    let transform = boxdd::Body::transform(&body);
    let transform_position = WorldTransform::position(transform);
    let local_point = boxdd::Body::local_point(&body, [1.0_f32, 1.5]);
    let world_point = boxdd::Body::world_point(&body, local_point);
    let local_vector = boxdd::Body::local_vector(&body, [1.0_f32, -0.5]);
    let world_vector = boxdd::Body::world_vector(&body, local_vector);

    boxdd::Body::set_angular_velocity(&mut body, 0.75);
    let local_point_velocity = boxdd::Body::local_point_velocity(&body, [0.25_f32, 0.0]);
    let world_point_velocity = boxdd::Body::try_world_point_velocity(&body, [0.75_f32, 1.0])
        .expect("world-point velocity query should succeed");

    let mass = boxdd::Body::mass(&body);
    let inertia = boxdd::Body::rotational_inertia(&body);
    let local_center = boxdd::Body::local_center_of_mass(&body);
    let world_center = boxdd::Body::try_world_center_of_mass(&body);
    let mass_data = boxdd::Body::mass_data(&body);
    boxdd::Body::set_mass_data(&mut body, mass_data);
    boxdd::Body::apply_mass_from_shapes(&mut body);

    let body_type = boxdd::Body::body_type(&body);
    boxdd::Body::set_body_type(&mut body, BodyType::Kinematic);
    boxdd::Body::set_body_type(&mut body, BodyType::Dynamic);

    let gravity_scale = boxdd::Body::gravity_scale(&body);
    let linear_damping = boxdd::Body::linear_damping(&body);
    let angular_damping = boxdd::Body::angular_damping(&body);

    boxdd::Body::disable(&mut body);
    boxdd::Body::enable(&mut body);
    let target = WorldTransform::from_pos_angle([0.75_f32, 1.25], 0.1);
    boxdd::Body::set_target_transform(&mut body, target, 1.0 / 60.0, true);
    boxdd::Body::set_position_and_rotation(&mut body, [0.5_f32, 1.0], 0.0);
    boxdd::Body::try_set_position_and_rotation(&mut body, [0.5_f32, 1.0], 0.0)
        .expect("validated position update should succeed");

    boxdd::Body::apply_force(&mut body, [2.0_f32, -1.0], [0.5_f32, 1.0], true);
    boxdd::Body::apply_force_to_center(&mut body, [1.0_f32, 0.5], true);
    boxdd::Body::apply_torque(&mut body, 0.25, true);
    boxdd::Body::apply_linear_impulse(&mut body, [0.2_f32, 0.1], [0.5_f32, 1.0], true);
    boxdd::Body::apply_linear_impulse_to_center(&mut body, [-0.1_f32, 0.2], true);
    boxdd::Body::apply_angular_impulse(&mut body, 0.15, true);

    boxdd::Body::set_user_data(&mut body, 17_u32);
    let user_data =
        boxdd::Body::try_user_data_ptr_raw(&body).expect("user-data query should succeed");
    boxdd::Body::clear_forces(&mut body);
    let user_data_cleared = boxdd::Body::clear_user_data(&mut body);
    std::mem::drop(body);

    let owned_def = BodyBuilder::build(BodyBuilder::new());
    let owned = boxdd::World::create_body_owned(&mut world, owned_def);
    let owned_valid = boxdd::OwnedBody::is_valid(&owned);
    boxdd::OwnedBody::destroy(owned);
    std::mem::drop(world);

    assert!(world_id.is_ok());
    assert!(world_is_valid);
    assert!(shape_exists);
    assert!(approx_world_eq(position.x, 0.5));
    assert!(approx_world_eq(position.y, 1.0));
    assert!(approx_world_eq(transform_position.x, position.x));
    assert!(approx_world_eq(transform_position.y, position.y));
    assert!(local_point.x.is_finite() && local_point.y.is_finite());
    assert!(approx_world_eq(world_point.x, 1.0));
    assert!(approx_world_eq(world_point.y, 1.5));
    assert!(local_vector.x.is_finite() && local_vector.y.is_finite());
    assert!(approx_eq(world_vector.x, 1.0));
    assert!(approx_eq(world_vector.y, -0.5));
    assert!(local_point_velocity.x.is_finite() && local_point_velocity.y.is_finite());
    assert!(world_point_velocity.x.is_finite() && world_point_velocity.y.is_finite());
    assert!(mass > 0.0);
    assert!(inertia > 0.0);
    assert!(local_center.x.is_finite() && local_center.y.is_finite());
    assert!(world_center.is_ok());
    assert!(mass_data.mass > 0.0);
    assert_eq!(body_type, BodyType::Dynamic);
    assert!(approx_eq(gravity_scale, 0.8));
    assert!(approx_eq(linear_damping, 0.2));
    assert!(approx_eq(angular_damping, 0.3));
    assert!(!user_data.is_null());
    assert!(user_data_cleared);
    assert!(owned_valid);
}

#[test]
fn body_runtime_flags_enumeration_and_motion_paths_succeed() {
    let mut world: World = World::new(WorldDef::default()).expect("world creation should succeed");
    let body_a_builder = BodyBuilder::body_type(BodyBuilder::new(), BodyType::Dynamic);
    let body_a = boxdd::World::create_body_id(&mut world, BodyBuilder::build(body_a_builder));
    let body_b_builder = BodyBuilder::body_type(BodyBuilder::new(), BodyType::Dynamic);
    let body_b_builder = BodyBuilder::position(body_b_builder, [2.0_f32, 0.0]);
    let body_b = boxdd::World::create_body_id(&mut world, BodyBuilder::build(body_b_builder));

    let shape_builder = boxdd::ShapeDefBuilder::density(ShapeDef::builder(), 1.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_builder);
    let polygon = shapes::box_polygon(0.5_f32, 0.5);
    let shape_id = boxdd::World::create_polygon_shape_for(&mut world, body_a, &shape_def, &polygon);
    let joint_base = boxdd::JointBase::new(body_a, body_b);
    let joint_def = boxdd::DistanceJointDef::length(boxdd::DistanceJointDef::new(joint_base), 2.0);
    let joint_id = boxdd::World::create_distance_joint_id(&mut world, &joint_def);

    let desired_locks = boxdd::MotionLocks::new(false, true, false);
    boxdd::World::set_body_motion_locks(&mut world, body_a, desired_locks);
    let observed_locks = boxdd::World::body_motion_locks(&world, body_a);
    let recycling_enabled = boxdd::World::try_body_is_contact_recycling_enabled(&world, body_a)
        .expect("contact-recycling query should succeed");

    let mut body: boxdd::Body<'_> =
        boxdd::World::try_body(&mut world, body_a).expect("body should remain valid");
    boxdd::Body::set_linear_velocity(&mut body, [1.0_f32, -0.5]);
    boxdd::Body::set_angular_velocity(&mut body, 0.75);
    boxdd::Body::set_linear_damping(&mut body, 0.2);
    boxdd::Body::set_angular_damping(&mut body, 0.3);
    boxdd::Body::set_gravity_scale(&mut body, 0.8);
    boxdd::Body::enable_sleep(&mut body, true);
    boxdd::Body::set_sleep_threshold(&mut body, 0.25);
    boxdd::Body::set_awake(&mut body, true);
    boxdd::Body::set_bullet(&mut body, true);
    boxdd::Body::enable_contact_events(&mut body, true);
    boxdd::Body::enable_hit_events(&mut body, true);
    boxdd::Body::set_name(&mut body, "contract");

    let bounds = boxdd::Body::aabb(&body);
    let bounds_valid = boxdd::Aabb::is_valid(bounds);
    let linear_velocity = boxdd::Body::linear_velocity(&body);
    let angular_velocity = boxdd::Body::angular_velocity(&body);
    let rotation = boxdd::Body::rotation(&body);
    let rotation_valid = boxdd::Rot::is_valid(rotation);
    let shape_count = boxdd::Body::shape_count(&body);
    let shapes = boxdd::Body::shapes(&body);
    let joint_count = boxdd::Body::joint_count(&body);
    let joints = boxdd::Body::joints(&body);
    let sleep_threshold = boxdd::Body::sleep_threshold(&body);
    let sleep_enabled = boxdd::Body::is_sleep_enabled(&body);
    let awake = boxdd::Body::is_awake(&body);
    let enabled = boxdd::Body::is_enabled(&body);
    let bullet = boxdd::Body::is_bullet(&body);
    let name = boxdd::Body::name(&body);
    std::mem::drop(body);
    std::mem::drop(world);

    assert_eq!(observed_locks, desired_locks);
    assert!(recycling_enabled);
    assert!(bounds_valid);
    assert!(approx_eq(linear_velocity.x, 1.0));
    assert!(approx_eq(linear_velocity.y, -0.5));
    assert!(approx_eq(angular_velocity, 0.75));
    assert!(rotation_valid);
    assert_eq!(shape_count, 1);
    assert_eq!(shapes, [shape_id]);
    assert_eq!(joint_count, 1);
    assert_eq!(joints, [joint_id]);
    assert!(approx_eq(sleep_threshold, 0.25));
    assert!(sleep_enabled);
    assert!(awake);
    assert!(enabled);
    assert!(bullet);
    assert_eq!(name.as_deref(), Some("contract"));
}

#[test]
fn world_runtime_controls_queries_callbacks_and_debug_draw_succeed() {
    let mut world: World = World::new(WorldDef::default()).expect("world creation should succeed");
    let body_builder = BodyBuilder::body_type(BodyBuilder::new(), BodyType::Dynamic);
    let body = boxdd::World::create_body_id(&mut world, BodyBuilder::build(body_builder));
    let shape_builder = boxdd::ShapeDefBuilder::density(ShapeDef::builder(), 1.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_builder);
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let shape = boxdd::World::create_circle_shape_for(&mut world, body, &shape_def, &circle);

    boxdd::World::enable_sleeping(&mut world, true);
    boxdd::World::enable_continuous(&mut world, true);
    boxdd::World::enable_warm_starting(&mut world, true);
    boxdd::World::set_restitution_threshold(&mut world, 1.5);
    boxdd::World::set_hit_event_threshold(&mut world, 2.5);
    boxdd::World::set_contact_tuning(&mut world, 30.0, 1.0, 3.0);
    boxdd::World::set_maximum_linear_speed(&mut world, 120.0);
    boxdd::World::set_gravity(&mut world, [0.0_f32, -9.8]);
    boxdd::World::set_custom_filter(&mut world, accept_contact_pair);
    boxdd::World::set_pre_solve(&mut world, accept_pre_solve);
    boxdd::World::try_set_worker_count(&mut world, 2).expect("worker-count update should succeed");

    let explosion = boxdd::ExplosionDef::mask_bits(boxdd::ExplosionDef::new(), u64::MAX);
    let explosion = boxdd::ExplosionDef::position(explosion, Position::ZERO);
    let explosion = boxdd::ExplosionDef::radius(explosion, 1.0);
    let explosion = boxdd::ExplosionDef::falloff(explosion, 1.0);
    let explosion = boxdd::ExplosionDef::impulse_per_length(explosion, 1.0);
    boxdd::World::explode(&mut world, &explosion);

    let gravity = boxdd::World::gravity(&world);
    let counters = boxdd::World::counters(&world);
    let _profile = boxdd::World::profile(&world);
    let awake_body_count = boxdd::World::awake_body_count(&world);
    let sleeping_enabled = boxdd::World::is_sleeping_enabled(&world);
    let continuous_enabled = boxdd::World::is_continuous_enabled(&world);
    let warm_starting_enabled = boxdd::World::is_warm_starting_enabled(&world);
    let restitution_threshold = boxdd::World::restitution_threshold(&world);
    let hit_event_threshold = boxdd::World::hit_event_threshold(&world);
    let maximum_linear_speed = boxdd::World::maximum_linear_speed(&world);
    let bounds = boxdd::World::bounds(&world);
    let bounds_valid = boxdd::Aabb::is_valid(bounds);
    let _maximum_capacity = boxdd::World::maximum_capacity(&world);
    let recycle_distance = boxdd::World::try_contact_recycle_distance(&world)
        .expect("contact-recycle query should succeed");
    let worker_count = boxdd::World::worker_count(&world);

    let query_bounds = boxdd::Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]);
    let overlaps =
        boxdd::World::overlap_aabb(&world, Position::ZERO, query_bounds, QueryFilter::default());
    let proxy = [
        Vec2::new(-0.75, -0.75),
        Vec2::new(0.75, -0.75),
        Vec2::new(0.75, 0.75),
        Vec2::new(-0.75, 0.75),
    ];
    let polygon_overlaps = boxdd::World::overlap_polygon_points(
        &world,
        Position::ZERO,
        proxy,
        0.0,
        QueryFilter::default(),
    );
    let closest_hit = boxdd::World::cast_ray_closest(
        &world,
        Position::new(-2.0, 0.0),
        [4.0_f32, 0.0],
        QueryFilter::default(),
    );
    let mover_fraction = boxdd::World::cast_mover(
        &world,
        Position::new(-2.0, 0.0),
        [0.0_f32, -0.25],
        [0.0_f32, 0.25],
        0.25,
        [4.0_f32, 0.0],
        QueryFilter::default(),
    );
    let mut drawer = NoopDebugDraw;
    boxdd::World::debug_draw(&mut world, &mut drawer, boxdd::DebugDrawOptions::default());
    boxdd::World::clear_custom_filter(&mut world);
    boxdd::World::clear_pre_solve(&mut world);
    std::mem::drop(world);

    assert!(approx_eq(gravity.y, -9.8));
    assert_eq!(counters.body_count, 1);
    assert!(awake_body_count >= 1);
    assert!(sleeping_enabled);
    assert!(continuous_enabled);
    assert!(warm_starting_enabled);
    assert!(approx_eq(restitution_threshold, 1.5));
    assert!(approx_eq(hit_event_threshold, 2.5));
    assert!(approx_eq(maximum_linear_speed, 120.0));
    assert!(bounds_valid);
    assert!(recycle_distance.is_finite());
    assert_eq!(worker_count.get(), 2);
    assert!(overlaps.contains(&shape));
    assert!(polygon_overlaps.contains(&shape));
    assert!(closest_hit.is_some());
    assert!((0.0..=1.0).contains(&mover_fraction));
}

#[cfg(feature = "double-precision")]
#[test]
fn body_world_coordinate_apis_preserve_double_precision() {
    let origin = Position::new(1_000_000_000_000.25, -1_000_000_000_000.5);
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let builder = BodyBuilder::body_type(BodyBuilder::new(), BodyType::Dynamic);
    let builder = BodyBuilder::position(builder, origin);
    let body_id = boxdd::World::create_body_id(&mut world, BodyBuilder::build(builder));

    let mut body: boxdd::Body<'_> =
        boxdd::World::try_body(&mut world, body_id).expect("body should remain valid");
    let initial_position = boxdd::Body::position(&body);
    let initial_transform = boxdd::Body::transform(&body);
    let initial_transform_position = WorldTransform::position(initial_transform);
    let initial_center = boxdd::Body::world_center_of_mass(&body);

    let local_point = Vec2::new(0.5, -0.25);
    let world_point = boxdd::Body::world_point(&body, local_point);
    let round_trip_local_point = boxdd::Body::local_point(&body, world_point);
    let world_point_velocity = boxdd::Body::world_point_velocity(&body, world_point);
    let world_point_velocity_valid = Vec2::is_valid(world_point_velocity);

    let moved = Position::new(origin.x + 4.0, origin.y + 8.0);
    boxdd::Body::set_position_and_rotation(&mut body, moved, 0.0);
    let moved_position = boxdd::Body::position(&body);
    boxdd::Body::apply_force(&mut body, [1.0_f32, 0.0], moved, true);
    boxdd::Body::apply_linear_impulse(&mut body, [0.25_f32, 0.0], moved, true);
    let target = WorldTransform::new(moved, Rot::IDENTITY);
    boxdd::Body::set_target_transform(&mut body, target, 1.0 / 60.0, true);
    std::mem::drop(body);
    std::mem::drop(world);

    assert_eq!(initial_position, origin);
    assert_eq!(initial_transform_position, origin);
    assert_eq!(initial_center, origin);
    assert_eq!(world_point, Position::new(origin.x + 0.5, origin.y - 0.25));
    assert_eq!(round_trip_local_point, local_point);
    assert!(world_point_velocity_valid);
    assert_eq!(moved_position, moved);
}

fn settled_contact_world() -> (World, BodyId) {
    let world_builder = WorldDef::builder();
    let world_builder = boxdd::WorldBuilder::gravity(world_builder, [0.0_f32, -10.0]);
    let mut world = World::new(boxdd::WorldBuilder::build(world_builder))
        .expect("world creation should succeed");
    let ground_def = BodyBuilder::build(BodyBuilder::new());
    let ground = boxdd::World::create_body_id(&mut world, ground_def);
    let ground_polygon = shapes::box_polygon(10.0_f32, 0.5);
    boxdd::World::create_polygon_shape_for(
        &mut world,
        ground,
        &ShapeDef::default(),
        &ground_polygon,
    );

    let dynamic_builder = BodyBuilder::body_type(BodyBuilder::new(), BodyType::Dynamic);
    let dynamic_builder = BodyBuilder::position(dynamic_builder, [0.0_f32, 3.0]);
    let dynamic = boxdd::World::create_body_id(&mut world, BodyBuilder::build(dynamic_builder));
    let shape_builder = boxdd::ShapeDefBuilder::density(ShapeDef::builder(), 1.0);
    let dynamic_polygon = shapes::box_polygon(0.5_f32, 0.5);
    boxdd::World::create_polygon_shape_for(
        &mut world,
        dynamic,
        &boxdd::ShapeDefBuilder::build(shape_builder),
        &dynamic_polygon,
    );

    for _ in 0..180 {
        boxdd::World::step(&mut world, 1.0 / 60.0, 4);
    }

    (world, dynamic)
}

#[test]
fn live_body_contact_and_contact_snapshot_succeed() {
    let (mut world, dynamic) = settled_contact_world();

    let body: boxdd::Body<'_> =
        boxdd::World::try_body(&mut world, dynamic).expect("dynamic body should remain valid");
    let contacts = boxdd::Body::contact_data(&body);
    let contact_id = contacts[0].contact_id;
    std::mem::drop(body);
    let contact_valid = boxdd::World::contact_is_valid(&world, contact_id);
    let snapshot = boxdd::World::try_contact_data(&world, contact_id)
        .expect("live contact should produce a snapshot");
    std::mem::drop(world);

    assert!(!contacts.is_empty());
    assert!(contact_valid);
    assert_eq!(snapshot.contact_id, contact_id);
}

#[test]
fn chain_runtime_creation_queries_and_destroy_succeed() {
    let default_def = boxdd::ChainDef::default();
    let default_points = boxdd::ChainDef::points(&default_def);

    let chain_builder = boxdd::ChainDefBuilder::points(
        ChainDef::builder(),
        [
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ],
    );
    let chain_def = boxdd::ChainDefBuilder::build(chain_builder);
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let body_def = BodyBuilder::build(BodyBuilder::new());
    let body_id = boxdd::World::create_body_id(&mut world, body_def);
    let chain_id = boxdd::World::create_chain_for_id(&mut world, body_id, &chain_def);

    let chain: boxdd::Chain<'_> =
        boxdd::World::try_chain(&mut world, chain_id).expect("chain should remain valid");
    let chain_world = boxdd::Chain::try_world_id_raw(&chain);
    let segment_count = boxdd::Chain::segment_count(&chain);
    let segments = boxdd::Chain::segments(&chain);
    let material_count = boxdd::Chain::surface_material_count(&chain);
    let material = boxdd::Chain::surface_material(&chain, 0);
    let material_friction = boxdd::SurfaceMaterial::friction(&material);
    boxdd::Chain::destroy(chain);
    std::mem::drop(world);

    assert!(default_points.is_empty());
    assert!(chain_world.is_ok());
    assert!(segment_count > 0);
    assert!(!segments.is_empty());
    assert!(material_count > 0);
    assert!(material_friction.is_finite());
}

#[test]
fn foundation_collision_and_math_runtime_paths_succeed() {
    let valid_float = is_valid_float(1.25);
    let units_per_meter = length_units_per_meter();
    let angle = boxdd::atan2(1.0, 0.0);
    let no_planes: [CollisionPlane; 0] = [];
    let clipped = boxdd::clip_vector(Vec2::new(3.0, 4.0), &no_planes);
    let cos_sin = boxdd::compute_cos_sin(0.25);
    let between = boxdd::Rot::from_unit_vectors(Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0));

    let sweep = boxdd::Sweep::new(
        [0.0_f32, 0.0],
        [0.0_f32, 0.0],
        [2.0_f32, 0.0],
        Rot::IDENTITY,
        Rot::IDENTITY,
    );
    let checked_sweep_transform = boxdd::Sweep::try_transform_at(sweep, 0.5);
    let sweep_transform = boxdd::Sweep::transform_at(sweep, 0.5);

    let explosion = boxdd::ExplosionDef::default();
    let blast_radius = boxdd::ExplosionDef::blast_radius(&explosion);

    let points = [
        Vec2::new(-0.75, -0.5),
        Vec2::new(0.75, -0.5),
        Vec2::new(0.75, 0.5),
        Vec2::new(-0.75, 0.5),
    ];
    let hull_valid = boxdd::Polygon::hull_is_valid(points);

    let box_polygon = boxdd::Polygon::box_polygon(0.5, 0.25);
    let rounded_box = boxdd::Polygon::rounded_box_polygon(0.5, 0.25, 0.05);
    let offset = Transform::from_pos_angle([1.0_f32, -0.5], 0.2);
    let offset_box = boxdd::Polygon::offset_box_polygon(0.5, 0.25, offset);
    let offset_rounded_box = boxdd::Polygon::offset_rounded_box_polygon(0.5, 0.25, 0.05, offset);
    let offset_polygon = boxdd::Polygon::offset_from_points(points, 0.0, offset);
    let offset_rounded_polygon = boxdd::Polygon::offset_from_points(points, 0.05, offset);

    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let body_def = BodyBuilder::build(BodyBuilder::new());
    let body_id = boxdd::World::create_body_id(&mut world, body_def);
    let mut body: boxdd::Body<'_> =
        boxdd::World::try_body(&mut world, body_id).expect("polygon body should remain valid");
    let shape =
        boxdd::Body::create_polygon_from_points(&mut body, &ShapeDef::default(), points, 0.0);

    let cos_sin_cosine = Rot::cosine(cos_sin);
    let cos_sin_sine = Rot::sine(cos_sin);
    let between_valid = Rot::is_valid(between);
    let checked_sweep_transform =
        checked_sweep_transform.expect("valid sweep transform should succeed");
    let checked_sweep_position = Transform::position(checked_sweep_transform);
    let sweep_position = Transform::position(sweep_transform);
    let box_count = boxdd::Polygon::count(&box_polygon);
    let rounded_box_count = boxdd::Polygon::count(&rounded_box);
    let offset_box_count = boxdd::Polygon::count(&offset_box);
    let offset_rounded_box_count = boxdd::Polygon::count(&offset_rounded_box);
    let shape_exists = shape.is_some();
    std::mem::drop(shape);
    std::mem::drop(body);
    std::mem::drop(world);

    assert!(valid_float);
    assert!(units_per_meter.is_finite() && units_per_meter > 0.0);
    assert!(approx_eq(angle, core::f32::consts::FRAC_PI_2));
    assert!(approx_eq(clipped.x, 3.0) && approx_eq(clipped.y, 4.0));
    assert!(cos_sin_cosine.is_finite() && cos_sin_sine.is_finite());
    assert!(between_valid);
    assert_eq!(checked_sweep_position, sweep_position);
    assert!(approx_eq(sweep_position.x, 1.0));
    assert!(approx_eq(sweep_position.y, 0.0));
    assert!(blast_radius.is_finite() && blast_radius >= 0.0);
    assert!(hull_valid);
    assert_eq!(box_count, 4);
    assert_eq!(rounded_box_count, 4);
    assert_eq!(offset_box_count, 4);
    assert_eq!(offset_rounded_box_count, 4);
    assert!(offset_polygon.is_some());
    assert!(offset_rounded_polygon.is_some());
    assert!(shape_exists);
}

#[test]
fn world_queries_events_user_data_and_callback_cleanup_succeed() {
    let mut world = World::new(WorldDef::default()).expect("world creation should succeed");
    let ground_def = BodyBuilder::build(BodyBuilder::new());
    let ground = boxdd::World::create_body_id(&mut world, ground_def);
    let ground_shape_builder =
        boxdd::ShapeDefBuilder::enable_contact_events(ShapeDef::builder(), true);
    let ground_shape_def = boxdd::ShapeDefBuilder::build(ground_shape_builder);
    let ground_polygon = shapes::box_polygon(10.0_f32, 0.5);
    let ground_shape = boxdd::World::create_polygon_shape_for(
        &mut world,
        ground,
        &ground_shape_def,
        &ground_polygon,
    );
    let ground_shape_exists = boxdd::World::try_shape(&mut world, ground_shape).is_ok();

    let wall_builder = BodyBuilder::position(BodyBuilder::new(), [1.0_f32, 1.0]);
    let wall = boxdd::World::create_body_id(&mut world, BodyBuilder::build(wall_builder));
    let wall_polygon = shapes::box_polygon(0.25_f32, 1.0);
    let wall_shape = boxdd::World::create_polygon_shape_for(
        &mut world,
        wall,
        &ShapeDef::default(),
        &wall_polygon,
    );
    let wall_shape_exists = boxdd::World::try_shape(&mut world, wall_shape).is_ok();

    let dynamic_builder = BodyBuilder::body_type(BodyBuilder::new(), BodyType::Dynamic);
    let dynamic_builder = BodyBuilder::position(dynamic_builder, [0.0_f32, 0.8]);
    let dynamic_builder = BodyBuilder::linear_velocity(dynamic_builder, [0.1_f32, 0.0]);
    let dynamic = boxdd::World::create_body_id(&mut world, BodyBuilder::build(dynamic_builder));
    let dynamic_shape_builder = boxdd::ShapeDefBuilder::density(ShapeDef::builder(), 1.0);
    let dynamic_shape_builder =
        boxdd::ShapeDefBuilder::enable_contact_events(dynamic_shape_builder, true);
    let dynamic_shape_def = boxdd::ShapeDefBuilder::build(dynamic_shape_builder);
    let dynamic_polygon = shapes::box_polygon(0.4_f32, 0.4);
    let dynamic_shape = boxdd::World::create_polygon_shape_for(
        &mut world,
        dynamic,
        &dynamic_shape_def,
        &dynamic_polygon,
    );
    let dynamic_shape_exists = boxdd::World::try_shape(&mut world, dynamic_shape).is_ok();

    let sensor_builder = BodyBuilder::position(BodyBuilder::new(), [3.0_f32, 1.0]);
    let sensor_body = boxdd::World::create_body_id(&mut world, BodyBuilder::build(sensor_builder));
    let sensor_shape_builder = boxdd::ShapeDefBuilder::sensor(ShapeDef::builder(), true);
    let sensor_shape_builder =
        boxdd::ShapeDefBuilder::enable_sensor_events(sensor_shape_builder, true);
    let sensor_shape_def = boxdd::ShapeDefBuilder::build(sensor_shape_builder);
    let sensor_polygon = shapes::box_polygon(0.75_f32, 0.75);
    let sensor_shape = boxdd::World::create_polygon_shape_for(
        &mut world,
        sensor_body,
        &sensor_shape_def,
        &sensor_polygon,
    );
    let sensor_shape_exists = boxdd::World::try_shape(&mut world, sensor_shape).is_ok();

    let visitor_builder = BodyBuilder::body_type(BodyBuilder::new(), BodyType::Dynamic);
    let visitor_builder = BodyBuilder::position(visitor_builder, [3.0_f32, 1.0]);
    let visitor = boxdd::World::create_body_id(&mut world, BodyBuilder::build(visitor_builder));
    let visitor_shape_builder = boxdd::ShapeDefBuilder::density(ShapeDef::builder(), 1.0);
    let visitor_shape_builder =
        boxdd::ShapeDefBuilder::enable_sensor_events(visitor_shape_builder, true);
    let visitor_shape_def = boxdd::ShapeDefBuilder::build(visitor_shape_builder);
    let visitor_polygon = shapes::box_polygon(0.25_f32, 0.25);
    let visitor_shape = boxdd::World::create_polygon_shape_for(
        &mut world,
        visitor,
        &visitor_shape_def,
        &visitor_polygon,
    );
    let visitor_shape_exists = boxdd::World::try_shape(&mut world, visitor_shape).is_ok();

    boxdd::World::try_step(&mut world, 1.0 / 60.0, 4)
        .expect("world step should capture owned event data");

    let ray_hits = boxdd::World::cast_ray_all(
        &world,
        Position::new(0.0, 5.0),
        [0.0_f32, -10.0],
        QueryFilter::default(),
    );
    let cast_points = [
        Vec2::new(-0.25, 2.0),
        Vec2::new(0.25, 2.0),
        Vec2::new(0.25, 2.5),
        Vec2::new(-0.25, 2.5),
    ];
    let shape_hits = boxdd::World::cast_shape_points(
        &world,
        Position::ZERO,
        cast_points,
        0.0,
        [0.0_f32, -4.0],
        QueryFilter::default(),
    );
    let local_cast_points = [
        Vec2::new(-0.25, -0.25),
        Vec2::new(0.25, -0.25),
        Vec2::new(0.25, 0.25),
        Vec2::new(-0.25, 0.25),
    ];
    let offset_shape_hits = boxdd::World::cast_shape_points_with_offset(
        &world,
        Position::ZERO,
        local_cast_points,
        0.0,
        [0.0_f32, 2.0],
        0.0_f32,
        [0.0_f32, -4.0],
        QueryFilter::default(),
    );
    let mover_planes = boxdd::World::collide_mover(
        &world,
        Position::ZERO,
        [5.0_f32, 0.7],
        [5.0_f32, 1.5],
        0.25,
        QueryFilter::default(),
    );

    let body_events = boxdd::World::body_events(&world);
    let contact_events = boxdd::World::contact_events(&world);
    let joint_events = boxdd::World::joint_events(&world);
    let sensor_events = boxdd::World::sensor_events(&world);

    boxdd::World::set_user_data(&mut world, 29_u32);
    let has_user_data = boxdd::World::has_user_data(&world);
    let user_data_cleared = boxdd::World::clear_user_data(&mut world);

    boxdd::World::set_friction_callback(&mut world, average_coefficient);
    boxdd::World::set_restitution_callback(&mut world, average_coefficient);
    boxdd::World::clear_friction_callback(&mut world);
    boxdd::World::clear_restitution_callback(&mut world);
    std::mem::drop(world);

    assert!(ground_shape_exists);
    assert!(wall_shape_exists);
    assert!(dynamic_shape_exists);
    assert!(sensor_shape_exists);
    assert!(visitor_shape_exists);
    assert!(!ray_hits.is_empty());
    assert!(!shape_hits.is_empty());
    assert!(!offset_shape_hits.is_empty());
    assert!(!mover_planes.is_empty());
    assert!(!body_events.is_empty());
    assert!(!contact_events.begin.is_empty());
    assert!(joint_events.is_empty());
    assert!(!sensor_events.begin.is_empty());
    assert!(has_user_data);
    assert!(user_data_cleared);
}
