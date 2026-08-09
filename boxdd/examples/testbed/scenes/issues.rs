use boxdd as bd;
use dear_imgui_rs as imgui;

// Issues: small repros; here we add a sensor band with multiple visitors

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    // Sensor band across x-axis at configurable y
    let sensor_def = bd::ShapeDef::builder()
        .sensor(true)
        .enable_sensor_events(true)
        .build()
        .expect("valid testbed definition");
    let _ = app
        .world
        .body(ground)
        .expect("valid testbed operation")
        .create_segment(
            &sensor_def,
            &bd::shapes::segment([-3.0_f32, app.sensor_band_y], [3.0, app.sensor_band_y])
                .expect("issue sensor segment must be valid"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;

    // Dynamic visitors
    for i in 0..app.issues_visitors.max(0) {
        let x = -3.0 + i as f32 * 0.6;
        let id = app
            .world
            .create_body(
                bd::BodyBuilder::from(app.foundation.body_def())
                    .body_type(bd::BodyType::Dynamic)
                    .position([x, app.sensor_band_y])
                    .build()
                    .expect("valid testbed definition"),
            )
            .expect("valid testbed operation");
        app.created_bodies += 1;
        let _ = app
            .world
            .body(id)
            .expect("valid testbed operation")
            .create_circle(
                &bd::ShapeDef::builder()
                    .density(1.0)
                    .build()
                    .expect("valid testbed definition"),
                &bd::shapes::circle([0.0_f32, 0.0], app.sensor_radius.max(0.01))
                    .expect("issue sensor circle must be valid"),
            )
            .expect("valid testbed operation");
        app.created_shapes += 1;
    }
}

pub fn tick(app: &mut super::PhysicsApp, events: Option<&bd::StepEventsSnapshot>) {
    if let Some(events) = events {
        app.ev_sens_beg += events.sensor.begin.len();
        app.ev_sens_end += events.sensor.end.len();
    }
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
