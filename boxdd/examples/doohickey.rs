// Doohickey (two wheels + bars)
//
// Notes
// - This simplified version omits the prismatic slider between bars to avoid Debug assertions.
//   The slider can be restored after aligning local frames/limits/motor params to upstream.
// - The revolute motors on the wheels remain to demonstrate actuation.
use boxdd::prelude::*;

// Simplified port of the Doohickey: two wheels and two bars with a prismatic slider between bars.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    let scale = 1.0_f32;

    // Common defs
    let bdef_dyn = BodyBuilder::new().body_type(BodyType::Dynamic);
    let filt = boxdd::filter::Filter {
        group_index: -1,
        ..Default::default()
    };
    let sdef_rr = ShapeDef::builder()
        .material(SurfaceMaterial::default().rolling_resistance(0.1))
        .filter_ex(filt)
        .build();
    let circle = shapes::circle([0.0_f32, 0.0], 1.0 * scale);
    let bar = shapes::capsule([-3.5 * scale, 0.0], [3.5 * scale, 0.0], 0.15 * scale);

    // Bodies
    let p_w1 = [-5.0 * scale, 3.0 * scale];
    let p_w2 = [5.0 * scale, 3.0 * scale];
    let p_b1 = [-1.5 * scale, 3.0 * scale];
    let p_b2 = [1.5 * scale, 3.0 * scale];

    let w1 = world.create_body_id(bdef_dyn.clone().position(p_w1).build());
    let _ = world.create_circle_shape_for(w1, &sdef_rr, &circle);
    let w2 = world.create_body_id(bdef_dyn.clone().position(p_w2).build());
    let _ = world.create_circle_shape_for(w2, &sdef_rr, &circle);

    let b1 = world.create_body_id(bdef_dyn.clone().position(p_b1).build());
    let _ = world.create_capsule_shape_for(b1, &sdef_rr, &bar);
    let b2 = world.create_body_id(bdef_dyn.clone().position(p_b2).build());
    let _ = world.create_capsule_shape_for(b2, &sdef_rr, &bar);

    // Revolute joints at wheel centers
    {
        let base = world.joint_base_from_world_points(w1, b1, p_w1, p_w1);
        let rdef = RevoluteJointDef::new(base)
            .enable_motor(true)
            .max_motor_torque(2.0 * scale);
        let _ = world.create_revolute_joint_id(&rdef);
    }
    {
        let base = world.joint_base_from_world_points(w2, b2, p_w2, p_w2);
        let rdef = RevoluteJointDef::new(base)
            .enable_motor(true)
            .max_motor_torque(2.0 * scale);
        let _ = world.create_revolute_joint_id(&rdef);
    }

    // Prismatic slider between bars along X axis (aligned with upstream)
    let anchor_a = [p_b1[0] + 2.0 * scale, p_b1[1]];
    let anchor_b = [p_b2[0] - 2.0 * scale, p_b2[1]];
    let axis = [1.0_f32, 0.0];
    let base = world.joint_base_from_world_with_axis(b1, b2, anchor_a, anchor_b, axis);
    let pdef = PrismaticJointDef::new(base)
        .enable_limit(true)
        .lower_translation(-2.0 * scale)
        .upper_translation(2.0 * scale)
        .enable_motor(true)
        .max_motor_force(2.0 * scale)
        .enable_spring(true)
        .hertz(1.0)
        .damping_ratio(0.5);
    let _ = world.create_prismatic_joint_id(&pdef);

    // Step and report
    for _ in 0..240 {
        world.step(1.0 / 60.0, 8);
    }
    let b1p = world.body_position(b1);
    let b2p = world.body_position(b2);
    println!(
        "doohickey: b1=({:.2},{:.2}) b2=({:.2},{:.2})",
        b1p.x, b1p.y, b2p.x, b2p.y
    );
    Ok(())
}
