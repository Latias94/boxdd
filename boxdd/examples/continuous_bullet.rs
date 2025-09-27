// Continuous collision (bullet) example
//
// Tuning tips
// - Enable continuous physics: world.enable_continuous(true)
// - Lower hit event threshold (approach speed): world.set_hit_event_threshold(small)
// - Use small time step and more sub-steps to improve TOI resolution
// - Enable hit/contact events on both bullet and target shapes
// - If still no hits: increase bullet radius/velocity or thicken the wall
use boxdd::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, 0.0]).build())?;
    // Enable continuous physics for high-speed bullets and lower hit-event threshold
    world.enable_continuous(true);
    world.set_hit_event_threshold(0.001);

    // Vertical wall at x = 5
    let wall = world.create_body_id(BodyBuilder::new().position([5.0_f32, 0.0]).build());
    let wall_def = ShapeDef::builder()
        .density(0.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();
    let _wshape = world.create_polygon_shape_for(wall, &wall_def, &shapes::box_polygon(0.5, 3.0));

    // Bullet: dynamic circle moving fast towards wall
    let bullet = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 0.0])
            .bullet(true)
            .build(),
    );
    let sdef = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();
    let circ = shapes::circle([0.0_f32, 0.0], 0.3);
    let _bshape = world.create_circle_shape_for(bullet, &sdef, &circ);
    world.set_body_linear_velocity(bullet, [60.0_f32, 0.0]);

    let mut hit_frames = 0;
    let mut begin_frames = 0;
    let mut hit_events = 0usize;
    let steps = 240;
    for _ in 0..steps {
        world.step(1.0 / 240.0, 16);
        let ev = world.contact_events();
        if !ev.begin.is_empty() {
            begin_frames += 1;
        }
        hit_events += ev.hit.len();
    }
    println!(
        "bullet: begin_frames={} hit_events={} (steps={})",
        begin_frames, hit_events, steps
    );
    Ok(())
}
