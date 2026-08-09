use boxdd::{BodyType, ContactId, ShapeDef, World, shapes};

fn world_with_live_contact() -> (World, ContactId) {
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
        .expect("world creation should succeed");
    let body_a = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([-1.0_f32, 0.0])
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
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .build()
        .unwrap();
    let polygon = shapes::box_polygon(0.5_f32, 0.5).expect("valid polygon geometry");
    world
        .body(body_a)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();
    world
        .body(body_b)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();
    world
        .body(body_a)
        .unwrap()
        .set_linear_velocity([2.0_f32, 0.0])
        .unwrap();
    world
        .body(body_b)
        .unwrap()
        .set_linear_velocity([-2.0_f32, 0.0])
        .unwrap();

    for _ in 0..180 {
        let contact = {
            let completed = world.step(1.0 / 60.0, 4).unwrap();
            completed
                .contact_events()
                .unwrap()
                .begin()
                .first()
                .map(|event| event.contact_id)
        };
        if let Some(contact) = contact {
            return (world, contact);
        }
    }

    panic!("expected a live contact id from a contact-begin event");
}

#[test]
fn contact_identity_and_snapshot_are_runtime_witnesses() {
    let (world, contact_id) = world_with_live_contact();
    let world: boxdd::World = world;
    let contact_id: boxdd::ContactId = contact_id;
    let contact_valid = boxdd::World::contact_is_valid(&world, contact_id)
        .expect("live contact validity should be readable");
    let contact_data = boxdd::World::contact_data(&world, contact_id);

    assert!(contact_valid);
    assert_eq!(
        contact_data
            .expect("live contact should produce a snapshot")
            .contact_id,
        contact_id
    );
}
