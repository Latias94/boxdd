use boxdd as bd;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let chassis = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([0.0_f32, 2.0])
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
        .body(chassis)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef,
            &bd::shapes::box_polygon(1.25, 0.25).expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    let wheel_radius = 0.4;
    let offx = 0.8;
    let offy = -0.3;
    let w1 = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([-offx, 2.0 + offy])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let w2 = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([offx, 2.0 + offy])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let circle =
        bd::shapes::circle([0.0_f32, 0.0], wheel_radius).expect("car wheel circle must be valid");
    let _ = app
        .world
        .body(w1)
        .expect("valid testbed operation")
        .create_circle(&sdef, &circle)
        .expect("valid testbed operation");
    app.created_shapes += 1;
    let _ = app
        .world
        .body(w2)
        .expect("valid testbed operation")
        .create_circle(&sdef, &circle)
        .expect("valid testbed operation");
    app.created_shapes += 1;
    let axis = [0.0_f32, 1.0];
    let base1 = app
        .world
        .joint_base_from_world_with_axis(
            chassis,
            w1,
            [-offx, 2.0 + offy],
            [-offx, 2.0 + offy],
            axis,
        )
        .expect("valid testbed operation");
    let wdef1 = bd::WheelJointDef::new(base1)
        .enable_spring(true)
        .hertz(app.car_hz)
        .damping_ratio(app.car_dr)
        .enable_motor(true)
        .max_motor_torque(app.car_motor_torque * 0.5)
        .motor_speed(0.0);
    let _ = app
        .world
        .create_wheel_joint(&wdef1)
        .expect("valid testbed operation");
    app.created_joints += 1;
    let base2 = app
        .world
        .joint_base_from_world_with_axis(chassis, w2, [offx, 2.0 + offy], [offx, 2.0 + offy], axis)
        .expect("valid testbed operation");
    let wdef2 = bd::WheelJointDef::new(base2)
        .enable_spring(true)
        .hertz(app.car_hz)
        .damping_ratio(app.car_dr)
        .enable_motor(true)
        .max_motor_torque(app.car_motor_torque)
        .motor_speed(app.car_motor_speed);
    let _ = app
        .world
        .create_wheel_joint(&wdef2)
        .expect("valid testbed operation");
    app.created_joints += 1;
}

use dear_imgui_rs as imgui;
pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut hz = app.car_hz;
    let mut dr = app.car_dr;
    let mut sp = app.car_motor_speed;
    let mut tq = app.car_motor_torque;
    if ui.slider("Spring Hz", 0.5, 20.0, &mut hz) {
        app.car_hz = hz;
        let _ = app.reset();
    }
    if ui.slider("Spring DR", 0.0, 2.0, &mut dr) {
        app.car_dr = dr;
        let _ = app.reset();
    }
    if ui.slider("Motor Speed", 0.0, 30.0, &mut sp) {
        app.car_motor_speed = sp;
        let _ = app.reset();
    }
    if ui.slider("Motor Torque", 0.0, 200.0, &mut tq) {
        app.car_motor_torque = tq;
        let _ = app.reset();
    }
}
