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

    // Elevator platform
    let platform = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position([0.0_f32, 1.0])
            .build()?,
    )?;
    let _ps = world.body(platform)?.create_polygon(
        &ShapeDef::builder().density(1.0).build()?,
        &shapes::box_polygon(1.0, 0.2).expect("valid polygon geometry"),
    )?;

    // Prismatic joint: vertical axis, limits, motor
    let axis = [0.0_f32, 1.0];
    let anchor = [0.0_f32, 1.0];
    let base = world.joint_base_from_world_with_axis(ground, platform, anchor, anchor, axis)?;
    let pdef = PrismaticJointDef::new(base)
        .enable_limit(true)
        .lower_translation(0.0)
        .upper_translation(4.0)
        .enable_motor(true)
        .max_motor_force(100.0)
        .motor_speed(2.0); // m/s up
    let _pj = world.create_prismatic_joint(&pdef)?;

    for _ in 0..240 {
        drop(world.step(1.0 / 60.0, 4)?);
    }

    let pos = world.body(platform)?.position()?;
    println!("elevator platform y: {:.2}", pos.y);
    Ok(())
}
