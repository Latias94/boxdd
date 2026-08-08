use boxdd::prelude::*;

// Goal: stable non-zero sensor begin/end + overlap counts.
// Strategy: place a wide horizontal sensor band and a dynamic circle above it so
// it reliably passes through under gravity. Use small dt and more substeps.
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, -10.0])
            .build()?,
    )?;

    // Horizontal sensor band at y = 1.5
    let sensor_body = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position([0.0_f32, 1.5])
            .build()?,
    )?;
    let sensor_def = ShapeDef::builder()
        .density(0.0)
        .sensor(true)
        .enable_sensor_events(true)
        .build()?;
    // Thicker band to avoid tunneling (half-height 0.3 => 0.6m thick)
    let sensor_shape = world.body(sensor_body)?.create_polygon(
        &sensor_def,
        &shapes::box_polygon(2.0, 0.3).expect("valid polygon geometry"),
    )?;

    // Dynamic circle that falls through the band
    let mover = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.0])
            .build()?,
    )?;
    let _ms = world.body(mover)?.create_circle(
        &ShapeDef::builder()
            .density(1.0)
            .enable_sensor_events(true)
            .build()?,
        &shapes::circle([0.0_f32, 0.0], 0.25)?,
    )?;

    let mut begin = 0usize;
    let mut end = 0usize;
    let mut overlaps_total = 0usize;
    for _ in 0..180 {
        let completed = world.step(1.0 / 120.0, 8)?;
        let sensor_events = completed.sensor_events()?;
        begin += sensor_events.begin().len();
        end += sensor_events.end().len();
        drop(completed);
        overlaps_total += world.shape(sensor_shape)?.sensor_overlaps()?.len();
    }
    println!(
        "sensors: begin={} end={} overlaps_sum={} (over frames)",
        begin, end, overlaps_total
    );
    Ok(())
}
