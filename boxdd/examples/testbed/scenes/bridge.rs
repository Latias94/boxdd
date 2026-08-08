use boxdd as bd;

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    let plank_count = app.bridge_planks.max(2) as usize;
    let plank_half = bd::Vec2::new(0.5, 0.1);
    let plank_poly =
        bd::shapes::box_polygon(plank_half.x, plank_half.y).expect("valid polygon geometry");
    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .build()
        .expect("valid testbed definition");
    let mut planks = Vec::with_capacity(plank_count);
    let start_x = -(plank_count as f32) * plank_half.x;
    for i in 0..plank_count {
        let x = start_x + i as f32 * (plank_half.x * 2.2);
        let b = app
            .world
            .create_body(
                bd::BodyBuilder::from(app.foundation.body_def())
                    .body_type(bd::BodyType::Dynamic)
                    .position([x, 4.0])
                    .build()
                    .expect("valid testbed definition"),
            )
            .expect("valid testbed operation");
        app.created_bodies += 1;
        let _ = app
            .world
            .body(b)
            .expect("valid testbed operation")
            .create_polygon(&sdef, &plank_poly)
            .expect("valid testbed operation");
        app.created_shapes += 1;
        planks.push(b);
    }
    let left_anchor = bd::Vec2::new(start_x - plank_half.x, 4.0);
    let right_anchor = bd::Vec2::new(-start_x + plank_half.x, 4.0);
    let _ = app
        .world
        .create_revolute_joint_world(ground, planks[0], left_anchor)
        .expect("valid testbed operation");
    app.created_joints += 1;
    // Right end: safe index using saturating_sub to avoid underflow if constraints change
    let _ = app
        .world
        .create_revolute_joint_world(ground, planks[plank_count.saturating_sub(1)], right_anchor)
        .expect("valid testbed operation");
    app.created_joints += 1;
    // Internal joints: use saturating_sub to protect 0/1 plank edge cases
    let joint_count = plank_count.saturating_sub(1);
    for i in 0..joint_count {
        let a = planks[i];
        let b = planks[i + 1];
        let anchor = app
            .world
            .body(a)
            .expect("valid testbed operation")
            .position()
            .expect("valid testbed operation");
        let right = anchor.offset(bd::Vec2::new(plank_half.x, 0.0));
        let left = anchor.offset(bd::Vec2::new(-plank_half.x, 0.0));
        let base = app
            .world
            .joint_base_from_world_points(a, b, right, left)
            .expect("valid testbed operation");
        let rdef = bd::RevoluteJointDef::new(base);
        let _ = app
            .world
            .create_revolute_joint(&rdef)
            .expect("valid testbed operation");
        app.created_joints += 1;
    }
}

use dear_imgui_rs as imgui;
pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut n = app.bridge_planks;
    if ui.slider("Planks", 4, 60, &mut n) {
        app.bridge_planks = n;
        let _ = app.reset();
    }
}
