use boxdd as bd;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let y1 = 2.0_f32;
    let y2 = y1 + app.contact_gap;
    let b1 = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([0.0_f32, y1])
            .build(),
    );
    app.created_bodies += 1;
    let b2 = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([0.0_f32, y2])
            .build(),
    );
    app.created_bodies += 1;
    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();
    let _ = app.world.create_polygon_shape_for(
        b1,
        &sdef,
        &bd::shapes::box_polygon(app.contact_box_half, app.contact_box_half),
    );
    app.created_shapes += 1;
    let _ = app.world.create_polygon_shape_for(
        b2,
        &sdef,
        &bd::shapes::box_polygon(app.contact_box_half, app.contact_box_half),
    );
    app.created_shapes += 1;
    app.world
        .set_body_linear_velocity(b1, [0.0_f32, app.contact_speed]);
    app.world
        .set_body_linear_velocity(b2, [0.0_f32, -app.contact_speed]);
}

use dear_imgui as imgui;
pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut half = app.contact_box_half;
    let mut sp = app.contact_speed;
    let mut gap = app.contact_gap;
    if ui.slider("Box Half", 0.1, 2.0, &mut half) {
        app.contact_box_half = half;
        let _ = app.reset();
    }
    if ui.slider("Speed", 0.1, 10.0, &mut sp) {
        app.contact_speed = sp;
        let _ = app.reset();
    }
    if ui.slider("Gap", 0.5, 4.0, &mut gap) {
        app.contact_gap = gap;
        let _ = app.reset();
    }
}
