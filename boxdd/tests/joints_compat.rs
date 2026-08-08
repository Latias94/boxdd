use boxdd::prelude::*;

#[test]
fn revolute_and_prismatic_limits_smoke() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, -10.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let body_a = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.0_f32, 2.0])
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
                .position([1.0_f32, 2.0])
                .build()
                .unwrap(),
        )
        .unwrap();

    let revolute = world
        .create_revolute_joint(
            &RevoluteJointDef::new(world.joint_base(body_a, body_b).unwrap())
                .limit_deg(-15.0, 15.0),
        )
        .unwrap();
    let prismatic = world
        .create_prismatic_joint(
            &PrismaticJointDef::new(world.joint_base(body_a, body_b).unwrap())
                .enable_limit(true)
                .lower_translation(-0.5)
                .upper_translation(0.5),
        )
        .unwrap();

    world
        .joint(prismatic)
        .unwrap()
        .into_prismatic()
        .unwrap()
        .set_target_translation(0.0)
        .unwrap();

    for _ in 0..120 {
        let _ = world.step(1.0 / 60.0, 4).unwrap();
    }

    let angle = world
        .joint(revolute)
        .unwrap()
        .into_revolute()
        .unwrap()
        .angle()
        .unwrap();
    assert!(angle <= 15.0_f32.to_radians() + 1.0e-3 && angle >= -15.0_f32.to_radians() - 1.0e-3);

    let translation = world
        .joint(prismatic)
        .unwrap()
        .into_prismatic()
        .unwrap()
        .translation()
        .unwrap();
    assert!(translation.is_finite());
}
