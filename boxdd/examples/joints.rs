use boxdd::{
    BodyBuilder, DistanceJointDef, JointBaseBuilder, ShapeDef, Vec2, World, WorldDef, shapes,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // World
    let def = WorldDef::builder().gravity(Vec2::new(0.0, -9.8)).build();
    let mut world = World::new(def)?;

    // Ground body (static)
    let ground_id = world.create_body_id(BodyBuilder::new().build());
    let ground_shape = shapes::box_polygon(10.0, 0.5);
    let sdef = ShapeDef::builder().density(0.0).build();
    let _gs = world.create_polygon_shape_for(ground_id, &sdef, &ground_shape);

    // Dynamic bodies (create then immediately leak wrappers to release &mut world borrow)
    let b1 = world.create_body_id(BodyBuilder::new().position(Vec2::new(0.0, 2.0)).build());
    let bshape = shapes::box_polygon(0.5, 0.5);
    let sdef_dyn = ShapeDef::builder().density(1.0).build();
    let id1 = b1;
    let _s1 = world.create_polygon_shape_for(id1, &sdef_dyn, &bshape);

    let b2 = world.create_body_id(BodyBuilder::new().position(Vec2::new(1.0, 2.0)).build());
    let id2 = b2;
    let _s2 = world.create_polygon_shape_for(id2, &sdef_dyn, &bshape);

    // Simple distance joint between body origins
    let base = JointBaseBuilder::new().bodies_by_id(id1, id2).build();
    let ddef = DistanceJointDef::new(base)
        .length(1.0)
        .enable_spring(true)
        .hertz(4.0)
        .damping_ratio(0.7);
    let _dj = world.create_distance_joint_id(&ddef);

    for _ in 0..60 {
        world.step(1.0 / 60.0, 4);
    }
    Ok(())
}
