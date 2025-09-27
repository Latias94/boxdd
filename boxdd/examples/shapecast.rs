use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Static ground and a static block
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _g = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(50.0, 1.0),
    );
    let block = world.create_body_id(BodyBuilder::new().position([2.0_f32, 2.0]).build());
    let _b = world.create_polygon_shape_for(
        block,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(0.7, 0.7),
    );

    // Proxy square to cast
    let square = [
        Vec2::new(-0.4, -0.4),
        Vec2::new(0.4, -0.4),
        Vec2::new(0.4, 0.4),
        Vec2::new(-0.4, 0.4),
    ];
    let trans = [3.0_f32, 0.0];
    let hits = world.cast_shape_points(square, 0.02, trans, QueryFilter::default());
    println!("shape cast hits: {}", hits.len());
    if let Some(min) = hits
        .iter()
        .map(|h| h.fraction)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
    {
        println!("earliest collision fraction: {:.3}", min);
    }
    Ok(())
}
