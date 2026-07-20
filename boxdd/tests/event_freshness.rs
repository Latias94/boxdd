use boxdd::{prelude::*, shapes};

#[test]
fn completed_step_sensor_snapshot_survives_destroy_and_rejects_stale_ids() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    let handle = world.handle();

    let sensor_body = world.create_body_id(BodyBuilder::new().build());
    let sensor_shape = world.create_circle_shape_for(
        sensor_body,
        &ShapeDef::builder()
            .sensor(true)
            .enable_sensor_events(true)
            .build(),
        &shapes::circle([0.0_f32, 0.0], 1.0),
    );
    let visitor_body =
        world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let visitor_shape = world.create_circle_shape_for(
        visitor_body,
        &ShapeDef::builder().enable_sensor_events(true).build(),
        &shapes::circle([0.0_f32, 0.0], 0.25),
    );

    world.step(1.0 / 60.0, 4);
    assert!(world.sensor_events().begin.iter().any(|event| {
        event.sensor_shape == sensor_shape && event.visitor_shape == visitor_shape
    }));

    world.destroy_body_id(visitor_body);
    world.step(1.0 / 60.0, 4);

    // Box2D explicitly permits end events to contain destroyed shape ids. Destroying the other
    // participant before the first read also invalidates the native event data, so safe access must
    // use the snapshot captured immediately when the step completed.
    world.destroy_body_id(sensor_body);

    let events = world.sensor_events();
    let event = events
        .end
        .iter()
        .find(|event| event.sensor_shape == sensor_shape && event.visitor_shape == visitor_shape)
        .expect("expected the completed-step sensor end event");
    assert_eq!(
        world.try_shape_aabb(event.sensor_shape),
        Err(ApiError::InvalidShapeId)
    );
    assert_eq!(
        handle.try_shape_aabb(event.visitor_shape),
        Err(ApiError::InvalidShapeId)
    );

    let viewed = world.with_sensor_events_view(|_, mut end| {
        end.any(|event| {
            event.sensor_shape() == sensor_shape && event.visitor_shape() == visitor_shape
        })
    });
    assert!(viewed);

    let handle_events = handle.sensor_events();
    assert!(handle_events.end.iter().any(|event| {
        event.sensor_shape == sensor_shape && event.visitor_shape == visitor_shape
    }));
}
