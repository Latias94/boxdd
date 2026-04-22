use super::*;

#[test]
fn try_world_runtime_extras_return_in_callback() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let handle = world.handle();
    let explosion = crate::ExplosionDef::new()
        .position([0.0_f32, 0.0])
        .radius(1.0)
        .falloff(0.5)
        .impulse_per_length(2.0);

    let _g = crate::core::callback_state::CallbackGuard::enter();

    assert_eq!(
        world.try_enable_sleeping(false).unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_is_sleeping_enabled().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        handle.try_gravity().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        handle.try_counters().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        handle.try_profile().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        handle.try_awake_body_count().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        handle.try_is_sleeping_enabled().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        handle.try_is_continuous_enabled().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        handle.try_is_warm_starting_enabled().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        handle.try_restitution_threshold().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        handle.try_hit_event_threshold().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        handle.try_maximum_linear_speed().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_profile().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_enable_speculative(true).unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_explode(&explosion).unwrap_err(),
        crate::ApiError::InCallback
    );
}

#[test]
fn try_world_callback_sensitive_entrypoints_return_in_callback() {
    struct NoopDrawer;

    impl crate::DebugDraw for NoopDrawer {}

    impl crate::debug_draw::RawDebugDraw for NoopDrawer {
        fn draw_polygon(&mut self, _vertices: &[boxdd_sys::ffi::b2Vec2], _color: crate::HexColor) {}
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let mut cmds = Vec::new();
    let mut drawer = NoopDrawer;
    let mut raw_drawer = NoopDrawer;
    let _g = crate::core::callback_state::CallbackGuard::enter();

    assert_eq!(
        world.try_step(1.0 / 60.0, 1).unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_flush_deferred_destroys().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world
            .try_debug_draw_collect(crate::DebugDrawOptions::default())
            .unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world
            .try_debug_draw_collect_into(&mut cmds, crate::DebugDrawOptions::default())
            .unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world
            .try_debug_draw(&mut drawer, crate::DebugDrawOptions::default())
            .unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world
            .try_debug_draw_raw(&mut raw_drawer, crate::DebugDrawOptions::default())
            .unwrap_err(),
        crate::ApiError::InCallback
    );
}

#[test]
fn try_world_shape_and_chain_creation_return_in_callback() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(crate::BodyBuilder::new().build());
    let shape_def = crate::ShapeDef::default();
    let circle = crate::shapes::circle([0.0_f32, 0.0], 0.5);
    let chain_def = crate::shapes::chain::ChainDef::builder()
        .points([
            crate::Vec2::new(-1.0, 0.0),
            crate::Vec2::new(0.0, 0.0),
            crate::Vec2::new(1.0, 0.0),
            crate::Vec2::new(2.0, 0.0),
        ])
        .build();

    let _g = crate::core::callback_state::CallbackGuard::enter();

    assert_eq!(
        world
            .try_create_circle_shape_for(body, &shape_def, &circle)
            .unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world
            .try_create_circle_shape_for_owned(body, &shape_def, &circle)
            .err()
            .unwrap(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_create_chain_for_id(body, &chain_def).unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world
            .try_create_chain_for_owned(body, &chain_def)
            .err()
            .unwrap(),
        crate::ApiError::InCallback
    );
}

#[test]
fn try_world_scoped_handle_borrows_return_in_callback() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(crate::BodyBuilder::new().build());
    let shape_id = world.create_circle_shape_for(
        body_id,
        &crate::ShapeDef::default(),
        &crate::shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let chain_id = world.create_chain_for_id(
        body_id,
        &crate::shapes::chain::ChainDef::builder()
            .points([
                crate::Vec2::new(-1.0, 0.0),
                crate::Vec2::new(0.0, 0.0),
                crate::Vec2::new(1.0, 0.0),
                crate::Vec2::new(2.0, 0.0),
            ])
            .build(),
    );
    let other_body = world.create_body_id(crate::BodyBuilder::new().build());
    let joint_id = world.create_distance_joint_id(
        &crate::DistanceJointDef::new(
            crate::JointBaseBuilder::new()
                .bodies_by_id(body_id, other_body)
                .build(),
        )
        .length(1.0),
    );

    let _g = crate::core::callback_state::CallbackGuard::enter();

    assert_eq!(
        world.try_body(body_id).err().unwrap(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_shape(shape_id).err().unwrap(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_chain(chain_id).err().unwrap(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_joint(joint_id).unwrap_err(),
        crate::ApiError::InCallback
    );
}

#[test]
fn try_world_callback_registration_returns_in_callback() {
    fn always_true_filter(_a: ShapeId, _b: ShapeId) -> bool {
        true
    }

    fn always_true_pre(
        _a: ShapeId,
        _b: ShapeId,
        _p: crate::types::Vec2,
        _n: crate::types::Vec2,
    ) -> bool {
        true
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let _g = crate::core::callback_state::CallbackGuard::enter();

    assert_eq!(
        world
            .try_set_custom_filter_with_ctx(|_, _, _| true)
            .unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_set_custom_filter(always_true_filter).unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_clear_custom_filter().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world
            .try_set_custom_filter_callback(Some(always_true_filter))
            .unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_set_custom_filter_callback(None).unwrap_err(),
        crate::ApiError::InCallback
    );

    assert_eq!(
        world
            .try_set_pre_solve_with_ctx(|_, _, _, _, _| true)
            .unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_set_pre_solve(always_true_pre).unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_clear_pre_solve().unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world
            .try_set_pre_solve_callback(Some(always_true_pre))
            .unwrap_err(),
        crate::ApiError::InCallback
    );
    assert_eq!(
        world.try_set_pre_solve_callback(None).unwrap_err(),
        crate::ApiError::InCallback
    );
}
