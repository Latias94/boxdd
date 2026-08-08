//! Demonstrates zero-copy event views without exposing raw FFI types.
use boxdd::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0_f32, -10.0])
            .enable_continuous(true)
            .hit_event_threshold(0.2)
            .build()?,
    )?;

    // Ground + sensor
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _ = world.body(ground)?.create_segment(
        &ShapeDef::default(),
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0])?,
    )?;
    let sensor_def = ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build()?;
    let _sensor = world
        .body(ground)?
        .create_segment(&sensor_def, &shapes::segment([-5.0_f32, 1.0], [5.0, 1.0])?)?;

    // Dynamic bodies to generate contact/hit events
    let dyn_def = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build()?;
    let a = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([-0.5, 3.0])
            .build()?,
    )?;
    let b = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([0.5, 4.2])
            .build()?,
    )?;
    let _ = world.body(a)?.create_polygon(
        &dyn_def,
        &shapes::box_polygon(0.4, 0.4).expect("valid event shape"),
    )?;
    let _ = world.body(b)?.create_polygon(
        &dyn_def,
        &shapes::box_polygon(0.4, 0.4).expect("valid event shape"),
    )?;

    let mut move_count = 0usize;
    let mut sleep_transitions = 0usize;
    let mut sensor_begin = 0usize;
    let mut sensor_end = 0usize;
    let mut contact_begin = 0usize;
    let mut contact_end = 0usize;
    let mut contact_hit = 0usize;
    let mut joint_count = 0usize;

    for _ in 0..60 {
        let completed = world.step(1.0 / 60.0, 8)?;

        let moves = completed.body_events()?;
        for event in &moves {
            let _ = event.body_id;
            move_count += 1;
            sleep_transitions += usize::from(event.fell_asleep);
        }
        let sensors = completed.sensor_events()?;
        sensor_begin += sensors.begin().len();
        sensor_end += sensors.end().len();
        let contacts = completed.contact_events()?;
        contact_begin += contacts.begin().len();
        contact_end += contacts.end().len();
        contact_hit += contacts.hit().len();
        joint_count += completed.joint_events()?.len();
    }
    println!(
        "events_view: move={} asleep={} sensor(b={},e={}) contact(b={},e={},hit={}) joints={}",
        move_count,
        sleep_transitions,
        sensor_begin,
        sensor_end,
        contact_begin,
        contact_end,
        contact_hit,
        joint_count
    );
    Ok(())
}
