use boxdd as bd;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let sensor_body = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .position([0.0_f32, app.sensor_band_y])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let sensor_def = bd::ShapeDef::builder()
        .density(0.0)
        .sensor(true)
        .enable_sensor_events(true)
        .build()
        .expect("valid testbed definition");
    let _ = app
        .world
        .body(sensor_body)
        .expect("valid testbed operation")
        .create_polygon(
            &sensor_def,
            &bd::shapes::box_polygon(4.0, app.sensor_half_thickness)
                .expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    let mover = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([0.0_f32, app.sensor_mover_start_y])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let _ = app
        .world
        .body(mover)
        .expect("valid testbed operation")
        .create_circle(
            &bd::ShapeDef::builder()
                .density(1.0)
                .enable_sensor_events(true)
                .build()
                .expect("valid testbed definition"),
            &bd::shapes::circle([0.0_f32, 0.0], app.sensor_radius)
                .expect("sensor circle must be valid"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
}

use dear_imgui_rs as imgui;
pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut y = app.sensor_band_y;
    let mut h = app.sensor_half_thickness;
    let mut sy = app.sensor_mover_start_y;
    let mut r = app.sensor_radius;
    if ui.slider("Band Y", -5.0, 5.0, &mut y) {
        app.sensor_band_y = y;
        let _ = app.reset();
    }
    if ui.slider("Band Half-Height", 0.05, 1.0, &mut h) {
        app.sensor_half_thickness = h;
        let _ = app.reset();
    }
    if ui.slider("Mover Start Y", -1.0, 6.0, &mut sy) {
        app.sensor_mover_start_y = sy;
        let _ = app.reset();
    }
    if ui.slider("Mover Radius", 0.05, 1.0, &mut r) {
        app.sensor_radius = r;
        let _ = app.reset();
    }
}
