use boxdd::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, -10.0])
            .build()?,
    )?;

    // Static ground and a static block
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _g = world.body(ground)?.create_polygon(
        &ShapeDef::builder().density(0.0).build()?,
        &shapes::box_polygon(50.0, 1.0)?,
    )?;
    let block = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position([2.0_f32, 2.0])
            .build()?,
    )?;
    let _b = world.body(block)?.create_polygon(
        &ShapeDef::builder().density(0.0).build()?,
        &shapes::box_polygon(0.7, 0.7)?,
    )?;

    // Proxy square to cast
    let square = ShapeProxy::new(
        [
            Vec2::new(-0.4, -0.4),
            Vec2::new(0.4, -0.4),
            Vec2::new(0.4, 0.4),
            Vec2::new(-0.4, 0.4),
        ],
        0.02,
    )?;
    let trans = [3.0_f32, 0.0];
    let hits = world
        .query()?
        .cast_shape(Position::ZERO, square, trans, QueryFilter::default())?;
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
