use boxdd as bd;
use dear_imgui_rs as imgui;

// Continuous Lab: Bullet, Ghost Bumps, Restitution Threshold, Pinball, Segment Slide

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    match app.cl_mode {
        // Bullet
        0 => {
            app.world
                .enable_continuous(true)
                .expect("valid testbed operation");
            app.world
                .set_hit_event_threshold(app.bullet_threshold)
                .expect("valid testbed operation");
            let wall = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .position([5.0, 0.0])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("valid testbed operation");
            app.created_bodies += 1;
            app.world
                .body(wall)
                .expect("valid testbed operation")
                .create_polygon(
                    &bd::ShapeDef::builder()
                        .density(0.0)
                        .enable_contact_events(true)
                        .enable_hit_events(true)
                        .build()
                        .expect("valid testbed definition"),
                    &bd::shapes::box_polygon(0.5, 3.0).expect("valid polygon geometry"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            let bullet = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .body_type(bd::BodyType::Dynamic)
                        .position([0.0, 0.0])
                        .bullet(true)
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("valid testbed operation");
            app.created_bodies += 1;
            app.world
                .body(bullet)
                .expect("valid testbed operation")
                .create_circle(
                    &bd::ShapeDef::builder()
                        .density(1.0)
                        .enable_contact_events(true)
                        .enable_hit_events(true)
                        .build()
                        .expect("valid testbed definition"),
                    &bd::shapes::circle([0.0, 0.0], app.bullet_radius)
                        .expect("bullet radius must remain positive"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            app.world
                .body(bullet)
                .expect("valid testbed operation")
                .set_linear_velocity([app.bullet_speed, 0.0])
                .expect("valid testbed operation");
        }
        // Ghost Bumps
        1 => {
            app.world
                .enable_continuous(true)
                .expect("valid testbed operation");
            app.world
                .set_hit_event_threshold(0.05)
                .expect("valid testbed operation");
            let h = app.gb_bump_h.max(0.0);
            let step = 0.5_f32;
            let count = 40_i32;
            let sdef = bd::ShapeDef::builder()
                .density(0.0)
                .build()
                .expect("valid testbed definition");
            for i in -count..=count {
                let x0 = (i as f32) * step;
                let x1 = (i as f32 + 1.0) * step;
                let y0 = if i % 2 == 0 { 0.0 } else { h };
                let y1 = if (i + 1) % 2 == 0 { 0.0 } else { h };
                app.world
                    .body(ground)
                    .expect("valid testbed operation")
                    .create_segment(
                        &sdef,
                        &bd::shapes::segment([x0, y0], [x1, y1])
                            .expect("ghost-bump segment must be valid"),
                    )
                    .expect("valid testbed operation");
                app.created_shapes += 1;
            }
            let mover = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .body_type(bd::BodyType::Dynamic)
                        .position([-10.0, 1.5])
                        .bullet(true)
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("valid testbed operation");
            app.created_bodies += 1;
            app.world
                .body(mover)
                .expect("valid testbed operation")
                .create_circle(
                    &bd::ShapeDef::builder()
                        .density(1.0)
                        .build()
                        .expect("valid testbed definition"),
                    &bd::shapes::circle([0.0, 0.0], 0.3)
                        .expect("ghost-bump mover circle must be valid"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            app.world
                .body(mover)
                .expect("valid testbed operation")
                .set_linear_velocity([app.gb_speed, 0.0])
                .expect("valid testbed operation");
        }
        // Restitution Threshold
        2 => {
            app.world
                .enable_continuous(true)
                .expect("valid testbed operation");
            app.world
                .set_restitution_threshold(app.cr_threshold)
                .expect("valid testbed operation");
            // The ground was created by caller; spawn a ball
            let ball = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .body_type(bd::BodyType::Dynamic)
                        .position([0.0, app.cr_drop_y])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("valid testbed operation");
            app.created_bodies += 1;
            let sdef = bd::ShapeDef::builder()
                .density(1.0)
                .material(
                    bd::SurfaceMaterial::default()
                        .with_restitution(app.cr_restitution)
                        .expect("restitution slider must remain valid"),
                )
                .build()
                .expect("valid testbed definition");
            app.world
                .body(ball)
                .expect("valid testbed operation")
                .create_circle(
                    &sdef,
                    &bd::shapes::circle([0.0, 0.0], 0.5)
                        .expect("restitution ball circle must be valid"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
        }
        // Pinball
        3 => {
            // Walls and floor
            let sdef = bd::ShapeDef::builder()
                .density(0.0)
                .build()
                .expect("valid testbed definition");
            app.world
                .body(ground)
                .expect("valid testbed operation")
                .create_segment(
                    &sdef,
                    &bd::shapes::segment([-8.0, 0.0], [-8.0, 14.0])
                        .expect("pinball wall segment must be valid"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            app.world
                .body(ground)
                .expect("valid testbed operation")
                .create_segment(
                    &sdef,
                    &bd::shapes::segment([8.0, 0.0], [8.0, 14.0])
                        .expect("pinball wall segment must be valid"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            app.world
                .body(ground)
                .expect("valid testbed operation")
                .create_segment(
                    &sdef,
                    &bd::shapes::segment([-8.0, 0.0], [0.0, -2.0])
                        .expect("pinball floor segment must be valid"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            app.world
                .body(ground)
                .expect("valid testbed operation")
                .create_segment(
                    &sdef,
                    &bd::shapes::segment([8.0, 0.0], [0.0, -2.0])
                        .expect("pinball floor segment must be valid"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            app.world
                .body(ground)
                .expect("valid testbed operation")
                .create_segment(
                    &sdef,
                    &bd::shapes::segment([-8.0, -2.0], [8.0, -2.0])
                        .expect("pinball floor segment must be valid"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            // Bumpers
            let mat = bd::shapes::SurfaceMaterial::default()
                .with_restitution(1.2)
                .expect("pinball restitution must be valid")
                .with_friction(0.2)
                .expect("pinball friction must be valid");
            let bdef = bd::ShapeDef::builder()
                .material(mat)
                .density(0.0)
                .build()
                .expect("valid testbed definition");
            for &c in &[[-4.0, 6.0], [0.0, 8.0], [4.0, 6.0]] {
                app.world
                    .body(ground)
                    .expect("valid testbed operation")
                    .create_circle(
                        &bdef,
                        &bd::shapes::circle(c, 0.8).expect("pinball bumper circle must be valid"),
                    )
                    .expect("valid testbed operation");
                app.created_shapes += 1;
            }
            // Optional flippers (motors)
            if app.pb_flippers {
                let l = app
                    .world
                    .create_body(
                        bd::BodyBuilder::from(app.foundation.body_def())
                            .body_type(bd::BodyType::Dynamic)
                            .position([-5.5, -1.2])
                            .build()
                            .expect("valid testbed definition"),
                    )
                    .expect("valid testbed operation");
                app.created_bodies += 1;
                app.world
                    .body(l)
                    .expect("valid testbed operation")
                    .create_polygon(
                        &bd::ShapeDef::builder()
                            .density(1.0)
                            .build()
                            .expect("valid testbed definition"),
                        &bd::shapes::box_polygon(1.4, 0.15).expect("valid polygon geometry"),
                    )
                    .expect("valid testbed operation");
                app.created_shapes += 1;
                let lj = app
                    .world
                    .create_revolute_joint_world(ground, l, [-5.5, -1.2])
                    .expect("valid testbed operation");
                app.world
                    .joint(lj)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .enable_limit(true)
                    .expect("valid testbed operation");
                let to_rad = std::f32::consts::PI / 180.0;
                app.world
                    .joint(lj)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .set_limits(
                        app.pb_left_lower_deg * to_rad,
                        app.pb_left_upper_deg * to_rad,
                    )
                    .expect("valid testbed operation");
                app.world
                    .joint(lj)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .enable_motor(true)
                    .expect("valid testbed operation");
                app.world
                    .joint(lj)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .set_motor_speed(0.0)
                    .expect("valid testbed operation");
                app.world
                    .joint(lj)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .set_max_motor_torque(app.pb_flipper_torque)
                    .expect("valid testbed operation");
                app.pb_left_joint = Some(lj);
                let r = app
                    .world
                    .create_body(
                        bd::BodyBuilder::from(app.foundation.body_def())
                            .body_type(bd::BodyType::Dynamic)
                            .position([5.0 + 0.5, -1.2])
                            .build()
                            .expect("valid testbed definition"),
                    )
                    .expect("valid testbed operation");
                app.created_bodies += 1;
                app.world
                    .body(r)
                    .expect("valid testbed operation")
                    .create_polygon(
                        &bd::ShapeDef::builder()
                            .density(1.0)
                            .build()
                            .expect("valid testbed definition"),
                        &bd::shapes::box_polygon(1.4, 0.15).expect("valid polygon geometry"),
                    )
                    .expect("valid testbed operation");
                app.created_shapes += 1;
                let rj = app
                    .world
                    .create_revolute_joint_world(ground, r, [5.5, -1.2])
                    .expect("valid testbed operation");
                app.world
                    .joint(rj)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .enable_limit(true)
                    .expect("valid testbed operation");
                let to_rad = std::f32::consts::PI / 180.0;
                app.world
                    .joint(rj)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .set_limits(
                        app.pb_right_lower_deg * to_rad,
                        app.pb_right_upper_deg * to_rad,
                    )
                    .expect("valid testbed operation");
                app.world
                    .joint(rj)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .enable_motor(true)
                    .expect("valid testbed operation");
                app.world
                    .joint(rj)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .set_motor_speed(0.0)
                    .expect("valid testbed operation");
                app.world
                    .joint(rj)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .set_max_motor_torque(app.pb_flipper_torque)
                    .expect("valid testbed operation");
                app.pb_right_joint = Some(rj);
                app.pb_left_flipper = Some(l);
                app.pb_right_flipper = Some(r);
            }
        }
        // Segment Slide
        4 => {
            let ang = app.ss_slope_deg.to_radians();
            let len = 14.0_f32;
            let dx = 0.5 * len * ang.cos();
            let dy = 0.5 * len * ang.sin();
            let p1 = [-dx, 0.0 + dy];
            let p2 = [dx, 0.0 - dy];
            let sdef = bd::ShapeDef::builder()
                .density(0.0)
                .build()
                .expect("valid testbed definition");
            app.world
                .body(ground)
                .expect("valid testbed operation")
                .create_segment(
                    &sdef,
                    &bd::shapes::segment(p1, p2).expect("segment-slide slope must be valid"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            let dxv = p2[0] - p1[0];
            let dyv = p2[1] - p1[1];
            let lenv = (dxv * dxv + dyv * dyv).sqrt().max(1e-6);
            let dirx = dxv / lenv;
            let diry = dyv / lenv;
            let start = bd::Vec2::new(-dirx * 5.0, -diry * 5.0 + 4.0);
            let b = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .body_type(bd::BodyType::Dynamic)
                        .position([start.x, start.y])
                        .bullet(true)
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("valid testbed operation");
            app.created_bodies += 1;
            let sdef_dyn = bd::ShapeDef::builder()
                .density(1.0)
                .build()
                .expect("valid testbed definition");
            let cap = bd::shapes::capsule([-1.2, 0.0], [1.2, 0.0], 0.08)
                .expect("segment-slide capsule must be valid");
            app.world
                .body(b)
                .expect("valid testbed operation")
                .create_capsule(&sdef_dyn, &cap)
                .expect("valid testbed operation");
            app.created_shapes += 1;
            app.world
                .body(b)
                .expect("valid testbed operation")
                .set_linear_velocity([dirx * app.ss_speed, diry * app.ss_speed])
                .expect("valid testbed operation");
        }
        _ => {}
    }
}

pub fn tick(app: &mut super::PhysicsApp, events: Option<&bd::StepEventsSnapshot>) {
    match app.cl_mode {
        1 => {
            // ghost bumps: accumulate hits
            if let Some(events) = events {
                app.gb_hits += events.contact.hit.len();
            }
        }
        3 => {
            // pinball flipper hold control
            let speed_rad = app.pb_flip_speed_deg * std::f32::consts::PI / 180.0;
            let ls = if app.pb_hold_left { speed_rad } else { 0.0 };
            let rs = if app.pb_hold_right { -speed_rad } else { 0.0 };
            if let Some(j) = app.pb_left_joint {
                app.world
                    .joint(j)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .set_motor_speed(ls)
                    .expect("valid testbed operation");
            }
            if let Some(j) = app.pb_right_joint {
                app.world
                    .joint(j)
                    .expect("valid testbed operation")
                    .into_revolute()
                    .expect("valid testbed operation")
                    .set_motor_speed(rs)
                    .expect("valid testbed operation");
            }
        }
        _ => {}
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let names = [
        "Bullet",
        "Ghost Bumps",
        "Restitution Threshold",
        "Pinball",
        "Segment Slide",
    ];
    let mut m = app.cl_mode;
    if ui.combo_simple_string("Continuous Lab", &mut m, &names) && m != app.cl_mode {
        app.cl_mode = m;
        let _ = app.reset();
        return;
    }
    match app.cl_mode {
        0 => {
            let mut dt = app.bullet_dt;
            let mut sub = app.bullet_substeps;
            let mut sp = app.bullet_speed;
            let mut rad = app.bullet_radius;
            let mut th = app.bullet_threshold;
            if ui.slider("dt", 1.0 / 1000.0, 1.0 / 30.0, &mut dt) {
                app.bullet_dt = dt.max(1e-5);
            }
            if ui.slider("Substeps", 1, 64, &mut sub) {
                app.bullet_substeps = sub.max(1);
            }
            if ui.slider("Speed", 1.0, 120.0, &mut sp) {
                app.bullet_speed = sp;
                let _ = app.reset();
            }
            if ui.slider("Radius", 0.05, 1.0, &mut rad) {
                app.bullet_radius = rad;
                let _ = app.reset();
            }
            if ui.slider("Hit Threshold", 0.0, 2.0, &mut th) {
                app.bullet_threshold = th;
                let _ = app.reset();
            }
        }
        1 => {
            let mut sp = app.gb_speed;
            let mut h = app.gb_bump_h;
            let changed = ui.slider("Speed", 1.0, 120.0, &mut sp)
                || ui.slider("Bump Height", 0.0, 1.0, &mut h);
            if changed {
                app.gb_speed = sp;
                app.gb_bump_h = h.max(0.0);
                let _ = app.reset();
            }
            if ui.button("Reset Hit Counter") {
                app.gb_hits = 0;
            }
            ui.text(format!("Ghost Bumps: hits={} (accumulated)", app.gb_hits));
        }
        2 => {
            let mut th = app.cr_threshold;
            let mut rest = app.cr_restitution;
            let mut drop_y = app.cr_drop_y;
            let changed = ui.slider("Restitution Threshold", 0.0, 5.0, &mut th)
                || ui.slider("Restitution", 0.0, 1.0, &mut rest)
                || ui.slider("Drop Height", 1.0, 20.0, &mut drop_y);
            if changed {
                app.cr_threshold = th.max(0.0);
                app.cr_restitution = rest.clamp(0.0, 1.0);
                app.cr_drop_y = drop_y.max(1.0);
                let _ = app.reset();
            }
        }
        3 => {
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
                || ui.slider(
                    "Flipper Speed (deg/s)",
                    0.0,
                    720.0,
                    &mut app.pb_flip_speed_deg,
                )
                || ui.slider(
                    "Left Limit Lower (deg)",
                    -120.0,
                    0.0,
                    &mut app.pb_left_lower_deg,
                )
                || ui.slider(
                    "Left Limit Upper (deg)",
                    0.0,
                    120.0,
                    &mut app.pb_left_upper_deg,
                )
                || ui.slider(
                    "Right Limit Lower (deg)",
                    -120.0,
                    0.0,
                    &mut app.pb_right_lower_deg,
                )
                || ui.slider(
                    "Right Limit Upper (deg)",
                    0.0,
                    120.0,
                    &mut app.pb_right_upper_deg,
                );
            if changed {
                app.pb_restitution = r;
                app.pb_ball_radius = rad;
                app.pb_flip_impulse = impulse;
                if flippers != app.pb_flippers {
                    app.pb_flippers = flippers;
                    let _ = app.reset();
                } else if (torque - app.pb_flipper_torque).abs() > f32::EPSILON {
                    app.pb_flipper_torque = torque;
                    if let Some(j) = app.pb_left_joint {
                        app.world
                            .joint(j)
                            .expect("valid testbed operation")
                            .into_revolute()
                            .expect("valid testbed operation")
                            .set_max_motor_torque(torque)
                            .expect("valid testbed operation");
                    }
                    if let Some(j) = app.pb_right_joint {
                        app.world
                            .joint(j)
                            .expect("valid testbed operation")
                            .into_revolute()
                            .expect("valid testbed operation")
                            .set_max_motor_torque(torque)
                            .expect("valid testbed operation");
                    }
                }
                let to_rad = std::f32::consts::PI / 180.0;
                if let Some(j) = app.pb_left_joint {
                    app.world
                        .joint(j)
                        .expect("valid testbed operation")
                        .into_revolute()
                        .expect("valid testbed operation")
                        .set_limits(
                            app.pb_left_lower_deg * to_rad,
                            app.pb_left_upper_deg * to_rad,
                        )
                        .expect("valid testbed operation");
                }
                if let Some(j) = app.pb_right_joint {
                    app.world
                        .joint(j)
                        .expect("valid testbed operation")
                        .into_revolute()
                        .expect("valid testbed operation")
                        .set_limits(
                            app.pb_right_lower_deg * to_rad,
                            app.pb_right_upper_deg * to_rad,
                        )
                        .expect("valid testbed operation");
                }
            }
            if ui.button("Spawn Ball") {
                // spawn ball
                let b = app
                    .world
                    .create_body(
                        bd::BodyBuilder::from(app.foundation.body_def())
                            .body_type(bd::BodyType::Dynamic)
                            .position([0.0, 12.0])
                            .bullet(true)
                            .build()
                            .expect("valid testbed definition"),
                    )
                    .expect("valid testbed operation");
                app.created_bodies += 1;
                let mat = bd::shapes::SurfaceMaterial::default()
                    .with_restitution(app.pb_restitution)
                    .expect("pinball restitution must be valid")
                    .with_friction(0.2)
                    .expect("pinball friction must be valid");
                let sdef = bd::ShapeDef::builder()
                    .material(mat)
                    .density(1.0)
                    .build()
                    .expect("valid testbed definition");
                app.world
                    .body(b)
                    .expect("valid testbed operation")
                    .create_circle(
                        &sdef,
                        &bd::shapes::circle([0.0, 0.0], app.pb_ball_radius)
                            .expect("pinball radius must remain positive"),
                    )
                    .expect("valid testbed operation");
                app.created_shapes += 1;
                app.world
                    .body(b)
                    .expect("valid testbed operation")
                    .set_linear_velocity([6.0, -2.0])
                    .expect("valid testbed operation");
                app.pb_ball_count += 1;
            }
        }
        4 => {
            let mut slope = app.ss_slope_deg;
            let mut speed = app.ss_speed;
            let changed = ui.slider("Slope (deg)", -45.0, 45.0, &mut slope)
                || ui.slider("Speed", 1.0, 80.0, &mut speed);
            if changed {
                app.ss_slope_deg = slope;
                app.ss_speed = speed;
                let _ = app.reset();
            }
            ui.text("Segment Slide: thin body tracks segment via CCD");
        }
        _ => {}
    }
}
