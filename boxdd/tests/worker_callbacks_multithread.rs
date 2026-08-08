use boxdd::prelude::*;
use boxdd::shapes;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

struct WorkerPanicPayload {
    ordinal: usize,
    drops: Arc<AtomicUsize>,
}

impl Drop for WorkerPanicPayload {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

fn multithreaded_contact_world() -> World {
    let foundation = boxdd::Foundation::initialize_default().unwrap();
    let definition = foundation
        .world_builder()
        .gravity([0.0_f32, 0.0])
        .worker_count(WorkerCount::new(4).unwrap())
        .build()
        .unwrap();
    let mut world = foundation
        .create_world(definition)
        .expect("multithreaded world should be created");

    // The collision update pass uses a 64-contact minimum range. 256 isolated overlapping pairs
    // therefore produce at least four native tasks with worker_count=4.
    let static_shape = ShapeDef::builder()
        .density(0.0)
        .enable_pre_solve_events(true)
        .build()
        .unwrap();
    let dynamic_shape = ShapeDef::builder()
        .density(1.0)
        .enable_pre_solve_events(true)
        .build()
        .unwrap();
    let polygon = shapes::box_polygon(0.45, 0.45).expect("valid polygon geometry");
    for index in 0..256 {
        let x = index as f32 * 2.0;
        let static_body = world
            .create_body(
                foundation
                    .body_builder()
                    .position([x, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let dynamic_body = world
            .create_body(
                foundation
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .position([x, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        world
            .body(static_body)
            .unwrap()
            .create_polygon(&static_shape, &polygon)
            .unwrap();
        world
            .body(dynamic_body)
            .unwrap()
            .create_polygon(&dynamic_shape, &polygon)
            .unwrap();
    }

    // Establish the contacts before installing the panicking callback. The next step then enters
    // the parallel contact-update path deterministically instead of depending on pair creation.
    drop(world.step(1.0 / 60.0, 1).unwrap());
    world
}

fn assert_worker_callback_panic_returns_at_owner_boundary() {
    let mut world = multithreaded_contact_world();
    let entered = Arc::new(AtomicUsize::new(0));
    let payload_drops = Arc::new(AtomicUsize::new(0));
    let callback_threads = Arc::new(Mutex::new(HashSet::new()));
    world
        .set_pre_solve({
            let entered = Arc::clone(&entered);
            let payload_drops = Arc::clone(&payload_drops);
            let callback_threads = Arc::clone(&callback_threads);
            move |_, _, _, _| {
                let ordinal = entered.fetch_add(1, Ordering::SeqCst) + 1;
                callback_threads
                    .lock()
                    .expect("callback thread set mutex")
                    .insert(thread::current().id());

                // Let other native tasks cross the C -> Rust boundary before the first callback
                // unwinds. The timeout keeps the test bounded if a platform schedules only one task.
                let deadline = Instant::now() + Duration::from_millis(250);
                while entered.load(Ordering::SeqCst) < 4 && Instant::now() < deadline {
                    thread::yield_now();
                }
                std::panic::panic_any(WorkerPanicPayload {
                    ordinal,
                    drops: Arc::clone(&payload_drops),
                });
            }
        })
        .unwrap();

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(world.step(1.0 / 60.0, 1).unwrap());
    }))
    .expect_err("worker callback panic must resume at the owner boundary");
    let primary = panic
        .downcast_ref::<WorkerPanicPayload>()
        .expect("the primary worker payload must reach the owner boundary");
    assert!(primary.ordinal > 0);

    let callback_thread_count = callback_threads
        .lock()
        .expect("callback thread set mutex")
        .len();
    assert!(
        entered.load(Ordering::SeqCst) >= 2,
        "expected competing worker callbacks, got {}",
        entered.load(Ordering::SeqCst)
    );
    assert!(
        callback_thread_count >= 2,
        "expected callbacks on multiple native workers, got {callback_thread_count} thread(s)"
    );
    let entered = entered.load(Ordering::SeqCst);
    assert_eq!(payload_drops.load(Ordering::SeqCst), entered - 1);
    drop(panic);
    assert_eq!(payload_drops.load(Ordering::SeqCst), entered);

    // Every worker panic is consumed by `step`; no panic state or task handle may poison the world.
    // Clearing the callback and stepping again verifies normal teardown remains possible.
    world.clear_pre_solve().unwrap();
    drop(
        world
            .step(1.0 / 60.0, 1)
            .expect("world should remain usable after worker callback panic"),
    );
}

#[test]
fn built_in_scheduler_worker_callback_panic_returns_at_owner_boundary() {
    assert_worker_callback_panic_returns_at_owner_boundary();
}
