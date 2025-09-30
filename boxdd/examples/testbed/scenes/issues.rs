use boxdd as bd;
use dear_imgui_rs as imgui;

// Issues: small repros; here we add a sensor band with multiple visitors

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    // Sensor band across x-axis at configurable y
    let sensor_def = bd::ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build();
    let _ = app.world.create_segment_shape_for(
        ground,
        &sensor_def,
        &bd::shapes::segment([-3.0_f32, app.sensor_band_y], [3.0, app.sensor_band_y]),
    );
    app.created_shapes += 1;

    // Dynamic visitors
    for i in 0..app.issues_visitors.max(0) {
        let x = -3.0 + i as f32 * 0.6;
        let id = app
            .world
            .create_body_id(
                bd::BodyBuilder::new()
                    .body_type(bd::BodyType::Dynamic)
                    .position([x, app.sensor_band_y])
                    .build(),
            );
        app.created_bodies += 1;
        let _ = app.world.create_circle_shape_for(
            id,
            &bd::ShapeDef::builder().density(1.0).build(),
            &bd::shapes::circle([0.0_f32, 0.0], app.sensor_radius.max(0.01)),
        );
        app.created_shapes += 1;
    }
}

pub fn tick(app: &mut super::PhysicsApp) {
    // Reuse events counters for convenience
    let se = app.world.sensor_events();
    app.ev_sens_beg += se.begin.len();
    app.ev_sens_end += se.end.len();
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut y = app.sensor_band_y;
    let mut r = app.sensor_radius;
    let mut n = app.issues_visitors;
    let changed = ui.slider("Sensor Y", -2.0, 4.0, &mut y)
        || ui.slider("Visitor Radius", 0.05, 0.5, &mut r)
        || ui.slider("Visitor Count", 0, 50, &mut n);
    if changed {
        app.sensor_band_y = y;
        app.sensor_radius = r.max(0.01);
        app.issues_visitors = n.max(0);
        let _ = app.reset();
    }
    if ui.button("Reset Counters") {
        app.ev_sens_beg = 0;
        app.ev_sens_end = 0;
    }
    ui.text(format!(
        "Issues: sensor begins={} ends={}",
        app.ev_sens_beg, app.ev_sens_end
    ));
}
