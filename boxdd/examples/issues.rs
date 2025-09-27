use boxdd::prelude::*;

// A grab-bag of smaller issue repros: filtering groups, sensor overlaps, safe joint destroy.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build())?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_segment_shape_for(
        ground,
        &ShapeDef::builder().build(),
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0]),
    );

    // 1) Sensor overlaps: a sensor area and multiple dynamic visitors
    let sensor_def = ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build();
    let _sensor = world.create_segment_shape_for(
        ground,
        &sensor_def,
        &shapes::segment([-3.0_f32, 0.5], [3.0, 0.5]),
    );
    for i in 0..10 {
        let x = -3.0 + i as f32 * 0.6;
        let id = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([x, 0.5_f32])
                .build(),
        );
        let _ = world.create_circle_shape_for(
            id,
            &ShapeDef::builder().density(1.0).build(),
            &shapes::circle([0.0_f32, 0.0], 0.2),
        );
    }

    // Step a bit, collect events
    let mut sensor_begin = 0usize;
    for _ in 0..240 {
        world.step(1.0 / 60.0, 4);
        let se = world.sensor_events();
        sensor_begin += se.begin.len();
    }

    println!("issues: sensor_begin={}", sensor_begin);
    Ok(())
}
