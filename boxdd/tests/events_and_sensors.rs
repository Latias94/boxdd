use boxdd::{prelude::*, shapes};
use boxdd_sys::ffi;

fn shape_key(id: ShapeId) -> (i32, u16, u16) {
    (id.index1, id.world0, id.generation)
}

#[test]
fn contact_and_sensor_events_smoke() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    // Two dynamic boxes head-on to ensure a contact event regardless of gravity
    let b1 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 2.0])
            .build(),
    );
    let b2 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.5])
            .build(),
    );
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();
    let _s1 = world.create_polygon_shape_for(b1, &sdef, &shapes::box_polygon(0.5, 0.5));
    let _s2 = world.create_polygon_shape_for(b2, &sdef, &shapes::box_polygon(0.5, 0.5));
    world.set_body_linear_velocity(b1, [0.0_f32, 2.0]);
    world.set_body_linear_velocity(b2, [0.0_f32, -2.0]);

    // Sensor segment sweeping through
    let sensor_body = world.create_body_id(BodyBuilder::new().build());
    let sensor_seg = shapes::segment([-1.0_f32, 1.5], [1.0, 1.5]);
    let _ss = world.create_segment_shape_for(
        sensor_body,
        &ShapeDef::builder()
            .sensor(true)
            .enable_sensor_events(true)
            .build(),
        &sensor_seg,
    );

    // Step and accumulate events
    let mut begin_sum = 0;
    let mut _end_sum = 0;
    for _ in 0..180 {
        world.step(1.0 / 60.0, 4);
        let ev = world.contact_events();
        begin_sum += ev.begin.len();
        _end_sum += ev.end.len();
    }
    // We should have seen at least one contact begin
    assert!(begin_sum > 0, "expected some contact begin events");

    // Sensor overlaps: capacity can be zero if no overlaps; test does not assert, just exercises API
}

#[test]
fn contact_event_view_matches_owned_snapshot() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    let b1 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 2.0])
            .build(),
    );
    let b2 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.5])
            .build(),
    );
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();
    let _s1 = world.create_polygon_shape_for(b1, &sdef, &shapes::box_polygon(0.5, 0.5));
    let _s2 = world.create_polygon_shape_for(b2, &sdef, &shapes::box_polygon(0.5, 0.5));
    world.set_body_linear_velocity(b1, [0.0_f32, 2.0]);
    world.set_body_linear_velocity(b2, [0.0_f32, -2.0]);

    let owned = step_until_contact_begin(&mut world);
    let view = world.with_contact_events_view(|begin, end, hit| {
        (
            begin
                .map(|event| (shape_key(event.shape_a()), shape_key(event.shape_b())))
                .collect::<Vec<_>>(),
            end.count(),
            hit.count(),
        )
    });
    let owned_begin = owned
        .begin
        .iter()
        .map(|event| (shape_key(event.shape_a), shape_key(event.shape_b)))
        .collect::<Vec<_>>();

    assert_eq!(view.0, owned_begin);
    assert_eq!(view.1, owned.end.len());
    assert_eq!(view.2, owned.hit.len());

    let stored = owned.clone();
    world.step(1.0 / 60.0, 4);
    assert_eq!(stored.begin.len(), owned_begin.len());
}

#[test]
fn sensor_event_view_matches_owned_snapshot() {
    let mut world = World::new(WorldDef::builder().build()).unwrap();

    let wall = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Static)
            .position([1.5_f32, 11.0])
            .build(),
    );
    let wall_shape_def = ShapeDef::builder().enable_sensor_events(true).build();
    let _wall_shape =
        world.create_polygon_shape_for(wall, &wall_shape_def, &shapes::box_polygon(0.5, 10.0));

    let bullet = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .bullet(true)
            .gravity_scale(0.0)
            .position([7.39814_f32, 4.0])
            .linear_velocity([-20.0_f32, 0.0])
            .build(),
    );
    let bullet_shape_def = ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build();
    let circle = shapes::circle([0.0_f32, 0.0], 0.1);
    let _bullet_shape = world.create_circle_shape_for(bullet, &bullet_shape_def, &circle);

    for _ in 0..600 {
        world.step(1.0 / 60.0, 4);
        let owned = world.sensor_events();
        if owned.begin.is_empty() {
            continue;
        }

        let view = world.with_sensor_events_view(|begin, end| {
            (
                begin
                    .map(|event| {
                        (
                            shape_key(event.sensor_shape()),
                            shape_key(event.visitor_shape()),
                        )
                    })
                    .collect::<Vec<_>>(),
                end.count(),
            )
        });
        let owned_begin = owned
            .begin
            .iter()
            .map(|event| {
                (
                    shape_key(event.sensor_shape),
                    shape_key(event.visitor_shape),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(view.0, owned_begin);
        assert_eq!(view.1, owned.end.len());

        let stored = owned.clone();
        world.step(1.0 / 60.0, 4);
        assert_eq!(stored.begin.len(), owned_begin.len());
        return;
    }

    panic!("expected at least one sensor begin event");
}

#[test]
fn dropping_owned_body_inside_event_view_defers_destroy_until_view_exits() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let body_id = body.id();
    let mut body = Some(body);

    let _move_event_count = world.with_body_events_view(|moves| {
        let count = moves.count();
        drop(body.take());
        assert!(unsafe { ffi::b2Body_IsValid(body_id.into_raw()) });
        count
    });

    assert!(body.is_none());
    assert_eq!(
        world.try_body_position(body_id).unwrap_err(),
        ApiError::InvalidBodyId
    );
}

#[test]
fn sensor_bullet_through_wall_precise() {
    let mut world = World::new(WorldDef::builder().build()).unwrap();

    // Wall from x = 1 to x = 2 at y around 11, matching upstream
    let wall = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Static)
            .position([1.5_f32, 11.0])
            .build(),
    );
    let wall_shape_def = ShapeDef::builder().enable_sensor_events(true).build();
    let _wall_shape =
        world.create_polygon_shape_for(wall, &wall_shape_def, &shapes::box_polygon(0.5, 10.0));

    // Bullet fired towards the wall
    let bullet = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .bullet(true)
            .gravity_scale(0.0)
            .position([7.39814_f32, 4.0])
            .linear_velocity([-20.0_f32, 0.0])
            .build(),
    );
    let bullet_shape_def = ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build();
    let circle = shapes::circle([0.0_f32, 0.0], 0.1);
    let _bullet_shape = world.create_circle_shape_for(bullet, &bullet_shape_def, &circle);

    let mut begin_count = 0;
    let mut end_count = 0;

    loop {
        world.step(1.0 / 60.0, 4);

        let p = world.body_position(bullet);
        let ev = world.sensor_events();
        if !ev.begin.is_empty() {
            begin_count += 1;
        }
        if !ev.end.is_empty() {
            end_count += 1;
        }
        if p.x < -1.0 {
            break;
        }
    }

    assert_eq!(begin_count, 1);
    assert_eq!(end_count, 1);
}

fn step_until_contact_begin(world: &mut World) -> ContactEvents {
    for _ in 0..180 {
        world.step(1.0 / 60.0, 4);
        let events = world.contact_events();
        if !events.begin.is_empty() {
            return events;
        }
    }
    panic!("expected at least one contact begin event");
}
