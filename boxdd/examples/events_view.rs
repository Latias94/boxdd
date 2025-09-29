//! Demonstrates zero-copy event views without exposing raw FFI types.
use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .enable_continuous(true)
            .hit_event_threshold(0.2)
            .worker_count(1)
            .build(),
    )?;

    // Ground + sensor
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_segment_shape_for(
        ground,
        &ShapeDef::default(),
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0]),
    );
    let sensor_def = ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build();
    let _sensor = world.create_segment_shape_for(
        ground,
        &sensor_def,
        &shapes::segment([-5.0_f32, 1.0], [5.0, 1.0]),
    );

    // Dynamic bodies to generate contact/hit events
    let dyn_def = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();
    let a = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-0.5, 3.0])
            .build(),
    );
    let b = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.5, 4.2])
            .build(),
    );
    let _ = world.create_polygon_shape_for(a, &dyn_def, &shapes::box_polygon(0.4, 0.4));
    let _ = world.create_polygon_shape_for(b, &dyn_def, &shapes::box_polygon(0.4, 0.4));

    for _ in 0..60 {
        world.step(1.0 / 60.0, 8);

        // Zero-copy views
        world.with_body_events_view(|moves| {
            for m in moves {
                let _ = (m.body_id(), m.fell_asleep());
            }
        });
        world.with_sensor_events_view(|beg, end| {
            let _ = (beg.count(), end.count());
        });
        world.with_contact_events_view(|b, e, h| {
            let _ = (b.count(), e.count(), h.count());
        });
        world.with_joint_events_view(|j| {
            let _ = j.count();
        });
    }
    Ok(())
}
