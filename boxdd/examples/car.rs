use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _gs = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(50.0, 1.0),
    );

    // Chassis
    let chassis = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
    let sdef = ShapeDef::builder().density(1.0).build();
    let _ch_shape =
        world.create_polygon_shape_for(chassis, &sdef, &shapes::box_polygon(1.25, 0.25));

    // Wheels
    let wheel_radius = 0.4;
    let wheel_offset_x = 0.8;
    let wheel_offset_y = -0.3;
    let w1 = world.create_body_id(
        BodyBuilder::new()
            .position([-wheel_offset_x, 2.0 + wheel_offset_y])
            .build(),
    );
    let w2 = world.create_body_id(
        BodyBuilder::new()
            .position([wheel_offset_x, 2.0 + wheel_offset_y])
            .build(),
    );
    let circle = boxdd::shapes::circle([0.0_f32, 0.0], wheel_radius);
    let _sw1 = world.create_circle_shape_for(w1, &sdef, &circle);
    let _sw2 = world.create_circle_shape_for(w2, &sdef, &circle);

    // Wheel joints (suspension + motor on rear)
    let axis = Vec2::new(0.0, 1.0);
    let base = world.joint_base_from_world_with_axis(
        chassis,
        w1,
        [-wheel_offset_x, 2.0 + wheel_offset_y],
        [-wheel_offset_x, 2.0 + wheel_offset_y],
        axis,
    );
    let wdef1 = WheelJointDef::new(base.clone())
        .enable_spring(true)
        .hertz(4.0)
        .damping_ratio(0.7)
        .enable_motor(true)
        .max_motor_torque(20.0)
        .motor_speed(0.0);
    let _wj1 = world.create_wheel_joint_id(&wdef1);
    let base2 = world.joint_base_from_world_with_axis(
        chassis,
        w2,
        [wheel_offset_x, 2.0 + wheel_offset_y],
        [wheel_offset_x, 2.0 + wheel_offset_y],
        axis,
    );
    let wdef2 = WheelJointDef::new(base2)
        .enable_spring(true)
        .hertz(4.0)
        .damping_ratio(0.7)
        .enable_motor(true)
        .max_motor_torque(40.0)
        .motor_speed(15.0);
    let _wj2 = world.create_wheel_joint_id(&wdef2);

    for _ in 0..240 {
        world.step(1.0 / 60.0, 4);
    }

    let p = world.body_position(chassis);
    println!("car chassis at: ({:.2}, {:.2})", p.x, p.y);
    Ok(())
}
