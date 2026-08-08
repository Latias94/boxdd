use boxdd as bd;
use dear_imgui_rs as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Ground + a step obstacle
    let step = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .position([1.0_f32, 0.5])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("character-mover step body must be valid");
    app.created_bodies += 1;
    let _ = app
        .world
        .body(step)
        .expect("character-mover step body must stay live")
        .create_polygon(
            &bd::ShapeDef::builder()
                .density(0.0)
                .build()
                .expect("valid testbed definition"),
            &bd::shapes::box_polygon(0.5, 0.5)
                .expect("character-mover step geometry must be valid"),
        )
        .expect("character-mover step shape must be valid");
    app.created_shapes += 1;
}

pub fn tick(app: &mut super::PhysicsApp, _events: Option<&bd::StepEventsSnapshot>) {
    if let Ok(query) = app.world.query()
        && let Ok(fraction) = query.cast_mover(
            bd::Position::ZERO,
            [0.0_f32, app.cm_c1_y],
            [0.0, app.cm_c2_y],
            app.cm_radius,
            [app.cm_move_x, 0.0_f32],
            bd::QueryFilter::default(),
        )
    {
        app.cm_fraction = fraction;
    }
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
