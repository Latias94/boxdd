use boxdd::prelude::*;
use std::collections::BTreeSet;
use std::env;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::Command;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};

const MULTI_PANIC_CHILD: &str = "BOXDD_MULTI_USER_DATA_PANIC_CHILD";

#[test]
fn concurrent_worlds_hold_counted_foundation_access_and_unique_native_slots() {
    const WORLD_COUNT: usize = 24;
    let ready = Arc::new(Barrier::new(WORLD_COUNT + 1));
    let release = Arc::new(Barrier::new(WORLD_COUNT + 1));
    let (ids_tx, ids_rx) = mpsc::channel();
    let mut threads = Vec::new();

    for _ in 0..WORLD_COUNT {
        let ready = Arc::clone(&ready);
        let release = Arc::clone(&release);
        let ids_tx = ids_tx.clone();
        threads.push(std::thread::spawn(move || {
            let world = World::new(WorldDef::default()).unwrap();
            let raw = world.world_id_raw();
            ids_tx.send((raw.index1, raw.generation)).unwrap();
            ready.wait();
            release.wait();
            drop(world);
        }));
    }
    drop(ids_tx);

    ready.wait();
    let ids = (0..WORLD_COUNT)
        .map(|_| ids_rx.recv().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), WORLD_COUNT);
    assert_eq!(foundation().activity().ordinary_worlds, WORLD_COUNT as u32);
    assert_eq!(foundation().activity().transient_calls, 0);
    assert!(!foundation().activity().replay_active);

    release.wait();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(foundation().activity().ordinary_worlds, 0);

    struct PanicOnDrop;
    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("intentional world user-data drop panic");
        }
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let raw = world.world_id_raw();
    world.set_user_data(PanicOnDrop);
    assert!(catch_unwind(AssertUnwindSafe(|| drop(world))).is_err());
    assert!(!unsafe { boxdd_sys::ffi::b2World_IsValid(raw) });
    assert_eq!(foundation().activity().ordinary_worlds, 0);

    struct NamedPanicOnDrop(&'static str);
    impl Drop for NamedPanicOnDrop {
        fn drop(&mut self) {
            std::panic::panic_any(self.0);
        }
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let raw = world.world_id_raw();
    let filter_drop = NamedPanicOnDrop("custom filter drop");
    world.set_custom_filter(move |_, _| {
        let _keep_capture_alive = &filter_drop;
        true
    });
    let pre_solve_drop = NamedPanicOnDrop("pre-solve drop");
    world.set_pre_solve(move |_, _, _, _| {
        let _keep_capture_alive = &pre_solve_drop;
        true
    });
    world.set_user_data(NamedPanicOnDrop("user data drop"));

    let result = catch_unwind(AssertUnwindSafe(|| drop(world)));
    let payload = result.expect_err("the first cleanup panic must resume after teardown");
    assert_eq!(
        payload.downcast_ref::<&'static str>(),
        Some(&"custom filter drop")
    );
    assert!(!unsafe { boxdd_sys::ffi::b2World_IsValid(raw) });
    assert_eq!(foundation().activity().ordinary_worlds, 0);
}

struct CountedDrop {
    count: Arc<AtomicUsize>,
    panic_message: Option<&'static str>,
}

impl Drop for CountedDrop {
    fn drop(&mut self) {
        self.count.fetch_add(1, Ordering::SeqCst);
        if let Some(message) = self.panic_message {
            std::panic::panic_any(message);
        }
    }
}

fn run_multi_user_data_panic_child() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new(WorldDef::default()).unwrap();
    let raw = world.world_id_raw();
    let mut body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let mut shape = world.create_circle_shape_for_owned(
        body.id(),
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );
    world.set_user_data(CountedDrop {
        count: Arc::clone(&drops),
        panic_message: Some("first user-data panic"),
    });
    body.set_user_data(CountedDrop {
        count: Arc::clone(&drops),
        panic_message: Some("second user-data panic"),
    });
    shape.set_user_data(CountedDrop {
        count: Arc::clone(&drops),
        panic_message: None,
    });

    let result = catch_unwind(AssertUnwindSafe(|| drop(world)));
    let payload = result.expect_err("the first user-data panic must resume after cleanup");
    assert_eq!(
        payload.downcast_ref::<&'static str>(),
        Some(&"first user-data panic")
    );
    assert_eq!(drops.load(Ordering::SeqCst), 3);
    assert!(!unsafe { boxdd_sys::ffi::b2World_IsValid(raw) });
    assert_eq!(foundation().activity().ordinary_worlds, 0);

    drop(shape);
    drop(body);

    let drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_id = body.id();
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let mut first_shape =
        world.create_circle_shape_for_owned(body_id, &ShapeDef::default(), &circle);
    let mut second_shape =
        world.create_circle_shape_for_owned(body_id, &ShapeDef::default(), &circle);
    body.set_user_data(CountedDrop {
        count: Arc::clone(&drops),
        panic_message: Some("first object user-data panic"),
    });
    first_shape.set_user_data(CountedDrop {
        count: Arc::clone(&drops),
        panic_message: Some("second object user-data panic"),
    });
    second_shape.set_user_data(CountedDrop {
        count: Arc::clone(&drops),
        panic_message: None,
    });

    let result = catch_unwind(AssertUnwindSafe(|| body.destroy()));
    let payload = result.expect_err("the first object user-data panic must resume after cleanup");
    assert_eq!(
        payload.downcast_ref::<&'static str>(),
        Some(&"first object user-data panic")
    );
    assert_eq!(drops.load(Ordering::SeqCst), 3);
    assert_eq!(
        world.try_body_position(body_id),
        Err(ApiError::InvalidBodyId)
    );
    assert_eq!(foundation().activity().ordinary_worlds, 1);

    drop(first_shape);
    drop(second_shape);
    drop(world);
    assert_eq!(foundation().activity().ordinary_worlds, 0);
}

#[test]
fn world_teardown_isolates_multiple_panicking_user_data_destructors() {
    if env::var_os(MULTI_PANIC_CHILD).is_some() {
        run_multi_user_data_panic_child();
        return;
    }

    let output = Command::new(env::current_exe().expect("test executable path must be available"))
        .arg("--exact")
        .arg("world_teardown_isolates_multiple_panicking_user_data_destructors")
        .arg("--nocapture")
        .env(MULTI_PANIC_CHILD, "1")
        .output()
        .expect("multi-panic child process must start");

    assert!(
        output.status.success(),
        "multi-panic child aborted\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
