use boxdd::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, -10.0])
            .build()?,
    )?;

    // Ground
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _g = world.body(ground)?.create_polygon(
        &ShapeDef::builder().density(0.0).build()?,
        &shapes::box_polygon(50.0, 1.0).expect("valid polygon geometry"),
    )?;

    // Rotor at anchor
    let rotor = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position([0.0_f32, 2.0])
            .build()?,
    )?;
    let _rshape = world.body(rotor)?.create_polygon(
        &ShapeDef::builder().density(1.0).build()?,
        &shapes::box_polygon(1.0, 0.1).expect("valid polygon geometry"),
    )?;

    // Revolute joint with motor and limits
    let base = world.joint_base_from_world_points(ground, rotor, [0.0_f32, 2.0], [0.0_f32, 2.0])?;
    let rdef = RevoluteJointDef::new(base)
        .limit_deg(-45.0, 45.0)
        .enable_motor(true)
        .max_motor_torque(50.0)
        .motor_speed(2.0); // radians/sec
    let _jid = world.create_revolute_joint(&rdef)?;

    for _ in 0..240 {
        drop(world.step(1.0 / 60.0, 4)?);
    }

    let angle = world.body(rotor)?.transform()?.rotation().angle();
    println!("revolute motor angle: {:.2} rad", angle);
    Ok(())
}
