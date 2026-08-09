use boxdd as bd;
use dear_imgui_rs as imgui;

// Demonstrates linear and angular motion locks on a dynamic body.

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Create a dynamic box we can lock/unlock
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
    let _ = app
        .world
        .body(b)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef,
            &bd::shapes::box_polygon(0.6, 0.4).expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    app.ml_body = Some(b);

    // Apply initial velocity so locks are visible
    app.world
        .body(b)
        .expect("valid testbed operation")
        .set_linear_velocity([5.0, 0.0])
        .expect("valid testbed operation");
    app.world
        .body(b)
        .expect("valid testbed operation")
        .set_angular_velocity(2.0)
        .expect("valid testbed operation");

    // Apply current locks
    apply_locks(app);
}

fn apply_locks(app: &mut super::PhysicsApp) {
    if let Some(bid) = app.ml_body {
        let locks = bd::MotionLocks::new(app.ml_lock_x, app.ml_lock_y, app.ml_lock_rot);
        app.world
            .body(bid)
            .expect("valid testbed operation")
            .set_motion_locks(locks)
            .expect("valid testbed operation");
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut lx = app.ml_lock_x;
    let mut ly = app.ml_lock_y;
    let mut lr = app.ml_lock_rot;
    let changed = ui.checkbox("Lock Linear X", &mut lx)
        || ui.checkbox("Lock Linear Y", &mut ly)
        || ui.checkbox("Lock Rotation", &mut lr);
    if changed {
        app.ml_lock_x = lx;
        app.ml_lock_y = ly;
        app.ml_lock_rot = lr;
        apply_locks(app);
    }
    if ui.button("Impulse +X")
        && let Some(id) = app.ml_body
    {
        app.world
            .body(id)
            .expect("valid testbed operation")
            .apply_linear_impulse_to_center([15.0, 0.0], true)
            .expect("valid testbed operation");
    }
    ui.same_line();
    if ui.button("Impulse +Y")
        && let Some(id) = app.ml_body
    {
        app.world
            .body(id)
            .expect("valid testbed operation")
            .apply_linear_impulse_to_center([0.0, 15.0], true)
            .expect("valid testbed operation");
    }
    ui.same_line();
    if ui.button("Spin")
        && let Some(id) = app.ml_body
    {
        app.world
            .body(id)
            .expect("valid testbed operation")
            .apply_angular_impulse(8.0, true)
            .expect("valid testbed operation");
    }
    ui.text("Motion Locks: toggle constraints and apply impulses");
}
