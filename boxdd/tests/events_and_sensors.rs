use boxdd::{prelude::*, shapes};

fn create_head_on_pair(world: &mut World) -> (BodyId, BodyId, ShapeId, ShapeId) {
    let body_a = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([-1.0_f32, 0.0])
                .linear_velocity([2.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let body_b = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([1.0_f32, 0.0])
                .linear_velocity([-2.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build()
        .unwrap();
    let polygon = shapes::box_polygon(0.5, 0.5).unwrap();
    let shape_a = world
        .body(body_a)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();
    let shape_b = world
        .body(body_b)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();
    (body_a, body_b, shape_a, shape_b)
}

fn step_until_contact_begin(world: &mut World) -> ContactEvents {
    for _ in 0..180 {
        let completed = world.step(1.0 / 60.0, 4).unwrap();
        let view = completed.contact_events().unwrap();
        let events = view.to_owned().unwrap();
        if !view.begin().is_empty() {
            return events;
        }
    }
    panic!("expected at least one contact begin event");
}

fn step_until_contact_end(world: &mut World) -> ContactEvents {
    for _ in 0..10 {
        let completed = world.step(1.0 / 60.0, 4).unwrap();
        let view = completed.contact_events().unwrap();
        let events = view.to_owned().unwrap();
        if !view.end().is_empty() {
            return events;
        }
    }
    panic!("expected at least one contact end event");
}

#[test]
fn completed_step_contact_views_materialize_stable_owned_snapshots() {
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
    world.set_hit_event_threshold(0.0).unwrap();
    let (_body_a, body_b, shape_a, shape_b) = create_head_on_pair(&mut world);

    let begin = step_until_contact_begin(&mut world);
    assert!(begin.begin.iter().any(|event| {
        let shapes = [event.shape_a, event.shape_b];
        shapes.contains(&shape_a) && shapes.contains(&shape_b)
    }));
    assert!(begin.hit.iter().all(|event| {
        event.point.x.is_finite()
            && event.point.y.is_finite()
            && event.normal.is_valid()
            && event.approach_speed.is_finite()
            && event.approach_speed >= 0.0
    }));

    let stored = begin.clone();
    world
        .body(body_b)
        .unwrap()
        .set_position_and_rotation([10.0_f32, 0.0], 0.0)
        .unwrap();
    let end = step_until_contact_end(&mut world);
    assert!(!end.end.is_empty());
    assert_eq!(stored.begin.len(), begin.begin.len());
    assert_eq!(stored.hit.len(), begin.hit.len());
}

#[test]
fn fast_sensor_emits_one_begin_and_one_end_event() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let wall = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Static)
                .position([1.5_f32, 11.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let wall_shape = world
        .body(wall)
        .unwrap()
        .create_box(
            &ShapeDef::builder()
                .enable_sensor_events(true)
                .build()
                .unwrap(),
            0.5,
            10.0,
        )
        .unwrap();

    let bullet = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .bullet(true)
                .gravity_scale(0.0)
                .position([7.39814_f32, 4.0])
                .linear_velocity([-20.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let sensor_shape = world
        .body(bullet)
        .unwrap()
        .create_centered_circle(
            &ShapeDef::builder()
                .sensor(true)
                .enable_sensor_events(true)
                .build()
                .unwrap(),
            0.1,
        )
        .unwrap();

    let mut begin_count = 0;
    let mut end_count = 0;
    loop {
        let events = world
            .step(1.0 / 60.0, 4)
            .unwrap()
            .sensor_events()
            .unwrap()
            .to_owned()
            .unwrap();
        begin_count += events
            .begin
            .iter()
            .filter(|event| event.sensor_shape == sensor_shape && event.visitor_shape == wall_shape)
            .count();
        end_count += events
            .end
            .iter()
            .filter(|event| event.sensor_shape == sensor_shape && event.visitor_shape == wall_shape)
            .count();

        if world.body(bullet).unwrap().position().unwrap().x < -1.0 {
            break;
        }
    }

    assert_eq!(begin_count, 1);
    assert_eq!(end_count, 1);
}
