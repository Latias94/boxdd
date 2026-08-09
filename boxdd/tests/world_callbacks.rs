use boxdd::{prelude::*, shapes};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct PanicOnDrop {
    panicked: Arc<AtomicUsize>,
}

impl PanicOnDrop {
    fn touch(&self) {}
}

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        if self.panicked.fetch_add(1, Ordering::SeqCst) == 0 {
            panic!("intentional callback drop panic");
        }
    }
}

fn create_box(world: &mut World, position: [f32; 2], shape_def: &ShapeDef) -> BodyId {
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position(position)
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(body)
        .unwrap()
        .create_polygon(shape_def, &shapes::box_polygon(0.5, 0.5).unwrap())
        .unwrap();
    body
}

#[test]
fn custom_filter_can_disable_every_contact() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, -10.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_custom_filtering(true)
        .build()
        .unwrap();
    create_box(&mut world, [0.0, 2.0], &shape_def);
    create_box(&mut world, [0.0, 2.4], &shape_def);

    world
        .set_custom_filter({
            let calls = Arc::clone(&calls);
            move |_, _| {
                calls.fetch_add(1, Ordering::SeqCst);
                false
            }
        })
        .unwrap();

    for _ in 0..10 {
        let events = world
            .step(1.0 / 60.0, 2)
            .unwrap()
            .contact_events()
            .unwrap()
            .to_owned()
            .unwrap();
        assert!(events.begin.is_empty() && events.end.is_empty() && events.hit.is_empty());
    }
    assert!(calls.load(Ordering::SeqCst) > 0);
    world.clear_custom_filter().unwrap();
}

#[test]
fn pre_solve_can_disable_contacts_for_each_step() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, -10.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let ground = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    world
        .body(ground)
        .unwrap()
        .create_box(&ShapeDef::default(), 20.0, 0.5)
        .unwrap();
    create_box(
        &mut world,
        [0.0, 3.0],
        &ShapeDef::builder()
            .density(1.0)
            .enable_contact_events(true)
            .enable_pre_solve_events(true)
            .build()
            .unwrap(),
    );

    world
        .set_pre_solve({
            let calls = Arc::clone(&calls);
            move |_, _, _, _| {
                calls.fetch_add(1, Ordering::SeqCst);
                false
            }
        })
        .unwrap();
    for _ in 0..90 {
        drop(world.step(1.0 / 60.0, 2).unwrap());
    }
    assert!(calls.load(Ordering::SeqCst) > 0);
    world.clear_pre_solve().unwrap();
}

#[test]
fn custom_filter_replacement_survives_old_callback_drop_panic() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity(Vec2::ZERO)
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_custom_filtering(true)
        .build()
        .unwrap();
    create_box(&mut world, [0.0, 0.0], &shape_def);
    create_box(&mut world, [0.4, 0.0], &shape_def);

    let old_dropped = Arc::new(AtomicUsize::new(0));
    world
        .set_custom_filter({
            let marker = PanicOnDrop {
                panicked: Arc::clone(&old_dropped),
            };
            move |_, _| {
                marker.touch();
                true
            }
        })
        .unwrap();

    let replacement_calls = Arc::new(AtomicUsize::new(0));
    let replacement = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world
            .set_custom_filter({
                let replacement_calls = Arc::clone(&replacement_calls);
                move |_, _| {
                    replacement_calls.fetch_add(1, Ordering::SeqCst);
                    false
                }
            })
            .unwrap();
    }));
    assert!(replacement.is_err());
    assert_eq!(old_dropped.load(Ordering::SeqCst), 1);

    for _ in 0..5 {
        drop(world.step(1.0 / 60.0, 2).unwrap());
    }
    assert!(replacement_calls.load(Ordering::SeqCst) > 0);
    world.clear_custom_filter().unwrap();
}

#[test]
fn pre_solve_replacement_survives_old_callback_drop_panic() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, -10.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let ground = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    world
        .body(ground)
        .unwrap()
        .create_box(&ShapeDef::default(), 20.0, 0.5)
        .unwrap();
    create_box(
        &mut world,
        [0.0, 2.0],
        &ShapeDef::builder()
            .density(1.0)
            .enable_pre_solve_events(true)
            .build()
            .unwrap(),
    );

    let old_dropped = Arc::new(AtomicUsize::new(0));
    world
        .set_pre_solve({
            let marker = PanicOnDrop {
                panicked: Arc::clone(&old_dropped),
            };
            move |_, _, _, _| {
                marker.touch();
                true
            }
        })
        .unwrap();

    let replacement_calls = Arc::new(AtomicUsize::new(0));
    let replacement = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world
            .set_pre_solve({
                let replacement_calls = Arc::clone(&replacement_calls);
                move |_, _, _, _| {
                    replacement_calls.fetch_add(1, Ordering::SeqCst);
                    true
                }
            })
            .unwrap();
    }));
    assert!(replacement.is_err());
    assert_eq!(old_dropped.load(Ordering::SeqCst), 1);

    for _ in 0..120 {
        drop(world.step(1.0 / 60.0, 2).unwrap());
        if replacement_calls.load(Ordering::SeqCst) > 0 {
            break;
        }
    }
    assert!(replacement_calls.load(Ordering::SeqCst) > 0);
    world.clear_pre_solve().unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
mod outer_unwind_subprocess {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::process::Command;

    const CHILD_CASE: &str = "BOXDD_CALLBACK_CLEANUP_OUTER_UNWIND_CASE";
    const PRIMARY_PANIC: &str = "callback cleanup outer unwind remains primary";
    const TEST_NAME: &str =
        "outer_unwind_subprocess::callback_cleanup_during_outer_unwind_does_not_abort";
    const CASES: &[&str] = &[
        "custom-filter-replace",
        "custom-filter-clear",
        "pre-solve-replace",
        "pre-solve-clear",
        "custom-filter-rejected",
        "pre-solve-rejected",
        "friction-rejected",
        "restitution-rejected",
    ];

    struct InvokeOnDrop<F: FnOnce()>(Option<F>);

    impl<F: FnOnce()> Drop for InvokeOnDrop<F> {
        fn drop(&mut self) {
            if let Some(invoke) = self.0.take() {
                invoke();
            }
        }
    }

    fn during_outer_unwind(invoke: impl FnOnce()) {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _invoke = InvokeOnDrop(Some(invoke));
            std::panic::panic_any(PRIMARY_PANIC);
        }));
        let payload = result.expect_err("the outer panic must keep unwinding");
        assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
    }

    fn create_world() -> World {
        boxdd::Foundation::initialize_default()
            .unwrap()
            .create_world(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap()
    }

    fn install_panicking_custom_filter(world: &mut World, panicked: &Arc<AtomicUsize>) {
        let marker = PanicOnDrop {
            panicked: Arc::clone(panicked),
        };
        world
            .set_custom_filter(move |_, _| {
                marker.touch();
                true
            })
            .unwrap();
    }

    fn install_panicking_pre_solve(world: &mut World, panicked: &Arc<AtomicUsize>) {
        let marker = PanicOnDrop {
            panicked: Arc::clone(panicked),
        };
        world
            .set_pre_solve(move |_, _, _, _| {
                marker.touch();
                true
            })
            .unwrap();
    }

    fn custom_filter_replace_case() {
        let mut world = create_world();
        let panicked = Arc::new(AtomicUsize::new(0));
        install_panicking_custom_filter(&mut world, &panicked);

        during_outer_unwind(|| world.set_custom_filter(|_, _| true).unwrap());

        assert_eq!(panicked.load(Ordering::SeqCst), 1);
        world.clear_custom_filter().unwrap();
    }

    fn custom_filter_clear_case() {
        let mut world = create_world();
        let panicked = Arc::new(AtomicUsize::new(0));
        install_panicking_custom_filter(&mut world, &panicked);

        during_outer_unwind(|| world.clear_custom_filter().unwrap());

        assert_eq!(panicked.load(Ordering::SeqCst), 1);
        world.clear_custom_filter().unwrap();
    }

    fn pre_solve_replace_case() {
        let mut world = create_world();
        let panicked = Arc::new(AtomicUsize::new(0));
        install_panicking_pre_solve(&mut world, &panicked);

        during_outer_unwind(|| world.set_pre_solve(|_, _, _, _| true).unwrap());

        assert_eq!(panicked.load(Ordering::SeqCst), 1);
        world.clear_pre_solve().unwrap();
    }

    fn pre_solve_clear_case() {
        let mut world = create_world();
        let panicked = Arc::new(AtomicUsize::new(0));
        install_panicking_pre_solve(&mut world, &panicked);

        during_outer_unwind(|| world.clear_pre_solve().unwrap());

        assert_eq!(panicked.load(Ordering::SeqCst), 1);
        world.clear_pre_solve().unwrap();
    }

    fn terminalize(world: &mut World) {
        let snapshot = world.snapshot().unwrap();
        let error = world
            .prepare_restore(&snapshot)
            .unwrap()
            .commit_with(|_| Err(boxdd::Error::SnapshotManifestMismatch))
            .unwrap_err();
        assert_eq!(error, boxdd::Error::SnapshotManifestMismatch);
    }

    fn rejected_custom_filter_case() {
        let mut world = create_world();
        terminalize(&mut world);
        let panicked = Arc::new(AtomicUsize::new(0));
        let rejected = Arc::new(AtomicBool::new(false));
        let observed_panicked = Arc::clone(&panicked);
        let observed_rejected = Arc::clone(&rejected);

        during_outer_unwind(move || {
            let marker = PanicOnDrop {
                panicked: observed_panicked,
            };
            observed_rejected.store(
                matches!(
                    world.set_custom_filter(move |_, _| {
                        marker.touch();
                        true
                    }),
                    Err(boxdd::Error::WorldDestroyed)
                ),
                Ordering::SeqCst,
            );
        });

        assert_eq!(panicked.load(Ordering::SeqCst), 1);
        assert!(rejected.load(Ordering::SeqCst));
    }

    fn rejected_pre_solve_case() {
        let mut world = create_world();
        terminalize(&mut world);
        let panicked = Arc::new(AtomicUsize::new(0));
        let rejected = Arc::new(AtomicBool::new(false));
        let observed_panicked = Arc::clone(&panicked);
        let observed_rejected = Arc::clone(&rejected);

        during_outer_unwind(move || {
            let marker = PanicOnDrop {
                panicked: observed_panicked,
            };
            observed_rejected.store(
                matches!(
                    world.set_pre_solve(move |_, _, _, _| {
                        marker.touch();
                        true
                    }),
                    Err(boxdd::Error::WorldDestroyed)
                ),
                Ordering::SeqCst,
            );
        });

        assert_eq!(panicked.load(Ordering::SeqCst), 1);
        assert!(rejected.load(Ordering::SeqCst));
    }

    fn rejected_friction_case() {
        let mut world = create_world();
        terminalize(&mut world);
        let panicked = Arc::new(AtomicUsize::new(0));
        let rejected = Arc::new(AtomicBool::new(false));
        let observed_panicked = Arc::clone(&panicked);
        let observed_rejected = Arc::clone(&rejected);

        during_outer_unwind(move || {
            let marker = PanicOnDrop {
                panicked: observed_panicked,
            };
            observed_rejected.store(
                matches!(
                    world.set_friction_callback(
                        boxdd::MixerId::from_bytes([0x31; 32]),
                        move |_, _| {
                            marker.touch();
                            0.5
                        }
                    ),
                    Err(boxdd::Error::WorldDestroyed)
                ),
                Ordering::SeqCst,
            );
        });

        assert_eq!(panicked.load(Ordering::SeqCst), 1);
        assert!(rejected.load(Ordering::SeqCst));
    }

    fn rejected_restitution_case() {
        let mut world = create_world();
        terminalize(&mut world);
        let panicked = Arc::new(AtomicUsize::new(0));
        let rejected = Arc::new(AtomicBool::new(false));
        let observed_panicked = Arc::clone(&panicked);
        let observed_rejected = Arc::clone(&rejected);

        during_outer_unwind(move || {
            let marker = PanicOnDrop {
                panicked: observed_panicked,
            };
            observed_rejected.store(
                matches!(
                    world.set_restitution_callback(
                        boxdd::MixerId::from_bytes([0x32; 32]),
                        move |_, _| {
                            marker.touch();
                            0.5
                        }
                    ),
                    Err(boxdd::Error::WorldDestroyed)
                ),
                Ordering::SeqCst,
            );
        });

        assert_eq!(panicked.load(Ordering::SeqCst), 1);
        assert!(rejected.load(Ordering::SeqCst));
    }

    fn run_child(case: &str) {
        match case {
            "custom-filter-replace" => custom_filter_replace_case(),
            "custom-filter-clear" => custom_filter_clear_case(),
            "pre-solve-replace" => pre_solve_replace_case(),
            "pre-solve-clear" => pre_solve_clear_case(),
            "custom-filter-rejected" => rejected_custom_filter_case(),
            "pre-solve-rejected" => rejected_pre_solve_case(),
            "friction-rejected" => rejected_friction_case(),
            "restitution-rejected" => rejected_restitution_case(),
            other => panic!("unknown callback-cleanup outer-unwind child case: {other}"),
        }
        eprintln!("boxdd-callback-cleanup-outer-unwind: completed {case}");
    }

    #[test]
    fn callback_cleanup_during_outer_unwind_does_not_abort() {
        if let Some(case) = std::env::var_os(CHILD_CASE) {
            if case == "all" {
                for case in CASES {
                    run_child(case);
                }
            } else {
                run_child(&case.to_string_lossy());
            }
            return;
        }

        let output =
            Command::new(std::env::current_exe().expect("test executable path must be available"))
                .args(["--exact", TEST_NAME, "--nocapture"])
                .env(CHILD_CASE, "all")
                .output()
                .expect("callback-cleanup outer-unwind child process must start");

        assert!(
            output.status.success(),
            "callback-cleanup outer-unwind child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        for case in CASES {
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!(
                    "boxdd-callback-cleanup-outer-unwind: completed {case}"
                )),
                "callback-cleanup outer-unwind child {case} did not complete its assertion path\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}
