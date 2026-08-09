use boxdd::{prelude::*, shapes};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const MIXER_V1: MixerId = MixerId::from_bytes([0x61; 32]);
const MIXER_V2: MixerId = MixerId::from_bytes([0x62; 32]);

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

fn add_overlapping_material_pair(world: &mut World, x_offset: f32) {
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .material(
            SurfaceMaterial::default()
                .with_friction(0.5)
                .unwrap()
                .with_restitution(0.5)
                .unwrap()
                .with_user_material_id(1),
        )
        .build()
        .unwrap();
    let polygon = shapes::box_polygon(0.5, 0.5).unwrap();
    for x in [0.0_f32, 0.4] {
        let body = world
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .position([x + x_offset, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        world
            .body(body)
            .unwrap()
            .create_polygon(&shape_def, &polygon)
            .unwrap();
    }
}

fn overlapping_material_world() -> World {
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
    add_overlapping_material_pair(&mut world, 0.0);
    world
}

#[test]
fn material_mix_callbacks_receive_material_ids_and_override_restitution() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, -10.0])
                .restitution_threshold(0.0)
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
        .create_box(
            &ShapeDef::builder()
                .material(
                    SurfaceMaterial::default()
                        .with_friction(0.9)
                        .unwrap()
                        .with_user_material_id(11),
                )
                .build()
                .unwrap(),
            20.0,
            0.5,
        )
        .unwrap();
    let ball = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.0_f32, 5.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .body(ball)
        .unwrap()
        .create_centered_circle(
            &ShapeDef::builder()
                .density(1.0)
                .material(
                    SurfaceMaterial::default()
                        .with_friction(0.8)
                        .unwrap()
                        .with_user_material_id(22),
                )
                .build()
                .unwrap(),
            0.5,
        )
        .unwrap();

    let friction_seen = Arc::new(AtomicBool::new(false));
    world
        .set_friction_callback(MIXER_V1, {
            let friction_seen = Arc::clone(&friction_seen);
            move |a, b| {
                let expected = (a.user_material_id == 11 && b.user_material_id == 22)
                    || (a.user_material_id == 22 && b.user_material_id == 11);
                friction_seen.fetch_or(expected, Ordering::SeqCst);
                0.0
            }
        })
        .unwrap();
    let restitution_seen = Arc::new(AtomicBool::new(false));
    world
        .set_restitution_callback(MIXER_V1, {
            let restitution_seen = Arc::clone(&restitution_seen);
            move |a, b| {
                let expected = (a.user_material_id == 11 && b.user_material_id == 22)
                    || (a.user_material_id == 22 && b.user_material_id == 11);
                restitution_seen.fetch_or(expected, Ordering::SeqCst);
                1.0
            }
        })
        .unwrap();

    let mut bounced = false;
    for _ in 0..240 {
        drop(world.step(1.0 / 120.0, 8).unwrap());
        if world.body(ball).unwrap().linear_velocity().unwrap().y > 0.1 {
            bounced = true;
            break;
        }
    }
    assert!(friction_seen.load(Ordering::SeqCst));
    assert!(restitution_seen.load(Ordering::SeqCst));
    assert!(bounced);
}

#[test]
fn callback_panics_and_invalid_results_do_not_prevent_replacement() {
    let mut panicking_world = overlapping_material_world();
    panicking_world
        .set_friction_callback(MIXER_V1, |_, _| -> f32 {
            panic!("boom in friction mix");
        })
        .unwrap();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(panicking_world.step(1.0 / 60.0, 1).unwrap());
    }));
    assert!(panic.is_err());
    panicking_world.clear_friction_callback().unwrap();

    for invalid in [f32::NAN, f32::INFINITY, -0.5] {
        let mut world = overlapping_material_world();
        world
            .set_friction_callback(MIXER_V1, move |_, _| invalid)
            .unwrap();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            drop(world.step(1.0 / 60.0, 2).unwrap());
        }));
        assert!(panic.is_err(), "invalid coefficient {invalid} must panic");
        world.clear_friction_callback().unwrap();

        let replacement_calls = Arc::new(AtomicUsize::new(0));
        world
            .set_friction_callback(MIXER_V2, {
                let replacement_calls = Arc::clone(&replacement_calls);
                move |a, b| {
                    replacement_calls.fetch_add(1, Ordering::SeqCst);
                    (a.coefficient * b.coefficient).sqrt()
                }
            })
            .unwrap();
        add_overlapping_material_pair(&mut world, 4.0);
        drop(world.step(1.0 / 60.0, 2).unwrap());
        assert!(replacement_calls.load(Ordering::SeqCst) > 0);
        world.clear_friction_callback().unwrap();
    }
}

#[test]
fn clearing_material_mix_callbacks_releases_shared_slots_in_either_order() {
    let mut worlds = Vec::new();
    for index in 0..65 {
        let mut world = boxdd::Foundation::initialize_default()
            .unwrap()
            .create_world(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        world
            .set_friction_callback(MIXER_V1, |a, b| a.coefficient.min(b.coefficient))
            .unwrap();
        world
            .set_restitution_callback(MIXER_V1, |a, b| a.coefficient.max(b.coefficient))
            .unwrap();
        if index % 2 == 0 {
            world.clear_friction_callback().unwrap();
            world.clear_restitution_callback().unwrap();
        } else {
            world.clear_restitution_callback().unwrap();
            world.clear_friction_callback().unwrap();
        }
        worlds.push(world);
    }
    assert_eq!(worlds.len(), 65);
}

#[test]
fn friction_replacement_survives_old_callback_drop_panic() {
    let mut world = overlapping_material_world();
    let old_dropped = Arc::new(AtomicUsize::new(0));
    world
        .set_friction_callback(MIXER_V1, {
            let marker = PanicOnDrop {
                panicked: Arc::clone(&old_dropped),
            };
            move |_, _| {
                marker.touch();
                0.5
            }
        })
        .unwrap();
    let replacement_calls = Arc::new(AtomicUsize::new(0));
    let replacement = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world
            .set_friction_callback(MIXER_V2, {
                let replacement_calls = Arc::clone(&replacement_calls);
                move |_, _| {
                    replacement_calls.fetch_add(1, Ordering::SeqCst);
                    0.5
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
    world.clear_friction_callback().unwrap();
}

#[test]
fn restitution_replacement_survives_old_callback_drop_panic() {
    let mut world = overlapping_material_world();
    let old_dropped = Arc::new(AtomicUsize::new(0));
    world
        .set_restitution_callback(MIXER_V1, {
            let marker = PanicOnDrop {
                panicked: Arc::clone(&old_dropped),
            };
            move |_, _| {
                marker.touch();
                0.5
            }
        })
        .unwrap();
    let replacement_calls = Arc::new(AtomicUsize::new(0));
    let replacement = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world
            .set_restitution_callback(MIXER_V2, {
                let replacement_calls = Arc::clone(&replacement_calls);
                move |_, _| {
                    replacement_calls.fetch_add(1, Ordering::SeqCst);
                    0.5
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
    world.clear_restitution_callback().unwrap();
}

#[test]
fn world_drop_runs_both_panicking_mixer_destructors_before_resuming() {
    let friction_dropped = Arc::new(AtomicUsize::new(0));
    let restitution_dropped = Arc::new(AtomicUsize::new(0));
    let mut world = overlapping_material_world();
    world
        .set_friction_callback(MIXER_V1, {
            let marker = PanicOnDrop {
                panicked: Arc::clone(&friction_dropped),
            };
            move |_, _| {
                marker.touch();
                0.5
            }
        })
        .unwrap();
    world
        .set_restitution_callback(MIXER_V1, {
            let marker = PanicOnDrop {
                panicked: Arc::clone(&restitution_dropped),
            };
            move |_, _| {
                marker.touch();
                0.5
            }
        })
        .unwrap();

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(world)));

    assert!(panic.is_err());
    assert_eq!(friction_dropped.load(Ordering::SeqCst), 1);
    assert_eq!(restitution_dropped.load(Ordering::SeqCst), 1);

    let mut replacement = overlapping_material_world();
    replacement
        .set_friction_callback(MIXER_V2, |a, b| (a.coefficient * b.coefficient).sqrt())
        .unwrap();
    replacement.clear_friction_callback().unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
mod outer_unwind_subprocess {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::process::Command;

    const CHILD_CASE: &str = "BOXDD_MATERIAL_MIX_CLEANUP_OUTER_UNWIND_CASE";
    const PRIMARY_PANIC: &str = "material mix cleanup outer unwind remains primary";
    const TEST_NAME: &str =
        "outer_unwind_subprocess::material_mix_cleanup_during_outer_unwind_does_not_abort";
    const CASES: &[&str] = &["replace", "clear", "world-teardown"];

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

    fn install_panicking_callbacks(
        world: &mut World,
        friction_panicked: &Arc<AtomicUsize>,
        restitution_panicked: &Arc<AtomicUsize>,
    ) {
        let friction_marker = PanicOnDrop {
            panicked: Arc::clone(friction_panicked),
        };
        world
            .set_friction_callback(MIXER_V1, move |_, _| {
                friction_marker.touch();
                0.5
            })
            .unwrap();

        let restitution_marker = PanicOnDrop {
            panicked: Arc::clone(restitution_panicked),
        };
        world
            .set_restitution_callback(MIXER_V1, move |_, _| {
                restitution_marker.touch();
                0.5
            })
            .unwrap();
    }

    fn assert_both_panicked(friction: &Arc<AtomicUsize>, restitution: &Arc<AtomicUsize>) {
        assert_eq!(friction.load(Ordering::SeqCst), 1);
        assert_eq!(restitution.load(Ordering::SeqCst), 1);
    }

    fn replace_case() {
        let friction_panicked = Arc::new(AtomicUsize::new(0));
        let restitution_panicked = Arc::new(AtomicUsize::new(0));
        let mut world = overlapping_material_world();
        install_panicking_callbacks(&mut world, &friction_panicked, &restitution_panicked);

        during_outer_unwind(|| {
            world.set_friction_callback(MIXER_V2, |_, _| 0.5).unwrap();
            world
                .set_restitution_callback(MIXER_V2, |_, _| 0.5)
                .unwrap();
        });

        assert_both_panicked(&friction_panicked, &restitution_panicked);
        world.clear_friction_callback().unwrap();
        world.clear_restitution_callback().unwrap();
    }

    fn clear_case() {
        let friction_panicked = Arc::new(AtomicUsize::new(0));
        let restitution_panicked = Arc::new(AtomicUsize::new(0));
        let mut world = overlapping_material_world();
        install_panicking_callbacks(&mut world, &friction_panicked, &restitution_panicked);

        during_outer_unwind(|| {
            world.clear_friction_callback().unwrap();
            world.clear_restitution_callback().unwrap();
        });

        assert_both_panicked(&friction_panicked, &restitution_panicked);
        world.clear_friction_callback().unwrap();
        world.clear_restitution_callback().unwrap();
    }

    fn world_teardown_case() {
        let friction_panicked = Arc::new(AtomicUsize::new(0));
        let restitution_panicked = Arc::new(AtomicUsize::new(0));
        let mut world = overlapping_material_world();
        install_panicking_callbacks(&mut world, &friction_panicked, &restitution_panicked);

        during_outer_unwind(move || drop(world));

        assert_both_panicked(&friction_panicked, &restitution_panicked);
    }

    fn run_child(case: &str) {
        match case {
            "replace" => replace_case(),
            "clear" => clear_case(),
            "world-teardown" => world_teardown_case(),
            other => panic!("unknown material-mix cleanup outer-unwind child case: {other}"),
        }
        eprintln!("boxdd-material-mix-cleanup-outer-unwind: completed {case}");
    }

    fn install_child_panic_hook() {
        std::panic::set_hook(Box::new(|info| {
            let message = info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
                .unwrap_or("non-string panic payload");
            if let Some(location) = info.location() {
                eprintln!(
                    "boxdd-material-mix-cleanup panic at {}:{}:{}: {message}",
                    location.file(),
                    location.line(),
                    location.column()
                );
            } else {
                eprintln!("boxdd-material-mix-cleanup panic: {message}");
            }
        }));
    }

    #[test]
    fn material_mix_cleanup_during_outer_unwind_does_not_abort() {
        if let Some(case) = std::env::var_os(CHILD_CASE) {
            install_child_panic_hook();
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
                .expect("material-mix cleanup outer-unwind child process must start");

        assert!(
            output.status.success(),
            "material-mix cleanup outer-unwind child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        for case in CASES {
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!(
                    "boxdd-material-mix-cleanup-outer-unwind: completed {case}"
                )),
                "material-mix cleanup outer-unwind child {case} did not complete its assertion path\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}
