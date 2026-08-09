use boxdd::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

fn create_polygon(world: &mut World, body: BodyId, def: &ShapeDef, polygon: &shapes::Polygon) {
    world
        .body(body)
        .unwrap()
        .create_polygon(def, polygon)
        .unwrap();
}

#[test]
fn custom_filter_panic_is_caught_and_resumed_after_step() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .set_custom_filter(|_, _| -> bool {
            panic!("boom in custom filter");
        })
        .unwrap();

    let a = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let b = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_custom_filtering(true)
        .build()
        .unwrap();
    let poly = shapes::box_polygon(0.5, 0.5).unwrap();
    create_polygon(&mut world, a, &sdef, &poly);
    create_polygon(&mut world, b, &sdef, &poly);

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = world.step(1.0 / 60.0, 1);
    }));
    assert!(r.is_err());

    world.clear_custom_filter().unwrap();
    drop(world.step(1.0 / 60.0, 1).unwrap());
    assert!(world.body(a).unwrap().position().is_ok());
    assert!(world.body(b).unwrap().position().is_ok());
}

#[test]
fn pre_solve_panic_is_caught_and_resumed_after_step() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world
        .set_pre_solve(|_, _, _, _| -> bool {
            panic!("boom in pre-solve");
        })
        .unwrap();

    let a = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let b = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_pre_solve_events(true)
        .build()
        .unwrap();
    let poly = shapes::box_polygon(0.5, 0.5).unwrap();
    create_polygon(&mut world, a, &sdef, &poly);
    create_polygon(&mut world, b, &sdef, &poly);

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = world.step(1.0 / 60.0, 1);
    }));
    assert!(r.is_err());

    world.clear_pre_solve().unwrap();
    let replacement_calls = Arc::new(AtomicUsize::new(0));
    world
        .set_pre_solve({
            let replacement_calls = Arc::clone(&replacement_calls);
            move |_, _, _, _| {
                replacement_calls.fetch_add(1, Ordering::SeqCst);
                true
            }
        })
        .unwrap();
    for _ in 0..5 {
        drop(world.step(1.0 / 60.0, 1).unwrap());
    }
    assert!(replacement_calls.load(Ordering::SeqCst) > 0);
    assert!(world.body(a).unwrap().position().is_ok());
    assert!(world.body(b).unwrap().position().is_ok());
}

#[test]
fn debug_draw_panic_is_caught_and_resumed() {
    struct Panicker;
    impl DebugDraw for Panicker {
        fn draw_solid_polygon(
            &mut self,
            _transform: boxdd::WorldTransform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: HexColor,
        ) {
            panic!("boom in debug draw");
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
    let sdef = ShapeDef::builder().density(1.0).build().unwrap();
    let poly = shapes::box_polygon(0.5, 0.5).unwrap();
    create_polygon(&mut world, body, &sdef, &poly);
    let mut drawer = Panicker;
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world
            .debug_draw(&mut drawer, DebugDrawOptions::default())
            .unwrap();
    }));
    assert!(r.is_err());
}

#[test]
fn draw_bounds_panic_flushes_world_teardown_before_resuming() {
    struct BoundsPanicker {
        world: Option<World>,
    }

    impl DebugDraw for BoundsPanicker {
        fn draw_bounds(&mut self, _bounds: Aabb, _color: HexColor) {
            drop(self.world.take());
            panic!("boom in draw bounds");
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
    create_polygon(
        &mut world,
        body_id,
        &ShapeDef::builder().density(1.0).build().unwrap(),
        &shapes::box_polygon(0.5, 0.5).unwrap(),
    );
    let doomed_world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let worlds_before = Foundation::initialize_default()
        .unwrap()
        .activity()
        .ordinary_worlds;
    let mut drawer = BoundsPanicker {
        world: Some(doomed_world),
    };
    let options = DebugDrawOptions {
        draw_bounds: true,
        ..DebugDrawOptions::default()
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.debug_draw(&mut drawer, options).unwrap();
    }));

    assert!(result.is_err());
    assert!(drawer.world.is_none());
    assert_eq!(
        Foundation::initialize_default()
            .unwrap()
            .activity()
            .ordinary_worlds,
        worlds_before - 1
    );
    world
        .debug_draw(&mut drawer, DebugDrawOptions::default())
        .unwrap();
}

#[test]
fn debug_draw_reentrant_boxdd_call_panics() {
    struct Reenter {
        world: World,
        body: BodyId,
    }
    impl DebugDraw for Reenter {
        fn draw_solid_polygon(
            &mut self,
            _transform: boxdd::WorldTransform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: HexColor,
        ) {
            self.world.body(self.body).unwrap().position().unwrap();
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
    let sdef = ShapeDef::builder().density(1.0).build().unwrap();
    let poly = shapes::box_polygon(0.5, 0.5).unwrap();
    create_polygon(&mut world, body_id, &sdef, &poly);

    let mut callback_world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let callback_body = callback_world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut drawer = Reenter {
        world: callback_world,
        body: callback_body,
    };
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world
            .debug_draw(&mut drawer, DebugDrawOptions::default())
            .unwrap();
    }));
    assert!(r.is_err());
}

#[test]
fn debug_draw_reentrant_try_boxdd_call_returns_in_callback() {
    struct ReenterTry {
        world: World,
        body: BodyId,
        observed: Option<Error>,
    }

    impl DebugDraw for ReenterTry {
        fn draw_solid_polygon(
            &mut self,
            _transform: boxdd::WorldTransform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: HexColor,
        ) {
            self.observed = self.world.body(self.body).err();
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
    let sdef = ShapeDef::builder().density(1.0).build().unwrap();
    let poly = shapes::box_polygon(0.5, 0.5).unwrap();
    create_polygon(&mut world, body_id, &sdef, &poly);

    let mut callback_world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let callback_body = callback_world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut drawer = ReenterTry {
        world: callback_world,
        body: callback_body,
        observed: None,
    };
    world
        .debug_draw(&mut drawer, DebugDrawOptions::default())
        .unwrap();

    assert_eq!(drawer.observed, Some(Error::InCallback));
    assert!(drawer.world.body(drawer.body).unwrap().position().is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
mod outer_unwind_subprocess {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::process::Command;

    const CHILD_CASE: &str = "BOXDD_OUTER_UNWIND_CALLBACK_CASE";
    const PRIMARY_PANIC: &str = "outer unwind remains primary";
    const TEST_NAME: &str =
        "outer_unwind_subprocess::callback_panics_during_outer_unwind_do_not_abort";
    const CASES: &[&str] = &[
        "dynamic-tree",
        "query",
        "query-rejected",
        "world-step",
        "debug-draw",
        "replay-step",
        "replay-draw",
        "snapshot-restore",
    ];

    struct InvokeOnDrop(Option<Box<dyn FnOnce()>>);

    impl Drop for InvokeOnDrop {
        fn drop(&mut self) {
            if let Some(invoke) = self.0.take() {
                invoke();
            }
        }
    }

    fn during_outer_unwind(invoke: impl FnOnce() + 'static) {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _invoke = InvokeOnDrop(Some(Box::new(invoke)));
            std::panic::panic_any(PRIMARY_PANIC);
        }));
        let payload = result.expect_err("the outer panic must keep unwinding");
        assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
    }

    fn assert_callback_entered(entered: &AtomicUsize) {
        assert_eq!(
            entered.load(Ordering::SeqCst),
            1,
            "the child did not reach its native callback"
        );
    }

    fn dynamic_tree_case() {
        boxdd::Foundation::initialize_default().unwrap();
        let entered = Arc::new(AtomicUsize::new(0));
        let callback_entered = Arc::clone(&entered);
        let mut tree = DynamicTree::new().unwrap();
        tree.create_proxy(
            Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap(),
            u64::MAX,
            7,
        )
        .unwrap();

        during_outer_unwind(move || {
            let _ = tree.query_all(
                Aabb::new([-2.0_f32, -2.0], [2.0_f32, 2.0]).unwrap(),
                &mut |_, _| -> bool {
                    callback_entered.fetch_add(1, Ordering::SeqCst);
                    panic!("secondary dynamic-tree callback panic");
                },
            );
        });
        assert_callback_entered(&entered);
    }

    fn query_case() {
        let entered = Arc::new(AtomicUsize::new(0));
        let callback_entered = Arc::clone(&entered);
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
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        world
            .body(body)
            .unwrap()
            .create_circle(
                &ShapeDef::default(),
                &shapes::circle([0.0_f32, 0.0], 0.5).unwrap(),
            )
            .unwrap();

        during_outer_unwind(move || {
            let _ = world.query().unwrap().visit_overlap_aabb(
                Position::ZERO,
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap(),
                QueryFilter::default(),
                |_| -> bool {
                    callback_entered.fetch_add(1, Ordering::SeqCst);
                    panic!("secondary world-query callback panic");
                },
            );
        });
        assert_callback_entered(&entered);
    }

    fn rejected_query_visitor_case() {
        struct PanicOnDrop(Arc<AtomicUsize>);

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
                panic!("secondary rejected-query visitor destructor panic");
            }
        }

        let dropped = Arc::new(AtomicUsize::new(0));
        let rejected = Arc::new(AtomicUsize::new(0));
        let observed_dropped = Arc::clone(&dropped);
        let observed_rejected = Arc::clone(&rejected);
        let world = boxdd::Foundation::initialize_default()
            .unwrap()
            .create_world(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();

        during_outer_unwind(move || {
            let marker = PanicOnDrop(observed_dropped);
            let result = world.query().unwrap().visit_overlap_aabb(
                Position::new(WorldScalar::NAN, 0.0),
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]).unwrap(),
                QueryFilter::default(),
                move |_| {
                    let _ = &marker;
                    true
                },
            );
            if matches!(result, Err(Error::InvalidArgument { .. })) {
                observed_rejected.fetch_add(1, Ordering::SeqCst);
            }
        });

        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        assert_eq!(rejected.load(Ordering::SeqCst), 1);
    }

    fn step_case() {
        let entered = Arc::new(AtomicUsize::new(0));
        let callback_entered = Arc::clone(&entered);
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
        world
            .set_custom_filter(move |_, _| -> bool {
                callback_entered.fetch_add(1, Ordering::SeqCst);
                panic!("secondary world-step callback panic");
            })
            .unwrap();
        let first = world
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let second = world
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let shape_def = ShapeDef::builder()
            .density(1.0)
            .enable_custom_filtering(true)
            .build()
            .unwrap();
        let polygon = shapes::box_polygon(0.5, 0.5).unwrap();
        create_polygon(&mut world, first, &shape_def, &polygon);
        create_polygon(&mut world, second, &shape_def, &polygon);

        during_outer_unwind(move || {
            let _ = world.step(1.0 / 60.0, 1);
        });
        assert_callback_entered(&entered);
    }

    struct PanicDrawer(Arc<AtomicUsize>);

    impl DebugDraw for PanicDrawer {
        fn draw_solid_polygon(
            &mut self,
            _transform: WorldTransform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: HexColor,
        ) {
            self.0.fetch_add(1, Ordering::SeqCst);
            panic!("secondary debug-draw callback panic");
        }
    }

    fn debug_draw_case() {
        let entered = Arc::new(AtomicUsize::new(0));
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
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        create_polygon(
            &mut world,
            body,
            &ShapeDef::builder().density(1.0).build().unwrap(),
            &shapes::box_polygon(0.5, 0.5).unwrap(),
        );
        let mut drawer = PanicDrawer(Arc::clone(&entered));

        during_outer_unwind(move || {
            let _ = world.debug_draw(&mut drawer, DebugDrawOptions::default());
        });
        assert_callback_entered(&entered);
    }

    const MIXER_ID: MixerId = MixerId::from_bytes([0x81; 32]);

    fn mixer_recording() -> Recording {
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
        world
            .set_friction_callback(MIXER_ID, |a, b| a.coefficient.max(b.coefficient))
            .unwrap();
        let material = SurfaceMaterial::default()
            .with_friction(0.5)
            .unwrap()
            .with_user_material_id(7);
        let shape_def = ShapeDef::builder()
            .density(1.0)
            .material(material)
            .build()
            .unwrap();
        let polygon = shapes::box_polygon(0.5, 0.5).unwrap();
        let first = world
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let second = world
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .position([0.25_f32, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        create_polygon(&mut world, first, &shape_def, &polygon);
        create_polygon(&mut world, second, &shape_def, &polygon);
        let mut session = world.start_recording(RecordingLimits::default()).unwrap();
        drop(session.step(1.0 / 60.0, 2).unwrap());
        let recording = session.finish().unwrap();
        world.clear_friction_callback().unwrap();
        drop(world);
        recording
    }

    fn replay_step_case() {
        let entered = Arc::new(AtomicUsize::new(0));
        let callback_entered = Arc::clone(&entered);
        let recording = mixer_recording();
        let config = ReplayConfig::default().with_friction_mixer(MIXER_ID, move |_, _| -> f32 {
            callback_entered.fetch_add(1, Ordering::SeqCst);
            panic!("secondary replay-step callback panic");
        });
        let mut player = ReplayPlayer::open(
            boxdd::Foundation::initialize_default().unwrap(),
            &recording,
            config,
        )
        .unwrap();

        during_outer_unwind(move || {
            let _ = player.step();
        });
        assert_callback_entered(&entered);
    }

    fn replay_draw_case() {
        let entered = Arc::new(AtomicUsize::new(0));
        let recording = mixer_recording();
        let config = ReplayConfig::default()
            .with_friction_mixer(MIXER_ID, |a, b| a.coefficient.max(b.coefficient));
        let mut player = ReplayPlayer::open(
            boxdd::Foundation::initialize_default().unwrap(),
            &recording,
            config,
        )
        .unwrap();
        player.step().unwrap();
        let mut drawer = PanicDrawer(Arc::clone(&entered));

        during_outer_unwind(move || {
            let _ = player.draw(&mut drawer, DebugDrawOptions::default(), None);
        });
        assert_callback_entered(&entered);
    }

    fn snapshot_restore_case() {
        struct PanicOnDrop(Arc<AtomicUsize>);

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
                panic!("secondary snapshot user-data destructor panic");
            }
        }

        let entered = Arc::new(AtomicUsize::new(0));
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
                    .body_builder()
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let snapshot = world.snapshot().unwrap();
        world
            .body(body)
            .unwrap()
            .set_user_data(PanicOnDrop(Arc::clone(&entered)))
            .unwrap();

        let restore_result = Arc::new(Mutex::new(None));
        let observed_restore_result = Arc::clone(&restore_result);
        during_outer_unwind(move || {
            *observed_restore_result.lock().unwrap() = Some(world.restore(&snapshot));
        });
        let restore_result = restore_result
            .lock()
            .unwrap()
            .take()
            .expect("snapshot restore must complete during the outer unwind");
        assert!(matches!(restore_result, Err(Error::SnapshotCommitPanicked)));
        assert_callback_entered(&entered);
    }

    fn run_child(case: &str) {
        match case {
            "dynamic-tree" => dynamic_tree_case(),
            "query" => query_case(),
            "query-rejected" => rejected_query_visitor_case(),
            "world-step" => step_case(),
            "debug-draw" => debug_draw_case(),
            "replay-step" => replay_step_case(),
            "replay-draw" => replay_draw_case(),
            "snapshot-restore" => snapshot_restore_case(),
            other => panic!("unknown outer-unwind child case: {other}"),
        }
        eprintln!("boxdd-outer-unwind-callback: completed {case}");
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
                    "boxdd-outer-unwind-callback panic at {}:{}:{}: {message}",
                    location.file(),
                    location.line(),
                    location.column()
                );
            } else {
                eprintln!("boxdd-outer-unwind-callback panic: {message}");
            }
        }));
    }

    #[test]
    fn callback_panics_during_outer_unwind_do_not_abort() {
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
                .expect("outer-unwind callback child process must start");

        assert!(
            output.status.success(),
            "outer-unwind callback child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        for case in CASES {
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains(&format!("boxdd-outer-unwind-callback: completed {case}")),
                "outer-unwind callback child {case} did not complete its assertion path\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}
