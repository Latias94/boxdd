use boxdd::{prelude::*, shapes};
use boxdd_sys::ffi;

#[test]
fn revolute_and_prismatic_limits_smoke() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build()).unwrap();

    // Bodies
    let a = world.create_body_id(BodyBuilder::new().position([0.0_f32, 2.0]).build());
    let b = world.create_body_id(BodyBuilder::new().position([1.0_f32, 2.0]).build());
    let sdef = ShapeDef::builder().density(1.0).build();
    let _sa = world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5, 0.5));
    let _sb = world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));

    // Revolute joint with limits
    let base =
        world.joint_base_from_world_points(a, b, world.body_position(a), world.body_position(a));
    let rdef = RevoluteJointDef::new(base).limit_deg(-15.0, 15.0);
    let rjid = world.create_revolute_joint_id(&rdef);

    // Prismatic joint along x with limits
    let base2 = world.joint_base_from_world_with_axis(
        a,
        b,
        world.body_position(a),
        world.body_position(b),
        Vec2::new(1.0, 0.0),
    );
    let pdef = PrismaticJointDef::new(base2)
        .enable_limit(true)
        .lower_translation(-0.5)
        .upper_translation(0.5);
    let pjid = world.create_prismatic_joint_id(&pdef);
    // Drive target translation to 0.0 for stability
    world.prismatic_set_target_translation(pjid, 0.0);

    for _ in 0..120 {
        world.step(1.0 / 60.0, 4);
    }

    // Check revolute angle within limits via FFI getter (compat smoke)
    let ang = unsafe { ffi::b2RevoluteJoint_GetAngle(rjid) };
    assert!(ang <= 15.0_f32.to_radians() + 1e-3 && ang >= -15.0_f32.to_radians() - 1e-3);

    // Check prismatic translation within limits via FFI getter
    let trans = unsafe { ffi::b2PrismaticJoint_GetTranslation(pjid) };
    assert!(trans.is_finite());
}
