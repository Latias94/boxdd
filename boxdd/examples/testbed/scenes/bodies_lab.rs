use boxdd as bd;
use dear_imgui_rs as imgui;

// Bodies Lab: Set Velocity, Kinematic Platform, Wake Touching

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    match app.bl_mode {
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
            app.bsv_body = Some(body);
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
            app.world.set_body_linear_velocity(platform, [app.bk_speed, 0.0]);
            app.bk_platform = Some(platform);
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
            app.wt_ground_body = Some(waker);
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
    match app.bl_mode {
        0 => {
            if let Some(id) = app.bsv_body {
                app.world.set_body_linear_velocity(id, [app.bsv_vx, app.bsv_vy]);
            }
        }
        1 => {
            if let Some(id) = app.bk_platform {
                app.world.set_body_linear_velocity(id, [app.bk_speed, 0.0]);
            }
        }
        _ => {}
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let names = ["Set Velocity", "Kinematic", "Wake Touching"];
    let mut m = app.bl_mode;
    if ui.combo_simple_string("Bodies Lab", &mut m, &names) && m != app.bl_mode {
        app.bl_mode = m;
        let _ = app.reset();
        return;
    }
    match app.bl_mode {
        0 => {
            let mut vx = app.bsv_vx;
            let mut vy = app.bsv_vy;
            let changed = ui.slider("VX", -50.0, 50.0, &mut vx) || ui.slider("VY", -50.0, 50.0, &mut vy);
            if changed {
                app.bsv_vx = vx;
                app.bsv_vy = vy;
            }
        }
        1 => {
            let mut sp = app.bk_speed;
            if ui.slider("Speed", -10.0, 10.0, &mut sp) { app.bk_speed = sp; }
        }
        2 => {
            if ui.button("Wake Touching (platform)")
                && let Some(id) = app.wt_ground_body
            {
                unsafe { boxdd_sys::ffi::b2Body_WakeTouching(id) };
                app.wt_wakes += 1;
            }
            ui.text(format!("Wake Touching: triggered {} times", app.wt_wakes));
        }
        _ => {}
    }
}

