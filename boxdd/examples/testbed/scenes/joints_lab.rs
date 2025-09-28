use boxdd as bd;
use dear_imgui as imgui;

// Unified Joints Lab: quickly switch between joint samples without leaving the scene.

// Distance joint
fn build_distance(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let a = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([-2.0, 4.0]).build());
    app.created_bodies += 1;
    let b = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([2.0, 4.0]).build());
    app.created_bodies += 1;
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    app.world.create_polygon_shape_for(a, &sdef, &bd::shapes::box_polygon(0.4, 0.4));
    app.created_shapes += 1;
    app.world.create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.4, 0.4));
    app.created_shapes += 1;
    let _j = app
        .world
        .distance(a, b)
        .anchors_world([-2.0, 4.0], [2.0, 4.0])
        .length(app.dist_length)
        .spring(app.dist_hz, app.dist_dr)
        .build();
    app.created_joints += 1;
}
fn ui_distance(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut len = app.dist_length;
    let mut hz = app.dist_hz;
    let mut dr = app.dist_dr;
    if ui.slider("Length", 0.5, 8.0, &mut len) {
        app.dist_length = len;
        let _ = app.reset();
    }
    if ui.slider("Hertz", 0.0, 20.0, &mut hz) {
        app.dist_hz = hz.max(0.0);
        let _ = app.reset();
    }
    if ui.slider("Damping", 0.0, 1.0, &mut dr) {
        app.dist_dr = dr.clamp(0.0, 1.0);
        let _ = app.reset();
    }
}

// Motor joint
fn build_motor(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    let body = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([0.0, 2.5]).build());
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(
        body,
        &bd::ShapeDef::builder().density(1.0).build(),
        &bd::shapes::box_polygon(0.5, 0.3),
    );
    app.created_shapes += 1;
    let _j = app
        .world
        .motor_joint(ground, body)
        .linear_velocity([app.motor_lin_x, app.motor_lin_y])
        .angular_velocity(app.motor_ang_w)
        .max_velocity_force(app.motor_max_force)
        .max_velocity_torque(app.motor_max_torque)
        .linear_spring(app.motor_lin_hz, app.motor_lin_dr)
        .angular_spring(app.motor_ang_hz, app.motor_ang_dr)
        .build();
    app.created_joints += 1;
}
fn ui_motor(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut vx = app.motor_lin_x;
    let mut vy = app.motor_lin_y;
    let mut w = app.motor_ang_w;
    let mut f = app.motor_max_force;
    let mut t = app.motor_max_torque;
    let mut lhz = app.motor_lin_hz;
    let mut ldr = app.motor_lin_dr;
    let mut ahz = app.motor_ang_hz;
    let mut adr = app.motor_ang_dr;
    let changed = ui.slider("Linear VX", -10.0, 10.0, &mut vx)
        || ui.slider("Linear VY", -10.0, 10.0, &mut vy)
        || ui.slider("Angular W", -10.0, 10.0, &mut w)
        || ui.slider("Max Force", 0.0, 500.0, &mut f)
        || ui.slider("Max Torque", 0.0, 500.0, &mut t)
        || ui.slider("Lin Hz", 0.0, 20.0, &mut lhz)
        || ui.slider("Lin Damping", 0.0, 1.0, &mut ldr)
        || ui.slider("Ang Hz", 0.0, 20.0, &mut ahz)
        || ui.slider("Ang Damping", 0.0, 1.0, &mut adr);
    if changed {
        app.motor_lin_x = vx;
        app.motor_lin_y = vy;
        app.motor_ang_w = w;
        app.motor_max_force = f.max(0.0);
        app.motor_max_torque = t.max(0.0);
        app.motor_lin_hz = lhz.max(0.0);
        app.motor_lin_dr = ldr.clamp(0.0, 1.0);
        app.motor_ang_hz = ahz.max(0.0);
        app.motor_ang_dr = adr.clamp(0.0, 1.0);
        let _ = app.reset();
    }
}

// Wheel joint
fn build_wheel(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    let chassis = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([0.0, 3.0]).build());
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(
        chassis,
        &bd::ShapeDef::builder().density(1.0).build(),
        &bd::shapes::box_polygon(1.2, 0.25),
    );
    app.created_shapes += 1;
    let wheel = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([-0.8, 2.0]).build());
    app.created_bodies += 1;
    app.world.create_circle_shape_for(
        wheel,
        &bd::ShapeDef::builder().density(1.0).build(),
        &bd::shapes::circle([0.0, 0.0], 0.5),
    );
    app.created_shapes += 1;
    let mut builder = app
        .world
        .wheel(ground, wheel)
        .anchors_world([0.0, 2.0], [-0.8, 2.0])
        .axis_world([0.0, 1.0]);
    if app.wheel_enable_limit {
        builder = builder.limit(app.wheel_lower, app.wheel_upper);
    }
    if app.wheel_enable_motor {
        builder = builder.motor_deg(app.wheel_motor_torque, app.wheel_motor_speed_deg);
    }
    if app.wheel_enable_spring {
        builder = builder.spring(app.wheel_hz, app.wheel_dr);
    }
    let _j = builder.build();
    app.created_joints += 1;
    drop(_j);
    let _ = app
        .world
        .prismatic(ground, chassis)
        .anchors_world([0.0, 3.0], [0.0, 3.0])
        .axis_world([1.0, 0.0])
        .limit(-10.0, 10.0)
        .build();
    app.created_joints += 1;
}
fn ui_wheel(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut lim = app.wheel_enable_limit;
    let mut lo = app.wheel_lower;
    let mut hi = app.wheel_upper;
    let mut mot = app.wheel_enable_motor;
    let mut spd = app.wheel_motor_speed_deg;
    let mut tq = app.wheel_motor_torque;
    let mut spr = app.wheel_enable_spring;
    let mut hz = app.wheel_hz;
    let mut dr = app.wheel_dr;
    let changed = ui.checkbox("Enable Limit", &mut lim)
        || ui.slider("Lower (m)", -2.0, 0.0, &mut lo)
        || ui.slider("Upper (m)", 0.0, 2.0, &mut hi)
        || ui.checkbox("Enable Motor", &mut mot)
        || ui.slider("Motor Speed (deg/s)", -720.0, 720.0, &mut spd)
        || ui.slider("Max Motor Torque", 0.0, 500.0, &mut tq)
        || ui.checkbox("Enable Spring", &mut spr)
        || ui.slider("Hertz", 0.0, 20.0, &mut hz)
        || ui.slider("Damping", 0.0, 1.0, &mut dr);
    if changed {
        app.wheel_enable_limit = lim;
        app.wheel_lower = lo.min(hi);
        app.wheel_upper = hi.max(lo);
        app.wheel_enable_motor = mot;
        app.wheel_motor_speed_deg = spd;
        app.wheel_motor_torque = tq.max(0.0);
        app.wheel_enable_spring = spr;
        app.wheel_hz = hz.max(0.0);
        app.wheel_dr = dr.clamp(0.0, 1.0);
        let _ = app.reset();
    }
}

// Revolute motor
fn build_revolute(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    let rotor = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([0.0, 2.0]).build());
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(
        rotor,
        &bd::ShapeDef::builder().density(1.0).build(),
        &bd::shapes::box_polygon(1.0, 0.1),
    );
    app.created_shapes += 1;
    let base = app
        .world
        .joint_base_from_world_points(ground, rotor, [0.0, 2.0], [0.0, 2.0]);
    let rdef = bd::RevoluteJointDef::new(base)
        .limit_deg(app.revolute_lower_deg, app.revolute_upper_deg)
        .enable_motor(true)
        .max_motor_torque(app.revolute_torque)
        .motor_speed(app.revolute_speed);
    let _ = app.world.create_revolute_joint_id(&rdef);
    app.created_joints += 1;
}
fn ui_revolute(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut lo = app.revolute_lower_deg;
    let mut hi = app.revolute_upper_deg;
    let mut sp = app.revolute_speed;
    let mut tq = app.revolute_torque;
    if ui.slider("Lower (deg)", -180.0, 0.0, &mut lo) {
        app.revolute_lower_deg = lo;
        let _ = app.reset();
    }
    if ui.slider("Upper (deg)", 0.0, 180.0, &mut hi) {
        app.revolute_upper_deg = hi;
        let _ = app.reset();
    }
    if ui.slider("Motor Speed (rad/s)", 0.0, 10.0, &mut sp) {
        app.revolute_speed = sp;
        let _ = app.reset();
    }
    if ui.slider("Max Torque", 0.0, 200.0, &mut tq) {
        app.revolute_torque = tq;
        let _ = app.reset();
    }
}

// Prismatic elevator
fn build_prismatic(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    let platform = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([0.0, 1.0]).build());
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(
        platform,
        &bd::ShapeDef::builder().density(1.0).build(),
        &bd::shapes::box_polygon(1.0, 0.2),
    );
    app.created_shapes += 1;
    let axis = [0.0, 1.0];
    let anchor = [0.0, 1.0];
    let base = app
        .world
        .joint_base_from_world_with_axis(ground, platform, anchor, anchor, axis);
    let pdef = bd::PrismaticJointDef::new(base)
        .enable_limit(true)
        .lower_translation(app.prism_lower)
        .upper_translation(app.prism_upper)
        .enable_motor(true)
        .max_motor_force(app.prism_force)
        .motor_speed(app.prism_speed);
    let _ = app.world.create_prismatic_joint_id(&pdef);
    app.created_joints += 1;
}
fn ui_prismatic(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut lo = app.prism_lower;
    let mut hi = app.prism_upper;
    let mut sp = app.prism_speed;
    let mut f = app.prism_force;
    if ui.slider("Lower", -5.0, 5.0, &mut lo) {
        app.prism_lower = lo;
        let _ = app.reset();
    }
    if ui.slider("Upper", 0.0, 10.0, &mut hi) {
        app.prism_upper = hi;
        let _ = app.reset();
    }
    if ui.slider("Speed (m/s)", 0.0, 10.0, &mut sp) {
        app.prism_speed = sp;
        let _ = app.reset();
    }
    if ui.slider("Max Force", 0.0, 500.0, &mut f) {
        app.prism_force = f;
        let _ = app.reset();
    }
}

// Weld joint
fn build_weld(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    let a = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([-0.8, 4.0]).build());
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(a, &sdef, &bd::shapes::box_polygon(0.6, 0.4));
    app.created_shapes += 1;
    let b = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([0.8, 4.0]).build());
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.6, 0.4));
    app.created_shapes += 1;
    let _j = app
        .world
        .weld(a, b)
        .anchor_world([0.0, 4.0])
        .with_stiffness(app.wj_hz, app.wj_dr, app.wj_hz, app.wj_dr)
        .build();
    app.created_joints += 1;
    unsafe {
        boxdd_sys::ffi::b2Body_ApplyLinearImpulseToCenter(
            a,
            boxdd_sys::ffi::b2Vec2 { x: -10.0, y: 0.0 },
            true,
        );
        boxdd_sys::ffi::b2Body_ApplyLinearImpulseToCenter(
            b,
            boxdd_sys::ffi::b2Vec2 { x: 10.0, y: 0.0 },
            true,
        );
    }
}
fn ui_weld(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut hz = app.wj_hz;
    let mut dr = app.wj_dr;
    let changed = ui.slider("Hertz", 0.0, 30.0, &mut hz) || ui.slider("Damping Ratio", 0.0, 1.0, &mut dr);
    if changed {
        app.wj_hz = hz;
        app.wj_dr = dr;
        let _ = app.reset();
    }
}

// Filter joint
fn build_filter(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();
    let a = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([-0.5, 6.0]).build());
    app.created_bodies += 1;
    let b = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([0.5, 8.0]).build());
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(a, &sdef, &bd::shapes::box_polygon(0.4, 0.4));
    app.created_shapes += 1;
    app.world.create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.4, 0.4));
    app.created_shapes += 1;
    if app.fj_disable_collide {
        let _ = app.world.filter_joint(a, b).collide_connected(false).build();
        app.created_joints += 1;
    }
}
fn ui_filter(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut disable = app.fj_disable_collide;
    if ui.checkbox("Disable Collision Between Boxes", &mut disable) {
        app.fj_disable_collide = disable;
        let _ = app.reset();
    }
    if ui.button("Reset Hit Counter") {
        app.fj_hits = 0;
    }
    ui.text(format!(
        "Filter Joint: collide={} hits={} (accumulated)",
        !app.fj_disable_collide, app.fj_hits
    ));
}

fn build_one(app: &mut super::PhysicsApp, ground: bd::types::BodyId, mode: usize) {
    match mode {
        0 => build_distance(app, ground),
        1 => build_motor(app, ground),
        2 => build_wheel(app, ground),
        3 => build_revolute(app, ground),
        4 => build_prismatic(app, ground),
        5 => build_weld(app, ground),
        6 => build_filter(app, ground),
        _ => build_distance(app, ground),
    }
}

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    build_one(app, ground, app.jl_mode);
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let names = [
        "Distance",
        "Motor",
        "Wheel",
        "Revolute Motor",
        "Prismatic Elevator",
        "Weld",
        "Filter (collideConnected)",
    ];
    let mut m = app.jl_mode;
    if ui.combo_simple_string("Joints Lab", &mut m, &names) && m != app.jl_mode {
        app.jl_mode = m;
        let _ = app.reset();
        return;
    }
    match app.jl_mode {
        0 => ui_distance(app, ui),
        1 => ui_motor(app, ui),
        2 => ui_wheel(app, ui),
        3 => ui_revolute(app, ui),
        4 => ui_prismatic(app, ui),
        5 => ui_weld(app, ui),
        6 => ui_filter(app, ui),
        _ => {}
    }
}

pub fn tick(app: &mut super::PhysicsApp) {
    match app.jl_mode {
        6 => {
            // filter joint scene uses contact hit accumulation
            let ce = app.world.contact_events();
            app.fj_hits += ce.hit.len();
        }
        _ => {}
    }
}
