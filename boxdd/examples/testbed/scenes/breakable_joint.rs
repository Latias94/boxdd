use boxdd as bd;
use dear_imgui as imgui;

// Breakable joint: uses joint base force/torque thresholds and listens for joint events.

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    // Two dynamic boxes linked by a distance joint with thresholds
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    let a = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([-2.0, 4.0]).build());
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(a, &sdef, &bd::shapes::box_polygon(0.5, 0.5));
    app.created_shapes += 1;

    let b = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([2.0, 4.0]).build());
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.5, 0.5));
    app.created_shapes += 1;

    // A static platform in the middle so we can smash A into it
    let _ = app.world.create_polygon_shape_for(
        ground,
        &bd::ShapeDef::builder().density(0.0).build(),
        &bd::shapes::box_polygon(0.2, 1.2),
    );
    app.created_shapes += 1;

    // Distance joint between A and B; thresholds will be set after creation
    let j = app
        .world
        .distance(a, b)
        .anchors_world([-1.5, 4.0], [1.5, 4.0])
        .length(4.0)
        .build();
    app.created_joints += 1;
    unsafe {
        boxdd_sys::ffi::b2Joint_SetForceThreshold(j.id(), app.bj_force_thres);
        boxdd_sys::ffi::b2Joint_SetTorqueThreshold(j.id(), app.bj_torque_thres);
    }
}

pub fn tick(app: &mut super::PhysicsApp) {
    // Count joint events (breaks)
    let evs = app.world.joint_events();
    if !evs.is_empty() {
        app.bj_broken += evs.len();
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut f = app.bj_force_thres;
    let mut t = app.bj_torque_thres;
    let changed = ui.slider("Force Threshold", 0.0, 200.0, &mut f)
        || ui.slider("Torque Threshold", 0.0, 200.0, &mut t);
    if changed {
        app.bj_force_thres = f;
        app.bj_torque_thres = t;
        let _ = app.reset();
    }
    if ui.button("Smash Left Box +X") {
        // Apply impulse to left box to stress the joint
        // Best-effort: search the body at approximately [-2, 4] (first dynamic we created)
        // In this minimal setup we simply cast a ray and nudge the first hit body.
        let hits = app.world.cast_ray_all([-3.0, 4.0], [2.0, 0.0], bd::QueryFilter::default());
        if let Some(h) = hits.first() {
            // Convert hit shape to body id
            let sid = h.shape_id;
            let bid = unsafe { boxdd_sys::ffi::b2Shape_GetBody(sid) };
            unsafe { boxdd_sys::ffi::b2Body_ApplyLinearImpulseToCenter(bid, boxdd_sys::ffi::b2Vec2 { x: 50.0, y: 0.0 }, true) };
        }
    }
    ui.text(format!("Breakable: joint events seen={}", app.bj_broken));
}
