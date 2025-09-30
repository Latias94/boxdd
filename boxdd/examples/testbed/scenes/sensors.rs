use boxdd as bd;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let sensor_body = app
        .world
        .create_body_id(bd::BodyBuilder::new().position([0.0_f32, app.sensor_band_y]).build());
    app.created_bodies += 1;
    let sensor_def = bd::ShapeDef::builder()
        .density(0.0)
        .sensor(true)
        .enable_sensor_events(true)
        .build();
    let _ = app.world.create_polygon_shape_for(
        sensor_body,
        &sensor_def,
        &bd::shapes::box_polygon(4.0, app.sensor_half_thickness),
    );
    app.created_shapes += 1;
    let mover = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([0.0_f32, app.sensor_mover_start_y])
            .build(),
    );
    app.created_bodies += 1;
    let _ = app.world.create_circle_shape_for(
        mover,
        &bd::ShapeDef::builder()
            .density(1.0)
            .enable_sensor_events(true)
            .build(),
        &bd::shapes::circle([0.0_f32, 0.0], app.sensor_radius),
    );
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
