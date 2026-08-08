use boxdd as bd;
use dear_imgui_rs as imgui;

// Bodies Lab: Set Velocity, Kinematic Platform, Wake Touching

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let state = &mut app.bodies_lab;
    match state.mode {
        // Set Velocity
        0 => {
            let g = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .position([0.0, -0.25])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("valid testbed operation");
            app.created_bodies += 1;
            app.world
                .body(g)
                .expect("valid testbed operation")
                .create_polygon(
                    &bd::ShapeDef::builder()
                        .density(0.0)
                        .build()
                        .expect("valid testbed definition"),
                    &bd::shapes::box_polygon(20.0, 0.25).expect("valid polygon geometry"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            let body = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .body_type(bd::BodyType::Dynamic)
                        .position([0.0, 0.5])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("valid testbed operation");
            app.created_bodies += 1;
            app.world
                .body(body)
                .expect("valid testbed operation")
                .create_polygon(
                    &bd::ShapeDef::builder()
                        .density(1.0)
                        .build()
                        .expect("valid testbed definition"),
                    &bd::shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            state.set_velocity_body = Some(body);
        }
        // Kinematic Platform
        1 => {
            let platform = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .body_type(bd::BodyType::Kinematic)
                        .position([0.0, 2.0])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("valid testbed operation");
            app.created_bodies += 1;
            app.world
                .body(platform)
                .expect("valid testbed operation")
                .create_polygon(
                    &bd::ShapeDef::builder()
                        .density(0.0)
                        .build()
                        .expect("valid testbed definition"),
                    &bd::shapes::box_polygon(2.0, 0.25).expect("valid polygon geometry"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            app.world
                .body(platform)
                .expect("valid testbed operation")
                .set_linear_velocity([state.kinematic_speed, 0.0])
                .expect("valid testbed operation");
            state.kinematic_platform = Some(platform);
            let sdef = bd::ShapeDef::builder()
                .density(1.0)
                .build()
                .expect("valid testbed definition");
            for i in 0..5 {
                let b = app
                    .world
                    .create_body(
                        bd::BodyBuilder::from(app.foundation.body_def())
                            .body_type(bd::BodyType::Dynamic)
                            .position([-2.0 + i as f32, 5.0])
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
                        &bd::shapes::box_polygon(0.3, 0.3).expect("valid polygon geometry"),
                    )
                    .expect("valid testbed operation");
                app.created_shapes += 1;
            }
        }
        // Wake Touching
        2 => {
            let waker = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .body_type(bd::BodyType::Static)
                        .position([0.0, 0.0])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("valid testbed operation");
            app.created_bodies += 1;
            app.world
                .body(waker)
                .expect("valid testbed operation")
                .create_polygon(
                    &bd::ShapeDef::builder()
                        .density(0.0)
                        .build()
                        .expect("valid testbed definition"),
                    &bd::shapes::box_polygon(5.0, 0.25).expect("valid polygon geometry"),
                )
                .expect("valid testbed operation");
            app.created_shapes += 1;
            state.wake_touch_ground_body = Some(waker);
            let sdef = bd::ShapeDef::builder()
                .density(1.0)
                .build()
                .expect("valid testbed definition");
            for i in 0..4 {
                for j in 0..3 {
                    let x = -3.0 + i as f32 * 2.0;
                    let y = 1.0 + j as f32 * 1.1;
                    let b = app
                        .world
                        .create_body(
                            bd::BodyBuilder::from(app.foundation.body_def())
                                .body_type(bd::BodyType::Dynamic)
                                .position([x, y])
                                .awake(false)
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
                }
            }
        }
        _ => {}
    }
}

#[allow(dead_code)]
pub fn tick(app: &mut super::PhysicsApp, _events: Option<&bd::StepEventsSnapshot>) {
    let state = &mut app.bodies_lab;
    match state.mode {
        0 => {
            if let Some(id) = state.set_velocity_body {
                app.world
                    .body(id)
                    .expect("valid testbed operation")
                    .set_linear_velocity([state.set_velocity_x, state.set_velocity_y])
                    .expect("valid testbed operation");
            }
        }
        1 => {
            if let Some(id) = state.kinematic_platform {
                app.world
                    .body(id)
                    .expect("valid testbed operation")
                    .set_linear_velocity([state.kinematic_speed, 0.0])
                    .expect("valid testbed operation");
            }
        }
        _ => {}
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let state = &mut app.bodies_lab;
    let names = ["Set Velocity", "Kinematic", "Wake Touching"];
    let mut m = state.mode;
    if ui.combo_simple_string("Bodies Lab", &mut m, &names) && m != state.mode {
        state.mode = m;
        let _ = app.reset();
        return;
    }
    match state.mode {
        0 => {
            let mut vx = state.set_velocity_x;
            let mut vy = state.set_velocity_y;
            let changed =
                ui.slider("VX", -50.0, 50.0, &mut vx) || ui.slider("VY", -50.0, 50.0, &mut vy);
            if changed {
                state.set_velocity_x = vx;
                state.set_velocity_y = vy;
            }
        }
        1 => {
            let mut sp = state.kinematic_speed;
            if ui.slider("Speed", -10.0, 10.0, &mut sp) {
                state.kinematic_speed = sp;
            }
        }
        2 => {
            if ui.button("Wake Touching (platform)")
                && let Some(id) = state.wake_touch_ground_body
            {
                app.world
                    .body(id)
                    .expect("valid testbed operation")
                    .wake_touching()
                    .expect("valid testbed operation");
                state.wake_touch_count += 1;
            }
            ui.text(format!(
                "Wake Touching: triggered {} times",
                state.wake_touch_count
            ));
        }
        _ => {}
    }
}
