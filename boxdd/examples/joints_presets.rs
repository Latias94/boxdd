use boxdd::{
    shapes, BodyBuilder, DistanceJointDef, RevoluteJointDef, ShapeDef, Vec2, World, WorldDef,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
    let mut world = World::new(def)?;

    // Two dynamic boxes
    let a = world.create_body_id(BodyBuilder::new().position(Vec2::new(-1.0, 2.0)).build());
    let b = world.create_body_id(BodyBuilder::new().position(Vec2::new(1.0, 2.0)).build());
    let sdef = ShapeDef::builder().density(1.0).build();
    let _sa = world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5, 0.5));
    let _sb = world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));

    // Distance joint (springy)
    let wa = Vec2::new(-1.0, 2.0);
    let wb = Vec2::new(1.0, 2.0);
    let base = world.joint_base_from_world_points(a, b, wa, wb);
    let ddef = DistanceJointDef::new(base)
        .enable_spring(true)
        .hertz(4.0)
        .damping_ratio(0.7)
        .length_from_world_points(wa, wb);
    let _dj = world.create_distance_joint_id(&ddef);

    // Revolute joint (limits + motor)
    let base2 = world.joint_base_from_world_points(a, b, wa, wa);
    let rdef = RevoluteJointDef::new(base2)
        .limit_deg(-30.0, 30.0)
        .enable_motor(true)
        .motor_speed_deg(90.0)
        .max_motor_torque(10.0);
    let _rj = world.create_revolute_joint_id(&rdef);

    for _ in 0..60 {
        world.step(1.0 / 60.0, 4);
    }
    Ok(())
}
