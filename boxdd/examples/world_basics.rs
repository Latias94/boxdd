use boxdd::prelude::*;

// Headless port of world-level tuning: toggles gravity/continuous/contact params and steps.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .enable_continuous(true)
            .contact_hertz(30.0)
            .contact_damping_ratio(0.7)
            .hit_event_threshold(1.0)
            .build(),
    )?;

    // A simple tower
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_segment_shape_for(
        ground,
        &ShapeDef::builder().build(),
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0]),
    );
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .build();
    let boxp = shapes::box_polygon(0.5, 0.5);
    let mut ids = Vec::new();
    for i in 0..6 {
        let id = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([0.0_f32, 0.5 + i as f32 * 1.05])
                .build(),
        );
        let _ = world.create_polygon_shape_for(id, &sdef, &boxp);
        ids.push(id);
    }

    // Step with gravity
    for _ in 0..120 {
        world.step(1.0 / 60.0, 4);
    }
    let y1 = world.body_position(*ids.last().unwrap()).y;

    // Flip gravity and step again
    world.set_gravity(Vec2::new(0.0, 10.0));
    for _ in 0..120 {
        world.step(1.0 / 60.0, 4);
    }
    let y2 = world.body_position(*ids.last().unwrap()).y;

    // Collect contact stats
    let ev = world.contact_events();
    println!(
        "world_basics: top_y1={:.2} top_y2={:.2} begin={} end={} hit={}",
        y1,
        y2,
        ev.begin.len(),
        ev.end.len(),
        ev.hit.len()
    );
    Ok(())
}
