use boxdd as bd;
use dear_imgui_rs as imgui;

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    if app.chain_mode == 0 {
        // Simple walkway: a field of small dynamic boxes with sinusoidal X velocity
        let sdef = bd::ShapeDef::builder()
            .density(1.0)
            .build()
            .expect("valid testbed definition");
        let poly = bd::shapes::box_polygon(0.2, 0.2).expect("valid polygon geometry");
        let n = app.chain_boxes.max(0) as usize;
        let mut ids = Vec::with_capacity(n);
        for i in 0..n {
            let x = -2.0 + (i as f32) * 0.5;
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
                .create_polygon(&sdef, &poly)
                .expect("valid testbed operation");
            app.created_shapes += 1;
            ids.push(b);
        }
        for (i, &b) in ids.iter().enumerate() {
            let phase = i as f32 * 0.2;
            let vel = app.chain_amp * (phase * app.chain_freq).sin();
            app.world
                .body(b)
                .expect("valid testbed operation")
                .set_linear_velocity([vel, 0.0_f32])
                .expect("valid testbed operation");
        }
    } else {
        // Chain link: small links connected by revolute joints; first linked to ground
        let n = app.chain_boxes.max(1) as usize;
        let link_half = 0.25f32;
        let spacing = 0.55f32;
        let y = 4.0f32;
        let sdef = bd::ShapeDef::builder()
            .density(1.0)
            .build()
            .expect("valid testbed definition");
        let link_poly = bd::shapes::box_polygon(link_half, 0.12).expect("valid polygon geometry");
        let mut prev = ground;
        for i in 0..n {
            let x = -3.0 + i as f32 * spacing;
            let b = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .body_type(bd::BodyType::Dynamic)
                        .position([x, y])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("valid testbed operation");
            app.created_bodies += 1;
            let _ = app
                .world
                .body(b)
                .expect("valid testbed operation")
                .create_polygon(&sdef, &link_poly)
                .expect("valid testbed operation");
            app.created_shapes += 1;
            // Revolute joint (ID API to persist)
            let jid = app
                .world
                .create_revolute_joint_world(prev, b, [x - link_half, y])
                .expect("valid testbed operation");
            app.created_joints += 1;
            app.world
                .joint(jid)
                .expect("valid testbed operation")
                .into_revolute()
                .expect("valid testbed operation")
                .enable_limit(true)
                .expect("valid testbed operation");
            app.world
                .joint(jid)
                .expect("valid testbed operation")
                .into_revolute()
                .expect("valid testbed operation")
                .set_limits(-0.5, 0.5)
                .expect("valid testbed operation");
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
