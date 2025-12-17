use boxdd::prelude::*;

#[test]
fn scoped_body_panics_after_owned_body_drop() {
    let mut world = World::new(WorldDef::builder().build()).unwrap();

    let owned = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());
    let id = owned.id();

    let body = world.body(id).unwrap();
    drop(owned);

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = body.position();
    }));
    assert!(r.is_err());
}

#[test]
fn scoped_shape_panics_after_owned_shape_drop() {
    let mut world = World::new(WorldDef::builder().build()).unwrap();

    let body_id = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());

    let sdef = ShapeDef::builder().density(1.0).build();
    let circle = shapes::circle([0.0, 0.0], 0.5);
    let owned_shape = world.create_circle_shape_for_owned(body_id, &sdef, &circle);
    let shape_id = owned_shape.id();

    let shape = world.shape(shape_id).unwrap();
    drop(owned_shape);

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = shape.density();
    }));
    assert!(r.is_err());
}
