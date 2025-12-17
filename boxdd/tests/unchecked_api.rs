#![cfg(feature = "unchecked")]

use boxdd::prelude::*;

#[test]
fn unchecked_hotpath_methods_compile_and_work() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let mut body = world.create_body_owned(BodyBuilder::new().body_type(BodyType::Dynamic).build());

    unsafe {
        let _p0 = body.position_unchecked();
        body.set_linear_velocity_unchecked(Vec2::new(1.0, 0.0));
        world.set_body_angular_velocity_unchecked(body.id(), 2.0);
        let _t = world.body_transform_unchecked(body.id());
    }
}
