use boxdd as bd;
use dear_imgui_rs as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Ground + a step obstacle
    let step = app
        .world
        .create_body_id(bd::BodyBuilder::new().position([1.0_f32, 0.5]).build());
    app.created_bodies += 1;
    let _ = app.world.create_polygon_shape_for(
        step,
        &bd::ShapeDef::builder().density(0.0).build(),
        &bd::shapes::box_polygon(0.5, 0.5),
    );
    app.created_shapes += 1;
}

pub fn tick(app: &mut super::PhysicsApp) {
    let frac = app.world.cast_mover(
        [0.0_f32, app.cm_c1_y],
        [0.0, app.cm_c2_y],
        app.cm_radius,
        [app.cm_move_x, 0.0_f32],
        bd::QueryFilter::default(),
    );
    app.cm_fraction = frac;
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut c1 = app.cm_c1_y;
    let mut c2 = app.cm_c2_y;
    let mut r = app.cm_radius;
    let mut dx = app.cm_move_x;
    let changed = ui.slider("C1.y", 0.0, 3.0, &mut c1)
        || ui.slider("C2.y", 0.0, 3.0, &mut c2)
        || ui.slider("Radius", 0.05, 1.0, &mut r)
        || ui.slider("Move X", 0.0, 5.0, &mut dx);
    if changed {
        app.cm_c1_y = c1;
        app.cm_c2_y = c2;
        app.cm_radius = r;
        app.cm_move_x = dx;
    }
    ui.text(format!("Mover fraction: {:.3}", app.cm_fraction));
}
