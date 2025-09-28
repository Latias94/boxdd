use boxdd::prelude::*;

// Consolidated events example: body moves, sensor begin/end, contact begin/end/hit, joint events.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tune world to generate hit events more easily and keep single-threaded for determinism
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .enable_continuous(true)
            .hit_event_threshold(0.2)
            .worker_count(1)
            .build(),
    )?;

    // Ground + a horizontal sensor segment at y=1
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_segment_shape_for(
        ground,
        &ShapeDef::builder().build(),
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0]),
    );
    let sensor_def = ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build();
    let _sensor = world.create_segment_shape_for(
        ground,
        &sensor_def,
        &shapes::segment([-5.0_f32, 1.0], [5.0, 1.0]),
    );

    // Two dynamic boxes to collide and produce contacts + hits
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();
    let a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-0.5_f32, 4.0])
            .build(),
    );
    let b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.5_f32, 6.0])
            .build(),
    );
    let _ = world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.4, 0.4));
    let _ = world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.4, 0.4));

    // No joints needed; focus on body/sensor/contact/hit events.

    // Step and collect events
    let mut moves = 0usize;
    let mut sens_beg = 0usize;
    let mut sens_end = 0usize;
    let mut con_beg = 0usize;
    let mut con_end = 0usize;
    let mut con_hit = 0usize;
    let mut joint_ev = 0usize;
    for _ in 0..240 {
        world.step(1.0 / 60.0, 8);
        moves += world.body_events().len();
        let se = world.sensor_events();
        sens_beg += se.begin.len();
        sens_end += se.end.len();
        let ce = world.contact_events();
        con_beg += ce.begin.len();
        con_end += ce.end.len();
        con_hit += ce.hit.len();
        joint_ev += world.joint_events().len();
    }
    let c = world.counters();
    println!(
        "events_summary: move={} sensor(b={},e={}) contact(b={},e={},hit={}) joints={} counters bodies={} shapes={} contacts={} joints={} islands={}",
        moves,
        sens_beg,
        sens_end,
        con_beg,
        con_end,
        con_hit,
        joint_ev,
        c.body_count,
        c.shape_count,
        c.contact_count,
        c.joint_count,
        c.island_count
    );
    Ok(())
}
