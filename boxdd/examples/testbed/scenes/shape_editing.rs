use boxdd as bd;
use dear_imgui as imgui;

// Shape Editing: demonstrate runtime shape replacement on a body (box / rounded box).

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Create a single dynamic body in the center and attach an initial box shape.
    let b = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([0.0, 4.0]).build());
    app.created_bodies += 1;
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    let poly = bd::shapes::box_polygon(app.se_hx, app.se_hy);
    let sid = app.world.create_polygon_shape_for(b, &sdef, &poly);
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
            app.world.destroy_shape_id(sid, true);
            let sdef = bd::ShapeDef::builder().density(1.0).build();
            let new_sid = if app.se_mode == 0 {
                let poly = bd::shapes::box_polygon(app.se_hx, app.se_hy);
                app.world.create_polygon_shape_for(bid, &sdef, &poly)
            } else {
                let poly = unsafe { boxdd_sys::ffi::b2MakeRoundedBox(app.se_hx, app.se_hy, app.se_radius) };
                app.world.create_polygon_shape_for(bid, &sdef, &poly)
            };
            app.se_shape = Some(new_sid);
        }
    }
}

