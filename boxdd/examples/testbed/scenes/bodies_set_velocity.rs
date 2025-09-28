use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Ground
    let g = app
        .world
        .create_body_id(bd::BodyBuilder::new().position([0.0_f32, -0.25]).build());
    app.created_bodies += 1;
    let _ = app
        .world
        .create_polygon_shape_for(g, &bd::ShapeDef::builder().density(0.0).build(), &bd::shapes::box_polygon(20.0, 0.25));
    app.created_shapes += 1;

    // Dynamic box whose velocity we set each tick
    let body = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([0.0_f32, 0.5]).build());
    app.created_bodies += 1;
    let _ = app
        .world
        .create_polygon_shape_for(body, &bd::ShapeDef::builder().density(1.0).build(), &bd::shapes::box_polygon(0.5, 0.5));
    app.created_shapes += 1;
    app.bsv_body = Some(body);
}

pub fn tick(app: &mut super::PhysicsApp) {
    if let Some(id) = app.bsv_body {
        app.world.set_body_linear_velocity(id, [app.bsv_vx, app.bsv_vy]);
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut vx = app.bsv_vx;
    let mut vy = app.bsv_vy;
    let changed = ui.slider("VX", -50.0, 50.0, &mut vx) || ui.slider("VY", -50.0, 50.0, &mut vy);
    if changed {
        app.bsv_vx = vx;
        app.bsv_vy = vy;
    }
    ui.text("Sets the linear velocity of a dynamic box each tick.");
}

