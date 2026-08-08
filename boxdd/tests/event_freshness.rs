use boxdd::{RecordingLimits, prelude::*, shapes};

fn add_contact_pair(world: &mut World) -> (BodyId, ShapeId, ShapeId) {
    let shape_def = ShapeDef::builder()
        .enable_contact_events(true)
        .density(1.0)
        .build()
        .unwrap();
    let first_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let first_shape = world
        .body(first_body)
        .unwrap()
        .create_circle(&shape_def, &shapes::circle(Vec2::ZERO, 1.0).unwrap())
        .unwrap();
    let second_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let second_shape = world
        .body(second_body)
        .unwrap()
        .create_circle(&shape_def, &shapes::circle(Vec2::ZERO, 0.25).unwrap())
        .unwrap();
    (second_body, first_shape, second_shape)
}

#[test]
fn owned_completed_step_snapshot_survives_destroy_and_preserves_stale_ids() {
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
    let sensor_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let sensor_shape = world
        .body(sensor_body)
        .unwrap()
        .create_circle(
            &ShapeDef::builder()
                .sensor(true)
                .enable_sensor_events(true)
                .build()
                .unwrap(),
            &shapes::circle(Vec2::ZERO, 1.0).unwrap(),
        )
        .unwrap();
    let visitor_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let visitor_shape = world
        .body(visitor_body)
        .unwrap()
        .create_circle(
            &ShapeDef::builder()
                .enable_sensor_events(true)
                .build()
                .unwrap(),
            &shapes::circle(Vec2::ZERO, 0.25).unwrap(),
        )
        .unwrap();

    let begin = world.step(1.0 / 60.0, 4).unwrap().to_owned().unwrap();
    assert!(begin.sensor.begin.iter().any(|event| {
        event.sensor_shape == sensor_shape && event.visitor_shape == visitor_shape
    }));

    world.body(visitor_body).unwrap().destroy().unwrap();
    let end = world.step(1.0 / 60.0, 4).unwrap().to_owned().unwrap();
    let event = end
        .sensor
        .end
        .iter()
        .find(|event| event.sensor_shape == sensor_shape && event.visitor_shape == visitor_shape)
        .expect("expected the completed-step sensor end event");

    world.body(sensor_body).unwrap().destroy().unwrap();
    assert_eq!(
        world.shape(event.sensor_shape).err().unwrap(),
        Error::InvalidShapeId
    );
    assert_eq!(
        world.shape(event.visitor_shape).err().unwrap(),
        Error::InvalidShapeId
    );
}

#[test]
fn contact_end_events_resolve_body_cascade_identities_after_the_following_step() {
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
    let (destroyed_body, first_shape, second_shape) = add_contact_pair(&mut world);

    let begin = world.step(1.0 / 60.0, 4).unwrap().to_owned().unwrap();
    assert!(begin.contact.begin.iter().any(|event| {
        (event.shape_a == first_shape && event.shape_b == second_shape)
            || (event.shape_a == second_shape && event.shape_b == first_shape)
    }));

    world.body(destroyed_body).unwrap().destroy().unwrap();
    let end = world.step(1.0 / 60.0, 4).unwrap().to_owned().unwrap();
    assert!(end.contact.end.iter().any(|event| {
        (event.shape_a == first_shape && event.shape_b == second_shape)
            || (event.shape_a == second_shape && event.shape_b == first_shape)
    }));
}

#[test]
fn recording_session_keeps_destroyed_sensor_identity_until_end_event_materialization() {
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
    let sensor_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let sensor_shape = world
        .body(sensor_body)
        .unwrap()
        .create_circle(
            &ShapeDef::builder()
                .sensor(true)
                .enable_sensor_events(true)
                .build()
                .unwrap(),
            &shapes::circle(Vec2::ZERO, 1.0).unwrap(),
        )
        .unwrap();
    let visitor_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .build()
                .unwrap(),
        )
        .unwrap();
    let visitor_shape = world
        .body(visitor_body)
        .unwrap()
        .create_circle(
            &ShapeDef::builder()
                .enable_sensor_events(true)
                .build()
                .unwrap(),
            &shapes::circle(Vec2::ZERO, 0.25).unwrap(),
        )
        .unwrap();
    let mut recording = world.start_recording(RecordingLimits::default()).unwrap();

    let begin = recording.step(1.0 / 60.0, 4).unwrap().to_owned().unwrap();
    assert!(begin.sensor.begin.iter().any(|event| {
        event.sensor_shape == sensor_shape && event.visitor_shape == visitor_shape
    }));

    recording.body(visitor_body).unwrap().destroy().unwrap();
    let end = recording.step(1.0 / 60.0, 4).unwrap().to_owned().unwrap();
    assert!(end.sensor.end.iter().any(|event| {
        event.sensor_shape == sensor_shape && event.visitor_shape == visitor_shape
    }));
}
