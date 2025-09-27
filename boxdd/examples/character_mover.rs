use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Ground + step obstacle
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _g = world.create_polygon_shape_for(
        ground,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(50.0, 0.5),
    );
    let step = world.create_body_id(BodyBuilder::new().position([1.0_f32, 0.5]).build());
    let _s = world.create_polygon_shape_for(
        step,
        &ShapeDef::builder().density(0.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );

    // Character mover capsule
    let c1 = Vec2::new(0.0, 1.0);
    let c2 = Vec2::new(0.0, 1.8);
    let radius = 0.25;

    for _ in 0..10 {
        world.step(1.0 / 60.0, 4);
    }

    // Try to move right by 2 meters; report fraction
    let frac = world.cast_mover(c1, c2, radius, [2.0_f32, 0.0], QueryFilter::default());
    println!("character mover fraction: {:.3}", frac);
    Ok(())
}
