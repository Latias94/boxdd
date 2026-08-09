use boxdd::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, -10.0])
            .build()?,
    )?;

    // Ground
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _gs = world.body(ground)?.create_polygon(
        &ShapeDef::builder().density(0.0).build()?,
        &shapes::box_polygon(50.0, 1.0).expect("valid polygon geometry"),
    )?;

    // Bridge planks
    let plank_half = Vec2::new(1.0, 0.125);
    let plank_poly =
        shapes::box_polygon(plank_half.x, plank_half.y).expect("valid polygon geometry");
    let sdef = ShapeDef::builder().density(1.0).build()?;

    let plank_count: usize = 20;
    let start_x = -(plank_count as f32) * plank_half.x;
    let y = 2.0;

    let mut planks: Vec<BodyId> = Vec::with_capacity(plank_count);
    for i in 0..plank_count {
        let x = start_x + (i as f32) * (plank_half.x * 2.0 + 0.02);
        let b = world.create_body(
            BodyBuilder::from(foundation.body_def())
                .position([x, y])
                .build()?,
        )?;
        let _s = world.body(b)?.create_polygon(&sdef, &plank_poly)?;
        planks.push(b);
    }

    // Revolute joints between planks
    // Use saturating_sub to avoid usize underflow if plank_count is ever 0
    let joint_count = plank_count.saturating_sub(1);
    for i in 0..joint_count {
        let a = planks[i];
        let b = planks[i + 1];
        let anchor = Vec2::new(
            start_x + (i as f32 + 1.0) * (plank_half.x * 2.0 + 0.02) - plank_half.x - 0.01,
            y,
        );
        let _jid = world.create_revolute_joint_world(a, b, anchor)?;
    }

    // Attach ends to ground with revolute joints
    let left_anchor = Vec2::new(start_x - plank_half.x, y);
    let right_anchor = Vec2::new(
        start_x + (plank_count as f32) * (plank_half.x * 2.0 + 0.02),
        y,
    );
    let _jleft = world.create_revolute_joint_world(ground, planks[0], left_anchor)?;
    // Right end: safe index even if constraints change
    let _jright = world.create_revolute_joint_world(
        ground,
        planks[plank_count.saturating_sub(1)],
        right_anchor,
    )?;

    for _ in 0..240 {
        drop(world.step(1.0 / 60.0, 4)?);
    }

    println!("bridge: {} planks", plank_count);
    Ok(())
}
