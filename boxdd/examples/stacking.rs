use boxdd::prelude::*;

// Rough port of "Stacking": build columns of boxes on the ground.
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, -10.0])
            .build()?,
    )?;

    // Ground
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _ground_shape = world.body(ground)?.create_segment(
        &ShapeDef::builder().build()?,
        &shapes::segment([-30.0_f32, 0.0], [30.0, 0.0])?,
    )?;

    let cols = 7usize;
    let rows = 10usize;
    let sdef = ShapeDef::builder().density(1.0).build()?;
    let boxp = shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry");

    let mut created = 0usize;
    for i in 0..cols {
        for j in 0..rows {
            let x = -10.0 + i as f32 * 3.0;
            let y = 0.5 + j as f32 * 1.05;
            let b = world.create_body(
                BodyBuilder::from(foundation.body_def())
                    .body_type(BodyType::Dynamic)
                    .position([x, y])
                    .build()?,
            )?;
            let _ = world.body(b)?.create_polygon(&sdef, &boxp)?;
            created += 1;
        }
    }

    for _ in 0..360 {
        drop(world.step(1.0 / 60.0, 8)?);
    }
    println!("stacking: {} boxes", created);
    Ok(())
}
