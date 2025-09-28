use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Continuous enabled helps with high-speed impacts
    app.world.enable_continuous(true);
    app.world.set_restitution_threshold(app.cr_threshold);

    // Ground
    let g = app
        .world
        .create_body_id(bd::BodyBuilder::new().position([0.0_f32, 0.0]).build());
    app.created_bodies += 1;
    let _ = app
        .world
        .create_polygon_shape_for(g, &bd::ShapeDef::builder().density(0.0).build(), &bd::shapes::box_polygon(50.0, 0.25));
    app.created_shapes += 1;

    // Ball dropped from height with configurable restitution
    let ball = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([0.0_f32, app.cr_drop_y]).build());
    app.created_bodies += 1;
    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .material(bd::SurfaceMaterial::default().restitution(app.cr_restitution))
        .build();
    let _ = app
        .world
        .create_circle_shape_for(ball, &sdef, &bd::shapes::circle([0.0, 0.0], 0.5));
    app.created_shapes += 1;
}

pub fn tick(_app: &mut super::PhysicsApp) {}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut th = app.cr_threshold;
    let mut rest = app.cr_restitution;
    let mut drop_y = app.cr_drop_y;
    let changed = ui.slider("Restitution Threshold", 0.0, 5.0, &mut th)
        || ui.slider("Restitution", 0.0, 1.0, &mut rest)
        || ui.slider("Drop Height", 1.0, 20.0, &mut drop_y);
    if changed {
        app.cr_threshold = th.max(0.0);
        app.cr_restitution = rest.clamp(0.0, 1.0);
        app.cr_drop_y = drop_y.max(1.0);
        let _ = app.reset();
    }
    ui.text("Tuning bounce response via restitution threshold & material.");
}

