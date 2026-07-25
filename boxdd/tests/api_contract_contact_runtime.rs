use boxdd::{BodyBuilder, BodyType, ContactId, ShapeDef, World, WorldDef, shapes};

fn world_with_live_contact() -> (World, ContactId) {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build())
        .expect("world creation should succeed");
    let body_a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-1.0_f32, 0.0])
            .build(),
    );
    let body_b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([1.0_f32, 0.0])
            .build(),
    );
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .build();
    world.create_polygon_shape_for(body_a, &shape_def, &shapes::box_polygon(0.5_f32, 0.5));
    world.create_polygon_shape_for(body_b, &shape_def, &shapes::box_polygon(0.5_f32, 0.5));
    world.set_body_linear_velocity(body_a, [2.0_f32, 0.0]);
    world.set_body_linear_velocity(body_b, [-2.0_f32, 0.0]);

    for _ in 0..180 {
        world.step(1.0 / 60.0, 4);
        if let Some(event) = world.contact_events().begin.first() {
            return (world, event.contact_id);
        }
    }

    panic!("expected a live contact id from a contact-begin event");
}

#[test]
fn contact_identity_and_snapshot_are_runtime_witnesses() {
    let (world, contact_id) = world_with_live_contact();
    let world: boxdd::World = world;
    let contact_id: boxdd::ContactId = contact_id;
    let contact_valid = boxdd::World::contact_is_valid(&world, contact_id);
    let contact_data = boxdd::World::try_contact_data(&world, contact_id);

    assert!(contact_valid);
    assert_eq!(
        contact_data
            .expect("live contact should produce a snapshot")
            .contact_id,
        contact_id
    );
}
