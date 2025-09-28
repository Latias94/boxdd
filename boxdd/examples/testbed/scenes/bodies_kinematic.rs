use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Kinematic platform moving horizontally
    let platform = app
        .world
        .create_body_id(
            bd::BodyBuilder::new()
                .body_type(bd::BodyType::Kinematic)
                .position([0.0_f32, 2.0])
                .build(),
        );
    app.created_bodies += 1;
    let _ = app
        .world
        .create_polygon_shape_for(platform, &bd::ShapeDef::builder().density(0.0).build(), &bd::shapes::box_polygon(2.0, 0.25));
    app.created_shapes += 1;
    app.world.set_body_linear_velocity(platform, [app.bk_speed, 0.0_f32]);
    app.bk_platform = Some(platform);

    // A few dynamic boxes to ride the platform
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    for i in 0..5 {
        let b = app
            .world
            .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([-2.0_f32 + i as f32, 5.0]).build());
        app.created_bodies += 1;
        let _ = app
            .world
            .create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.3, 0.3));
        app.created_shapes += 1;
    }
}

pub fn tick(app: &mut super::PhysicsApp) {
    if let Some(id) = app.bk_platform {
        app.world.set_body_linear_velocity(id, [app.bk_speed, 0.0_f32]);
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut sp = app.bk_speed;
    if ui.slider("Speed", -10.0, 10.0, &mut sp) {
        app.bk_speed = sp;
    }
    ui.text("Kinematic platform with dynamic boxes on top.");
}

