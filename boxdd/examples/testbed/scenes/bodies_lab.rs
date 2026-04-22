use boxdd as bd;
use dear_imgui_rs as imgui;

// Bodies Lab: Set Velocity, Kinematic Platform, Wake Touching

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let state = &mut app.bodies_lab;
    match state.mode {
        // Set Velocity
        0 => {
            let g = app.world.create_body_id(bd::BodyBuilder::new().position([0.0, -0.25]).build());
            app.created_bodies += 1;
            app.world.create_polygon_shape_for(g, &bd::ShapeDef::builder().density(0.0).build(), &bd::shapes::box_polygon(20.0, 0.25));
            app.created_shapes += 1;
            let body = app
                .world
                .create_body_id(
                    bd::BodyBuilder::new()
                        .body_type(bd::BodyType::Dynamic)
                        .position([0.0, 0.5])
                        .build(),
                );
            app.created_bodies += 1;
            app.world.create_polygon_shape_for(body, &bd::ShapeDef::builder().density(1.0).build(), &bd::shapes::box_polygon(0.5, 0.5));
            app.created_shapes += 1;
            state.set_velocity_body = Some(body);
        }
        // Kinematic Platform
        1 => {
            let platform = app
                .world
                .create_body_id(
                    bd::BodyBuilder::new()
                        .body_type(bd::BodyType::Kinematic)
                        .position([0.0, 2.0])
                        .build(),
                );
            app.created_bodies += 1;
            app.world.create_polygon_shape_for(platform, &bd::ShapeDef::builder().density(0.0).build(), &bd::shapes::box_polygon(2.0, 0.25));
            app.created_shapes += 1;
            app.world
                .set_body_linear_velocity(platform, [state.kinematic_speed, 0.0]);
            state.kinematic_platform = Some(platform);
            let sdef = bd::ShapeDef::builder().density(1.0).build();
            for i in 0..5 {
                let b = app
                    .world
                    .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([-2.0 + i as f32, 5.0]).build());
                app.created_bodies += 1;
                app.world.create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.3, 0.3));
                app.created_shapes += 1;
            }
        }
        // Wake Touching
        2 => {
            let waker = app
                .world
                .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Static).position([0.0, 0.0]).build());
            app.created_bodies += 1;
            app.world.create_polygon_shape_for(waker, &bd::ShapeDef::builder().density(0.0).build(), &bd::shapes::box_polygon(5.0, 0.25));
            app.created_shapes += 1;
            state.wake_touch_ground_body = Some(waker);
            let sdef = bd::ShapeDef::builder().density(1.0).build();
            for i in 0..4 {
                for j in 0..3 {
                    let x = -3.0 + i as f32 * 2.0;
                    let y = 1.0 + j as f32 * 1.1;
                    let b = app
                        .world
                        .create_body_id(
                            bd::BodyBuilder::new()
                                .body_type(bd::BodyType::Dynamic)
                                .position([x, y])
                                .awake(false)
                                .build(),
                        );
                    app.created_bodies += 1;
                    app.world.create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.5, 0.5));
                    app.created_shapes += 1;
                }
            }
        }
        _ => {}
    }
}

#[allow(dead_code)]
pub fn tick(app: &mut super::PhysicsApp) {
    let state = &mut app.bodies_lab;
    match state.mode {
        0 => {
            if let Some(id) = state.set_velocity_body {
                app.world
                    .set_body_linear_velocity(id, [state.set_velocity_x, state.set_velocity_y]);
            }
        }
        1 => {
            if let Some(id) = state.kinematic_platform {
                app.world
                    .set_body_linear_velocity(id, [state.kinematic_speed, 0.0]);
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
            let changed = ui.slider("VX", -50.0, 50.0, &mut vx) || ui.slider("VY", -50.0, 50.0, &mut vy);
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
                app.world.body_wake_touching(id);
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
