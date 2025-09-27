use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _g = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(50.0, 1.0),
    );

    // Rotor at anchor
    let rotor = world.create_body_id(BodyBuilder::new().position([0.0_f32, 2.0]).build());
    let _rshape = world.create_polygon_shape_for(
        rotor,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(1.0, 0.1),
    );

    // Revolute joint with motor and limits
    let base = world.joint_base_from_world_points(ground, rotor, [0.0_f32, 2.0], [0.0_f32, 2.0]);
    let rdef = RevoluteJointDef::new(base)
        .limit_deg(-45.0, 45.0)
        .enable_motor(true)
        .max_motor_torque(50.0)
        .motor_speed(2.0); // radians/sec
    let _jid = world.create_revolute_joint_id(&rdef);

    for _ in 0..240 {
        world.step(1.0 / 60.0, 4);
    }

    let angle = world.body_transform(rotor).rotation().angle();
    println!("revolute motor angle: {:.2} rad", angle);
    Ok(())
}
