use boxdd::prelude::*;
use std::time::Instant;

// Headless micro-benchmark similar to sample_benchmark: build a moderate scene and time stepping.
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let cols = 25usize;
    let rows = 15usize;
    let steps = 300usize;
    let sub_steps = 8i32;

    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0_f32, -10.0])
            .build()?,
    )?;

    // Ground
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _ = world.body(ground)?.create_segment(
        &ShapeDef::builder().build()?,
        &shapes::segment([-100.0_f32, 0.0], [100.0, 0.0])?,
    )?;

    // Stack of boxes
    let box_poly = shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry");
    let sdef = ShapeDef::builder().density(1.0).build()?;
    for i in 0..rows {
        for j in 0..cols {
            let x = -((cols as f32) * 0.55) + (j as f32) * 1.1;
            let y = 0.5 + (i as f32) * 1.05 + 2.0;
            let b = world.create_body(
                BodyBuilder::from(foundation.body_def())
                    .body_type(BodyType::Dynamic)
                    .position([x, y])
                    .build()?,
            )?;
            let _ = world.body(b)?.create_polygon(&sdef, &box_poly)?;
        }
    }

    let start = Instant::now();
    for _ in 0..steps {
        drop(world.step(1.0 / 60.0, sub_steps)?);
    }
    let dt = start.elapsed();
    let avg_ms = dt.as_secs_f64() * 1000.0 / (steps as f64);
    let c = world.counters()?;
    println!(
        "benchmark: bodies={} shapes={} contacts={} joints={} steps={} sub={} avg_ms_per_step={:.3}",
        c.body_count, c.shape_count, c.contact_count, c.joint_count, steps, sub_steps, avg_ms
    );
    Ok(())
}
