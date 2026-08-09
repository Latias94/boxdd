use boxdd::{
    BodyBuilder, BodyId, BodyType, ChainDef, CollisionPlane, Foundation, MaterialMixInput,
    Position, QueryFilter, Rot, ShapeDef, ShapeProxy, Transform, Vec2, World, WorldScalar,
    WorldTransform, is_valid_float, shapes,
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
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");
    let builder = world.body_builder();
    let builder = BodyBuilder::body_type(builder, BodyType::Dynamic);
    let builder = BodyBuilder::position(builder, [0.5_f32, 1.0]);
    let builder = BodyBuilder::linear_damping(builder, 0.2);
    let builder = BodyBuilder::angular_damping(builder, 0.3);
    let builder = BodyBuilder::gravity_scale(builder, 0.8);
    let body_id = world
        .create_body(BodyBuilder::build(builder).unwrap())
        .expect("body creation should succeed");

    let shape_builder = ShapeDef::builder();
    let shape_builder = boxdd::ShapeDefBuilder::density(shape_builder, 2.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_builder).unwrap();
    let polygon = shapes::box_polygon(0.5_f32, 0.25).unwrap();
    let shape_id = world
        .body(body_id)
        .expect("body should remain valid")
        .create_polygon(&shape_def, &polygon)
        .expect("polygon creation should succeed");
    let shape_exists = world.shape(shape_id).is_ok();

    let mut body = world.body(body_id).expect("body should remain valid");
    body.wake_touching().expect("wake should succeed");
    let position = body.position().expect("position should be readable");
    let transform = body.transform().expect("transform should be readable");
    let transform_position = WorldTransform::position(transform);
    let local_point = body
        .local_point([1.0_f32, 1.5])
        .expect("local-point conversion should succeed");
    let world_point = body
        .world_point(local_point)
        .expect("world-point conversion should succeed");
    let local_vector = body
        .local_vector([1.0_f32, -0.5])
        .expect("local-vector conversion should succeed");
    let world_vector = body
        .world_vector(local_vector)
        .expect("world-vector conversion should succeed");

    body.set_angular_velocity(0.75)
        .expect("angular velocity should be writable");
    let local_point_velocity = body
        .local_point_velocity([0.25_f32, 0.0])
        .expect("local-point velocity query should succeed");
    let world_point_velocity = body
        .world_point_velocity([0.75_f32, 1.0])
        .expect("world-point velocity query should succeed");

    let mass = body.mass().expect("mass should be readable");
    let inertia = body
        .rotational_inertia()
        .expect("inertia should be readable");
    let local_center = body
        .local_center_of_mass()
        .expect("local center should be readable");
    let world_center = body
        .world_center_of_mass()
        .expect("world center should be readable");
    let mass_data = body.mass_data().expect("mass data should be readable");
    body.set_mass_data(mass_data)
        .expect("mass data should be writable");
    body.apply_mass_from_shapes()
        .expect("shape mass application should succeed");

    let body_type = body.body_type().expect("body type should be readable");
    body.set_body_type(BodyType::Kinematic)
        .expect("body type should be writable");
    body.set_body_type(BodyType::Dynamic)
        .expect("body type should be writable");

    let gravity_scale = body
        .gravity_scale()
        .expect("gravity scale should be readable");
    let linear_damping = body
        .linear_damping()
        .expect("linear damping should be readable");
    let angular_damping = body
        .angular_damping()
        .expect("angular damping should be readable");

    body.disable().expect("body disable should succeed");
    body.enable().expect("body enable should succeed");
    let target = WorldTransform::from_pos_angle([0.75_f32, 1.25], 0.1).unwrap();
    body.set_target_transform(target, 1.0 / 60.0, true)
        .expect("target transform should be writable");
    body.set_position_and_rotation([0.5_f32, 1.0], 0.0)
        .expect("position update should succeed");

    body.apply_force([2.0_f32, -1.0], [0.5_f32, 1.0], true)
        .expect("force application should succeed");
    body.apply_force_to_center([1.0_f32, 0.5], true)
        .expect("center force application should succeed");
    body.apply_torque(0.25, true)
        .expect("torque application should succeed");
    body.apply_linear_impulse([0.2_f32, 0.1], [0.5_f32, 1.0], true)
        .expect("linear impulse application should succeed");
    body.apply_linear_impulse_to_center([-0.1_f32, 0.2], true)
        .expect("center impulse application should succeed");
    body.apply_angular_impulse(0.15, true)
        .expect("angular impulse application should succeed");

    body.set_user_data(17_u32)
        .expect("user data should be writable");
    let user_data = body
        .user_data_ptr_raw()
        .expect("user-data query should succeed");
    body.clear_forces().expect("force clearing should succeed");
    let user_data_cleared = body
        .clear_user_data()
        .expect("user data clearing should succeed");
    let destroy_id = world
        .create_body(
            BodyBuilder::build(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder(),
            )
            .unwrap(),
        )
        .expect("disposable body creation should succeed");
    world
        .body(destroy_id)
        .expect("disposable body should remain valid")
        .destroy()
        .expect("body destruction should succeed");

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
    assert!(world_center.x.is_finite() && world_center.y.is_finite());
    assert!(mass_data.mass() > 0.0);
    assert_eq!(body_type, BodyType::Dynamic);
    assert!(approx_eq(gravity_scale, 0.8));
    assert!(approx_eq(linear_damping, 0.2));
    assert!(approx_eq(angular_damping, 0.3));
    assert!(!user_data.is_null());
    assert!(user_data_cleared);
}

#[test]
fn body_runtime_flags_enumeration_and_motion_paths_succeed() {
    let mut world: World = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");
    let body_a_builder = BodyBuilder::body_type(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
        BodyType::Dynamic,
    );
    let body_a = world
        .create_body(BodyBuilder::build(body_a_builder).unwrap())
        .expect("first body creation should succeed");
    let body_b_builder = BodyBuilder::body_type(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
        BodyType::Dynamic,
    );
    let body_b_builder = BodyBuilder::position(body_b_builder, [2.0_f32, 0.0]);
    let body_b = world
        .create_body(BodyBuilder::build(body_b_builder).unwrap())
        .expect("second body creation should succeed");

    let shape_builder = boxdd::ShapeDefBuilder::density(ShapeDef::builder(), 1.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_builder).unwrap();
    let polygon = shapes::box_polygon(0.5_f32, 0.5).unwrap();
    let shape_id = world
        .body(body_a)
        .expect("first body should remain valid")
        .create_polygon(&shape_def, &polygon)
        .expect("polygon creation should succeed");
    let joint_base = world.joint_base(body_a, body_b).unwrap();
    let joint_def = boxdd::DistanceJointDef::length(boxdd::DistanceJointDef::new(joint_base), 2.0);
    let joint_id = world
        .create_distance_joint(&joint_def)
        .expect("distance joint creation should succeed");

    let desired_locks = boxdd::MotionLocks::new(false, true, false);
    let mut body = world.body(body_a).expect("body should remain valid");
    body.set_motion_locks(desired_locks)
        .expect("motion locks should be writable");
    let observed_locks = body
        .motion_locks()
        .expect("motion locks should be readable");
    let recycling_enabled = body
        .is_contact_recycling_enabled()
        .expect("contact-recycling query should succeed");

    body.set_linear_velocity([1.0_f32, -0.5])
        .expect("linear velocity should be writable");
    body.set_angular_velocity(0.75)
        .expect("angular velocity should be writable");
    body.set_linear_damping(0.2)
        .expect("linear damping should be writable");
    body.set_angular_damping(0.3)
        .expect("angular damping should be writable");
    body.set_gravity_scale(0.8)
        .expect("gravity scale should be writable");
    body.enable_sleep(true)
        .expect("sleep flag should be writable");
    body.set_sleep_threshold(0.25)
        .expect("sleep threshold should be writable");
    body.set_awake(true).expect("awake flag should be writable");
    body.set_bullet(true)
        .expect("bullet flag should be writable");
    body.enable_contact_events(true)
        .expect("contact events should be configurable");
    body.enable_hit_events(true)
        .expect("hit events should be configurable");
    body.set_name("contract")
        .expect("body name should be writable");

    let bounds = body.aabb().expect("body bounds should be readable");
    let bounds_valid = boxdd::Aabb::is_valid(bounds);
    let linear_velocity = body
        .linear_velocity()
        .expect("linear velocity should be readable");
    let angular_velocity = body
        .angular_velocity()
        .expect("angular velocity should be readable");
    let rotation = body.rotation().expect("rotation should be readable");
    let rotation_valid = boxdd::Rot::is_valid(rotation);
    let shape_count = body.shape_count().expect("shape count should be readable");
    let shapes = body.shapes().expect("shapes should be readable");
    let joint_count = body.joint_count().expect("joint count should be readable");
    let joints = body.joints().expect("joints should be readable");
    let sleep_threshold = body
        .sleep_threshold()
        .expect("sleep threshold should be readable");
    let sleep_enabled = body
        .is_sleep_enabled()
        .expect("sleep flag should be readable");
    let awake = body.is_awake().expect("awake flag should be readable");
    let enabled = body.is_enabled().expect("enabled flag should be readable");
    let bullet = body.is_bullet().expect("bullet flag should be readable");
    let name = body.name().expect("body name should be readable");

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
    let mut world: World = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");
    let body_builder = BodyBuilder::body_type(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
        BodyType::Dynamic,
    );
    let body = world
        .create_body(BodyBuilder::build(body_builder).unwrap())
        .expect("body creation should succeed");
    let shape_builder = boxdd::ShapeDefBuilder::density(ShapeDef::builder(), 1.0);
    let shape_def = boxdd::ShapeDefBuilder::build(shape_builder).unwrap();
    let circle = shapes::circle([0.0_f32, 0.0], 0.5).unwrap();
    let shape = world
        .body(body)
        .expect("body should remain valid")
        .create_circle(&shape_def, &circle)
        .expect("circle creation should succeed");

    world
        .enable_sleeping(true)
        .expect("sleeping should be configurable");
    world
        .enable_continuous(true)
        .expect("continuous collision should be configurable");
    world
        .enable_warm_starting(true)
        .expect("warm starting should be configurable");
    world
        .set_restitution_threshold(1.5)
        .expect("restitution threshold should be writable");
    world
        .set_hit_event_threshold(2.5)
        .expect("hit threshold should be writable");
    world
        .set_contact_tuning(30.0, 1.0, 3.0)
        .expect("contact tuning should be writable");
    world
        .set_maximum_linear_speed(120.0)
        .expect("maximum speed should be writable");
    world
        .set_gravity([0.0_f32, -9.8])
        .expect("gravity should be writable");
    world
        .set_custom_filter(accept_contact_pair)
        .expect("custom filter should be installable");
    world
        .set_pre_solve(accept_pre_solve)
        .expect("pre-solve callback should be installable");
    world
        .set_worker_count(boxdd::WorkerCount::new(2).expect("two workers should be supported"))
        .expect("worker-count update should succeed");

    let explosion = boxdd::ExplosionDef::mask_bits(boxdd::ExplosionDef::new(), u64::MAX);
    let explosion = boxdd::ExplosionDef::position(explosion, Position::ZERO);
    let explosion = boxdd::ExplosionDef::radius(explosion, 1.0);
    let explosion = boxdd::ExplosionDef::falloff(explosion, 1.0);
    let explosion = boxdd::ExplosionDef::impulse_per_length(explosion, 1.0);
    world.explode(&explosion).expect("explosion should succeed");

    let gravity = world.gravity().expect("gravity should be readable");
    let counters = world.counters().expect("counters should be readable");
    let _profile = world.profile().expect("profile should be readable");
    let awake_body_count = world
        .awake_body_count()
        .expect("awake-body count should be readable");
    let sleeping_enabled = world
        .is_sleeping_enabled()
        .expect("sleeping state should be readable");
    let continuous_enabled = world
        .is_continuous_enabled()
        .expect("continuous state should be readable");
    let warm_starting_enabled = world
        .is_warm_starting_enabled()
        .expect("warm-starting state should be readable");
    let restitution_threshold = world
        .restitution_threshold()
        .expect("restitution threshold should be readable");
    let hit_event_threshold = world
        .hit_event_threshold()
        .expect("hit-event threshold should be readable");
    let maximum_linear_speed = world
        .maximum_linear_speed()
        .expect("maximum speed should be readable");
    let bounds = world.bounds().expect("world bounds should be readable");
    let bounds_valid = boxdd::Aabb::is_valid(bounds);
    let _maximum_capacity = world
        .maximum_capacity()
        .expect("maximum capacity should be readable");
    let recycle_distance = world
        .contact_recycle_distance()
        .expect("contact-recycle query should succeed");
    let worker_count = world
        .worker_count()
        .expect("worker count should be readable");

    let query_bounds = boxdd::Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap();
    let proxy = ShapeProxy::new(
        [
            Vec2::new(-0.75, -0.75),
            Vec2::new(0.75, -0.75),
            Vec2::new(0.75, 0.75),
            Vec2::new(-0.75, 0.75),
        ],
        0.0,
    )
    .unwrap();
    let (overlaps, polygon_overlaps, closest_hit, mover_fraction) = {
        let query = world.query().expect("query capability should be available");
        let overlaps = query
            .overlap_aabb(Position::ZERO, query_bounds, QueryFilter::default())
            .expect("AABB query should succeed");
        let polygon_overlaps = query
            .overlap_shape(Position::ZERO, proxy, QueryFilter::default())
            .expect("polygon query should succeed");
        let closest_hit = query
            .cast_ray_closest(
                Position::new(-2.0, 0.0),
                [4.0_f32, 0.0],
                QueryFilter::default(),
            )
            .expect("closest ray query should succeed");
        let mover_fraction = query
            .cast_mover(
                Position::new(-2.0, 0.0),
                [0.0_f32, -0.25],
                [0.0_f32, 0.25],
                0.25,
                [4.0_f32, 0.0],
                QueryFilter::default(),
            )
            .expect("mover cast should succeed");
        (overlaps, polygon_overlaps, closest_hit, mover_fraction)
    };
    let mut drawer = NoopDebugDraw;
    world
        .debug_draw(&mut drawer, boxdd::DebugDrawOptions::default())
        .expect("debug draw should succeed");
    world
        .clear_custom_filter()
        .expect("custom filter should be clearable");
    world
        .clear_pre_solve()
        .expect("pre-solve callback should be clearable");

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
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");
    let builder = BodyBuilder::body_type(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
        BodyType::Dynamic,
    );
    let builder = BodyBuilder::position(builder, origin);
    let body_id = world
        .create_body(BodyBuilder::build(builder).unwrap())
        .expect("body creation should succeed");

    let mut body = world.body(body_id).expect("body should remain valid");
    let initial_position = body.position().expect("position should be readable");
    let initial_transform = body.transform().expect("transform should be readable");
    let initial_transform_position = WorldTransform::position(initial_transform);
    let initial_center = body
        .world_center_of_mass()
        .expect("world center should be readable");

    let local_point = Vec2::new(0.5, -0.25);
    let world_point = body
        .world_point(local_point)
        .expect("world-point conversion should succeed");
    let round_trip_local_point = body
        .local_point(world_point)
        .expect("local-point conversion should succeed");
    let world_point_velocity = body
        .world_point_velocity(world_point)
        .expect("world-point velocity should be readable");
    let world_point_velocity_valid = Vec2::is_valid(world_point_velocity);

    let moved = Position::new(origin.x + 4.0, origin.y + 8.0);
    body.set_position_and_rotation(moved, 0.0)
        .expect("position update should succeed");
    let moved_position = body.position().expect("position should be readable");
    body.apply_force([1.0_f32, 0.0], moved, true)
        .expect("force application should succeed");
    body.apply_linear_impulse([0.25_f32, 0.0], moved, true)
        .expect("impulse application should succeed");
    let target = WorldTransform::new(moved, Rot::IDENTITY).unwrap();
    body.set_target_transform(target, 1.0 / 60.0, true)
        .expect("target transform should be writable");

    assert_eq!(initial_position, origin);
    assert_eq!(initial_transform_position, origin);
    assert_eq!(initial_center, origin);
    assert_eq!(world_point, Position::new(origin.x + 0.5, origin.y - 0.25));
    assert_eq!(round_trip_local_point, local_point);
    assert!(world_point_velocity_valid);
    assert_eq!(moved_position, moved);
}

fn settled_contact_world() -> (World, BodyId, boxdd::ShapeId) {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let world_builder = foundation.world_builder();
    let world_builder = boxdd::WorldBuilder::gravity(world_builder, [0.0_f32, -10.0]);
    let mut world = foundation
        .create_world(boxdd::WorldBuilder::build(world_builder).unwrap())
        .expect("world creation should succeed");
    let ground_def = BodyBuilder::build(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
    )
    .unwrap();
    let ground = world
        .create_body(ground_def)
        .expect("ground body creation should succeed");
    let ground_polygon = shapes::box_polygon(10.0_f32, 0.5).unwrap();
    world
        .body(ground)
        .expect("ground body should remain valid")
        .create_polygon(&ShapeDef::default(), &ground_polygon)
        .expect("ground shape creation should succeed");

    let dynamic_builder = BodyBuilder::body_type(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
        BodyType::Dynamic,
    );
    let dynamic_builder = BodyBuilder::position(dynamic_builder, [0.0_f32, 3.0]);
    let dynamic = world
        .create_body(BodyBuilder::build(dynamic_builder).unwrap())
        .expect("dynamic body creation should succeed");
    let shape_builder = boxdd::ShapeDefBuilder::density(ShapeDef::builder(), 1.0);
    let dynamic_polygon = shapes::box_polygon(0.5_f32, 0.5).unwrap();
    let dynamic_shape = world
        .body(dynamic)
        .expect("dynamic body should remain valid")
        .create_polygon(
            &boxdd::ShapeDefBuilder::build(shape_builder).unwrap(),
            &dynamic_polygon,
        )
        .expect("dynamic shape creation should succeed");

    for _ in 0..180 {
        drop(
            world
                .step(1.0 / 60.0, 4)
                .expect("world step should succeed"),
        );
    }

    (world, dynamic, dynamic_shape)
}

#[test]
fn live_body_contact_and_contact_snapshot_succeed() {
    let (mut world, dynamic, dynamic_shape) = settled_contact_world();

    let contacts = world
        .body(dynamic)
        .expect("dynamic body should remain valid")
        .contact_data()
        .expect("body contact data should be readable");
    assert!(!contacts.is_empty());
    let contact_id = contacts[0].contact_id;
    let shape_contacts = world
        .shape(dynamic_shape)
        .expect("dynamic shape should remain valid")
        .contact_data()
        .expect("shape contact data should be readable");
    let contact_valid = world
        .contact_is_valid(contact_id)
        .expect("contact validity should be readable");
    let snapshot = world
        .contact_data(contact_id)
        .expect("live contact should produce a snapshot");

    assert!(
        shape_contacts
            .iter()
            .any(|contact| contact.contact_id == contact_id)
    );
    assert!(contact_valid);
    assert_eq!(snapshot.contact_id, contact_id);
}

#[test]
fn chain_runtime_creation_queries_and_destroy_succeed() {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let chain_builder = boxdd::ChainDefBuilder::points(
        ChainDef::builder(),
        [
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ],
    );
    let chain_def = boxdd::ChainDefBuilder::build(chain_builder).unwrap();
    let chain_point_count = boxdd::ChainDef::points(&chain_def).len();
    let uses_default_material = matches!(
        boxdd::ChainDef::material_layout(&chain_def),
        boxdd::ChainDefMaterialLayout::Default(_)
    );
    let mut world = foundation
        .create_world(foundation.world_def())
        .expect("world creation should succeed");
    let body_def = BodyBuilder::build(foundation.body_builder()).unwrap();
    let body_id = world
        .create_body(body_def)
        .expect("body creation should succeed");
    let chain_id = world
        .body(body_id)
        .expect("body should remain valid")
        .create_chain(&chain_def)
        .expect("chain creation should succeed");

    let chain = world.chain(chain_id).expect("chain should remain valid");
    let segment_count = chain
        .segment_count()
        .expect("segment count should be readable");
    let segments = chain.segments().expect("segments should be readable");
    let material_count = chain
        .surface_material_count()
        .expect("material count should be readable");
    let material = chain
        .surface_material(0)
        .expect("surface material should be readable");
    let material_friction = boxdd::SurfaceMaterial::friction(&material);
    chain.destroy().expect("chain destruction should succeed");

    assert_eq!(chain_point_count, 4);
    assert!(uses_default_material);
    assert!(segment_count > 0);
    assert!(!segments.is_empty());
    assert!(material_count > 0);
    assert!(material_friction.is_finite());
}

#[test]
fn foundation_collision_and_math_runtime_paths_succeed() {
    let valid_float = is_valid_float(1.25);
    let units_per_meter = Foundation::initialize_default()
        .expect("default foundation should initialize")
        .config()
        .length_units_per_meter();
    let angle = boxdd::atan2(1.0, 0.0).expect("atan2 should succeed");
    let no_planes: [CollisionPlane; 0] = [];
    let clipped = boxdd::clip_vector(Vec2::new(3.0, 4.0), &no_planes)
        .expect("vector clipping should succeed");
    let cos_sin = boxdd::compute_cos_sin(0.25).expect("rotation computation should succeed");
    let between = boxdd::Rot::from_unit_vectors(Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0))
        .expect("unit-vector rotation should succeed");

    let sweep = boxdd::Sweep::new(
        [0.0_f32, 0.0],
        [0.0_f32, 0.0],
        [2.0_f32, 0.0],
        Rot::IDENTITY,
        Rot::IDENTITY,
    )
    .unwrap();
    let sweep_transform =
        boxdd::Sweep::transform_at(sweep, 0.5).expect("valid sweep transform should succeed");

    let explosion = boxdd::ExplosionDef::default();
    let blast_radius = boxdd::ExplosionDef::blast_radius(&explosion);

    let points = [
        Vec2::new(-0.75, -0.5),
        Vec2::new(0.75, -0.5),
        Vec2::new(0.75, 0.5),
        Vec2::new(-0.75, 0.5),
    ];
    let hull_valid = boxdd::Polygon::hull_is_valid(points).expect("valid hull input");

    let box_polygon = boxdd::Polygon::box_polygon(0.5, 0.25).expect("valid box");
    let rounded_box =
        boxdd::Polygon::rounded_box_polygon(0.5, 0.25, 0.05).expect("valid rounded box");
    let offset = Transform::from_pos_angle([1.0_f32, -0.5], 0.2).unwrap();
    let offset_box =
        boxdd::Polygon::offset_box_polygon(0.5, 0.25, offset).expect("valid offset box");
    let offset_rounded_box = boxdd::Polygon::offset_rounded_box_polygon(0.5, 0.25, 0.05, offset)
        .expect("valid offset rounded box");
    let offset_polygon =
        boxdd::Polygon::offset_from_points(points, 0.0, offset).expect("valid offset polygon");
    let offset_rounded_polygon = boxdd::Polygon::offset_from_points(points, 0.05, offset)
        .expect("valid offset rounded polygon");

    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");
    let body_def = BodyBuilder::build(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
    )
    .unwrap();
    let body_id = world
        .create_body(body_def)
        .expect("polygon body creation should succeed");
    let mut body = world
        .body(body_id)
        .expect("polygon body should remain valid");
    let _shape = body
        .create_polygon_from_points(&ShapeDef::default(), points, 0.0)
        .expect("valid polygon shape");

    let cos_sin_cosine = Rot::cosine(cos_sin);
    let cos_sin_sine = Rot::sine(cos_sin);
    let between_valid = Rot::is_valid(between);
    let sweep_position = Transform::position(sweep_transform);
    let box_count = boxdd::Polygon::count(&box_polygon);
    let rounded_box_count = boxdd::Polygon::count(&rounded_box);
    let offset_box_count = boxdd::Polygon::count(&offset_box);
    let offset_rounded_box_count = boxdd::Polygon::count(&offset_rounded_box);
    assert!(valid_float);
    assert!(units_per_meter.is_finite() && units_per_meter > 0.0);
    assert!(approx_eq(angle, core::f32::consts::FRAC_PI_2));
    assert!(approx_eq(clipped.x, 3.0) && approx_eq(clipped.y, 4.0));
    assert!(cos_sin_cosine.is_finite() && cos_sin_sine.is_finite());
    assert!(between_valid);
    assert!(approx_eq(sweep_position.x, 1.0));
    assert!(approx_eq(sweep_position.y, 0.0));
    assert!(blast_radius.is_finite() && blast_radius >= 0.0);
    assert!(hull_valid);
    assert_eq!(box_count, 4);
    assert_eq!(rounded_box_count, 4);
    assert_eq!(offset_box_count, 4);
    assert_eq!(offset_rounded_box_count, 4);
    assert_eq!(offset_polygon.count(), 4);
    assert_eq!(offset_rounded_polygon.count(), 4);
}

#[test]
fn world_queries_events_user_data_and_callback_cleanup_succeed() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .expect("world creation should succeed");
    let ground_def = BodyBuilder::build(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
    )
    .unwrap();
    let ground = world
        .create_body(ground_def)
        .expect("ground body creation should succeed");
    let ground_shape_builder =
        boxdd::ShapeDefBuilder::enable_contact_events(ShapeDef::builder(), true);
    let ground_shape_def = boxdd::ShapeDefBuilder::build(ground_shape_builder).unwrap();
    let ground_polygon = shapes::box_polygon(10.0_f32, 0.5).unwrap();
    let ground_shape = world
        .body(ground)
        .expect("ground body should remain valid")
        .create_polygon(&ground_shape_def, &ground_polygon)
        .expect("ground shape creation should succeed");
    let ground_shape_exists = world.shape(ground_shape).is_ok();

    let wall_builder = BodyBuilder::position(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
        [1.0_f32, 1.0],
    );
    let wall = world
        .create_body(BodyBuilder::build(wall_builder).unwrap())
        .expect("wall body creation should succeed");
    let wall_polygon = shapes::box_polygon(0.25_f32, 1.0).unwrap();
    let wall_shape = world
        .body(wall)
        .expect("wall body should remain valid")
        .create_polygon(&ShapeDef::default(), &wall_polygon)
        .expect("wall shape creation should succeed");
    let wall_shape_exists = world.shape(wall_shape).is_ok();

    let dynamic_builder = BodyBuilder::body_type(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
        BodyType::Dynamic,
    );
    let dynamic_builder = BodyBuilder::position(dynamic_builder, [0.0_f32, 0.8]);
    let dynamic_builder = BodyBuilder::linear_velocity(dynamic_builder, [0.1_f32, 0.0]);
    let dynamic = world
        .create_body(BodyBuilder::build(dynamic_builder).unwrap())
        .expect("dynamic body creation should succeed");
    let dynamic_shape_builder = boxdd::ShapeDefBuilder::density(ShapeDef::builder(), 1.0);
    let dynamic_shape_builder =
        boxdd::ShapeDefBuilder::enable_contact_events(dynamic_shape_builder, true);
    let dynamic_shape_def = boxdd::ShapeDefBuilder::build(dynamic_shape_builder).unwrap();
    let dynamic_polygon = shapes::box_polygon(0.4_f32, 0.4).unwrap();
    let dynamic_shape = world
        .body(dynamic)
        .expect("dynamic body should remain valid")
        .create_polygon(&dynamic_shape_def, &dynamic_polygon)
        .expect("dynamic shape creation should succeed");
    let dynamic_shape_exists = world.shape(dynamic_shape).is_ok();

    let sensor_builder = BodyBuilder::position(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
        [3.0_f32, 1.0],
    );
    let sensor_body = world
        .create_body(BodyBuilder::build(sensor_builder).unwrap())
        .expect("sensor body creation should succeed");
    let sensor_shape_builder = boxdd::ShapeDefBuilder::sensor(ShapeDef::builder(), true);
    let sensor_shape_builder =
        boxdd::ShapeDefBuilder::enable_sensor_events(sensor_shape_builder, true);
    let sensor_shape_def = boxdd::ShapeDefBuilder::build(sensor_shape_builder).unwrap();
    let sensor_polygon = shapes::box_polygon(0.75_f32, 0.75).unwrap();
    let sensor_shape = world
        .body(sensor_body)
        .expect("sensor body should remain valid")
        .create_polygon(&sensor_shape_def, &sensor_polygon)
        .expect("sensor shape creation should succeed");
    let sensor_shape_exists = world.shape(sensor_shape).is_ok();

    let visitor_builder = BodyBuilder::body_type(
        boxdd::Foundation::get()
            .expect("Foundation must be initialized before constructing a BodyDef")
            .body_builder(),
        BodyType::Dynamic,
    );
    let visitor_builder = BodyBuilder::position(visitor_builder, [3.0_f32, 1.0]);
    let visitor = world
        .create_body(BodyBuilder::build(visitor_builder).unwrap())
        .expect("visitor body creation should succeed");
    let visitor_shape_builder = boxdd::ShapeDefBuilder::density(ShapeDef::builder(), 1.0);
    let visitor_shape_builder =
        boxdd::ShapeDefBuilder::enable_sensor_events(visitor_shape_builder, true);
    let visitor_shape_def = boxdd::ShapeDefBuilder::build(visitor_shape_builder).unwrap();
    let visitor_polygon = shapes::box_polygon(0.25_f32, 0.25).unwrap();
    let visitor_shape = world
        .body(visitor)
        .expect("visitor body should remain valid")
        .create_polygon(&visitor_shape_def, &visitor_polygon)
        .expect("visitor shape creation should succeed");
    let visitor_shape_exists = world.shape(visitor_shape).is_ok();

    let completed = world
        .step(1.0 / 60.0, 4)
        .expect("world step should succeed");
    let events = completed
        .to_owned()
        .expect("completed-step events should be readable");
    drop(completed);

    let cast_points = [
        Vec2::new(-0.25, 2.0),
        Vec2::new(0.25, 2.0),
        Vec2::new(0.25, 2.5),
        Vec2::new(-0.25, 2.5),
    ];
    let local_cast_points = [
        Vec2::new(-0.25, -0.25),
        Vec2::new(0.25, -0.25),
        Vec2::new(0.25, 0.25),
        Vec2::new(-0.25, 0.25),
    ];
    let cast_proxy = ShapeProxy::new(cast_points, 0.0).unwrap();
    let offset_cast_proxy = ShapeProxy::offset_from_points(
        local_cast_points,
        0.0,
        Transform::from_pos_angle([0.0_f32, 2.0], 0.0).unwrap(),
    )
    .unwrap();
    let (ray_hits, shape_hits, offset_shape_hits, mover_planes) = {
        let query = world.query().expect("query capability should be available");
        let ray_hits = query
            .cast_ray_all(
                Position::new(0.0, 5.0),
                [0.0_f32, -10.0],
                QueryFilter::default(),
            )
            .expect("ray query should succeed");
        let shape_hits = query
            .cast_shape(
                Position::ZERO,
                cast_proxy,
                [0.0_f32, -4.0],
                QueryFilter::default(),
            )
            .expect("shape cast should succeed");
        let offset_shape_hits = query
            .cast_shape(
                Position::ZERO,
                offset_cast_proxy,
                [0.0_f32, -4.0],
                QueryFilter::default(),
            )
            .expect("offset shape cast should succeed");
        let mover_planes = query
            .collide_mover(
                Position::ZERO,
                [5.0_f32, 0.7],
                [5.0_f32, 1.5],
                0.25,
                QueryFilter::default(),
            )
            .expect("mover collision query should succeed");
        (ray_hits, shape_hits, offset_shape_hits, mover_planes)
    };

    world
        .set_user_data(29_u32)
        .expect("world user data should be writable");
    let has_user_data = world
        .has_user_data()
        .expect("world user data should be queryable");
    let user_data_cleared = world
        .clear_user_data()
        .expect("world user data should be clearable");

    world
        .set_friction_callback(boxdd::MixerId::from_bytes([0xA1; 32]), average_coefficient)
        .expect("friction callback should be installable");
    world
        .set_restitution_callback(boxdd::MixerId::from_bytes([0xA2; 32]), average_coefficient)
        .expect("restitution callback should be installable");
    world
        .clear_friction_callback()
        .expect("friction callback should be clearable");
    world
        .clear_restitution_callback()
        .expect("restitution callback should be clearable");

    assert!(ground_shape_exists);
    assert!(wall_shape_exists);
    assert!(dynamic_shape_exists);
    assert!(sensor_shape_exists);
    assert!(visitor_shape_exists);
    assert!(!ray_hits.is_empty());
    assert!(!shape_hits.is_empty());
    assert!(!offset_shape_hits.is_empty());
    assert!(!mover_planes.is_empty());
    assert!(!events.body.is_empty());
    assert!(!events.contact.begin.is_empty());
    assert!(events.joint.is_empty());
    assert!(!events.sensor.begin.is_empty());
    assert!(has_user_data);
    assert!(user_data_cleared);
}
