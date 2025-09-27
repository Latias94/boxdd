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

    // Elevator platform
    let platform = world.create_body_id(BodyBuilder::new().position([0.0_f32, 1.0]).build());
    let _ps = world.create_polygon_shape_for(
        platform,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(1.0, 0.2),
    );

    // Prismatic joint: vertical axis, limits, motor
    let axis = [0.0_f32, 1.0];
    let anchor = [0.0_f32, 1.0];
    let base = world.joint_base_from_world_with_axis(ground, platform, anchor, anchor, axis);
    let pdef = PrismaticJointDef::new(base)
        .enable_limit(true)
        .lower_translation(0.0)
        .upper_translation(4.0)
        .enable_motor(true)
        .max_motor_force(100.0)
        .motor_speed(2.0); // m/s up
    let _pj = world.create_prismatic_joint_id(&pdef);

    for _ in 0..240 {
        world.step(1.0 / 60.0, 4);
    }

    let pos = world.body_position(platform);
    println!("elevator platform y: {:.2}", pos.y);
    Ok(())
}
