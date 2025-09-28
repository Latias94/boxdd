use boxdd::prelude::*;

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

    world.step(1.0, 1);
}
