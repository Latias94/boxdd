use boxdd::{prelude::*, shapes};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

static FILTER_CALLS: AtomicUsize = AtomicUsize::new(0);
static PRESOLVE_CALLS: AtomicUsize = AtomicUsize::new(0);

struct PanicOnDrop {
    panicked: Arc<AtomicBool>,
}

impl PanicOnDrop {
    fn touch(&self) {}
}

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        if !self.panicked.swap(true, Ordering::SeqCst) {
            panic!("intentional callback drop panic");
        }
    }
}

#[test]
fn custom_filter_closure_disables_contact() {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _g = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    FILTER_CALLS.store(0, Ordering::SeqCst);
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    // Two dynamic boxes stacked so they would normally collide
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_custom_filtering(true)
        .build();

    let a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 2.0])
            .build(),
    );
    let _sa = world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5, 0.5));
    // Start already overlapping to ensure filter is exercised immediately
    let b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 2.4])
            .build(),
    );
    let _sb = world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));

    // Closure: count invocations and disable all collisions
    world.set_custom_filter(|_x, _y| {
        FILTER_CALLS.fetch_add(1, Ordering::SeqCst);
        false
    });

    for _ in 0..10 {
        world.step(1.0 / 60.0, 2);
        let ev = world.contact_events();
        // should have no contacts due to custom filter
        assert!(ev.begin.is_empty() && ev.end.is_empty() && ev.hit.is_empty());
    }

    assert!(FILTER_CALLS.load(Ordering::SeqCst) > 0);
    world.clear_custom_filter();
}

#[test]
fn pre_solve_closure_blocks_contact_this_step() {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _g = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    PRESOLVE_CALLS.store(0, Ordering::SeqCst);
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    // Ground
    let g = world.create_body_id(BodyBuilder::new().position([0.0_f32, 0.0]).build());
    let _gs = world.create_polygon_shape_for(
        g,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(20.0, 0.5),
    );

    // Dynamic body above ground with pre-solve enabled
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_pre_solve_events(true)
        .build();
    let d = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.0])
            .build(),
    );
    let _ds = world.create_polygon_shape_for(d, &sdef, &shapes::box_polygon(0.5, 0.5));

    // Closure: count calls and disable contact for the step
    world.set_pre_solve(|_a, _b, _p, _n| {
        PRESOLVE_CALLS.fetch_add(1, Ordering::SeqCst);
        false
    });

    // Step enough frames to ensure proximity
    for _ in 0..90 {
        world.step(1.0 / 60.0, 2);
    }

    // Even if contact events are suppressed intermittently, the callback should have been invoked
    assert!(PRESOLVE_CALLS.load(Ordering::SeqCst) > 0);
    world.clear_pre_solve();
}

#[test]
fn custom_filter_replacement_survives_old_callback_drop_panic() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_custom_filtering(true)
        .build();
    let polygon = shapes::box_polygon(0.5, 0.5);
    for x in [0.0_f32, 0.4] {
        let body = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([x, 0.0])
                .build(),
        );
        let _ = world.create_polygon_shape_for(body, &shape_def, &polygon);
    }

    let old_dropped = Arc::new(AtomicBool::new(false));
    world.set_custom_filter({
        let marker = PanicOnDrop {
            panicked: Arc::clone(&old_dropped),
        };
        move |_, _| {
            marker.touch();
            true
        }
    });

    let replacement_calls = Arc::new(AtomicUsize::new(0));
    let replacement = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.set_custom_filter({
            let replacement_calls = Arc::clone(&replacement_calls);
            move |_, _| {
                replacement_calls.fetch_add(1, Ordering::SeqCst);
                false
            }
        });
    }));

    assert!(replacement.is_err());
    assert!(old_dropped.load(Ordering::SeqCst));
    for _ in 0..5 {
        world.step(1.0 / 60.0, 2);
    }
    assert!(replacement_calls.load(Ordering::SeqCst) > 0);
    world.clear_custom_filter();
}

#[test]
fn pre_solve_replacement_survives_old_callback_drop_panic() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_polygon_shape_for(
        ground,
        &ShapeDef::default(),
        &shapes::box_polygon(20.0, 0.5),
    );
    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 2.0])
            .build(),
    );
    let _ = world.create_polygon_shape_for(
        body,
        &ShapeDef::builder()
            .density(1.0)
            .enable_pre_solve_events(true)
            .build(),
        &shapes::box_polygon(0.5, 0.5),
    );

    let old_dropped = Arc::new(AtomicBool::new(false));
    world.set_pre_solve({
        let marker = PanicOnDrop {
            panicked: Arc::clone(&old_dropped),
        };
        move |_, _, _, _| {
            marker.touch();
            true
        }
    });

    let replacement_calls = Arc::new(AtomicUsize::new(0));
    let replacement = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.set_pre_solve({
            let replacement_calls = Arc::clone(&replacement_calls);
            move |_, _, _, _| {
                replacement_calls.fetch_add(1, Ordering::SeqCst);
                true
            }
        });
    }));

    assert!(replacement.is_err());
    assert!(old_dropped.load(Ordering::SeqCst));
    for _ in 0..120 {
        world.step(1.0 / 60.0, 2);
        if replacement_calls.load(Ordering::SeqCst) > 0 {
            break;
        }
    }
    assert!(replacement_calls.load(Ordering::SeqCst) > 0);
    world.clear_pre_solve();
}
