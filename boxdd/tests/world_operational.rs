use boxdd::prelude::*;

#[test]
fn world_operational_controls_validate_and_round_trip() {
    let initial_capacity = WorldCapacity::new(0, 1, 0, 1, 0).unwrap();
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .worker_count(WorkerCount::new(2).unwrap())
            .capacity(initial_capacity)
            .build(),
    )
    .unwrap();

    assert_eq!(world.worker_count().get(), 2);
    assert_eq!(world.handle().try_worker_count().unwrap().get(), 2);

    world.try_set_worker_count(1).unwrap();
    assert_eq!(world.worker_count().get(), 1);
    world.try_set_worker_count(2).unwrap();
    assert_eq!(world.worker_count().get(), 2);
    world
        .try_set_worker_count(u32::from(B2_MAX_WORKERS))
        .unwrap();
    assert_eq!(world.worker_count().get(), B2_MAX_WORKERS);
    world.try_step(0.0, 1).unwrap();
    world.try_set_worker_count(1).unwrap();
    assert_eq!(world.worker_count().get(), 1);

    assert_eq!(
        world.try_set_worker_count(0).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        world
            .try_set_worker_count(u32::from(B2_MAX_WORKERS) + 1)
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    assert!(world.try_contact_recycle_distance().unwrap().is_finite());
    world.try_set_contact_recycle_distance(0.25).unwrap();
    assert_eq!(world.contact_recycle_distance(), 0.25);
    world.set_contact_recycle_distance(0.0);
    assert_eq!(world.contact_recycle_distance(), 0.0);
    assert_eq!(
        world.try_set_contact_recycle_distance(-1.0).unwrap_err(),
        ApiError::InvalidArgument
    );
    assert_eq!(
        world
            .try_set_contact_recycle_distance(f32::NAN)
            .unwrap_err(),
        ApiError::InvalidArgument
    );

    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    world.create_circle_shape_for(
        body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    let bounds = world.bounds();
    assert!(bounds.lower.x <= -0.5);
    assert!(bounds.upper.y >= 0.5);
    assert_eq!(world.handle().bounds(), bounds);

    // Box2D 3.2 performs the zero-time-step collision/event pass; only a
    // negative time step is rejected by the wrapper.
    world.try_step(0.0, 1).unwrap();
    let capacity = world.maximum_capacity();
    assert_eq!(capacity, initial_capacity);
    assert_eq!(capacity, world.handle().maximum_capacity());
    assert_eq!(
        world.try_step(-f32::EPSILON, 1).unwrap_err(),
        ApiError::InvalidArgument
    );
}

#[test]
fn zero_time_step_runs_collision_detection_and_publishes_contact_events() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let static_body = world.create_body_id(BodyDef::default());
    let dynamic_body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.75_f32, 0.0])
            .build(),
    );
    let shape_def = ShapeDef::builder().enable_contact_events(true).build();
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    world.create_circle_shape_for(static_body, &shape_def, &circle);
    world.create_circle_shape_for(dynamic_body, &shape_def, &circle);

    assert_eq!(world.counters().contact_count, 0);
    assert!(world.contact_events().begin.is_empty());
    let before = world.body_position(dynamic_body);

    world.try_step(0.0, 1).unwrap();

    assert_eq!(world.body_position(dynamic_body), before);
    assert_eq!(world.counters().contact_count, 1);
    assert_eq!(world.counters().awake_contact_count, 1);
    let events = world.contact_events();
    assert_eq!(events.begin.len(), 1);
    assert!(events.end.is_empty());
    assert!(events.hit.is_empty());
}

fn make_deterministic_world(worker_count: u32) -> (World, BodyId) {
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .worker_count(WorkerCount::new(worker_count).unwrap())
            .build(),
    )
    .unwrap();
    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 4.0])
            .build(),
    );
    world.create_circle_shape_for(
        body,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    (world, body)
}

#[test]
fn built_in_scheduler_matches_single_worker_for_deterministic_scene() {
    let (mut serial, serial_body) = make_deterministic_world(1);
    let (mut parallel, parallel_body) = make_deterministic_world(2);
    for _ in 0..30 {
        serial.step(1.0 / 60.0, 4);
        parallel.step(1.0 / 60.0, 4);
    }

    let serial_position = serial.body_position(serial_body);
    let parallel_position = parallel.body_position(parallel_body);
    assert_eq!(serial_position, parallel_position);
    assert_eq!(serial.counters().body_count, parallel.counters().body_count);
}

unsafe extern "C" fn synchronous_enqueue(
    task: boxdd_sys::ffi::b2TaskCallback,
    task_context: *mut core::ffi::c_void,
    _user_context: *mut core::ffi::c_void,
) -> *mut core::ffi::c_void {
    if let Some(task) = task {
        unsafe { task(task_context) };
    }
    core::ptr::null_mut()
}

unsafe extern "C" fn synchronous_finish(
    _task: *mut core::ffi::c_void,
    _user_context: *mut core::ffi::c_void,
) {
}

#[test]
fn raw_task_system_keeps_its_worker_count_contract() {
    let def = unsafe {
        WorldDef::builder()
            .task_system_raw(
                1,
                Some(synchronous_enqueue),
                Some(synchronous_finish),
                core::ptr::null_mut(),
            )
            .build()
    };
    let mut world = World::new(def).unwrap();
    assert_eq!(world.worker_count().get(), 1);
    assert_eq!(
        world.try_set_worker_count(2),
        Err(ApiError::RawTaskSystemWorkerCountFixed)
    );
    assert_eq!(world.worker_count().get(), 1);
}
