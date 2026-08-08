use boxdd::{prelude::*, shapes};
use std::sync::{Mutex, OnceLock};

static WORLD_RECYCLE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn destroyed_body_slots_can_be_reused_without_corrupting_the_world() {
    let _guard = WORLD_RECYCLE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap();
    const BODY_COUNT: usize = 10;
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let shape_def = ShapeDef::builder().density(1.0).build().unwrap();
    let polygon = shapes::box_polygon(0.5, 0.5).unwrap();

    let mut ids = Vec::with_capacity(BODY_COUNT);
    for _ in 0..BODY_COUNT {
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
            .create_polygon(&shape_def, &polygon)
            .unwrap();
        ids.push(body);
        drop(world.step(1.0 / 60.0, 3).unwrap());
    }

    for body in ids.drain(..) {
        world.body(body).unwrap().destroy().unwrap();
        drop(world.step(1.0 / 60.0, 3).unwrap());
    }
    assert_eq!(world.counters().unwrap().body_count, 0);

    for _ in 0..3 {
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
            .create_box(&shape_def, 0.25, 0.25)
            .unwrap();
    }
    drop(world.step(1.0 / 60.0, 1).unwrap());
    assert_eq!(world.counters().unwrap().body_count, 3);
}

#[test]
fn repeatedly_recycling_native_world_slots_is_stable() {
    let _guard = WORLD_RECYCLE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap();
    const WORLD_BATCH: usize = 8;
    const ROUNDS: usize = 5;

    for _ in 0..ROUNDS {
        let mut worlds = Vec::with_capacity(WORLD_BATCH);
        for _ in 0..WORLD_BATCH {
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
            world
                .body(body)
                .unwrap()
                .create_box(&ShapeDef::default(), 0.1, 0.1)
                .unwrap();
            worlds.push(world);
        }
        for world in &mut worlds {
            for _ in 0..10 {
                drop(world.step(1.0 / 60.0, 1).unwrap());
            }
        }
    }
}
