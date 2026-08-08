use boxdd::{
    BodyId, BodyType, ChainDef, DistanceJointDef, Error, ExplosionDef, Filter, FilterJointDef,
    JointType, MixerId, MotionLocks, MotorJointDef, Position, PrismaticJointDef, RecordingLimits,
    RecordingSession, ReplayConfig, ReplayPlayer, RevoluteJointDef, ShapeDef, SurfaceMaterial,
    Vec2, WeldJointDef, WheelJointDef, World, shapes,
};
use std::panic::{AssertUnwindSafe, catch_unwind};

const FRICTION_MIXER_ID: MixerId = MixerId::from_bytes([0x71; 32]);
const RESTITUTION_MIXER_ID: MixerId = MixerId::from_bytes([0x72; 32]);

fn assert_start_error(world: &mut World, expected: Error) {
    match world.start_recording(RecordingLimits::default()) {
        Ok(_) => panic!("expected recording start to fail with {expected}"),
        Err(error) => assert_eq!(error, expected),
    }
}

fn assert_invalid_argument<T>(result: boxdd::Result<T>) {
    assert!(matches!(result, Err(Error::InvalidArgument { .. })));
}

fn world_with_one_body() -> World {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    world
}

fn minimum_recording_limit(make_world: fn() -> World) -> u32 {
    const SEARCH_CEILING_BYTES: u32 = 1024 * 1024;

    let mut lower = 1;
    let mut upper = SEARCH_CEILING_BYTES;

    {
        let mut world = make_world();
        let session = world
            .start_recording(RecordingLimits::new(u64::from(upper)).unwrap())
            .expect("recording fixture must fit within the search ceiling");
        drop(session);
    }

    while lower < upper {
        let midpoint = lower + (upper - lower) / 2;
        let mut world = make_world();
        match world.start_recording(RecordingLimits::new(u64::from(midpoint)).unwrap()) {
            Ok(session) => {
                drop(session);
                upper = midpoint;
            }
            Err(Error::RecordingLimitExceeded) => lower = midpoint + 1,
            Err(error) => panic!("unexpected recording-start error at {midpoint} bytes: {error}"),
        }
    }

    lower
}

fn add_overlapping_material_pair(world: &mut World) -> BodyId {
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .material(
            SurfaceMaterial::default()
                .with_friction(0.5)
                .unwrap()
                .with_restitution(0.25)
                .unwrap()
                .with_user_material_id(7),
        )
        .build()
        .unwrap();
    let polygon = shapes::box_polygon(0.5, 0.5).unwrap();
    let first = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let second = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.4_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(first)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();
    world
        .body(second)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();
    first
}

#[test]
fn recording_start_limit_failure_leaves_world_reusable() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();

    match world.start_recording(RecordingLimits::new(1).unwrap()) {
        Err(Error::RecordingLimitExceeded) => {}
        Err(error) => panic!("unexpected recording-start error: {error}"),
        Ok(session) => {
            drop(session);
            panic!("one byte unexpectedly started a recording")
        }
    }

    assert!(world.counters().is_ok());
    world
        .start_recording(RecordingLimits::default())
        .unwrap()
        .finish()
        .unwrap();
}

#[test]
fn recording_writer_failure_from_body_capability_seals_the_session() {
    let limit = minimum_recording_limit(world_with_one_body);
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
    let mut session = world
        .start_recording(RecordingLimits::new(u64::from(limit)).unwrap())
        .unwrap();

    assert_eq!(
        session
            .body(body)
            .unwrap()
            .set_linear_velocity([1.0_f32, 0.0]),
        Err(Error::RecordingLimitExceeded)
    );
    assert_eq!(session.counters(), Err(Error::RecordingLimitExceeded));
    assert_eq!(
        session.body(body).err(),
        Some(Error::RecordingLimitExceeded)
    );

    drop(session);
    assert!(world.body(body).unwrap().linear_velocity().is_ok());
}

#[test]
fn recording_session_uses_common_object_capabilities() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity(Vec2::ZERO)
                .build()
                .unwrap(),
        )
        .unwrap();
    let mut session: RecordingSession<'_> =
        world.start_recording(RecordingLimits::default()).unwrap();

    session.enable_sleeping(true).unwrap();
    session.enable_continuous(true).unwrap();
    session.enable_warm_starting(true).unwrap();
    session.set_restitution_threshold(1.0).unwrap();
    session.set_hit_event_threshold(1.0).unwrap();
    session.set_gravity([0.0_f32, -9.8]).unwrap();
    session.set_contact_tuning(30.0, 1.0, 3.0).unwrap();
    session.set_contact_recycle_distance(0.01).unwrap();
    session.set_maximum_linear_speed(100.0).unwrap();

    let body_a = session
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let body_b = session
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([2.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let chain_body = session
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();

    let (shape, segment, chain_segment, capsule, polygon) = {
        let mut body = session.body(body_a).unwrap();
        body.set_name("recorded").unwrap();
        body.set_position_and_rotation(Position::ZERO, 0.0).unwrap();
        body.set_linear_velocity(Vec2::ZERO).unwrap();
        body.set_angular_velocity(0.0).unwrap();
        body.apply_force_to_center([1.0_f32, 0.0], true).unwrap();
        body.apply_torque(1.0, true).unwrap();
        body.clear_forces().unwrap();
        body.set_motion_locks(MotionLocks::new(false, false, false))
            .unwrap();
        body.wake_touching().unwrap();
        let shape = body
            .create_circle(
                &ShapeDef::builder().density(1.0).build().unwrap(),
                &shapes::circle(Vec2::ZERO, 0.5).unwrap(),
            )
            .unwrap();
        let segment = body
            .create_segment(
                &ShapeDef::default(),
                &shapes::segment([-0.5_f32, 0.0], [0.5_f32, 0.0]).unwrap(),
            )
            .unwrap();
        let chain_segment = body
            .create_chain_segment(
                &ShapeDef::default(),
                &shapes::chain_segment(
                    [-2.0_f32, 0.0],
                    [-1.0_f32, 0.0],
                    [1.0_f32, 0.0],
                    [2.0_f32, 0.0],
                )
                .unwrap(),
            )
            .unwrap();
        let capsule = body
            .create_capsule(
                &ShapeDef::default(),
                &shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25).unwrap(),
            )
            .unwrap();
        let polygon = body
            .create_polygon(
                &ShapeDef::default(),
                &shapes::box_polygon(0.5, 0.5).unwrap(),
            )
            .unwrap();
        (shape, segment, chain_segment, capsule, polygon)
    };

    {
        let mut shape_handle = session.shape(shape).unwrap();
        shape_handle.set_density(2.0, true).unwrap();
        shape_handle.set_friction(0.4).unwrap();
        shape_handle.set_restitution(0.2).unwrap();
        shape_handle.set_user_material(17).unwrap();
        shape_handle.set_filter(Filter::default()).unwrap();
        shape_handle
            .set_surface_material(
                &SurfaceMaterial::default()
                    .with_friction(0.4)
                    .unwrap()
                    .with_restitution(0.2)
                    .unwrap()
                    .with_user_material_id(17),
            )
            .unwrap();
        shape_handle
            .set_circle(&shapes::circle(Vec2::ZERO, 0.4).unwrap())
            .unwrap();
        assert_eq!(shape_handle.body_id().unwrap(), body_a);
        assert_eq!(shape_handle.parent_chain_id().unwrap(), None);
        assert!(shape_handle.test_point(Position::ZERO).unwrap());
    }
    session
        .shape(segment)
        .unwrap()
        .set_segment(&shapes::segment([-0.75_f32, 0.0], [0.75_f32, 0.0]).unwrap())
        .unwrap();
    session
        .shape(chain_segment)
        .unwrap()
        .set_chain_segment(
            &shapes::chain_segment(
                [-3.0_f32, 0.0],
                [-2.0_f32, 0.0],
                [2.0_f32, 0.0],
                [3.0_f32, 0.0],
            )
            .unwrap(),
        )
        .unwrap();
    session
        .shape(capsule)
        .unwrap()
        .set_capsule(&shapes::capsule([-0.75_f32, 0.0], [0.75_f32, 0.0], 0.2).unwrap())
        .unwrap();
    session
        .shape(polygon)
        .unwrap()
        .set_polygon(&shapes::box_polygon(0.75, 0.25).unwrap())
        .unwrap();

    let chain = {
        let def = ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build()
            .unwrap();
        session
            .body(chain_body)
            .unwrap()
            .create_chain(&def)
            .unwrap()
    };
    session
        .chain(chain)
        .unwrap()
        .set_surface_material(
            0,
            &SurfaceMaterial::default()
                .with_friction(0.75)
                .unwrap()
                .with_user_material_id(23),
        )
        .unwrap();
    let chain_segment_shape = session.chain(chain).unwrap().segments().unwrap()[0];
    assert_eq!(
        session
            .shape(chain_segment_shape)
            .unwrap()
            .parent_chain_id()
            .unwrap(),
        Some(chain)
    );

    let distance = session
        .create_distance_joint(&DistanceJointDef::new(
            session.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();
    {
        let mut joint = session.joint(distance).unwrap();
        joint.set_collide_connected(false).unwrap();
        joint.wake_bodies().unwrap();
        let mut joint = joint.into_distance().unwrap();
        joint.set_length(1.0).unwrap();
        joint.enable_spring(true).unwrap();
        joint.set_spring_hertz(5.0).unwrap();
    }

    let motor = session
        .create_motor_joint(&MotorJointDef::new(
            session.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();
    session
        .joint(motor)
        .unwrap()
        .into_motor()
        .unwrap()
        .set_linear_velocity([1.0_f32, 0.0])
        .unwrap();

    let filter = session
        .create_filter_joint(&FilterJointDef::new(
            session.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();
    assert_eq!(
        session.joint(filter).unwrap().into_filter().unwrap().id(),
        filter
    );

    let prismatic = session
        .create_prismatic_joint(&PrismaticJointDef::new(
            session.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();
    session
        .joint(prismatic)
        .unwrap()
        .into_prismatic()
        .unwrap()
        .set_target_translation(0.5)
        .unwrap();

    let revolute = session
        .create_revolute_joint(&RevoluteJointDef::new(
            session.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();
    session
        .joint(revolute)
        .unwrap()
        .into_revolute()
        .unwrap()
        .set_target_angle(0.25)
        .unwrap();

    let weld = session
        .create_weld_joint(&WeldJointDef::new(
            session.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();
    session
        .joint(weld)
        .unwrap()
        .into_weld()
        .unwrap()
        .set_linear_hertz(5.0)
        .unwrap();

    let wheel = session
        .create_wheel_joint(&WheelJointDef::new(
            session.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();
    session
        .joint(wheel)
        .unwrap()
        .into_wheel()
        .unwrap()
        .set_motor_speed(1.0)
        .unwrap();

    for joint in [distance, motor, filter, prismatic, revolute, weld, wheel] {
        session.joint(joint).unwrap().destroy(true).unwrap();
    }
    session.chain(chain).unwrap().destroy().unwrap();

    drop(session.step(1.0 / 60.0, 2).unwrap());
    assert_eq!(session.counters().unwrap().body_count, 3);
    session.finish().unwrap();
    assert_eq!(world.gravity().unwrap(), Vec2::new(0.0, -9.8));
    assert_eq!(
        world.body(body_a).unwrap().name().unwrap().as_deref(),
        Some("recorded")
    );
    assert!(
        world
            .shape(shape)
            .unwrap()
            .test_point(Position::ZERO)
            .unwrap()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn recording_query_uses_the_common_query_capability() {
    use boxdd::{Aabb, Query, QueryFilter};

    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let mut session = world.start_recording(RecordingLimits::default()).unwrap();
    let body = session
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let triangle =
        shapes::polygon_from_points([[0.0_f32, 0.0], [2.0_f32, 0.0], [0.0_f32, 2.0]], 0.0).unwrap();
    let shape = session
        .body(body)
        .unwrap()
        .create_polygon(&ShapeDef::default(), &triangle)
        .unwrap();

    {
        let query: Query<'_> = session.query().unwrap();
        let bounds = Aabb::new([-1.0_f32, -1.0], [2.0_f32, 2.0]).unwrap();
        assert_eq!(
            query
                .overlap_aabb(Position::ZERO, bounds, QueryFilter::default())
                .unwrap(),
            vec![shape]
        );
        let hit = query
            .cast_ray_closest_with_stats(
                Position::new(-1.0, 0.25),
                [4.0_f32, 0.0],
                QueryFilter::default(),
            )
            .unwrap();
        assert_eq!(hit.hit.map(|result| result.shape_id), Some(shape));
        assert!(hit.node_visits > 0);
        assert!(hit.leaf_visits > 0);
    }

    session.finish().unwrap();
}

#[test]
fn common_capabilities_reject_foreign_stale_and_invalid_inputs_before_ffi() {
    let mut source = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let foreign_body = source
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut target = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let mut session = target.start_recording(RecordingLimits::default()).unwrap();

    assert_eq!(session.body(foreign_body).err(), Some(Error::WrongWorld));

    let body = session
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let body_b = session
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    assert_invalid_argument(
        session
            .body(body)
            .unwrap()
            .set_linear_velocity(Vec2::new(f32::NAN, 0.0)),
    );
    let shape = session
        .body(body)
        .unwrap()
        .create_circle(
            &ShapeDef::default(),
            &shapes::circle(Vec2::ZERO, 0.5).unwrap(),
        )
        .unwrap();
    assert_invalid_argument(session.shape(shape).unwrap().set_density(-1.0, false));

    let distance = session
        .create_distance_joint(&DistanceJointDef::new(
            session.joint_base(body, body_b).unwrap(),
        ))
        .unwrap();
    let revolute = session
        .create_revolute_joint(&RevoluteJointDef::new(
            session.joint_base(body, body_b).unwrap(),
        ))
        .unwrap();
    assert_eq!(
        session.joint(revolute).unwrap().into_distance().err(),
        Some(Error::WrongJointType {
            expected: JointType::Distance,
            actual: JointType::Revolute,
        })
    );
    assert_invalid_argument(
        session
            .joint(distance)
            .unwrap()
            .into_distance()
            .unwrap()
            .set_length(-1.0),
    );
    session.joint(distance).unwrap().destroy(true).unwrap();
    assert_eq!(session.joint(distance).err(), Some(Error::InvalidJointId));

    let chain = session
        .body(body)
        .unwrap()
        .create_chain(
            &ChainDef::builder()
                .points([
                    Vec2::new(-2.0, 0.0),
                    Vec2::new(-1.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(2.0, 0.0),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();
    session.chain(chain).unwrap().destroy().unwrap();
    assert_eq!(session.chain(chain).err(), Some(Error::InvalidChainId));

    session.shape(shape).unwrap().destroy(false).unwrap();
    assert_eq!(session.shape(shape).err(), Some(Error::InvalidShapeId));
    session.joint(revolute).unwrap().destroy(true).unwrap();
    session.body(body_b).unwrap().destroy().unwrap();
    session.body(body).unwrap().destroy().unwrap();
    assert_eq!(session.body(body).err(), Some(Error::InvalidBodyId));

    session.finish().unwrap();
}

#[test]
fn recording_session_rejects_invalid_explosions_and_remains_replayable() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity(Vec2::ZERO)
                .build()
                .unwrap(),
        )
        .unwrap();
    let baseline = world
        .start_recording(RecordingLimits::default())
        .unwrap()
        .finish()
        .unwrap();

    let rejected = {
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
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
            assert_invalid_argument(session.explode(&def));
        }

        #[cfg(feature = "double-precision")]
        assert_invalid_argument(
            session.explode(&valid.position(Position::new(f64::from(f32::MAX) * 2.0, 0.0))),
        );

        session.finish().unwrap()
    };

    drop(world);
    let baseline_player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &baseline,
        ReplayConfig::default(),
    )
    .unwrap();
    let baseline_info = baseline_player.info();
    baseline_player.close().unwrap();
    let rejected_player = ReplayPlayer::open(
        boxdd::Foundation::initialize_default().unwrap(),
        &rejected,
        ReplayConfig::default(),
    )
    .unwrap();
    assert_eq!(rejected_player.info(), baseline_info);
    rejected_player.close().unwrap();
}

#[test]
fn drop_and_user_unwind_stop_recording_before_world_reuse() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();

    {
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        session.set_gravity([1.0_f32, -9.0]).unwrap();
    }
    assert_eq!(world.gravity().unwrap(), Vec2::new(1.0, -9.0));

    let unwind = catch_unwind(AssertUnwindSafe(|| {
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        drop(session.step(0.0, 1).unwrap());
        panic!("leave the recording scope through unwind");
    }));
    assert!(unwind.is_err());

    let mut session = world.start_recording(RecordingLimits::default()).unwrap();
    drop(session.step(0.0, 1).unwrap());
    session.finish().unwrap();
    assert!(world.counters().is_ok());
}

#[test]
fn installed_unsupported_callbacks_prevent_recording_start() {
    let mut custom_filter_world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    custom_filter_world.set_custom_filter(|_, _| true).unwrap();
    assert_start_error(
        &mut custom_filter_world,
        Error::RecordingCustomFilterUnsupported,
    );
    custom_filter_world.clear_custom_filter().unwrap();
    assert!(
        custom_filter_world
            .start_recording(RecordingLimits::default())
            .is_ok()
    );

    let mut pre_solve_world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    pre_solve_world.set_pre_solve(|_, _, _, _| true).unwrap();
    assert_start_error(&mut pre_solve_world, Error::RecordingPreSolveUnsupported);
    pre_solve_world.clear_pre_solve().unwrap();

    let session = pre_solve_world
        .start_recording(RecordingLimits::default())
        .unwrap();
    session.finish().unwrap();
}

#[test]
fn mixer_identities_are_owned_by_the_process_local_recording() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    world
        .set_friction_callback(FRICTION_MIXER_ID, |a, b| {
            (a.coefficient * b.coefficient).sqrt()
        })
        .unwrap();
    world
        .set_restitution_callback(RESTITUTION_MIXER_ID, |a, b| {
            a.coefficient.max(b.coefficient)
        })
        .unwrap();

    let session = world.start_recording(RecordingLimits::default()).unwrap();
    let identities = session.mixer_identities();
    assert_eq!(identities.friction(), Some(FRICTION_MIXER_ID));
    assert_eq!(identities.restitution(), Some(RESTITUTION_MIXER_ID));
    let recording = session.finish().unwrap();
    world.clear_friction_callback().unwrap();
    world.clear_restitution_callback().unwrap();
    drop(world);
    assert_eq!(recording.mixer_identities(), identities);
}

#[test]
fn worker_callback_panic_unwinds_then_session_tears_down() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity(Vec2::ZERO)
                .build()
                .unwrap(),
        )
        .unwrap();
    let body = add_overlapping_material_pair(&mut world);
    world
        .set_friction_callback(FRICTION_MIXER_ID, |_, _| -> f32 {
            panic!("recorded friction callback panic");
        })
        .unwrap();

    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        drop(session.step(1.0 / 60.0, 2).unwrap());
    }));
    assert!(result.is_err());

    world.clear_friction_callback().unwrap();
    let mut replacement = world.start_recording(RecordingLimits::default()).unwrap();
    drop(replacement.step(0.0, 1).unwrap());
    replacement.finish().unwrap();
    assert!(world.body(body).unwrap().position().is_ok());
}
