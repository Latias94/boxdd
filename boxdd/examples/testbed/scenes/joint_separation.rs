use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    // Create a small chain of wheel joints between ground and boxes
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    let mut prev = ground;
    app.js_joint_ids.clear();
    for i in 0..app.js_count.max(1) {
        let x = -5.0 + (i as f32) * 1.5;
        let b = app
            .world
            .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([x, 3.0]).build());
        app.created_bodies += 1;
        let _ = app
            .world
            .create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.5, 0.3));
        app.created_shapes += 1;
        let j = app
            .world
            .wheel(prev, b)
            .anchors_world([x - 0.75, 3.0], [x, 3.0])
            .axis_world([1.0_f32, 0.0])
            .spring(4.0, 0.7)
            .build();
        app.created_joints += 1;
        app.js_joint_ids.push(j.id());
        prev = b;
    }
}

pub fn tick(app: &mut super::PhysicsApp) {
    // Compute min/max separation across joints each frame
    let mut min_lin = f32::MAX;
    let mut max_lin = f32::MIN;
    let mut min_ang = f32::MAX;
    let mut max_ang = f32::MIN;
    for &jid in &app.js_joint_ids {
        let lin = unsafe { boxdd_sys::ffi::b2Joint_GetLinearSeparation(jid) };
        let ang = unsafe { boxdd_sys::ffi::b2Joint_GetAngularSeparation(jid) };
        min_lin = min_lin.min(lin);
        max_lin = max_lin.max(lin);
        min_ang = min_ang.min(ang);
        max_ang = max_ang.max(ang);
    }
    app.js_min_lin = if min_lin.is_finite() { min_lin } else { 0.0 };
    app.js_max_lin = if max_lin.is_finite() { max_lin } else { 0.0 };
    app.js_min_ang = if min_ang.is_finite() { min_ang } else { 0.0 };
    app.js_max_ang = if max_ang.is_finite() { max_ang } else { 0.0 };
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut n = app.js_count;
    if ui.slider("Joint Count", 1, 12, &mut n) {
        app.js_count = n.max(1);
        let _ = app.reset();
    }
    ui.text(format!(
        "Separation lin[min={:.3}, max={:.3}] ang[min={:.3}, max={:.3}]",
        app.js_min_lin, app.js_max_lin, app.js_min_ang, app.js_max_ang
    ));
}
