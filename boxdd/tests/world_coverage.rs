use boxdd::prelude::*;
use boxdd::shapes;
use boxdd::world::{Counters, Profile};

fn same_world_id(a: boxdd_sys::ffi::b2WorldId, b: boxdd_sys::ffi::b2WorldId) -> bool {
    a.index1 == b.index1 && a.generation == b.generation
}

#[test]
fn world_runtime_coverage_safe_api() {
    let mut world = World::new(WorldDef::builder().build()).unwrap();

    world.enable_sleeping(true);
    world.enable_sleeping(false);
    assert!(!world.is_sleeping_enabled());
    world.try_enable_sleeping(true).unwrap();
    assert!(world.try_is_sleeping_enabled().unwrap());

    world.enable_continuous(false);
    world.enable_continuous(true);
    assert!(world.is_continuous_enabled());
    world.try_enable_continuous(false).unwrap();
    assert!(!world.try_is_continuous_enabled().unwrap());
    world.try_enable_continuous(true).unwrap();

    world.set_restitution_threshold(0.0);
    world.set_restitution_threshold(2.0);
    assert_eq!(world.restitution_threshold(), 2.0);
    world.try_set_restitution_threshold(3.0).unwrap();
    assert_eq!(world.try_restitution_threshold().unwrap(), 3.0);

    world.set_hit_event_threshold(0.0);
    world.set_hit_event_threshold(100.0);
    assert_eq!(world.hit_event_threshold(), 100.0);
    world.try_set_hit_event_threshold(42.0).unwrap();
    assert_eq!(world.try_hit_event_threshold().unwrap(), 42.0);

    world.set_gravity([1.0_f32, 2.0]);
    let g = world.gravity();
    assert_eq!(g.x, 1.0);
    assert_eq!(g.y, 2.0);

    world.set_contact_tuning(10.0, 2.0, 4.0);
    world.try_set_contact_tuning(9.0, 1.5, 3.0).unwrap();
    world.enable_speculative(true);
    world.try_enable_speculative(false).unwrap();

    world.set_maximum_linear_speed(10.0);
    assert_eq!(world.maximum_linear_speed(), 10.0);
    world.try_set_maximum_linear_speed(12.0).unwrap();
    assert_eq!(world.try_maximum_linear_speed().unwrap(), 12.0);

    // Warm starting switch
    world.enable_warm_starting(true);
    assert!(world.is_warm_starting_enabled());
    world.try_enable_warm_starting(false).unwrap();
    assert!(!world.try_is_warm_starting_enabled().unwrap());

    // Register callbacks (minimal smoke) and then unregister
    fn always_true_filter(_a: ShapeId, _b: ShapeId) -> bool {
        true
    }
    fn always_true_pre(
        _a: ShapeId,
        _b: ShapeId,
        _p: boxdd::types::Vec2,
        _n: boxdd::types::Vec2,
    ) -> bool {
        true
    }
    world.set_custom_filter_callback(Some(always_true_filter));
    world.set_pre_solve_callback(Some(always_true_pre));
    world.set_custom_filter_callback(None);
    world.set_pre_solve_callback(None);
    world
        .try_set_custom_filter_with_ctx(|_, _, _| true)
        .unwrap();
    world
        .try_set_pre_solve_with_ctx(|_, _, _, _, _| true)
        .unwrap();
    world.try_set_custom_filter(always_true_filter).unwrap();
    world.try_set_pre_solve(always_true_pre).unwrap();
    world.try_clear_custom_filter().unwrap();
    world.try_clear_pre_solve().unwrap();
    world
        .try_set_custom_filter_callback(Some(always_true_filter))
        .unwrap();
    world
        .try_set_pre_solve_callback(Some(always_true_pre))
        .unwrap();
    world.try_set_custom_filter_callback(None).unwrap();
    world.try_set_pre_solve_callback(None).unwrap();

    assert_eq!(world.awake_body_count(), 0);

    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let locks = MotionLocks::new(true, false, true);
    world.set_body_motion_locks(body, locks);
    assert_eq!(world.body_motion_locks(body), locks);
    let _shape = world.create_circle_shape_for(
        body,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );

    let explosion = ExplosionDef::new()
        .mask_bits(u64::MAX)
        .position([0.0_f32, 0.0])
        .radius(1.0)
        .falloff(0.5)
        .impulse_per_length(2.0);
    world.explode(&explosion);
    world.try_explode(&explosion).unwrap();

    world.step(1.0, 1);
    let profile = world.profile();
    assert!(profile.step.is_finite());
    assert!(world.try_profile().unwrap().solve.is_finite());
}

#[test]
fn raw_world_id_escape_hatches_are_explicit() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let world_id = world.world_id_raw();
    let handle = world.handle();
    assert!(same_world_id(handle.world_id_raw(), world_id));

    let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    assert!(same_world_id(body.world_id_raw(), world_id));
    assert!(same_world_id(body.try_world_id_raw().unwrap(), world_id));

    let shape = world.create_circle_shape_for_owned(
        body.id(),
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    assert!(same_world_id(shape.world_id_raw(), world_id));
    assert!(same_world_id(shape.try_world_id_raw().unwrap(), world_id));

    let chain = world.create_chain_for_owned(
        body.id(),
        &boxdd::shapes::chain::ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build(),
    );
    assert!(same_world_id(chain.world_id_raw(), world_id));
    assert!(same_world_id(chain.try_world_id_raw().unwrap(), world_id));
}

#[test]
fn mass_data_and_motion_locks_round_trip_through_explicit_raw_conversions() {
    let mass_data = MassData::new(3.5, Vec2::new(1.0, -2.0), 4.25);
    assert_eq!(MassData::from_raw(mass_data.into_raw()), mass_data);

    let locks = MotionLocks::new(true, false, true);
    assert_eq!(MotionLocks::from_raw(locks.into_raw()), locks);
}

#[test]
fn body_type_counters_and_profile_use_explicit_raw_conversions() {
    assert_eq!(
        BodyType::from_raw(boxdd_sys::ffi::b2BodyType_b2_staticBody),
        BodyType::Static
    );
    assert_eq!(
        BodyType::from_raw(boxdd_sys::ffi::b2BodyType_b2_kinematicBody),
        BodyType::Kinematic
    );
    assert_eq!(
        BodyType::from_raw(boxdd_sys::ffi::b2BodyType_b2_dynamicBody),
        BodyType::Dynamic
    );

    assert_eq!(
        BodyType::Static.into_raw(),
        boxdd_sys::ffi::b2BodyType_b2_staticBody
    );
    assert_eq!(
        BodyType::Kinematic.into_raw(),
        boxdd_sys::ffi::b2BodyType_b2_kinematicBody
    );
    assert_eq!(
        BodyType::Dynamic.into_raw(),
        boxdd_sys::ffi::b2BodyType_b2_dynamicBody
    );

    let raw = boxdd_sys::ffi::b2Counters {
        bodyCount: 1,
        shapeCount: 2,
        contactCount: 3,
        jointCount: 4,
        islandCount: 5,
        stackUsed: 6,
        staticTreeHeight: 7,
        treeHeight: 8,
        byteCount: 9,
        taskCount: 10,
        colorCounts: core::array::from_fn(|i| i as i32),
    };
    let counters = Counters::from_raw(raw);

    assert_eq!(counters.body_count, 1);
    assert_eq!(counters.shape_count, 2);
    assert_eq!(counters.contact_count, 3);
    assert_eq!(counters.joint_count, 4);
    assert_eq!(counters.island_count, 5);
    assert_eq!(counters.stack_used, 6);
    assert_eq!(counters.static_tree_height, 7);
    assert_eq!(counters.tree_height, 8);
    assert_eq!(counters.byte_count, 9);
    assert_eq!(counters.task_count, 10);
    assert_eq!(counters.color_counts[0], 0);
    assert_eq!(counters.color_counts[23], 23);

    let raw_profile = boxdd_sys::ffi::b2Profile {
        step: 1.0,
        pairs: 2.0,
        collide: 3.0,
        solve: 4.0,
        prepareStages: 5.0,
        solveConstraints: 6.0,
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
    let profile = Profile::from_raw(raw_profile);

    assert_eq!(profile.step, 1.0);
    assert_eq!(profile.solve_constraints, 6.0);
    assert_eq!(profile.sleep_islands, 22.0);
    assert_eq!(profile.sensors, 23.0);
    assert_eq!(Profile::from_raw(profile.into_raw()), profile);
}

#[test]
fn explosion_def_uses_explicit_raw_conversions() {
    let raw = boxdd_sys::ffi::b2ExplosionDef {
        maskBits: 0x0f0f,
        position: boxdd_sys::ffi::b2Vec2 { x: 1.5, y: -2.5 },
        radius: 3.0,
        falloff: 4.0,
        impulsePerLength: 5.0,
    };
    let def = ExplosionDef::from_raw(raw);
    let roundtrip = def.into_raw();

    assert_eq!(roundtrip.maskBits, 0x0f0f);
    assert_eq!(roundtrip.position.x, 1.5);
    assert_eq!(roundtrip.position.y, -2.5);
    assert_eq!(roundtrip.radius, 3.0);
    assert_eq!(roundtrip.falloff, 4.0);
    assert_eq!(roundtrip.impulsePerLength, 5.0);
}
