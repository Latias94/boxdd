use boxdd::prelude::*;
use std::env;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::Command;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};

const MULTI_PANIC_CHILD: &str = "BOXDD_MULTI_USER_DATA_PANIC_CHILD";

#[test]
fn concurrent_worlds_hold_counted_foundation_access() {
    const WORLD_COUNT: usize = 24;
    let ready = Arc::new(Barrier::new(WORLD_COUNT + 1));
    let release = Arc::new(Barrier::new(WORLD_COUNT + 1));
    let mut threads = Vec::new();

    for _ in 0..WORLD_COUNT {
        let ready = Arc::clone(&ready);
        let release = Arc::clone(&release);
        threads.push(std::thread::spawn(move || {
            let world = boxdd::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    boxdd::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap();
            ready.wait();
            release.wait();
            drop(world);
        }));
    }
    ready.wait();
    assert_eq!(
        Foundation::initialize_default()
            .unwrap()
            .activity()
            .ordinary_worlds,
        WORLD_COUNT as u32
    );
    assert_eq!(
        Foundation::initialize_default()
            .unwrap()
            .activity()
            .transient_calls,
        0
    );
    assert!(
        !Foundation::initialize_default()
            .unwrap()
            .activity()
            .replay_active
    );

    release.wait();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(
        Foundation::initialize_default()
            .unwrap()
            .activity()
            .ordinary_worlds,
        0
    );

    struct PanicOnDrop;
    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("intentional world user-data drop panic");
        }
    }

    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    world.set_user_data(PanicOnDrop).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| drop(world))).is_err());
    assert_eq!(
        Foundation::initialize_default()
            .unwrap()
            .activity()
            .ordinary_worlds,
        0
    );

    struct NamedPanicOnDrop(&'static str);
    impl Drop for NamedPanicOnDrop {
        fn drop(&mut self) {
            std::panic::panic_any(self.0);
        }
    }

    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let filter_drop = NamedPanicOnDrop("custom filter drop");
    world
        .set_custom_filter(move |_, _| {
            let _keep_capture_alive = &filter_drop;
            true
        })
        .unwrap();
    let pre_solve_drop = NamedPanicOnDrop("pre-solve drop");
    world
        .set_pre_solve(move |_, _, _, _| {
            let _keep_capture_alive = &pre_solve_drop;
            true
        })
        .unwrap();
    world
        .set_user_data(NamedPanicOnDrop("user data drop"))
        .unwrap();

    let result = catch_unwind(AssertUnwindSafe(|| drop(world)));
    let payload = result.expect_err("the first cleanup panic must resume after teardown");
    assert_eq!(
        payload.downcast_ref::<&'static str>(),
        Some(&"custom filter drop")
    );
    assert_eq!(
        Foundation::initialize_default()
            .unwrap()
            .activity()
            .ordinary_worlds,
        0
    );
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
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape_id = world
        .body(body_id)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    world
        .set_user_data(CountedDrop {
            count: Arc::clone(&drops),
            panic_message: Some("first user-data panic"),
        })
        .unwrap();
    world
        .body(body_id)
        .unwrap()
        .set_user_data(CountedDrop {
            count: Arc::clone(&drops),
            panic_message: Some("second user-data panic"),
        })
        .unwrap();
    world
        .shape(shape_id)
        .unwrap()
        .set_user_data(CountedDrop {
            count: Arc::clone(&drops),
            panic_message: None,
        })
        .unwrap();

    let result = catch_unwind(AssertUnwindSafe(|| drop(world)));
    let payload = result.expect_err("the first user-data panic must resume after cleanup");
    assert_eq!(
        payload.downcast_ref::<&'static str>(),
        Some(&"first user-data panic")
    );
    assert_eq!(drops.load(Ordering::SeqCst), 3);
    assert_eq!(
        Foundation::initialize_default()
            .unwrap()
            .activity()
            .ordinary_worlds,
        0
    );

    let drops = Arc::new(AtomicUsize::new(0));
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let (first_shape, second_shape) = {
        let mut body = world.body(body_id).unwrap();
        body.set_user_data(CountedDrop {
            count: Arc::clone(&drops),
            panic_message: Some("first object user-data panic"),
        })
        .unwrap();
        let first = body
            .create_centered_circle(&ShapeDef::default(), 0.5)
            .unwrap();
        let second = body
            .create_centered_circle(&ShapeDef::default(), 0.5)
            .unwrap();
        (first, second)
    };
    world
        .shape(first_shape)
        .unwrap()
        .set_user_data(CountedDrop {
            count: Arc::clone(&drops),
            panic_message: Some("second object user-data panic"),
        })
        .unwrap();
    world
        .shape(second_shape)
        .unwrap()
        .set_user_data(CountedDrop {
            count: Arc::clone(&drops),
            panic_message: None,
        })
        .unwrap();

    let result = catch_unwind(AssertUnwindSafe(|| world.body(body_id).unwrap().destroy()));
    let payload = result.expect_err("the first object user-data panic must resume after cleanup");
    assert_eq!(
        payload.downcast_ref::<&'static str>(),
        Some(&"first object user-data panic")
    );
    assert_eq!(drops.load(Ordering::SeqCst), 3);
    assert_eq!(world.body(body_id).err().unwrap(), Error::InvalidBodyId);
    assert_eq!(
        Foundation::initialize_default()
            .unwrap()
            .activity()
            .ordinary_worlds,
        1
    );

    drop(world);
    assert_eq!(
        Foundation::initialize_default()
            .unwrap()
            .activity()
            .ordinary_worlds,
        0
    );
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
