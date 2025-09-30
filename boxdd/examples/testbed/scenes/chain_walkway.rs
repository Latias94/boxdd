use boxdd as bd;
use dear_imgui_rs as imgui;

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    if app.chain_mode == 0 {
        // Simple walkway: a field of small dynamic boxes with sinusoidal X velocity
        let sdef = bd::ShapeDef::builder().density(1.0).build();
        let poly = bd::shapes::box_polygon(0.2, 0.2);
        let n = app.chain_boxes.max(0) as usize;
        let mut ids = Vec::with_capacity(n);
        for i in 0..n {
            let x = -2.0 + (i as f32) * 0.5;
            let b = app
                .world
                .create_body_id(
                    bd::BodyBuilder::new()
                        .body_type(bd::BodyType::Dynamic)
                        .position([x, 4.0])
                        .build(),
                );
            app.created_bodies += 1;
            let _ = app
                .world
                .create_polygon_shape_for(b, &sdef, &poly);
            app.created_shapes += 1;
            ids.push(b);
        }
        for (i, &b) in ids.iter().enumerate() {
            let phase = i as f32 * 0.2;
            let vel = app.chain_amp * (phase * app.chain_freq).sin();
            app.world.set_body_linear_velocity(b, [vel, 0.0_f32]);
        }
    } else {
        // Chain link: small links connected by revolute joints; first linked to ground
        let n = app.chain_boxes.max(1) as usize;
        let link_half = 0.25f32;
        let spacing = 0.55f32;
        let y = 4.0f32;
        let sdef = bd::ShapeDef::builder().density(1.0).build();
        let link_poly = bd::shapes::box_polygon(link_half, 0.12);
        let mut prev = ground;
        for i in 0..n {
            let x = -3.0 + i as f32 * spacing;
            let b = app
                .world
                .create_body_id(
                    bd::BodyBuilder::new()
                        .body_type(bd::BodyType::Dynamic)
                        .position([x, y])
                        .build(),
                );
            app.created_bodies += 1;
            let _ = app
                .world
                .create_polygon_shape_for(b, &sdef, &link_poly);
            app.created_shapes += 1;
            // Revolute joint (ID API to persist)
            let jid = app
                .world
                .create_revolute_joint_world_id(prev, b, [x - link_half, y]);
            app.created_joints += 1;
            // Limit a bit for stability
            unsafe {
                boxdd_sys::ffi::b2RevoluteJoint_EnableLimit(jid, true);
                boxdd_sys::ffi::b2RevoluteJoint_SetLimits(jid, -0.5, 0.5);
            }
            prev = b;
        }
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut nb = app.chain_boxes;
    let mut amp = app.chain_amp;
    let mut fr = app.chain_freq;
    let mut mode = app.chain_mode;
    if ui.slider("Mode (0=Walkway,1=Chain)", 0, 1, &mut mode) {
        app.chain_mode = mode;
        let _ = app.reset();
    }
    if ui.slider("Boxes", 1, 50, &mut nb) {
        app.chain_boxes = nb;
        let _ = app.reset();
    }
    if app.chain_mode == 0 {
        if ui.slider("Amplitude", 0.0, 2.0, &mut amp) {
            app.chain_amp = amp;
            let _ = app.reset();
        }
        if ui.slider("Frequency", 0.1, 3.0, &mut fr) {
            app.chain_freq = fr;
            let _ = app.reset();
        }
    }
}
