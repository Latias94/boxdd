use boxdd::prelude::*;

// A grab-bag of smaller issue repros: filtering groups, sensor overlaps, safe joint destroy.
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0_f32, -10.0])
            .build()?,
    )?;

    // Ground
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _ = world.body(ground)?.create_segment(
        &ShapeDef::builder().build()?,
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0])?,
    )?;

    // 1) Sensor overlaps: a sensor area and multiple dynamic visitors
    let sensor_def = ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build()?;
    let _sensor = world
        .body(ground)?
        .create_segment(&sensor_def, &shapes::segment([-3.0_f32, 0.5], [3.0, 0.5])?)?;
    for i in 0..10 {
        let x = -3.0 + i as f32 * 0.6;
        let id = world.create_body(
            BodyBuilder::from(foundation.body_def())
                .body_type(BodyType::Dynamic)
                .position([x, 0.5_f32])
                .build()?,
        )?;
        let _ = world.body(id)?.create_circle(
            &ShapeDef::builder().density(1.0).build()?,
            &shapes::circle([0.0_f32, 0.0], 0.2)?,
        )?;
    }

    // Step a bit, collect events
    let mut sensor_begin = 0usize;
    for _ in 0..240 {
        let completed = world.step(1.0 / 60.0, 4)?;
        sensor_begin += completed.sensor_events()?.begin().len();
    }

    println!("issues: sensor_begin={}", sensor_begin);
    Ok(())
}
