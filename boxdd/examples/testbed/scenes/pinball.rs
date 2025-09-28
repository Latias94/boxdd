use boxdd as bd;
use dear_imgui as imgui;

// Pinball-style dense collision demo using CCD on the ball.
// Static boundaries and bumpers are attached to ground; balls are dynamic bullets.

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    // Walls and floor as segments
    let sdef = bd::ShapeDef::builder()
        .density(0.0)
        .build();
    // Left/right walls
    let _ = app.world.create_segment_shape_for(ground, &sdef, &bd::shapes::segment([-8.0, 0.0], [-8.0, 14.0]));
    app.created_shapes += 1;
    let _ = app.world.create_segment_shape_for(ground, &sdef, &bd::shapes::segment([8.0, 0.0], [8.0, 14.0]));
    app.created_shapes += 1;
    // Slanted sides into the center
    let _ = app.world.create_segment_shape_for(ground, &sdef, &bd::shapes::segment([-8.0, 0.0], [0.0, -2.0]));
    app.created_shapes += 1;
    let _ = app.world.create_segment_shape_for(ground, &sdef, &bd::shapes::segment([8.0, 0.0], [0.0, -2.0]));
    app.created_shapes += 1;
    // Bottom floor
    let _ = app.world.create_segment_shape_for(ground, &sdef, &bd::shapes::segment([-8.0, -2.0], [8.0, -2.0]));
    app.created_shapes += 1;

    // Three round bumpers
    let bumper_mat = bd::shapes::SurfaceMaterial::default().restitution(1.2).friction(0.2);
    let bumper_def = bd::ShapeDef::builder().material(bumper_mat).density(0.0).build();
    let bump_centers = [[-4.0, 6.0], [0.0, 8.0], [4.0, 6.0]];
    for &c in &bump_centers {
        let _ = app
            .world
            .create_circle_shape_for(ground, &bumper_def, &bd::shapes::circle(c, 0.8));
        app.created_shapes += 1;
    }

    // Optional flippers (revolute motors)
    if app.pb_flippers {
        // Left flipper
        let l = app
            .world
            .create_body_id(
                bd::BodyBuilder::new()
                    .body_type(bd::BodyType::Dynamic)
                    .position([-5.5, -1.2])
                    .build(),
            );
        app.created_bodies += 1;
        let _ = app
            .world
            .create_polygon_shape_for(l, &bd::ShapeDef::builder().density(1.0).build(), &bd::shapes::box_polygon(1.4, 0.15));
        app.created_shapes += 1;
        // Use ID API so we don't drop RAII joint immediately
        let lj = app.world.create_revolute_joint_world_id(ground, l, [-5.5, -1.2]);
        unsafe {
            boxdd_sys::ffi::b2RevoluteJoint_EnableLimit(lj, true);
            let to_rad = std::f32::consts::PI / 180.0;
            boxdd_sys::ffi::b2RevoluteJoint_SetLimits(
                lj,
                app.pb_left_lower_deg * to_rad,
                app.pb_left_upper_deg * to_rad,
            );
            boxdd_sys::ffi::b2RevoluteJoint_EnableMotor(lj, true);
            boxdd_sys::ffi::b2RevoluteJoint_SetMotorSpeed(lj, 0.0);
            boxdd_sys::ffi::b2RevoluteJoint_SetMaxMotorTorque(lj, app.pb_flipper_torque);
        }
        app.pb_left_joint = Some(lj);

        // Right flipper
        let r = app
            .world
            .create_body_id(
                bd::BodyBuilder::new()
                    .body_type(bd::BodyType::Dynamic)
                    .position([5.5, -1.2])
                    .build(),
            );
        app.created_bodies += 1;
        let _ = app
            .world
            .create_polygon_shape_for(r, &bd::ShapeDef::builder().density(1.0).build(), &bd::shapes::box_polygon(1.4, 0.15));
        app.created_shapes += 1;
        let rj = app.world.create_revolute_joint_world_id(ground, r, [5.5, -1.2]);
        unsafe {
            boxdd_sys::ffi::b2RevoluteJoint_EnableLimit(rj, true);
            let to_rad = std::f32::consts::PI / 180.0;
            boxdd_sys::ffi::b2RevoluteJoint_SetLimits(
                rj,
                app.pb_right_lower_deg * to_rad,
                app.pb_right_upper_deg * to_rad,
            );
            boxdd_sys::ffi::b2RevoluteJoint_EnableMotor(rj, true);
            boxdd_sys::ffi::b2RevoluteJoint_SetMotorSpeed(rj, 0.0);
            boxdd_sys::ffi::b2RevoluteJoint_SetMaxMotorTorque(rj, app.pb_flipper_torque);
        }
        app.pb_right_joint = Some(rj);

        // Store body ids for impulses on button press
        app.pb_left_flipper = Some(l);
        app.pb_right_flipper = Some(r);
    }
}

fn spawn_ball(app: &mut super::PhysicsApp) {
    let b = app
        .world
        .create_body_id(
            bd::BodyBuilder::new()
                .body_type(bd::BodyType::Dynamic)
                .position([0.0, 12.0])
                .bullet(true)
                .build(),
        );
    app.created_bodies += 1;
    let mat = bd::shapes::SurfaceMaterial::default()
        .restitution(app.pb_restitution)
        .friction(0.2);
    let sdef = bd::ShapeDef::builder().material(mat).density(1.0).build();
    let _ = app
        .world
        .create_circle_shape_for(b, &sdef, &bd::shapes::circle([0.0, 0.0], app.pb_ball_radius));
    app.created_shapes += 1;
    // Nudge with initial velocity for fun
    unsafe { boxdd_sys::ffi::b2Body_SetLinearVelocity(b, boxdd_sys::ffi::b2Vec2 { x: 6.0, y: -2.0 }) };
    app.pb_ball_count += 1;
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut r = app.pb_restitution;
    let mut rad = app.pb_ball_radius;
    let mut flippers = app.pb_flippers;
    let mut torque = app.pb_flipper_torque;
    let mut impulse = app.pb_flip_impulse;
    let changed = ui.slider("Ball Restitution", 0.0, 1.5, &mut r)
        || ui.slider("Ball Radius", 0.1, 0.6, &mut rad)
        || ui.checkbox("Enable Flippers", &mut flippers)
        || ui.slider("Flipper Torque (N·m)", 10.0, 200.0, &mut torque)
        || ui.slider("Flip Impulse", 0.0, 10.0, &mut impulse)
        || ui.checkbox("Hold L", &mut app.pb_hold_left)
        || ui.checkbox("Hold R", &mut app.pb_hold_right)
        || ui.slider("Flipper Speed (deg/s)", 0.0, 720.0, &mut app.pb_flip_speed_deg)
        || ui.slider("Left Limit Lower (deg)", -120.0, 0.0, &mut app.pb_left_lower_deg)
        || ui.slider("Left Limit Upper (deg)", 0.0, 120.0, &mut app.pb_left_upper_deg)
        || ui.slider("Right Limit Lower (deg)", -120.0, 0.0, &mut app.pb_right_lower_deg)
        || ui.slider("Right Limit Upper (deg)", 0.0, 120.0, &mut app.pb_right_upper_deg);
    if changed {
        app.pb_restitution = r;
        app.pb_ball_radius = rad;
        app.pb_flip_impulse = impulse;
        if flippers != app.pb_flippers {
            app.pb_flippers = flippers;
            let _ = app.reset();
        } else if (torque - app.pb_flipper_torque).abs() > f32::EPSILON {
            app.pb_flipper_torque = torque;
            // Update joints in place
            if let Some(j) = app.pb_left_joint {
                unsafe { boxdd_sys::ffi::b2RevoluteJoint_SetMaxMotorTorque(j, torque) };
            }
            if let Some(j) = app.pb_right_joint {
                unsafe { boxdd_sys::ffi::b2RevoluteJoint_SetMaxMotorTorque(j, torque) };
            }
        }
        // Update limits in place if changed
        let to_rad = std::f32::consts::PI / 180.0;
        if let Some(j) = app.pb_left_joint {
            unsafe {
                boxdd_sys::ffi::b2RevoluteJoint_SetLimits(
                    j,
                    app.pb_left_lower_deg * to_rad,
                    app.pb_left_upper_deg * to_rad,
                )
            };
        }
        if let Some(j) = app.pb_right_joint {
            unsafe {
                boxdd_sys::ffi::b2RevoluteJoint_SetLimits(
                    j,
                    app.pb_right_lower_deg * to_rad,
                    app.pb_right_upper_deg * to_rad,
                )
            };
        }
    }
    if ui.button("Spawn Ball") {
        spawn_ball(app);
    }
    ui.same_line();
    if ui.button("Flip L") {
        if let Some(id) = app.pb_left_flipper {
            unsafe { boxdd_sys::ffi::b2Body_ApplyAngularImpulse(id, app.pb_flip_impulse, true) };
        }
    }
    ui.same_line();
    if ui.button("Flip R") {
        if let Some(id) = app.pb_right_flipper {
            unsafe { boxdd_sys::ffi::b2Body_ApplyAngularImpulse(id, -app.pb_flip_impulse, true) };
        }
    }
    // Live joint telemetry
    if let Some(j) = app.pb_left_joint {
        let ang = unsafe { boxdd_sys::ffi::b2RevoluteJoint_GetAngle(j) };
        let tq = unsafe { boxdd_sys::ffi::b2RevoluteJoint_GetMotorTorque(j) };
        ui.text(format!("Left: angle={:.2} rad torque={:.1} N·m", ang, tq));
    }
    if let Some(j) = app.pb_right_joint {
        let ang = unsafe { boxdd_sys::ffi::b2RevoluteJoint_GetAngle(j) };
        let tq = unsafe { boxdd_sys::ffi::b2RevoluteJoint_GetMotorTorque(j) };
        ui.text(format!("Right: angle={:.2} rad torque={:.1} N·m", ang, tq));
    }
    ui.text(format!("Pinball: balls spawned={}", app.pb_ball_count));
}

pub fn tick(app: &mut super::PhysicsApp) {
    // Continuous motor control for flippers based on hold checkboxes.
    let speed_rad = app.pb_flip_speed_deg * std::f32::consts::PI / 180.0;
    let ls = if app.pb_hold_left { speed_rad } else { 0.0 };
    let rs = if app.pb_hold_right { -speed_rad } else { 0.0 };
    if let Some(j) = app.pb_left_joint {
        unsafe { boxdd_sys::ffi::b2RevoluteJoint_SetMotorSpeed(j, ls) };
    }
    if let Some(j) = app.pb_right_joint {
        unsafe { boxdd_sys::ffi::b2RevoluteJoint_SetMotorSpeed(j, rs) };
    }
}
