use boxdd::{BodyBuilder, DistanceJointDef, ShapeDef, Vec2, shapes};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    // World
    let def = boxdd::WorldBuilder::from(foundation.world_def())
        .gravity(Vec2::new(0.0, -9.8))
        .build()?;
    let mut world = foundation.create_world(def)?;

    // Ground body (static)
    let ground_id = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let ground_shape = shapes::box_polygon(10.0, 0.5).expect("valid polygon geometry");
    let sdef = ShapeDef::builder().density(0.0).build()?;
    let _gs = world
        .body(ground_id)?
        .create_polygon(&sdef, &ground_shape)?;

    // Dynamic bodies (create then immediately leak wrappers to release &mut world borrow)
    let b1 = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position(Vec2::new(0.0, 2.0))
            .build()?,
    )?;
    let bshape = shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry");
    let sdef_dyn = ShapeDef::builder().density(1.0).build()?;
    let id1 = b1;
    let _s1 = world.body(id1)?.create_polygon(&sdef_dyn, &bshape)?;

    let b2 = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position(Vec2::new(1.0, 2.0))
            .build()?,
    )?;
    let id2 = b2;
    let _s2 = world.body(id2)?.create_polygon(&sdef_dyn, &bshape)?;

    // Simple distance joint between body origins
    let base = world.joint_base(id1, id2)?;
    let ddef = DistanceJointDef::new(base)
        .length(1.0)
        .enable_spring(true)
        .hertz(4.0)
        .damping_ratio(0.7);
    let _dj = world.create_distance_joint(&ddef)?;

    for _ in 0..60 {
        drop(world.step(1.0 / 60.0, 4)?);
    }
    Ok(())
}
