use boxdd::{
    Aabb, ApiError, BodyBuilder, BodyDef, BodyType, ChainDef, ConstraintTuning, DistanceJointDef,
    ExplosionDef, Filter, FilterJointDef, JointBase, MassData, MotionLocks, MotorJointDef,
    OwnedBody, Position, PrismaticJointDef, QueryFilter, RecordingCapacity, RecordingSession,
    RevoluteJointDef, ShapeDef, SurfaceMaterial, Transform, Vec2, WeldJointDef, WheelJointDef,
    World, WorldDef, WorldTransform, shapes,
};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn assert_start_error(world: &mut World, expected: ApiError) {
    match world.try_start_recording(RecordingCapacity::default()) {
        Ok(_) => panic!("expected recording start to fail with {expected}"),
        Err(error) => assert_eq!(error, expected),
    }
}

fn add_overlapping_material_pair(world: &mut World) -> OwnedBody {
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .material(
            SurfaceMaterial::default()
                .with_friction(0.5)
                .with_restitution(0.25)
                .with_user_material_id(7),
        )
        .build();
    let polygon = shapes::box_polygon(0.5, 0.5);
    let first = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let second = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.4_f32, 0.0])
            .build(),
    );
    world.create_polygon_shape_for(first.id(), &shape_def, &polygon);
    world.create_polygon_shape_for(second, &shape_def, &polygon);
    first
}

#[test]
fn recording_closest_ray_stats_preserve_hit_and_miss_results() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut session = world.start_recording(RecordingCapacity::default());
    let body = session.create_body(BodyBuilder::new().build());
    let triangle =
        shapes::polygon_from_points([[0.0_f32, 0.0], [2.0_f32, 0.0], [0.0_f32, 2.0]], 0.0).unwrap();
    let shape = session.create_polygon_shape(body, &ShapeDef::default(), &triangle);

    let hit = session.cast_ray_closest_with_stats(
        Position::new(-1.0, 0.25),
        [4.0_f32, 0.0],
        QueryFilter::default(),
    );
    assert_eq!(hit.hit.map(|result| result.shape_id), Some(shape));
    assert!(hit.node_visits > 0);
    assert!(hit.leaf_visits > 0);

    let miss = session
        .try_cast_ray_closest_with_stats(
            Position::new(1.7, 1.7),
            [0.2_f32, 0.0],
            QueryFilter::default(),
        )
        .unwrap();
    assert!(miss.hit.is_none());
    assert!(miss.node_visits > 0);
    assert!(miss.leaf_visits > 0);
    assert!(
        session
            .cast_ray_closest(
                Position::new(1.7, 1.7),
                [0.2_f32, 0.0],
                QueryFilter::default(),
            )
            .is_none()
    );

    let recording = session.finish();
    assert!(!recording.is_empty());
}

#[test]
fn explicit_finish_owns_bytes_and_gates_preexisting_aliases() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();
    let owned = world.create_body_owned(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 2.0])
            .build(),
    );
    world.create_circle_shape_for(
        owned.id(),
        &ShapeDef::builder().density(1.0).build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let handle = world.handle();

    let recording = {
        let mut session = world.start_recording(RecordingCapacity::new(128).unwrap());
        assert_eq!(handle.try_gravity(), Err(ApiError::WorldBusy));
        assert_eq!(owned.try_position(), Err(ApiError::WorldBusy));
        assert_eq!(session.gravity().y, -10.0);
        session.set_gravity([0.0_f32, -5.0]);
        session.step(1.0 / 60.0, 4);
        assert_eq!(session.counters().body_count, 1);
        session.finish()
    };

    assert!(!recording.is_empty());
    assert!(recording.len() > 32);
    assert!(recording.mixer_requirements().is_empty());
    assert_eq!(world.gravity().y, -5.0);
    assert!(owned.try_position().is_ok());

    let expected = recording.as_bytes().to_vec();
    let bytes = recording.into_bytes();
    drop(world);
    assert_eq!(bytes, expected);
}

#[test]
fn session_owned_surface_records_lifecycle_mutations_and_e0_e8_queries() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let handle = world.handle();

    let (body, shape, recording) = {
        let mut session = world.start_recording(RecordingCapacity::default());
        let body = session.create_body(
            BodyBuilder::new()
                .body_type(BodyType::Static)
                .position([0.0_f32, 0.0])
                .build(),
        );
        assert_eq!(handle.try_body_position(body), Err(ApiError::WorldBusy));

        let shape = session.create_circle_shape(
            body,
            &ShapeDef::builder().density(1.0).build(),
            &shapes::circle([0.0_f32, 0.0], 0.5),
        );
        session.set_body_type(body, BodyType::Dynamic);
        session.set_body_position_and_rotation(body, Position::ZERO, 0.0);
        session.set_body_linear_velocity(body, Vec2::ZERO);
        session.set_body_angular_velocity(body, 0.0);
        session.body_apply_linear_impulse_to_center(body, Vec2::ZERO, true);
        session.body_apply_angular_impulse(body, 0.0, true);
        session.body_clear_forces(body);

        session.shape_set_circle(shape, &shapes::circle([0.0_f32, 0.0], 0.75));
        session.shape_set_density(shape, 2.0, true);
        session.shape_set_friction(shape, 0.4);
        session.shape_set_restitution(shape, 0.2);
        session.shape_set_user_material(shape, 17);
        session.shape_set_filter(shape, Filter::default());
        session.shape_set_surface_material(
            shape,
            &SurfaceMaterial::default()
                .with_friction(0.4)
                .with_restitution(0.2)
                .with_user_material_id(17),
        );

        session.step(1.0 / 60.0, 2);
        let bounds = Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]);
        assert_eq!(
            session.overlap_aabb(Position::ZERO, bounds, QueryFilter::default()),
            vec![shape]
        );
        let proxy = [
            Vec2::new(-0.1, -0.1),
            Vec2::new(0.1, -0.1),
            Vec2::new(0.1, 0.1),
            Vec2::new(-0.1, 0.1),
        ];
        assert_eq!(
            session.overlap_polygon_points(Position::ZERO, proxy, 0.0, QueryFilter::default(),),
            vec![shape]
        );
        assert!(
            session
                .cast_ray_all(
                    Position::new(-2.0, 0.0),
                    Vec2::new(4.0, 0.0),
                    QueryFilter::default(),
                )
                .iter()
                .any(|hit| hit.shape_id == shape)
        );
        assert!(
            session
                .cast_shape_points(
                    Position::new(-2.0, 0.0),
                    proxy,
                    0.0,
                    Vec2::new(4.0, 0.0),
                    QueryFilter::default(),
                )
                .iter()
                .any(|hit| hit.shape_id == shape)
        );
        let _planes = session.collide_mover(
            Position::ZERO,
            Vec2::new(0.0, -0.25),
            Vec2::new(0.0, 0.25),
            0.25,
            QueryFilter::default(),
        );
        let ray = session
            .cast_ray_closest(
                Position::new(-2.0, 0.0),
                Vec2::new(4.0, 0.0),
                QueryFilter::default(),
            )
            .expect("recorded ray should hit the session-created circle");
        assert_eq!(ray.shape_id, shape);
        let mover_fraction = session.cast_mover(
            Position::new(-2.0, 0.0),
            Vec2::new(0.0, -0.25),
            Vec2::new(0.0, 0.25),
            0.25,
            Vec2::new(4.0, 0.0),
            QueryFilter::default(),
        );
        assert!((0.0..=1.0).contains(&mover_fraction));
        assert!(session.shape_test_point(shape, Position::ZERO));
        assert!(
            session
                .shape_ray_cast(shape, Position::new(-2.0, 0.0), Vec2::new(4.0, 0.0))
                .hit
        );

        let recording = session.finish();
        (body, shape, recording)
    };

    assert!(!recording.is_empty());
    assert!(handle.try_body_position(body).is_ok());
    assert!(world.try_shape_test_point(shape, Position::ZERO).unwrap());
    world.destroy_shape_id(shape, true);
    world.destroy_body_id(body);
}

#[test]
fn session_surface_covers_all_logged_world_body_shape_chain_and_joint_mutations() {
    let mut world: boxdd::World =
        boxdd::World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let mut session: boxdd::RecordingSession<'_> =
        boxdd::World::start_recording(&mut world, RecordingCapacity::default());

    RecordingSession::enable_sleeping(&mut session, true);
    RecordingSession::enable_continuous(&mut session, true);
    RecordingSession::enable_warm_starting(&mut session, true);
    RecordingSession::set_restitution_threshold(&mut session, 1.0);
    RecordingSession::set_hit_event_threshold(&mut session, 1.0);
    RecordingSession::set_gravity(&mut session, [0.0_f32, -9.8]);
    RecordingSession::explode(
        &mut session,
        &ExplosionDef::default()
            .position(Position::ZERO)
            .radius(1.0)
            .falloff(1.0)
            .impulse_per_length(1.0),
    );
    RecordingSession::set_contact_tuning(&mut session, 30.0, 1.0, 3.0);
    RecordingSession::set_contact_recycle_distance(&mut session, 0.01);
    RecordingSession::set_maximum_linear_speed(&mut session, 100.0);

    let body_a = RecordingSession::create_body(
        &mut session,
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .motion_locks(MotionLocks::new(false, false, false))
            .build(),
    );
    let body_b = RecordingSession::create_body(
        &mut session,
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([2.0_f32, 0.0])
            .build(),
    );
    let chain_body = RecordingSession::create_body(&mut session, BodyBuilder::new().build());

    RecordingSession::set_body_name(&mut session, body_a, "recorded");
    RecordingSession::set_body_position_and_rotation(&mut session, body_a, Position::ZERO, 0.0);
    RecordingSession::set_body_linear_velocity(&mut session, body_a, Vec2::ZERO);
    RecordingSession::set_body_angular_velocity(&mut session, body_a, 0.0);
    RecordingSession::set_body_type(&mut session, body_a, BodyType::Dynamic);
    RecordingSession::set_body_target_transform(
        &mut session,
        body_a,
        WorldTransform::IDENTITY,
        1.0 / 60.0,
        true,
    );
    RecordingSession::body_apply_force(&mut session, body_a, [1.0_f32, 0.0], Position::ZERO, true);
    RecordingSession::body_apply_force_to_center(&mut session, body_a, [1.0_f32, 0.0], true);
    RecordingSession::body_apply_torque(&mut session, body_a, 1.0, true);
    RecordingSession::body_clear_forces(&mut session, body_a);
    RecordingSession::body_apply_linear_impulse(
        &mut session,
        body_a,
        [1.0_f32, 0.0],
        Position::ZERO,
        true,
    );
    RecordingSession::body_apply_linear_impulse_to_center(
        &mut session,
        body_a,
        [1.0_f32, 0.0],
        true,
    );
    RecordingSession::body_apply_angular_impulse(&mut session, body_a, 1.0, true);
    RecordingSession::set_body_mass_data(&mut session, body_a, MassData::new(1.0, Vec2::ZERO, 1.0));
    RecordingSession::body_apply_mass_from_shapes(&mut session, body_a);
    RecordingSession::set_body_linear_damping(&mut session, body_a, 0.1);
    RecordingSession::set_body_angular_damping(&mut session, body_a, 0.1);
    RecordingSession::set_body_gravity_scale(&mut session, body_a, 1.0);
    RecordingSession::set_body_awake(&mut session, body_a, true);
    RecordingSession::body_wake_touching(&mut session, body_a);
    RecordingSession::body_enable_sleep(&mut session, body_a, true);
    RecordingSession::set_body_sleep_threshold(&mut session, body_a, 0.1);
    RecordingSession::disable_body(&mut session, body_a);
    RecordingSession::enable_body(&mut session, body_a);
    RecordingSession::set_body_motion_locks(
        &mut session,
        body_a,
        MotionLocks::new(false, true, false),
    );
    RecordingSession::set_body_bullet(&mut session, body_a, true);
    RecordingSession::body_enable_contact_recycling(&mut session, body_a, true);
    RecordingSession::body_enable_contact_events(&mut session, body_a, true);
    RecordingSession::body_enable_hit_events(&mut session, body_a, true);

    let shape = RecordingSession::create_circle_shape(
        &mut session,
        body_a,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::circle(Vec2::ZERO, 0.5),
    );
    let segment_geometry = shapes::segment([-0.5_f32, 0.0], [0.5_f32, 0.0]);
    let segment = RecordingSession::create_segment_shape(
        &mut session,
        body_a,
        &ShapeDef::default(),
        &segment_geometry,
    );
    let chain_segment_geometry = shapes::chain_segment(
        [-2.0_f32, 0.0],
        [-1.0_f32, 0.0],
        [1.0_f32, 0.0],
        [2.0_f32, 0.0],
    );
    let chain_segment = RecordingSession::create_chain_segment_shape(
        &mut session,
        body_a,
        &ShapeDef::default(),
        &chain_segment_geometry,
    );
    let capsule_geometry = shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25);
    let capsule = RecordingSession::create_capsule_shape(
        &mut session,
        body_a,
        &ShapeDef::default(),
        &capsule_geometry,
    );
    let polygon_geometry = shapes::box_polygon(0.5, 0.5);
    let polygon = RecordingSession::create_polygon_shape(
        &mut session,
        body_a,
        &ShapeDef::default(),
        &polygon_geometry,
    );

    RecordingSession::shape_set_density(&mut session, shape, 1.0, true);
    RecordingSession::shape_set_friction(&mut session, shape, 0.5);
    RecordingSession::shape_set_restitution(&mut session, shape, 0.25);
    RecordingSession::shape_set_user_material(&mut session, shape, 7);
    RecordingSession::shape_set_surface_material(&mut session, shape, &SurfaceMaterial::default());
    RecordingSession::shape_set_filter(&mut session, shape, Filter::default());
    RecordingSession::shape_enable_sensor_events(&mut session, shape, true);
    RecordingSession::shape_enable_contact_events(&mut session, shape, true);
    RecordingSession::shape_enable_pre_solve_events(&mut session, shape, true);
    RecordingSession::shape_enable_hit_events(&mut session, shape, true);
    RecordingSession::shape_set_circle(&mut session, shape, &shapes::circle(Vec2::ZERO, 0.4));
    RecordingSession::shape_set_segment(&mut session, segment, &segment_geometry);
    RecordingSession::shape_set_chain_segment(&mut session, chain_segment, &chain_segment_geometry);
    RecordingSession::shape_set_capsule(&mut session, capsule, &capsule_geometry);
    RecordingSession::shape_set_polygon(&mut session, polygon, &polygon_geometry);
    RecordingSession::shape_apply_wind(&mut session, shape, [1.0_f32, 0.0], 0.5, 0.25, true);

    let chain_def = ChainDef::builder()
        .points([
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ])
        .build();
    let chain = RecordingSession::create_chain(&mut session, chain_body, &chain_def);
    RecordingSession::chain_set_surface_material(
        &mut session,
        chain,
        0,
        &SurfaceMaterial::default(),
    );

    let base = JointBase::new(body_a, body_b);
    let distance =
        RecordingSession::create_distance_joint(&mut session, &DistanceJointDef::new(base));
    RecordingSession::set_joint_local_frame_a(&mut session, distance, Transform::IDENTITY);
    RecordingSession::set_joint_local_frame_b(&mut session, distance, Transform::IDENTITY);
    RecordingSession::set_joint_collide_connected(&mut session, distance, false);
    RecordingSession::joint_wake_bodies(&mut session, distance);
    RecordingSession::set_joint_constraint_tuning(
        &mut session,
        distance,
        ConstraintTuning::new(5.0, 0.7),
    );
    RecordingSession::set_joint_force_threshold(&mut session, distance, 1.0);
    RecordingSession::set_joint_torque_threshold(&mut session, distance, 1.0);
    RecordingSession::distance_joint_set_length(&mut session, distance, 1.0);
    RecordingSession::distance_joint_enable_spring(&mut session, distance, true);
    RecordingSession::distance_joint_set_spring_force_range(&mut session, distance, -1.0, 1.0);
    RecordingSession::distance_joint_set_spring_hertz(&mut session, distance, 5.0);
    RecordingSession::distance_joint_set_spring_damping_ratio(&mut session, distance, 0.7);
    RecordingSession::distance_joint_enable_limit(&mut session, distance, true);
    RecordingSession::distance_joint_set_length_range(&mut session, distance, 0.5, 2.0);
    RecordingSession::distance_joint_enable_motor(&mut session, distance, true);
    RecordingSession::distance_joint_set_motor_speed(&mut session, distance, 1.0);
    RecordingSession::distance_joint_set_max_motor_force(&mut session, distance, 10.0);

    let motor = RecordingSession::create_motor_joint(&mut session, &MotorJointDef::new(base));
    RecordingSession::motor_joint_set_linear_velocity(&mut session, motor, [1.0_f32, 0.0]);
    RecordingSession::motor_joint_set_angular_velocity(&mut session, motor, 1.0);
    RecordingSession::motor_joint_set_max_velocity_force(&mut session, motor, 10.0);
    RecordingSession::motor_joint_set_max_velocity_torque(&mut session, motor, 10.0);
    RecordingSession::motor_joint_set_linear_hertz(&mut session, motor, 5.0);
    RecordingSession::motor_joint_set_linear_damping_ratio(&mut session, motor, 0.7);
    RecordingSession::motor_joint_set_angular_hertz(&mut session, motor, 5.0);
    RecordingSession::motor_joint_set_angular_damping_ratio(&mut session, motor, 0.7);
    RecordingSession::motor_joint_set_max_spring_force(&mut session, motor, 10.0);
    RecordingSession::motor_joint_set_max_spring_torque(&mut session, motor, 10.0);

    let filter = RecordingSession::create_filter_joint(&mut session, &FilterJointDef::new(base));
    let prismatic =
        RecordingSession::create_prismatic_joint(&mut session, &PrismaticJointDef::new(base));
    RecordingSession::prismatic_joint_enable_spring(&mut session, prismatic, true);
    RecordingSession::prismatic_joint_set_spring_hertz(&mut session, prismatic, 5.0);
    RecordingSession::prismatic_joint_set_spring_damping_ratio(&mut session, prismatic, 0.7);
    RecordingSession::prismatic_joint_set_target_translation(&mut session, prismatic, 0.5);
    RecordingSession::prismatic_joint_enable_limit(&mut session, prismatic, true);
    RecordingSession::prismatic_joint_set_limits(&mut session, prismatic, -1.0, 1.0);
    RecordingSession::prismatic_joint_enable_motor(&mut session, prismatic, true);
    RecordingSession::prismatic_joint_set_motor_speed(&mut session, prismatic, 1.0);
    RecordingSession::prismatic_joint_set_max_motor_force(&mut session, prismatic, 10.0);

    let revolute =
        RecordingSession::create_revolute_joint(&mut session, &RevoluteJointDef::new(base));
    RecordingSession::revolute_joint_enable_spring(&mut session, revolute, true);
    RecordingSession::revolute_joint_set_spring_hertz(&mut session, revolute, 5.0);
    RecordingSession::revolute_joint_set_spring_damping_ratio(&mut session, revolute, 0.7);
    RecordingSession::revolute_joint_set_target_angle(&mut session, revolute, 0.25);
    RecordingSession::revolute_joint_enable_limit(&mut session, revolute, true);
    RecordingSession::revolute_joint_set_limits(&mut session, revolute, -0.5, 0.5);
    RecordingSession::revolute_joint_enable_motor(&mut session, revolute, true);
    RecordingSession::revolute_joint_set_motor_speed(&mut session, revolute, 1.0);
    RecordingSession::revolute_joint_set_max_motor_torque(&mut session, revolute, 10.0);

    let weld = RecordingSession::create_weld_joint(&mut session, &WeldJointDef::new(base));
    RecordingSession::weld_joint_set_linear_hertz(&mut session, weld, 5.0);
    RecordingSession::weld_joint_set_linear_damping_ratio(&mut session, weld, 0.7);
    RecordingSession::weld_joint_set_angular_hertz(&mut session, weld, 5.0);
    RecordingSession::weld_joint_set_angular_damping_ratio(&mut session, weld, 0.7);

    let wheel = RecordingSession::create_wheel_joint(&mut session, &WheelJointDef::new(base));
    RecordingSession::wheel_joint_enable_spring(&mut session, wheel, true);
    RecordingSession::wheel_joint_set_spring_hertz(&mut session, wheel, 5.0);
    RecordingSession::wheel_joint_set_spring_damping_ratio(&mut session, wheel, 0.7);
    RecordingSession::wheel_joint_enable_limit(&mut session, wheel, true);
    RecordingSession::wheel_joint_set_limits(&mut session, wheel, -1.0, 1.0);
    RecordingSession::wheel_joint_enable_motor(&mut session, wheel, true);
    RecordingSession::wheel_joint_set_motor_speed(&mut session, wheel, 1.0);
    RecordingSession::wheel_joint_set_max_motor_torque(&mut session, wheel, 10.0);

    RecordingSession::destroy_joint(&mut session, distance, true);
    RecordingSession::destroy_joint(&mut session, motor, true);
    RecordingSession::destroy_joint(&mut session, filter, true);
    RecordingSession::destroy_joint(&mut session, prismatic, true);
    RecordingSession::destroy_joint(&mut session, revolute, true);
    RecordingSession::destroy_joint(&mut session, weld, true);
    RecordingSession::destroy_joint(&mut session, wheel, true);
    RecordingSession::destroy_chain(&mut session, chain);
    RecordingSession::destroy_shape(&mut session, shape, true);
    RecordingSession::destroy_shape(&mut session, segment, true);
    RecordingSession::destroy_shape(&mut session, chain_segment, true);
    RecordingSession::destroy_shape(&mut session, capsule, true);
    RecordingSession::destroy_shape(&mut session, polygon, true);
    RecordingSession::destroy_body(&mut session, chain_body);
    RecordingSession::destroy_body(&mut session, body_b);
    RecordingSession::destroy_body(&mut session, body_a);

    let recording = RecordingSession::finish(session);
    assert!(!recording.is_empty());
}

#[test]
fn recording_session_rejects_invalid_explosions_without_recording_them() {
    let mut world = World::new(WorldDef::builder().gravity(Vec2::ZERO).build()).unwrap();
    let baseline = world.start_recording(RecordingCapacity::default()).finish();

    let rejected = {
        let mut session = world.start_recording(RecordingCapacity::default());
        let valid = ExplosionDef::new()
            .position(Position::ZERO)
            .radius(1.0)
            .falloff(0.5)
            .impulse_per_length(-1.0);
        let invalid = [
            valid.position(Position::new(boxdd::WorldScalar::NAN, 0.0)),
            valid.radius(-1.0),
            valid.falloff(f32::INFINITY),
            valid.impulse_per_length(f32::NAN),
            valid.radius(f32::MAX).falloff(f32::MAX),
        ];

        for def in invalid {
            assert_eq!(session.try_explode(&def), Err(ApiError::InvalidArgument));
        }

        #[cfg(feature = "double-precision")]
        assert_eq!(
            session.try_explode(&valid.position(Position::new(f64::from(f32::MAX) * 2.0, 0.0))),
            Err(ApiError::InvalidArgument)
        );

        let panic = catch_unwind(AssertUnwindSafe(|| {
            session.explode(&valid.radius(f32::NAN));
        }));
        assert!(panic.is_err());
        session.finish()
    };

    assert_eq!(rejected.as_bytes(), baseline.as_bytes());
}

#[test]
fn session_try_surface_rejects_foreign_stale_and_invalid_inputs_before_ffi() {
    let mut source = World::new(WorldDef::default()).unwrap();
    let foreign_body = source.create_body_id(BodyDef::default());
    let foreign_body_b = source.create_body_id(BodyDef::default());
    let foreign_joint = source.create_distance_joint_id(&DistanceJointDef::new(JointBase::new(
        foreign_body,
        foreign_body_b,
    )));
    let mut target = World::new(WorldDef::default()).unwrap();

    let mut session = target.start_recording(RecordingCapacity::default());
    let circle = shapes::circle(Vec2::ZERO, 0.5);
    assert_eq!(
        session.try_create_circle_shape(foreign_body, &ShapeDef::default(), &circle),
        Err(ApiError::WrongWorld)
    );

    let body = session.try_create_body(BodyDef::default()).unwrap();
    let body_b = session.try_create_body(BodyDef::default()).unwrap();
    let shape = session
        .try_create_circle_shape(body, &ShapeDef::default(), &circle)
        .unwrap();
    assert_eq!(
        session.try_set_body_linear_velocity(body, Vec2::new(f32::NAN, 0.0)),
        Err(ApiError::InvalidArgument)
    );
    assert_eq!(
        session.try_shape_set_density(shape, -1.0, false),
        Err(ApiError::InvalidArgument)
    );

    let joint_base = JointBase::new(body, body_b);
    let distance = session
        .try_create_distance_joint(&DistanceJointDef::new(joint_base))
        .unwrap();
    let revolute = session
        .try_create_revolute_joint(&RevoluteJointDef::new(joint_base))
        .unwrap();
    assert_eq!(
        session.try_distance_joint_set_length(foreign_joint, f32::NAN),
        Err(ApiError::WrongWorld)
    );
    assert_eq!(
        session.try_distance_joint_set_length(revolute, 1.0),
        Err(ApiError::InvalidJointType)
    );
    assert_eq!(
        session.try_distance_joint_set_length(distance, -1.0),
        Err(ApiError::InvalidArgument)
    );
    session.try_destroy_joint(distance, true).unwrap();
    assert_eq!(
        session.try_distance_joint_set_length(distance, 1.0),
        Err(ApiError::InvalidJointId)
    );

    let chain_def = ChainDef::builder()
        .points([
            Vec2::new(-2.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ])
        .build();
    let chain = session.try_create_chain(body, &chain_def).unwrap();
    session.try_destroy_chain(chain).unwrap();
    assert_eq!(
        session.try_chain_set_surface_material(chain, 0, &SurfaceMaterial::default()),
        Err(ApiError::InvalidChainId)
    );

    session.try_destroy_shape(shape, false).unwrap();
    assert_eq!(
        session.try_shape_set_density(shape, 1.0, false),
        Err(ApiError::InvalidShapeId)
    );
    session.try_destroy_joint(revolute, true).unwrap();
    session.try_destroy_body(body_b).unwrap();
    session.try_destroy_body(body).unwrap();
    assert_eq!(
        session.try_set_body_type(body, BodyType::Static),
        Err(ApiError::InvalidBodyId)
    );

    assert!(!session.try_finish().unwrap().is_empty());
}

#[test]
fn drop_and_user_unwind_stop_recording_before_world_reuse() {
    let mut world = World::new(WorldDef::default()).unwrap();

    {
        let mut session = world.start_recording(RecordingCapacity::default());
        session.set_gravity([1.0_f32, -9.0]);
    }
    assert_eq!(world.gravity(), Vec2::new(1.0, -9.0));

    let unwind = catch_unwind(AssertUnwindSafe(|| {
        let mut session = world.start_recording(RecordingCapacity::new(1).unwrap());
        session.step(0.0, 1);
        panic!("leave the recording scope through unwind");
    }));
    assert!(unwind.is_err());

    let mut session = world.start_recording(RecordingCapacity::default());
    session.step(0.0, 1);
    let recording = session.finish();
    assert!(!recording.is_empty());
    assert!(world.try_counters().is_ok());
}

#[test]
fn session_exit_flushes_owned_handles_dropped_while_aliases_are_gated() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let handle = world.handle();

    let explicitly_finished = world.create_body_owned(BodyDef::default());
    let explicitly_finished_id = explicitly_finished.id();
    let session = world.start_recording(RecordingCapacity::default());
    drop(explicitly_finished);
    assert_eq!(
        handle.try_body_position(explicitly_finished_id),
        Err(ApiError::WorldBusy)
    );
    let recording = session.finish();
    assert!(!recording.is_empty());
    assert_eq!(
        handle.try_body_position(explicitly_finished_id),
        Err(ApiError::InvalidBodyId)
    );

    let stopped_on_drop = world.create_body_owned(BodyDef::default());
    let stopped_on_drop_id = stopped_on_drop.id();
    let session: RecordingSession<'_> = world.start_recording(RecordingCapacity::default());
    drop(stopped_on_drop);
    drop(session);
    assert_eq!(
        handle.try_body_position(stopped_on_drop_id),
        Err(ApiError::InvalidBodyId)
    );
}

#[test]
fn unsupported_callbacks_are_rejected_in_both_installation_orders() {
    let mut custom_filter_world = World::new(WorldDef::default()).unwrap();
    custom_filter_world.set_custom_filter(|_, _| true);
    assert_start_error(
        &mut custom_filter_world,
        ApiError::RecordingCustomFilterUnsupported,
    );
    custom_filter_world.clear_custom_filter();
    assert!(
        custom_filter_world
            .try_start_recording(RecordingCapacity::default())
            .is_ok()
    );

    let mut pre_solve_world = World::new(WorldDef::default()).unwrap();
    pre_solve_world.set_pre_solve(|_, _, _, _| true);
    assert_start_error(&mut pre_solve_world, ApiError::RecordingPreSolveUnsupported);
    pre_solve_world.clear_pre_solve();

    let mut session = pre_solve_world.start_recording(RecordingCapacity::default());
    assert_eq!(
        session.try_set_custom_filter(|_, _| true),
        Err(ApiError::RecordingCustomFilterUnsupported)
    );
    assert_eq!(
        session.try_set_pre_solve(|_, _, _, _| true),
        Err(ApiError::RecordingPreSolveUnsupported)
    );
    let recording = session.finish();
    assert!(!recording.is_empty());

    pre_solve_world.set_custom_filter(|_, _| true);
    pre_solve_world.clear_custom_filter();
    pre_solve_world.set_pre_solve(|_, _, _, _| true);
    pre_solve_world.clear_pre_solve();
}

#[test]
fn mixer_presence_becomes_owned_replay_requirement_metadata() {
    let mut world = World::new(WorldDef::default()).unwrap();
    world.set_friction_callback(|a, b| (a.coefficient * b.coefficient).sqrt());
    world.set_restitution_callback(|a, b| a.coefficient.max(b.coefficient));

    let session = world.start_recording(RecordingCapacity::default());
    let requirements = session.mixer_requirements();
    assert!(requirements.requires_friction());
    assert!(requirements.requires_restitution());
    let recording = session.finish();
    assert_eq!(recording.mixer_requirements(), requirements);

    let (bytes, owned_requirements) = recording.into_parts();
    world.clear_friction_callback();
    world.clear_restitution_callback();
    drop(world);
    assert!(!bytes.is_empty());
    assert_eq!(owned_requirements, requirements);
}

unsafe extern "C" fn raw_friction_mixer(
    friction_a: f32,
    _material_a: u64,
    friction_b: f32,
    _material_b: u64,
) -> f32 {
    (friction_a * friction_b).sqrt()
}

#[test]
fn raw_definition_mixer_presence_is_tracked_and_safe_clear_resets_it() {
    let mut raw = WorldDef::default().into_raw();
    raw.frictionCallback = Some(raw_friction_mixer);
    let def = unsafe { WorldDef::from_raw(raw) };
    let mut world = World::new(def).unwrap();

    let recording = world.start_recording(RecordingCapacity::default()).finish();
    assert!(recording.mixer_requirements().requires_friction());

    world.clear_friction_callback();
    let recording = world.start_recording(RecordingCapacity::default()).finish();
    assert!(recording.mixer_requirements().is_empty());
}

#[test]
fn worker_callback_panic_unwinds_after_ffi_then_session_tears_down() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let owned = add_overlapping_material_pair(&mut world);
    world.set_friction_callback(|_, _| -> f32 {
        panic!("recorded friction callback panic");
    });

    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut session = world.start_recording(RecordingCapacity::default());
        assert_eq!(owned.try_position(), Err(ApiError::WorldBusy));
        session.step(1.0 / 60.0, 2);
    }));
    assert!(result.is_err());

    world.clear_friction_callback();
    let mut replacement = world.start_recording(RecordingCapacity::default());
    replacement.step(0.0, 1);
    assert!(!replacement.finish().is_empty());
    assert!(owned.try_position().is_ok());
}
