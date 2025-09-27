use boxdd::prelude::*;

// Rough port of "Stacking": build columns of boxes on the ground.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ground_shape = world.create_segment_shape_for(
        ground,
        &ShapeDef::builder().build(),
        &shapes::segment([-30.0_f32, 0.0], [30.0, 0.0]),
    );

    let cols = 7usize;
    let rows = 10usize;
    let sdef = ShapeDef::builder().density(1.0).build();
    let boxp = shapes::box_polygon(0.5, 0.5);

    let mut created = 0usize;
    for i in 0..cols {
        for j in 0..rows {
            let x = -10.0 + i as f32 * 3.0;
            let y = 0.5 + j as f32 * 1.05;
            let b = world.create_body_id(
                BodyBuilder::new()
                    .body_type(BodyType::Dynamic)
                    .position([x, y])
                    .build(),
            );
            let _ = world.create_polygon_shape_for(b, &sdef, &boxp);
            created += 1;
        }
    }

    for _ in 0..360 {
        world.step(1.0 / 60.0, 8);
    }
    println!("stacking: {} boxes", created);
    Ok(())
}
