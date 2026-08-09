use super::*;

#[test]
fn world_runtime_controls_return_in_callback() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let explosion = crate::ExplosionDef::new()
        .position([0.0_f32, 0.0])
        .radius(1.0)
        .falloff(0.5)
        .impulse_per_length(2.0);
    let _guard = crate::core::callback_state::CallbackGuard::enter();

    assert_eq!(
        world.enable_sleeping(false).unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(
        world.is_sleeping_enabled().unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(world.gravity().unwrap_err(), crate::Error::InCallback);
    assert_eq!(world.counters().unwrap_err(), crate::Error::InCallback);
    assert_eq!(world.profile().unwrap_err(), crate::Error::InCallback);
    assert_eq!(
        world.awake_body_count().unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(world.bounds().unwrap_err(), crate::Error::InCallback);
    assert_eq!(
        world.maximum_capacity().unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(
        world.set_contact_recycle_distance(0.0).unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(
        world.set_worker_count(WorkerCount::default()).unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(
        world.explode(&explosion).unwrap_err(),
        crate::Error::InCallback
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn callback_sensitive_entrypoints_return_without_touching_outputs() {
    struct NoopDrawer;

    impl crate::DebugDraw for NoopDrawer {}

    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let mut commands = Vec::new();
    let mut drawer = NoopDrawer;
    let _guard = crate::core::callback_state::CallbackGuard::enter();

    assert_eq!(
        world.step(1.0 / 60.0, 1).unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(
        world
            .debug_draw_collect(crate::DebugDrawOptions::default())
            .unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(
        world
            .debug_draw_collect_into(&mut commands, crate::DebugDrawOptions::default())
            .unwrap_err(),
        crate::Error::InCallback
    );
    assert!(commands.is_empty());
    assert_eq!(
        world
            .debug_draw(&mut drawer, crate::DebugDrawOptions::default())
            .unwrap_err(),
        crate::Error::InCallback
    );
}

#[test]
fn object_capabilities_return_in_callback() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let shape_id = {
        let mut body = world.body(body_id).unwrap();
        body.create_centered_circle(&crate::ShapeDef::default(), 0.5)
            .unwrap()
    };

    let _guard = crate::core::callback_state::CallbackGuard::enter();
    assert_eq!(world.body(body_id).err().unwrap(), crate::Error::InCallback);
    assert_eq!(
        world.shape(shape_id).err().unwrap(),
        crate::Error::InCallback
    );
}

#[test]
fn body_creation_capability_returns_in_callback() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut body = world.body(body_id).unwrap();
    let chain_def = crate::ChainDef::builder()
        .points([[-2.0_f32, 0.0], [-1.0, 0.0], [1.0, 0.0], [2.0, 0.0]])
        .build()
        .unwrap();
    let _guard = crate::core::callback_state::CallbackGuard::enter();

    assert_eq!(body.position().unwrap_err(), crate::Error::InCallback);
    assert_eq!(
        body.create_centered_circle(&crate::ShapeDef::default(), 0.5)
            .unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(
        body.create_chain(&chain_def).unwrap_err(),
        crate::Error::InCallback
    );
}

#[test]
fn callback_registration_returns_in_callback_without_installing_state() {
    let mut world = crate::Foundation::initialize_default()
        .unwrap()
        .create_world(
            crate::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let _guard = crate::core::callback_state::CallbackGuard::enter();

    assert_eq!(
        world.set_custom_filter(|_, _| true).unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(
        world.clear_custom_filter().unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(
        world.set_pre_solve(|_, _, _, _| true).unwrap_err(),
        crate::Error::InCallback
    );
    assert_eq!(
        world.clear_pre_solve().unwrap_err(),
        crate::Error::InCallback
    );
    assert!(world.core().custom_filter.lock().unwrap().is_none());
    assert!(world.core().pre_solve.lock().unwrap().is_none());
}
