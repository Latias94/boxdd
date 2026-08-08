use boxdd as bd;
use dear_imgui_rs as imgui;

// Breakable joint: uses joint base force/torque thresholds and listens for joint events.

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    // Two dynamic boxes linked by a distance joint with thresholds
    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .build()
        .expect("valid testbed definition");
    let a = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([-2.0, 4.0])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    app.world
        .body(a)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef,
            &bd::shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;

    let b = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([2.0, 4.0])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    app.world
        .body(b)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef,
            &bd::shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;

    // A static platform in the middle so we can smash A into it
    let _ = app
        .world
        .body(ground)
        .expect("valid testbed operation")
        .create_polygon(
            &bd::ShapeDef::builder()
                .density(0.0)
                .build()
                .expect("valid testbed definition"),
            &bd::shapes::box_polygon(0.2, 1.2).expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;

    // Distance joint between A and B; thresholds will be set after creation
    let joint = app
        .world
        .distance(a, b)
        .anchors_world([-1.5, 4.0], [1.5, 4.0])
        .length(4.0)
        .build()
        .expect("valid testbed distance joint");
    app.created_joints += 1;
    let mut joint = app
        .world
        .joint(joint)
        .expect("valid testbed distance joint");
    joint
        .set_force_threshold(app.bj_force_thres)
        .expect("valid testbed force threshold");
    joint
        .set_torque_threshold(app.bj_torque_thres)
        .expect("valid testbed torque threshold");
}

pub fn tick(app: &mut super::PhysicsApp, events: Option<&bd::StepEventsSnapshot>) {
    if let Some(events) = events {
        app.bj_broken += events.joint.len();
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
        let query = app.world.query().expect("available testbed query");
        let scratch = &mut app.scratch;
        query
            .cast_ray_all_into(
                bd::Position::new(-3.0, 4.0),
                [2.0, 0.0],
                bd::QueryFilter::default(),
                &mut scratch.ray_hits,
            )
            .expect("valid testbed ray cast");
        if let Some(h) = scratch.ray_hits.as_slice().first() {
            // Convert hit shape to body id
            let sid = h.shape_id;
            let bid = app
                .world
                .shape(sid)
                .expect("valid testbed operation")
                .body_id()
                .expect("valid testbed operation");
            app.world
                .body(bid)
                .expect("valid testbed operation")
                .apply_linear_impulse_to_center([50.0, 0.0], true)
                .expect("valid testbed operation");
        }
    }
    ui.text(format!("Breakable: joint events seen={}", app.bj_broken));
}
