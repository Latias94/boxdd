use boxdd as bd;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let y1 = 2.0_f32;
    let y2 = y1 + app.contact_gap;
    let b1 = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([0.0_f32, y1])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let b2 = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([0.0_f32, y2])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build()
        .expect("valid testbed definition");
    let _ = app
        .world
        .body(b1)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef,
            &bd::shapes::box_polygon(app.contact_box_half, app.contact_box_half)
                .expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    let _ = app
        .world
        .body(b2)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef,
            &bd::shapes::box_polygon(app.contact_box_half, app.contact_box_half)
                .expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    app.world
        .body(b1)
        .expect("valid testbed operation")
        .set_linear_velocity([0.0_f32, app.contact_speed])
        .expect("valid testbed operation");
    app.world
        .body(b2)
        .expect("valid testbed operation")
        .set_linear_velocity([0.0_f32, -app.contact_speed])
        .expect("valid testbed operation");
}

use dear_imgui_rs as imgui;
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
