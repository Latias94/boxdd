use boxdd::prelude::*;
use boxdd::world::{Counters, Profile};

#[test]
fn world_def_is_a_readable_validated_rust_value() {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let capacity = WorldCapacity::new(10, 20, 30, 40, 50).unwrap();
    let def = foundation
        .world_builder()
        .gravity([1.0_f32, -9.5])
        .restitution_threshold(2.5)
        .hit_event_threshold(7.0)
        .contact_hertz(11.0)
        .contact_damping_ratio(0.6)
        .contact_speed(13.0)
        .maximum_linear_speed(42.0)
        .enable_sleep(false)
        .enable_continuous(false)
        .enable_contact_softening(false)
        .worker_count(WorkerCount::new(3).unwrap())
        .capacity(capacity)
        .build()
        .unwrap();
    assert_eq!(def.gravity(), Vec2::new(1.0, -9.5));
    assert_eq!(def.restitution_threshold(), 2.5);
    assert_eq!(def.hit_event_threshold(), 7.0);
    assert_eq!(def.contact_hertz(), 11.0);
    assert_eq!(def.contact_damping_ratio(), 0.6);
    assert_eq!(def.contact_speed(), 13.0);
    assert_eq!(def.maximum_linear_speed(), 42.0);
    assert!(!def.is_sleep_enabled());
    assert!(!def.is_continuous_enabled());
    assert!(!def.is_contact_softening_enabled());
    assert_eq!(def.worker_count().get(), 3);
    assert_eq!(def.capacity(), capacity);
    assert_eq!(def.validate(), Ok(()));

    for (result, expected) in [
        (
            foundation.world_builder().gravity([f32::NAN, 0.0]).build(),
            Error::invalid_argument("WorldDef::validate", "gravity", "a finite vector"),
        ),
        (
            foundation.world_builder().maximum_linear_speed(0.0).build(),
            Error::invalid_argument(
                "WorldDef::validate",
                "maximum_linear_speed",
                "a positive finite value whose square is finite",
            ),
        ),
        (
            foundation
                .world_builder()
                .maximum_linear_speed(f32::MAX)
                .build(),
            Error::invalid_argument(
                "WorldDef::validate",
                "maximum_linear_speed",
                "a positive finite value whose square is finite",
            ),
        ),
    ] {
        assert_eq!(result.unwrap_err(), expected);
    }
}

#[test]
fn foundational_runtime_values_round_trip_through_explicit_raw_conversions() {
    let mass_data = MassData::new(3.5, Vec2::new(1.0, -2.0), 4.25).unwrap();
    assert_eq!(MassData::from_raw(mass_data.into_raw()).unwrap(), mass_data);
    let locks = MotionLocks::new(true, false, true);
    assert_eq!(MotionLocks::from_raw(locks.into_raw()), locks);

    for body_type in [BodyType::Static, BodyType::Kinematic, BodyType::Dynamic] {
        assert_eq!(BodyType::from_raw(body_type.into_raw()), Some(body_type));
    }

    let raw_counters = boxdd_sys::ffi::b2Counters {
        byteCount: i64::from(i32::MAX) + 9,
        bodyCount: 5,
        shapeCount: 8,
        contactCount: 300,
        jointCount: 300,
        islandCount: 5,
        stackUsed: 6,
        staticTreeHeight: 7,
        treeHeight: 8,
        taskCount: 10,
        colorCounts: core::array::from_fn(|index| index as i32),
        awakeContactCount: 11,
        recycledContactCount: 12,
    };
    let counters = Counters::from_raw(raw_counters).unwrap();
    assert_eq!(counters.body_count, 5);
    assert_eq!(counters.shape_count, 8);
    assert_eq!(counters.contact_count, 300);
    assert_eq!(counters.joint_count, 300);
    assert_eq!(counters.byte_count, i64::from(i32::MAX) + 9);
    assert_eq!(counters.color_counts[23], 23);
    assert_eq!(counters.awake_contact_count, 11);
    assert_eq!(counters.recycled_contact_count, 12);

    let raw_profile = boxdd_sys::ffi::b2Profile {
        step: 1.0,
        pairs: 2.0,
        collide: 3.0,
        solve: 4.0,
        solverSetup: 5.0,
        constraints: 6.0,
        prepareConstraints: 7.0,
        integrateVelocities: 8.0,
        warmStart: 9.0,
        solveImpulses: 10.0,
        integratePositions: 11.0,
        relaxImpulses: 12.0,
        applyRestitution: 13.0,
        storeImpulses: 14.0,
        splitIslands: 15.0,
        transforms: 16.0,
        sensorHits: 17.0,
        jointEvents: 18.0,
        hitEvents: 19.0,
        refit: 20.0,
        bullets: 21.0,
        sleepIslands: 22.0,
        sensors: 23.0,
    };
    let profile = Profile::from_raw(raw_profile).unwrap();
    assert_eq!(profile.step, 1.0);
    assert_eq!(profile.solver_setup, 5.0);
    assert_eq!(profile.sleep_islands, 22.0);
    assert_eq!(Profile::from_raw(profile.into_raw()), Ok(profile));
}

#[test]
fn explosion_definition_round_trips_and_invalid_inputs_are_transactional() {
    let valid = ExplosionDef::new()
        .mask_bits(u64::MAX)
        .position(Position::ZERO)
        .radius(1.0)
        .falloff(0.5)
        .impulse_per_length(-2.0);
    let roundtrip = ExplosionDef::from_raw(valid.into_raw());
    assert_eq!(roundtrip.affected_mask_bits(), u64::MAX);
    assert_eq!(roundtrip.center(), Position::ZERO);
    assert_eq!(roundtrip.blast_radius(), 1.0);
    assert_eq!(roundtrip.falloff_distance(), 0.5);
    assert_eq!(roundtrip.impulse_per_unit_length(), -2.0);

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
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(body)
        .unwrap()
        .create_centered_circle(&ShapeDef::builder().density(1.0).build().unwrap(), 0.5)
        .unwrap();

    for (invalid, expected) in [
        (
            valid.position(Position::new(WorldScalar::NAN, 0.0)),
            Error::invalid_argument("World::explode", "position", "a finite world position"),
        ),
        (
            valid.radius(f32::NAN),
            Error::invalid_argument(
                "World::explode",
                "radius",
                "a finite value greater than or equal to zero",
            ),
        ),
        (
            valid.radius(-1.0),
            Error::invalid_argument(
                "World::explode",
                "radius",
                "a finite value greater than or equal to zero",
            ),
        ),
        (
            valid.falloff(f32::INFINITY),
            Error::invalid_argument(
                "World::explode",
                "falloff",
                "a finite value greater than or equal to zero",
            ),
        ),
        (
            valid.impulse_per_length(f32::NEG_INFINITY),
            Error::invalid_argument("World::explode", "impulse_per_length", "a finite value"),
        ),
        (
            valid.radius(f32::MAX).falloff(f32::MAX),
            Error::invalid_argument(
                "World::explode",
                "position/radius/falloff",
                "a query extent representable by finite local f32 bounds",
            ),
        ),
    ] {
        assert_eq!(world.explode(&invalid), Err(expected));
        let body = world.body(body).unwrap();
        assert_eq!(body.linear_velocity().unwrap(), Vec2::ZERO);
        assert_eq!(body.angular_velocity().unwrap(), 0.0);
    }

    world.explode(&valid.impulse_per_length(0.0)).unwrap();
}
