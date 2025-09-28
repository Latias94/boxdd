use boxdd::{
    BodyBuilder, PrismaticJointDef, ShapeDef, Vec2, WheelJointDef, World, WorldDef, shapes,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
    let mut world = World::new(def)?;

    // Ground
    let ground_id = world.create_body_id(BodyBuilder::new().build());
    let sdef = ShapeDef::builder().density(0.0).build();
    let _gs = world.create_polygon_shape_for(ground_id, &sdef, &shapes::box_polygon(10.0, 0.5));

    // Two bodies
    let id_a = world.create_body_id(BodyBuilder::new().position(Vec2::new(-1.0, 2.0)).build());
    let id_b = world.create_body_id(BodyBuilder::new().position(Vec2::new(1.0, 2.0)).build());
    // Build frames from world axis X (1,0) and anchors using safe helpers
    let axis_world = Vec2::new(1.0, 0.0);
    let anchor_a_world = Vec2::new(-1.0, 2.0);
    let anchor_b_world = Vec2::new(1.0, 2.0);
    let base = world.joint_base_from_world_with_axis(
        id_a,
        id_b,
        anchor_a_world,
        anchor_b_world,
        axis_world,
    );

    let pdef = PrismaticJointDef::new(base.clone())
        .enable_limit(true)
        .lower_translation(-0.5)
        .upper_translation(0.5);
    let wdef = WheelJointDef::new(base)
        .enable_spring(true)
        .hertz(4.0)
        .damping_ratio(0.7);

    let _pj = world.create_prismatic_joint_id(&pdef);
    let _wj = world.create_wheel_joint_id(&wdef);

    for _ in 0..60 {
        world.step(1.0 / 60.0, 4);
    }
    Ok(())
}
