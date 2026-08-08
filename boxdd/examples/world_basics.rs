use boxdd::prelude::*;

// Headless port of world-level tuning: toggles gravity/continuous/contact params and steps.
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0_f32, -10.0])
            .enable_continuous(true)
            .contact_hertz(30.0)
            .contact_damping_ratio(0.7)
            .hit_event_threshold(1.0)
            .build()?,
    )?;

    // A simple tower
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _ = world.body(ground)?.create_segment(
        &ShapeDef::builder().build()?,
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0])?,
    )?;
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .build()?;
    let boxp = shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry");
    let mut ids = Vec::with_capacity(6);
    for i in 0..6 {
        let id = world.create_body(
            BodyBuilder::from(foundation.body_def())
                .body_type(BodyType::Dynamic)
                .position([0.0_f32, 0.5 + i as f32 * 1.05])
                .build()?,
        )?;
        let _ = world.body(id)?.create_polygon(&sdef, &boxp)?;
        ids.push(id);
    }

    // Step with gravity
    for _ in 0..120 {
        drop(world.step(1.0 / 60.0, 4)?);
    }
    // Safe: ids has 6 bodies pushed above
    let y1 = world.body(*ids.last().unwrap())?.position()?.y;

    // Flip gravity and step again
    world.set_gravity(Vec2::new(0.0, 10.0))?;
    let mut begin = 0usize;
    let mut end = 0usize;
    let mut hit = 0usize;
    for _ in 0..120 {
        let completed = world.step(1.0 / 60.0, 4)?;
        let contact = completed.contact_events()?;
        begin += contact.begin().len();
        end += contact.end().len();
        hit += contact.hit().len();
    }
    // Safe: ids remains non-empty
    let y2 = world.body(*ids.last().unwrap())?.position()?.y;

    println!(
        "world_basics: top_y1={:.2} top_y2={:.2} begin={} end={} hit={}",
        y1, y2, begin, end, hit
    );
    Ok(())
}
