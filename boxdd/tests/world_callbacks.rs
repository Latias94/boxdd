use boxdd::{prelude::*, shapes};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static FILTER_CALLS: AtomicUsize = AtomicUsize::new(0);
static PRESOLVE_CALLS: AtomicUsize = AtomicUsize::new(0);

#[test]
#[ignore]
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
#[ignore]
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
