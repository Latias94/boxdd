use boxdd::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn custom_filter_panic_is_caught_and_resumed_after_step() {
    let mut world = World::new(WorldDef::builder().gravity([0.0, 0.0]).build()).unwrap();
    world.set_custom_filter(|_, _| -> bool {
        panic!("boom in custom filter");
    });

    let a = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let b = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_custom_filtering(true)
        .build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(a, &sdef, &poly);
    let _ = world.create_polygon_shape_for(b, &sdef, &poly);

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.step(1.0 / 60.0, 1);
    }));
    assert!(r.is_err());

    world.clear_custom_filter();
    world.step(1.0 / 60.0, 1);
    assert!(world.try_body_position(a).is_ok());
    assert!(world.try_body_position(b).is_ok());
}

#[test]
fn pre_solve_panic_is_caught_and_resumed_after_step() {
    let mut world = World::new(WorldDef::builder().gravity([0.0, 0.0]).build()).unwrap();
    world.set_pre_solve(|_, _, _, _| -> bool {
        panic!("boom in pre-solve");
    });

    let a = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let b = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_pre_solve_events(true)
        .build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(a, &sdef, &poly);
    let _ = world.create_polygon_shape_for(b, &sdef, &poly);

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.step(1.0 / 60.0, 1);
    }));
    assert!(r.is_err());

    world.clear_pre_solve();
    let replacement_calls = Arc::new(AtomicUsize::new(0));
    world.set_pre_solve({
        let replacement_calls = Arc::clone(&replacement_calls);
        move |_, _, _, _| {
            replacement_calls.fetch_add(1, Ordering::SeqCst);
            true
        }
    });
    for _ in 0..5 {
        world.step(1.0 / 60.0, 1);
    }
    assert!(replacement_calls.load(Ordering::SeqCst) > 0);
    assert!(world.try_body_position(a).is_ok());
    assert!(world.try_body_position(b).is_ok());
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

    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(body, &sdef, &poly);
    let mut drawer = Panicker;
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.debug_draw(&mut drawer, DebugDrawOptions::default());
    }));
    assert!(r.is_err());
}

#[test]
fn draw_bounds_panic_flushes_owned_body_teardown_before_resuming() {
    struct BoundsPanicker {
        body: Option<OwnedBody>,
    }

    impl DebugDraw for BoundsPanicker {
        fn draw_bounds(&mut self, _bounds: Aabb, _color: HexColor) {
            drop(self.body.take());
            panic!("boom in draw bounds");
        }
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_id = body.id();
    let _ = world.create_polygon_shape_for(
        body_id,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );
    let mut drawer = BoundsPanicker { body: Some(body) };
    let options = DebugDrawOptions {
        draw_bounds: true,
        ..DebugDrawOptions::default()
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.debug_draw(&mut drawer, options);
    }));

    assert!(result.is_err());
    assert!(drawer.body.is_none());
    assert_eq!(
        world.try_body_position(body_id),
        Err(ApiError::InvalidBodyId)
    );
    world.debug_draw(&mut drawer, DebugDrawOptions::default());
}

#[test]
fn debug_draw_reentrant_boxdd_call_panics() {
    struct Reenter {
        body: OwnedBody,
    }
    impl DebugDraw for Reenter {
        fn draw_solid_polygon(
            &mut self,
            _transform: boxdd::WorldTransform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: HexColor,
        ) {
            let _ = self.body.position();
        }
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_id = body.id();
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(body_id, &sdef, &poly);

    let mut drawer = Reenter { body };
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.debug_draw(&mut drawer, DebugDrawOptions::default());
    }));
    assert!(r.is_err());
}

#[test]
fn debug_draw_reentrant_try_boxdd_call_returns_in_callback() {
    struct ReenterTry {
        body: OwnedBody,
        observed: Option<ApiError>,
    }

    impl DebugDraw for ReenterTry {
        fn draw_solid_polygon(
            &mut self,
            _transform: boxdd::WorldTransform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: HexColor,
        ) {
            self.observed = Some(self.body.try_position().unwrap_err());
        }
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_id = body.id();
    let sdef = ShapeDef::builder().density(1.0).build();
    let poly = shapes::box_polygon(0.5, 0.5);
    let _ = world.create_polygon_shape_for(body_id, &sdef, &poly);

    let mut drawer = ReenterTry {
        body,
        observed: None,
    };
    world.debug_draw(&mut drawer, DebugDrawOptions::default());

    assert_eq!(drawer.observed, Some(ApiError::InCallback));
    assert!(drawer.body.try_position().is_ok());
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
        let entered = Arc::new(AtomicUsize::new(0));
        let callback_entered = Arc::clone(&entered);
        let mut tree = DynamicTree::new();
        tree.create_proxy(Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]), u64::MAX, 7);

        during_outer_unwind(move || {
            tree.query_all(
                Aabb::new([-2.0_f32, -2.0], [2.0_f32, 2.0]),
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
        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_id(BodyBuilder::new().build());
        world.create_circle_shape_for(
            body,
            &ShapeDef::default(),
            &shapes::circle([0.0_f32, 0.0], 0.5),
        );

        during_outer_unwind(move || {
            world.visit_overlap_aabb(
                Position::ZERO,
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]),
                QueryFilter::default(),
                |_| -> bool {
                    callback_entered.fetch_add(1, Ordering::SeqCst);
                    panic!("secondary world-query callback panic");
                },
            );
        });
        assert_callback_entered(&entered);
    }

    fn step_case() {
        let entered = Arc::new(AtomicUsize::new(0));
        let callback_entered = Arc::clone(&entered);
        let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
        world.set_custom_filter(move |_, _| -> bool {
            callback_entered.fetch_add(1, Ordering::SeqCst);
            panic!("secondary world-step callback panic");
        });
        let first = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let second = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let shape_def = ShapeDef::builder()
            .density(1.0)
            .enable_custom_filtering(true)
            .build();
        let polygon = shapes::box_polygon(0.5, 0.5);
        world.create_polygon_shape_for(first, &shape_def, &polygon);
        world.create_polygon_shape_for(second, &shape_def, &polygon);

        during_outer_unwind(move || world.step(1.0 / 60.0, 1));
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
        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        world.create_polygon_shape_for(
            body,
            &ShapeDef::builder().density(1.0).build(),
            &shapes::box_polygon(0.5, 0.5),
        );
        let mut drawer = PanicDrawer(Arc::clone(&entered));

        during_outer_unwind(move || {
            world.debug_draw(&mut drawer, DebugDrawOptions::default());
        });
        assert_callback_entered(&entered);
    }

    fn mixer_recording() -> Recording {
        let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
        world.set_friction_callback(|a, b| a.coefficient.max(b.coefficient));
        let material = SurfaceMaterial::default()
            .with_friction(0.5)
            .with_user_material_id(7);
        let shape_def = ShapeDef::builder().density(1.0).material(material).build();
        let polygon = shapes::box_polygon(0.5, 0.5);
        let first = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let second = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([0.25_f32, 0.0])
                .build(),
        );
        world.create_polygon_shape_for(first, &shape_def, &polygon);
        world.create_polygon_shape_for(second, &shape_def, &polygon);
        let mut session = world.start_recording(RecordingCapacity::default());
        session.step(1.0 / 60.0, 2);
        let recording = session.finish();
        world.clear_friction_callback();
        drop(world);
        recording
    }

    fn replay_step_case() {
        let entered = Arc::new(AtomicUsize::new(0));
        let callback_entered = Arc::clone(&entered);
        let recording = mixer_recording();
        let config = ReplayConfig::default().with_friction_mixer(move |_, _| -> f32 {
            callback_entered.fetch_add(1, Ordering::SeqCst);
            panic!("secondary replay-step callback panic");
        });
        let mut player = ReplayPlayer::open_recording(&recording, config).unwrap();

        during_outer_unwind(move || {
            let _ = player.step();
        });
        assert_callback_entered(&entered);
    }

    fn replay_draw_case() {
        let entered = Arc::new(AtomicUsize::new(0));
        let recording = mixer_recording();
        let config =
            ReplayConfig::default().with_friction_mixer(|a, b| a.coefficient.max(b.coefficient));
        let mut player = ReplayPlayer::open_recording(&recording, config).unwrap();
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
        let mut world = World::new(WorldDef::default()).unwrap();
        let body = world.create_body_id(BodyBuilder::new().build());
        let snapshot = world.snapshot();
        world
            .body(body)
            .unwrap()
            .set_user_data(PanicOnDrop(Arc::clone(&entered)));

        during_outer_unwind(move || {
            assert!(matches!(
                world.try_restore(&snapshot),
                Err(ApiError::WorldDestroyed)
            ));
        });
        assert_callback_entered(&entered);
    }

    fn run_child(case: &str) {
        match case {
            "dynamic-tree" => dynamic_tree_case(),
            "query" => query_case(),
            "world-step" => step_case(),
            "debug-draw" => debug_draw_case(),
            "replay-step" => replay_step_case(),
            "replay-draw" => replay_draw_case(),
            "snapshot-restore" => snapshot_restore_case(),
            other => panic!("unknown outer-unwind child case: {other}"),
        }
        eprintln!("boxdd-outer-unwind-callback: completed {case}");
    }

    #[test]
    fn callback_panics_during_outer_unwind_do_not_abort() {
        if let Some(case) = std::env::var_os(CHILD_CASE) {
            run_child(&case.to_string_lossy());
            return;
        }

        for case in [
            "dynamic-tree",
            "query",
            "world-step",
            "debug-draw",
            "replay-step",
            "replay-draw",
            "snapshot-restore",
        ] {
            let output = Command::new(
                std::env::current_exe().expect("test executable path must be available"),
            )
            .args(["--exact", TEST_NAME, "--nocapture"])
            .env(CHILD_CASE, case)
            .output()
            .expect("outer-unwind callback child process must start");

            assert!(
                output.status.success(),
                "outer-unwind callback child {case} aborted\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
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
