use boxdd::prelude::*;

// Goal: stable non-zero sensor begin/end + overlap counts.
// Strategy: place a wide horizontal sensor band and a dynamic circle above it so
// it reliably passes through under gravity. Use small dt and more substeps.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Horizontal sensor band at y = 1.5
    let sensor_body = world.create_body_id(BodyBuilder::new().position([0.0_f32, 1.5]).build());
    let sensor_def = ShapeDef::builder()
        .density(0.0)
        .sensor(true)
        .enable_sensor_events(true)
        .build();
    // Thicker band to avoid tunneling (half-height 0.3 => 0.6m thick)
    let sensor_shape =
        world.create_polygon_shape_for(sensor_body, &sensor_def, &shapes::box_polygon(2.0, 0.3));

    // Dynamic circle that falls through the band
    let mover = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 3.0])
            .build(),
    );
    let _ms = world.create_circle_shape_for(
        mover,
        &ShapeDef::builder()
            .density(1.0)
            .enable_sensor_events(true)
            .build(),
        &shapes::circle([0.0_f32, 0.0], 0.25),
    );

    let mut begin = 0usize;
    let mut end = 0usize;
    let mut overlaps_total = 0usize;
    for _ in 0..180 {
        world.step(1.0 / 120.0, 8);
        let ev = world.sensor_events();
        begin += ev.begin.len();
        end += ev.end.len();
        overlaps_total += world.shape_sensor_overlaps(sensor_shape).len();
    }
    println!(
        "sensors: begin={} end={} overlaps_sum={} (over frames)",
        begin, end, overlaps_total
    );
    Ok(())
}
