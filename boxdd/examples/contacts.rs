use boxdd::prelude::*;

// Goal: stable non-zero contact begin/end and hit events.
// Strategy: two dynamic boxes moving directly towards each other along Y.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0, 0.0])
            .hit_event_threshold(0.2)
            .build(),
    )?;

    // Two dynamic boxes that will collide head-on
    let b1 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 2.0])
            .build(),
    );
    let b2 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.5])
            .build(),
    );
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();
    let _s1 = world.create_polygon_shape_for(b1, &sdef, &shapes::box_polygon(0.5, 0.5));
    let _s2 = world.create_polygon_shape_for(b2, &sdef, &shapes::box_polygon(0.5, 0.5));
    // Set velocities to ensure impact above the hit-event threshold
    world.set_body_linear_velocity(b1, [0.0_f32, 2.0]);
    world.set_body_linear_velocity(b2, [0.0_f32, -2.0]);

    let mut begin = 0usize;
    let mut end = 0usize;
    let mut hit = 0usize;
    for _ in 0..180 {
        world.step(1.0 / 60.0, 8);
        let c = world.contact_events();
        begin += c.begin.len();
        end += c.end.len();
        hit += c.hit.len();
    }
    println!("contacts: begin={} end={} hit={}", begin, end, hit);
    Ok(())
}
