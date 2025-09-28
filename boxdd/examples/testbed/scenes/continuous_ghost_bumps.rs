use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    // Continuous collision setup
    app.world.enable_continuous(true);
    app.world.set_hit_event_threshold(0.05);

    // Build a bumpy floor: a sequence of short segments approximating bumps
    // Height is controlled via ui param gb_bump_h
    let h = app.gb_bump_h.max(0.0);
    let step = 0.5_f32;
    let count = 40_i32;
    let sdef = bd::ShapeDef::builder().density(0.0).build();
    for i in -count..=count {
        let x0 = (i as f32) * step;
        let x1 = (i as f32 + 1.0) * step;
        // simple triangular bump profile
        let y0 = if i % 2 == 0 { 0.0 } else { h };
        let y1 = if (i + 1) % 2 == 0 { 0.0 } else { h };
        let _ = app
            .world
            .create_segment_shape_for(ground, &sdef, &bd::shapes::segment([x0, y0], [x1, y1]));
        app.created_shapes += 1;
    }

    // A fast bullet-like circle to roll over the bumps
    let mover = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([-10.0_f32, 1.5])
            .bullet(true)
            .build(),
    );
    app.created_bodies += 1;
    let _ = app
        .world
        .create_circle_shape_for(mover, &bd::ShapeDef::builder().density(1.0).build(), &bd::shapes::circle([0.0, 0.0], 0.3));
    app.created_shapes += 1;
    app.world
        .set_body_linear_velocity(mover, [app.gb_speed, 0.0_f32]);
}

pub fn tick(app: &mut super::PhysicsApp) {
    // Accumulate hit events as a proxy for ghost contacts
    let ce = app.world.contact_events();
    app.gb_hits += ce.hit.len();
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut sp = app.gb_speed;
    let mut h = app.gb_bump_h;
    let changed = ui.slider("Speed", 1.0, 120.0, &mut sp) || ui.slider("Bump Height", 0.0, 1.0, &mut h);
    if changed {
        app.gb_speed = sp;
        app.gb_bump_h = h.max(0.0);
        let _ = app.reset();
    }
    if ui.button("Reset Hit Counter") {
        app.gb_hits = 0;
    }
    ui.text(format!("Ghost Bumps: hits={} (accumulated)", app.gb_hits));
}

