use boxdd::prelude::*;

// Goal: stable non-zero contact begin/end and hit events.
// Strategy: two dynamic boxes moving directly towards each other along Y.
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, 0.0])
            .hit_event_threshold(0.2)
            .build()?,
    )?;

    // Two dynamic boxes that will collide head-on
    let b1 = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 2.0])
            .build()?,
    )?;
    let b2 = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.5])
            .build()?,
    )?;
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build()?;
    let _s1 = world.body(b1)?.create_polygon(
        &sdef,
        &shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry"),
    )?;
    let _s2 = world.body(b2)?.create_polygon(
        &sdef,
        &shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry"),
    )?;
    // Set velocities to ensure impact above the hit-event threshold
    world.body(b1)?.set_linear_velocity([0.0_f32, 2.0])?;
    world.body(b2)?.set_linear_velocity([0.0_f32, -2.0])?;

    let mut begin = 0usize;
    let mut end = 0usize;
    let mut hit = 0usize;
    for _ in 0..180 {
        let completed = world.step(1.0 / 60.0, 8)?;
        let contacts = completed.contact_events()?;
        begin += contacts.begin().len();
        end += contacts.end().len();
        hit += contacts.hit().len();
    }
    println!("contacts: begin={} end={} hit={}", begin, end, hit);
    Ok(())
}
