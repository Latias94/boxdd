use boxdd::prelude::*;
use std::time::Instant;

// Headless micro-benchmark similar to sample_benchmark: build a moderate scene and time stepping.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cols = 25usize;
    let rows = 15usize;
    let steps = 300usize;
    let sub_steps = 8i32;

    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .worker_count(1)
            .build(),
    )?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_segment_shape_for(
        ground,
        &ShapeDef::builder().build(),
        &shapes::segment([-100.0_f32, 0.0], [100.0, 0.0]),
    );

    // Stack of boxes
    let box_poly = shapes::box_polygon(0.5, 0.5);
    let sdef = ShapeDef::builder().density(1.0).build();
    for i in 0..rows {
        for j in 0..cols {
            let x = -((cols as f32) * 0.55) + (j as f32) * 1.1;
            let y = 0.5 + (i as f32) * 1.05 + 2.0;
            let b = world.create_body_id(
                BodyBuilder::new()
                    .body_type(BodyType::Dynamic)
                    .position([x, y])
                    .build(),
            );
            let _ = world.create_polygon_shape_for(b, &sdef, &box_poly);
        }
    }

    let start = Instant::now();
    for _ in 0..steps {
        world.step(1.0 / 60.0, sub_steps);
    }
    let dt = start.elapsed();
    let avg_ms = dt.as_secs_f64() * 1000.0 / (steps as f64);
    let c = world.counters();
    println!(
        "benchmark: bodies={} shapes={} contacts={} joints={} steps={} sub={} avg_ms_per_step={:.3}",
        c.body_count, c.shape_count, c.contact_count, c.joint_count, steps, sub_steps, avg_ms
    );
    Ok(())
}
