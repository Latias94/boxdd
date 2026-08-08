use boxdd as bd;
use dear_imgui_rs as imgui;

// Shape Editing: demonstrate runtime shape replacement on a body (box / rounded box).

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Create a single dynamic body in the center and attach an initial box shape.
    let b = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([0.0, 4.0])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .build()
        .expect("valid testbed definition");
    let poly = bd::shapes::box_polygon(app.se_hx, app.se_hy).expect("valid polygon geometry");
    let sid = app
        .world
        .body(b)
        .expect("valid testbed operation")
        .create_polygon(&sdef, &poly)
        .expect("valid testbed operation");
    app.created_shapes += 1;

    // Store for later editing
    app.se_body = Some(b);
    app.se_shape = Some(sid);
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut mode = app.se_mode; // 0=Box, 1=Rounded Box
    let mut hx = app.se_hx;
    let mut hy = app.se_hy;
    let mut r = app.se_radius;
    let changed = ui.slider("Mode (0=Box,1=Rounded)", 0, 1, &mut mode)
        || ui.slider("Half X", 0.1, 2.5, &mut hx)
        || ui.slider("Half Y", 0.1, 2.5, &mut hy)
        || (mode == 1 && ui.slider("Corner Radius", 0.0, 0.8, &mut r));
    if changed {
        app.se_mode = mode;
        app.se_hx = hx;
        app.se_hy = hy;
        app.se_radius = r;
        // Replace shape in place
        if let (Some(bid), Some(sid)) = (app.se_body, app.se_shape) {
            app.world
                .shape(sid)
                .expect("valid testbed operation")
                .destroy(true)
                .expect("valid testbed operation");
            let sdef = bd::ShapeDef::builder()
                .density(1.0)
                .build()
                .expect("valid testbed definition");
            let new_sid = if app.se_mode == 0 {
                let poly =
                    bd::shapes::box_polygon(app.se_hx, app.se_hy).expect("valid polygon geometry");
                app.world
                    .body(bid)
                    .expect("valid testbed operation")
                    .create_polygon(&sdef, &poly)
                    .expect("valid testbed operation")
            } else {
                let poly = bd::shapes::rounded_box_polygon(app.se_hx, app.se_hy, app.se_radius)
                    .expect("valid polygon geometry");
                app.world
                    .body(bid)
                    .expect("valid testbed operation")
                    .create_polygon(&sdef, &poly)
                    .expect("valid testbed operation")
            };
            app.se_shape = Some(new_sid);
        }
    }
}
