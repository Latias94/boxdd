use boxdd::prelude::*;

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
}

#[test]
fn debug_draw_panic_is_caught_and_resumed() {
    struct Panicker;
    impl DebugDraw for Panicker {
        fn draw_solid_polygon(
            &mut self,
            _transform: boxdd::Transform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: u32,
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
        unsafe { world.debug_draw(&mut drawer, DebugDrawOptions::default()) };
    }));
    assert!(r.is_err());
}

#[test]
fn debug_draw_reentrant_boxdd_call_panics() {
    struct Reenter {
        body: OwnedBody,
    }
    impl DebugDraw for Reenter {
        fn draw_solid_polygon(
            &mut self,
            _transform: boxdd::Transform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: u32,
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
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        world.debug_draw(&mut drawer, DebugDrawOptions::default());
    }));
    assert!(r.is_err());
}
