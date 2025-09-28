use boxdd as bd;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    app.world.enable_continuous(true);
    app.world.set_hit_event_threshold(app.bullet_threshold);
    let wall = app
        .world
        .create_body_id(bd::BodyBuilder::new().position([5.0_f32, 0.0]).build());
    app.created_bodies += 1;
    let _ = app.world.create_polygon_shape_for(
        wall,
        &bd::ShapeDef::builder()
            .density(0.0)
            .enable_contact_events(true)
            .enable_hit_events(true)
            .build(),
        &bd::shapes::box_polygon(0.5, 3.0),
    );
    app.created_shapes += 1;
    let bullet = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([0.0_f32, 0.0])
            .bullet(true)
            .build(),
    );
    app.created_bodies += 1;
    let _ = app.world.create_circle_shape_for(
        bullet,
        &bd::ShapeDef::builder()
            .density(1.0)
            .enable_contact_events(true)
            .enable_hit_events(true)
            .build(),
        &bd::shapes::circle([0.0_f32, 0.0], app.bullet_radius),
    );
    app.created_shapes += 1;
    app.world
        .set_body_linear_velocity(bullet, [app.bullet_speed, 0.0_f32]);
}

use dear_imgui as imgui;
pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut dt = app.bullet_dt;
    let mut sub = app.bullet_substeps;
    let mut sp = app.bullet_speed;
    let mut rad = app.bullet_radius;
    let mut th = app.bullet_threshold;
    if ui.slider("dt", 1.0 / 1000.0, 1.0 / 30.0, &mut dt) {
        app.bullet_dt = dt.max(1e-5);
    }
    if ui.slider("Substeps", 1, 64, &mut sub) {
        app.bullet_substeps = sub.max(1);
    }
    if ui.slider("Speed", 1.0, 120.0, &mut sp) {
        app.bullet_speed = sp;
        let _ = app.reset();
    }
    if ui.slider("Radius", 0.05, 1.0, &mut rad) {
        app.bullet_radius = rad;
        let _ = app.reset();
    }
    if ui.slider("Hit Threshold", 0.0, 2.0, &mut th) {
        app.bullet_threshold = th;
        let _ = app.reset();
    }
}
