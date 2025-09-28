use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Build a simple soft ring from capsules welded end-to-end
    let sides = 7;
    let scale = app.soft_scale.max(0.25);
    let radius = 1.0 * scale;
    let delta = 2.0_f32 * core::f32::consts::PI / (sides as f32);
    let seg_len = 2.0 * core::f32::consts::PI * radius / (sides as f32);
    let half = 0.5 * seg_len;
    let capsule = bd::shapes::capsule([-half, 0.0], [half, 0.0], 0.2 * scale);

    let sdef = bd::ShapeDef::builder().density(1.0).build();

    let mut bodies = Vec::with_capacity(sides);
    for i in 0..sides {
        let angle = i as f32 * delta;
        // Orient capsule along tangent direction
        let tangent_angle = angle + core::f32::consts::FRAC_PI_2;
        let pos = [radius * angle.cos(), 2.5 + radius * angle.sin()];
        let body = app
            .world
            .create_body_id(
                bd::BodyBuilder::new()
                    .body_type(bd::BodyType::Dynamic)
                    .position(pos)
                    .angle(tangent_angle)
                    .build(),
            );
        app.created_bodies += 1;
        let _ = app.world.create_capsule_shape_for(body, &sdef, &capsule);
        app.created_shapes += 1;
        bodies.push(body);
    }

    // Weld joints between neighbors
    for i in 0..sides {
        let a = bodies[i];
        let b = bodies[(i + 1) % sides];
        // Anchor at the end of A (world-space) along the tangent direction
        let angle = (i as f32) * delta;
        let tangent_angle = angle + core::f32::consts::FRAC_PI_2;
        let dir = [tangent_angle.cos(), tangent_angle.sin()];
        let pos = [radius * angle.cos(), 2.5 + radius * angle.sin()];
        let anchor = [pos[0] + half * dir[0], pos[1] + half * dir[1]];
        let _ = app
            .world
            .weld(a, b)
            .anchor_world(anchor)
            .with_stiffness(0.0, 0.0, 5.0, 0.0)
            .build();
        app.created_joints += 1;
    }
}

pub fn tick(_app: &mut super::PhysicsApp) {}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut s = app.soft_scale;
    if ui.slider("Scale", 0.25, 3.0, &mut s) {
        app.soft_scale = s.max(0.25);
        let _ = app.reset();
    }
    ui.text("Capsule ring connected with weld joints (soft body).");
}
