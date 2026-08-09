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

    // Kinematic platform moving horizontally
    let platform = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Kinematic)
            .position([0.0_f32, 3.0])
            .build()?,
    )?;
    let _ps = world.body(platform)?.create_polygon(
        &ShapeDef::builder().density(0.0).build()?,
        &shapes::box_polygon(2.0, 0.2).expect("valid polygon geometry"),
    )?;
    world.body(platform)?.set_linear_velocity([1.5_f32, 0.0])?;

    // Dynamic box on top
    let box_id = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position([0.0_f32, 4.0])
            .build()?,
    )?;
    let _bs = world.body(box_id)?.create_polygon(
        &ShapeDef::builder().density(1.0).build()?,
        &shapes::box_polygon(0.3, 0.3).expect("valid polygon geometry"),
    )?;

    for _ in 0..240 {
        drop(world.step(1.0 / 60.0, 4)?);
    }
    let p = world.body(box_id)?.position()?;
    println!("box after ride: ({:.2}, {:.2})", p.x, p.y);
    Ok(())
}
