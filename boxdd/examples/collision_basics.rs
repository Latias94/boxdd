use boxdd::prelude::*;

// Simple collision/events demo: two boxes collide, print contact count.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_segment_shape_for(
        ground,
        &ShapeDef::builder().build(),
        &shapes::segment([-10.0_f32, 0.0], [10.0, 0.0]),
    );

    // Dynamic boxes with contact events enabled
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .build();
    let b1 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 6.0])
            .build(),
    );
    let _ = world.create_polygon_shape_for(b1, &sdef, &shapes::box_polygon(0.5, 0.5));

    let b2 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.6_f32, 8.5])
            .build(),
    );
    let _ = world.create_polygon_shape_for(b2, &sdef, &shapes::box_polygon(0.5, 0.5));

    // Run and collect contact events
    let mut begin = 0usize;
    let mut end = 0usize;
    for _ in 0..240 {
        world.step(1.0 / 60.0, 4);
        let ev = world.contact_events();
        begin += ev.begin.len();
        end += ev.end.len();
    }
    println!("collision_basics: begin={} end={}", begin, end);
    Ok(())
}
