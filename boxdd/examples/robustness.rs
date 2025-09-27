use boxdd::prelude::*;

// Robustness-oriented headless sample: bullets, mass ratios, slender stacks.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Continuous on for bullet tests
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .enable_continuous(true)
            .hit_event_threshold(0.05)
            .build(),
    )?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_segment_shape_for(
        ground,
        &ShapeDef::builder().build(),
        &shapes::segment([-40.0_f32, 0.0], [40.0, 0.0]),
    );

    // Thin wall made of segments at x=10
    let wall = world.create_body_id(BodyBuilder::new().build());
    for i in 0..20 {
        let y1 = i as f32 * 0.5;
        let y2 = y1 + 0.5;
        let seg = shapes::segment([10.0_f32, y1], [10.0_f32, y2]);
        let _ = world.create_segment_shape_for(
            wall,
            &ShapeDef::builder().enable_hit_events(true).build(),
            &seg,
        );
    }

    // Bullet: fast circle aimed at the wall
    let bullet = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 5.0])
            .bullet(true)
            .build(),
    );
    let _ = world.create_circle_shape_for(
        bullet,
        &ShapeDef::builder()
            .density(1.0)
            .enable_hit_events(true)
            .build(),
        &shapes::circle([0.0_f32, 0.0], 0.25),
    );
    world.set_body_linear_velocity(bullet, [80.0_f32, 0.0]);

    let mut hit = 0usize;
    for _ in 0..240 {
        world.step(1.0 / 240.0, 8);
        hit += world.contact_events().hit.len();
    }

    // Mass ratio: heavy on light plank
    let plank = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([5.0_f32, 1.0])
            .build(),
    );
    let _ = world.create_polygon_shape_for(
        plank,
        &ShapeDef::builder().density(0.1).build(),
        &shapes::box_polygon(2.0, 0.1),
    );
    let heavy = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([5.0_f32, 6.0])
            .build(),
    );
    let _ = world.create_polygon_shape_for(
        heavy,
        &ShapeDef::builder().density(50.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );
    for _ in 0..240 {
        world.step(1.0 / 60.0, 8);
    }
    let p_plank = world.body_position(plank);
    let p_heavy = world.body_position(heavy);

    // Slender stack
    let sdef = ShapeDef::builder().density(1.0).build();
    let mut upright = 0usize;
    let mut ids = Vec::new();
    for i in 0..10 {
        let id = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([-10.0_f32, 0.5 + i as f32 * 2.1])
                .build(),
        );
        let _ = world.create_polygon_shape_for(id, &sdef, &shapes::box_polygon(0.1, 1.0));
        ids.push(id);
    }
    for _ in 0..240 {
        world.step(1.0 / 60.0, 8);
    }
    for &b in &ids {
        let p = world.body_position(b);
        if p.y > 0.3 {
            upright += 1;
        }
    }

    println!(
        "robustness: bullet_hits={} plank_y={:.2} heavy_y={:.2} slender_upright={}",
        hit, p_plank.y, p_heavy.y, upright
    );
    Ok(())
}
