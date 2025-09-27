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

    // A dynamic box above ground
    let body = world.create_body_id(BodyBuilder::new().position([0.0, 5.0]).build());
    let _bs = world.create_polygon_shape_for(
        body,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );

    for _ in 0..30 {
        world.step(1.0 / 60.0, 4);
    }

    // Ray cast down from a grid of x positions
    let mut total_hits = 0;
    for i in -3..=3 {
        let x = i as f32 * 0.7;
        let hit = world.cast_ray_closest([x, 10.0_f32], [0.0, -20.0], QueryFilter::default());
        if hit.hit {
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
