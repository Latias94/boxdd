use boxdd::prelude::*;

// Consolidated events example: body moves, sensor begin/end, contact begin/end/hit, joint events.
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    // Tune world to generate hit events more easily and keep single-threaded for determinism
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0_f32, -10.0])
            .enable_continuous(true)
            .hit_event_threshold(0.2)
            .build()?,
    )?;

    // Ground + a horizontal sensor segment at y=1
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _ = world.body(ground)?.create_segment(
        &ShapeDef::builder().build()?,
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0])?,
    )?;
    let sensor_def = ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build()?;
    let _sensor = world
        .body(ground)?
        .create_segment(&sensor_def, &shapes::segment([-5.0_f32, 1.0], [5.0, 1.0])?)?;

    // Two dynamic boxes to collide and produce contacts + hits
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build()?;
    let a = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([-0.5_f32, 4.0])
            .build()?,
    )?;
    let b = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([0.5_f32, 6.0])
            .build()?,
    )?;
    let _ = world.body(a)?.create_polygon(
        &sdef,
        &shapes::box_polygon(0.4, 0.4).expect("valid polygon geometry"),
    )?;
    let _ = world.body(b)?.create_polygon(
        &sdef,
        &shapes::box_polygon(0.4, 0.4).expect("valid polygon geometry"),
    )?;

    // No joints needed; focus on body/sensor/contact/hit events.

    // Step and materialize only the event families used by this frame.
    let mut moves = 0usize;
    let mut sens_beg = 0usize;
    let mut sens_end = 0usize;
    let mut con_beg = 0usize;
    let mut con_end = 0usize;
    let mut con_hit = 0usize;
    let mut joint_ev = 0usize;
    for _ in 0..240 {
        let completed = world.step(1.0 / 60.0, 8)?;
        moves += completed.body_events()?.len();
        let sensor = completed.sensor_events()?;
        sens_beg += sensor.begin().len();
        sens_end += sensor.end().len();
        let contact = completed.contact_events()?;
        con_beg += contact.begin().len();
        con_end += contact.end().len();
        con_hit += contact.hit().len();
        joint_ev += completed.joint_events()?.len();
    }
    let c = world.counters()?;
    println!(
        "events_summary_into: move={} sensor(b={},e={}) contact(b={},e={},hit={}) joints={} counters bodies={} shapes={} contacts={} joints={} islands={}",
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
