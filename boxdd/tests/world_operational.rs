use boxdd::prelude::*;

#[test]
fn world_operational_controls_validate_and_round_trip() {
    let initial_capacity = WorldCapacity::new(0, 1, 0, 1, 0).unwrap();
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, -10.0])
                .worker_count(WorkerCount::new(2).unwrap())
                .capacity(initial_capacity)
                .build()
                .unwrap(),
        )
        .unwrap();

    assert_eq!(world.worker_count().unwrap().get(), 2);
    world
        .set_worker_count(WorkerCount::new(1).unwrap())
        .unwrap();
    assert_eq!(world.worker_count().unwrap().get(), 1);
    world
        .set_worker_count(WorkerCount::new(2).unwrap())
        .unwrap();
    assert_eq!(world.worker_count().unwrap().get(), 2);
    world
        .set_worker_count(WorkerCount::new(u32::from(B2_MAX_WORKERS)).unwrap())
        .unwrap();
    assert_eq!(world.worker_count().unwrap().get(), B2_MAX_WORKERS);
    drop(world.step(0.0, 1).unwrap());
    world
        .set_worker_count(WorkerCount::new(1).unwrap())
        .unwrap();

    assert!(world.contact_recycle_distance().unwrap().is_finite());
    world.set_contact_recycle_distance(0.25).unwrap();
    assert_eq!(world.contact_recycle_distance().unwrap(), 0.25);
    world.set_contact_recycle_distance(0.0).unwrap();
    assert_eq!(world.contact_recycle_distance().unwrap(), 0.0);
    assert_eq!(
        world.set_contact_recycle_distance(-1.0),
        Err(Error::invalid_argument(
            "World::set_contact_recycle_distance",
            "distance",
            "a finite value greater than or equal to zero",
        ))
    );
    assert_eq!(
        world.set_contact_recycle_distance(f32::NAN),
        Err(Error::invalid_argument(
            "World::set_contact_recycle_distance",
            "distance",
            "a finite value greater than or equal to zero",
        ))
    );

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
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    let bounds = world.bounds().unwrap();
    assert!(bounds.lower().x <= -0.5);
    assert!(bounds.upper().y >= 0.5);

    drop(world.step(0.0, 1).unwrap());
    assert_eq!(world.maximum_capacity().unwrap(), initial_capacity);
    assert_eq!(
        world.step(-f32::EPSILON, 1).unwrap_err(),
        Error::invalid_argument(
            "World::step",
            "time_step",
            "a finite value greater than or equal to zero",
        )
    );
}

#[test]
fn zero_time_step_runs_collision_detection_and_publishes_contact_events() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let static_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let dynamic_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.75_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape_def = ShapeDef::builder()
        .enable_contact_events(true)
        .build()
        .unwrap();
    let circle = shapes::circle([0.0_f32, 0.0], 0.5).unwrap();
    world
        .body(static_body)
        .unwrap()
        .create_circle(&shape_def, &circle)
        .unwrap();
    world
        .body(dynamic_body)
        .unwrap()
        .create_circle(&shape_def, &circle)
        .unwrap();

    assert_eq!(world.counters().unwrap().contact_count, 0);
    let before = world.body(dynamic_body).unwrap().position().unwrap();
    let events = world
        .step(0.0, 1)
        .unwrap()
        .contact_events()
        .unwrap()
        .to_owned()
        .unwrap();

    assert_eq!(
        world.body(dynamic_body).unwrap().position().unwrap(),
        before
    );
    let counters = world.counters().unwrap();
    assert_eq!(counters.contact_count, 1);
    assert_eq!(counters.awake_contact_count, 1);
    assert_eq!(events.begin.len(), 1);
    assert!(events.end.is_empty());
    assert!(events.hit.is_empty());
}

fn make_deterministic_world(worker_count: u32) -> (World, BodyId) {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, -10.0])
                .worker_count(WorkerCount::new(worker_count).unwrap())
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
                .position([0.0_f32, 4.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(body)
        .unwrap()
        .create_centered_circle(&ShapeDef::builder().density(1.0).build().unwrap(), 0.5)
        .unwrap();
    (world, body)
}

#[test]
fn built_in_scheduler_matches_single_worker_for_a_deterministic_scene() {
    let (mut serial, serial_body) = make_deterministic_world(1);
    let (mut parallel, parallel_body) = make_deterministic_world(2);
    for _ in 0..30 {
        drop(serial.step(1.0 / 60.0, 4).unwrap());
        drop(parallel.step(1.0 / 60.0, 4).unwrap());
    }

    assert_eq!(
        serial.body(serial_body).unwrap().position().unwrap(),
        parallel.body(parallel_body).unwrap().position().unwrap()
    );
    assert_eq!(
        serial.counters().unwrap().body_count,
        parallel.counters().unwrap().body_count
    );
}
