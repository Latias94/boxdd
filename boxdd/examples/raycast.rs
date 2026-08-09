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
    let _gs = world.body(ground)?.create_polygon(
        &ShapeDef::builder().density(0.0).build()?,
        &shapes::box_polygon(50.0, 1.0).expect("valid polygon geometry"),
    )?;

    // A dynamic box above ground
    let body = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position([0.0, 5.0])
            .build()?,
    )?;
    let _bs = world.body(body)?.create_polygon(
        &ShapeDef::builder().density(1.0).build()?,
        &shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry"),
    )?;

    for _ in 0..30 {
        drop(world.step(1.0 / 60.0, 4)?);
    }

    // Ray cast down from a grid of x positions
    let query = world.query()?;
    let mut total_hits = 0;
    for i in -3..=3 {
        let x = i as f32 * 0.7;
        let hit = query.cast_ray_closest(
            Position::new(WorldScalar::from(x), WorldScalar::from(10.0_f32)),
            [0.0, -20.0],
            QueryFilter::default(),
        )?;
        if let Some(hit) = hit {
            total_hits += 1;
            println!(
                "x={:+.2} -> hit at y={:.2}, frac={:.2}",
                x, hit.point.y, hit.fraction
            );
        }
    }
    println!("raycast hits: {}", total_hits);
    Ok(())
}
