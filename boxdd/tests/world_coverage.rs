use boxdd::prelude::*;
use boxdd::shapes;

fn same_world_id(a: boxdd_sys::ffi::b2WorldId, b: boxdd_sys::ffi::b2WorldId) -> bool {
    a.index1 == b.index1 && a.generation == b.generation
}

#[test]
fn world_runtime_coverage_safe_api() {
    let mut world = World::new(WorldDef::builder().build()).unwrap();

    world.enable_sleeping(true);
    world.enable_sleeping(false);
    assert!(!world.is_sleeping_enabled());

    world.enable_continuous(false);
    world.enable_continuous(true);
    assert!(world.is_continuous_enabled());

    world.set_restitution_threshold(0.0);
    world.set_restitution_threshold(2.0);
    assert_eq!(world.restitution_threshold(), 2.0);

    world.set_hit_event_threshold(0.0);
    world.set_hit_event_threshold(100.0);
    assert_eq!(world.hit_event_threshold(), 100.0);

    world.set_gravity([1.0_f32, 2.0]);
    let g = world.gravity();
    assert_eq!(g.x, 1.0);
    assert_eq!(g.y, 2.0);

    world.set_contact_tuning(10.0, 2.0, 4.0);

    world.set_maximum_linear_speed(10.0);
    assert_eq!(world.maximum_linear_speed(), 10.0);

    // Warm starting switch
    world.enable_warm_starting(true);
    assert!(world.is_warm_starting_enabled());

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

    assert_eq!(world.awake_body_count(), 0);

    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let locks = MotionLocks::new(true, false, true);
    world.set_body_motion_locks(body, locks);
    assert_eq!(world.body_motion_locks(body), locks);

    world.step(1.0, 1);
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
