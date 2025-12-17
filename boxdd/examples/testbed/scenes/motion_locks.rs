use boxdd as bd;
use dear_imgui_rs as imgui;

// Demonstrates linear and angular motion locks on a dynamic body.

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Create a dynamic box we can lock/unlock
    let b = app
        .world
        .create_body_id(
            bd::BodyBuilder::new()
                .body_type(bd::BodyType::Dynamic)
                .position([0.0, 4.0])
                .build(),
        );
    app.created_bodies += 1;
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    let _ = app
        .world
        .create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.6, 0.4));
    app.created_shapes += 1;
    app.ml_body = Some(b);

    // Apply initial velocity so locks are visible
    app.world.set_body_linear_velocity(b, [5.0, 0.0]);
    app.world.set_body_angular_velocity(b, 2.0);

    // Apply current locks
    apply_locks(app);
}

fn apply_locks(app: &mut super::PhysicsApp) {
    if let Some(bid) = app.ml_body {
        let locks = boxdd_sys::ffi::b2MotionLocks {
            linearX: app.ml_lock_x,
            linearY: app.ml_lock_y,
            angularZ: app.ml_lock_rot,
        };
        app.world.set_body_motion_locks(bid, locks);
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
    if ui.button("Impulse +X") && let Some(id) = app.ml_body {
        app.world
            .body_apply_linear_impulse_to_center(id, [15.0, 0.0], true);
    }
    ui.same_line();
    if ui.button("Impulse +Y") && let Some(id) = app.ml_body {
        app.world
            .body_apply_linear_impulse_to_center(id, [0.0, 15.0], true);
    }
    ui.same_line();
    if ui.button("Spin") && let Some(id) = app.ml_body {
        app.world.body_apply_angular_impulse(id, 8.0, true);
    }
    ui.text("Motion Locks: toggle constraints and apply impulses");
}
