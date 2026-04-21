// Donut (ring of segments)
//
// Notes
// - Weld neighbors with crate-owned joint frames computed from body rotations.
// - Keep the ring self-collision disabled with a negative filter group.
use boxdd::prelude::*;

// Port of the Donut helper: a ring of capsule bodies welded end-to-end.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    let sides = 16usize;
    let scale = 1.0_f32;
    let radius = 1.0_f32 * scale;
    let delta = std::f32::consts::TAU / (sides as f32);
    let length = std::f32::consts::TAU * radius / (sides as f32);

    // Capsule spanning the chord length with a small radius
    let cap = shapes::capsule([0.0_f32, -0.5 * length], [0.0, 0.5 * length], 0.25 * scale);

    // Common body/shape defs
    let mut bodies: Vec<BodyId> = Vec::with_capacity(sides);
    let bdef = BodyBuilder::new().body_type(BodyType::Dynamic);
    let filt = boxdd::filter::Filter {
        group_index: -1,
        ..Default::default()
    };
    let sdef = ShapeDef::builder()
        .material(SurfaceMaterial::default().with_friction(0.3))
        .filter(filt)
        .build();

    // Create bodies around the circle
    for i in 0..sides {
        let angle = (i as f32) * delta;
        let pos = [radius * angle.cos(), radius * angle.sin()];
        let id = world.create_body_id(bdef.clone().position(pos).angle(angle).build());
        let _ = world.create_capsule_shape_for(id, &sdef, &cap);
        bodies.push(id);
    }

    // Weld neighbors at capsule end points (aligned with upstream)
    for i in 0..sides {
        let prev = if i == 0 { sides - 1 } else { i - 1 };
        let a = bodies[prev];
        let b = bodies[i];
        let relative_angle = world.body_rotation(b).angle() - world.body_rotation(a).angle();
        let base = JointBaseBuilder::new()
            .bodies_by_id(a, b)
            .local_frames(
                [0.0_f32, 0.5 * length],
                relative_angle,
                [0.0_f32, -0.5 * length],
                0.0,
            )
            .build();
        let wdef = WeldJointDef::new(base)
            .angular_hertz(5.0)
            .angular_damping_ratio(0.0);
        let _ = world.create_weld_joint_id(&wdef);
    }

    for _ in 0..240 {
        world.step(1.0 / 60.0, 8);
    }
    println!(
        "donut: sides={} pos_first=({:.2},{:.2})",
        sides,
        world.body_position(bodies[0]).x,
        world.body_position(bodies[0]).y
    );
    Ok(())
}
