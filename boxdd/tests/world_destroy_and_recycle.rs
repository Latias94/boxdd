use boxdd::{prelude::*, shapes};
use std::sync::{Mutex, OnceLock};

#[test]
#[ignore]
fn create_then_destroy_all_bodies_and_recycle() {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _g = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    const BODY_COUNT: usize = 10;
    let mut world = World::new(WorldDef::builder().build()).unwrap();

    let mut ids: Vec<BodyId> = Vec::with_capacity(BODY_COUNT);
    let sdef = ShapeDef::builder().density(1.0).build();

    let mut creating = true;
    for _ in 0..(2 * BODY_COUNT + 10) {
        if creating {
            if ids.len() < BODY_COUNT {
                let b =
                    world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
                let _s = world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));
                ids.push(b);
            } else {
                creating = false;
            }
        } else if let Some(b) = ids.pop() {
            world.destroy_body_id(b);
        }

        world.step(1.0 / 60.0, 3);
    }

    let c = world.counters();
    assert_eq!(c.body_count, 0);

    // Recreate a few bodies to ensure world remains usable after destruction
    for _ in 0..3 {
        let b = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let _s = world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.25, 0.25));
    }
    world.step(1.0 / 60.0, 1);
    let c2 = world.counters();
    assert!(c2.body_count >= 3);
}

#[test]
#[ignore]
fn recycle_many_worlds_smoke() {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _g = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    // Create and drop multiple worlds repeatedly with a trivial body and a few steps
    const WORLD_BATCH: usize = 8;
    const ROUNDS: usize = 5;

    for _ in 0..ROUNDS {
        let mut worlds: Vec<World> = Vec::with_capacity(WORLD_BATCH);
        for _ in 0..WORLD_BATCH {
            let mut w = World::new(WorldDef::builder().build()).unwrap();
            let b = w.create_body_id(BodyBuilder::new().build());
            let _s = w.create_polygon_shape_for(
                b,
                &ShapeDef::builder().density(0.0).build(),
                &shapes::box_polygon(0.1, 0.1),
            );
            worlds.push(w);
        }
        for w in &mut worlds {
            for _ in 0..10 {
                w.step(1.0 / 60.0, 1);
            }
        }
        drop(worlds);
    }
}
