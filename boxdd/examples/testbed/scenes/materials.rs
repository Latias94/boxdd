use boxdd as bd;
use dear_imgui_rs as imgui;

// Materials demo: conveyor via tangent speed + rolling resistance showcase.

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Two conveyors (static bodies) left/right with opposite tangent speeds
    let conv_mat_l = bd::shapes::SurfaceMaterial::default()
        .friction(0.9)
        .restitution(0.0)
        .tangent_speed(app.mat_conv_speed);
    let conv_mat_r = bd::shapes::SurfaceMaterial::default()
        .friction(0.9)
        .restitution(0.0)
        .tangent_speed(-app.mat_conv_speed);

    let sdef_l = bd::ShapeDef::builder().density(0.0).material(conv_mat_l).build();
    let sdef_r = bd::ShapeDef::builder().density(0.0).material(conv_mat_r).build();

    // Left belt body at y ~ 0.5
    let left = app
        .world
        .create_body_id(
            bd::BodyBuilder::new()
                .body_type(bd::BodyType::Static)
                .position([app.mat_belt_left_x, app.mat_belt_left_y])
                .angle(app.mat_belt_left_angle_deg * std::f32::consts::PI / 180.0)
                .build(),
        );
    app.created_bodies += 1;
    let sid_l = app
        .world
        .create_polygon_shape_for(left, &sdef_l, &bd::shapes::box_polygon(app.mat_belt_half_len, app.mat_belt_thickness));
    app.created_shapes += 1;
    app.mat_belt_left = Some(sid_l);

    // Right belt body at y ~ 2.5
    let right = app
        .world
        .create_body_id(
            bd::BodyBuilder::new()
                .body_type(bd::BodyType::Static)
                .position([app.mat_belt_right_x, app.mat_belt_right_y])
                .angle(app.mat_belt_right_angle_deg * std::f32::consts::PI / 180.0)
                .build(),
        );
    app.created_bodies += 1;
    let sid_r = app
        .world
        .create_polygon_shape_for(right, &sdef_r, &bd::shapes::box_polygon(app.mat_belt_half_len, app.mat_belt_thickness));
    app.created_shapes += 1;
    app.mat_belt_right = Some(sid_r);

    // Spawners will use rolling resistance configured via UI
}

fn spawn_box(app: &mut super::PhysicsApp, x: f32, y: f32) {
    let b = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([x, y]).build());
    app.created_bodies += 1;
    let mat = bd::shapes::SurfaceMaterial::default()
        .friction(0.8)
        .restitution(0.1)
        .rolling_resistance(app.mat_roll_res);
    let sdef = bd::ShapeDef::builder().density(1.0).material(mat).build();
    let sid = app
        .world
        .create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.4, 0.25));
    app.created_shapes += 1;
    app.mat_shapes.push(sid);
    app.mat_spawned += 1;
}

fn spawn_ball(app: &mut super::PhysicsApp, x: f32, y: f32) {
    let b = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([x, y]).build());
    app.created_bodies += 1;
    let mat = bd::shapes::SurfaceMaterial::default()
        .friction(0.4)
        .restitution(0.2)
        .rolling_resistance(app.mat_roll_res);
    let sdef = bd::ShapeDef::builder().density(1.0).material(mat).build();
    let sid = app
        .world
        .create_circle_shape_for(b, &sdef, &bd::shapes::circle([0.0, 0.0], 0.25));
    app.created_shapes += 1;
    app.mat_shapes.push(sid);
    app.mat_spawned += 1;
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut cs = app.mat_conv_speed;
    let mut rr = app.mat_roll_res;
    let mut wx = app.mat_wind_x;
    let mut wy = app.mat_wind_y;
    let mut dr = app.mat_drag;
    let mut lf = app.mat_lift;
    let mut wk = app.mat_wake;
    let mut belt_mu = app.mat_belt_friction;
    let mut belt_re = app.mat_belt_restitution;
    let mut shp_mu = app.mat_shape_friction;
    let mut shp_re = app.mat_shape_restitution;
    let mut blx = app.mat_belt_left_x;
    let mut bly = app.mat_belt_left_y;
    let mut brx = app.mat_belt_right_x;
    let mut bry = app.mat_belt_right_y;
    let mut blen = app.mat_belt_half_len;
    let mut bth = app.mat_belt_thickness;
    // Cache angles locally so we can detect change precisely
    let mut lang = app.mat_belt_left_angle_deg;
    let mut rang = app.mat_belt_right_angle_deg;
    let changed = ui.slider("Conveyor Tangent Speed", 0.0, 5.0, &mut cs)
        || ui.slider("Rolling Resistance", 0.0, 3.0, &mut rr)
        || ui.slider("Belt Friction", 0.0, 2.0, &mut belt_mu)
        || ui.slider("Belt Restitution", 0.0, 1.5, &mut belt_re)
        || ui.slider("Shapes Friction", 0.0, 2.0, &mut shp_mu)
        || ui.slider("Shapes Restitution", 0.0, 1.5, &mut shp_re)
        || ui.slider("Left Belt X", -10.0, 10.0, &mut blx)
        || ui.slider("Left Belt Y", -2.0, 10.0, &mut bly)
        || ui.slider("Right Belt X", -10.0, 10.0, &mut brx)
        || ui.slider("Right Belt Y", -2.0, 10.0, &mut bry)
        || ui.slider("Belt Half Length", 0.5, 10.0, &mut blen)
        || ui.slider("Belt Thickness", 0.05, 1.0, &mut bth)
        || ui.slider("Left Belt Angle (deg)", -90.0, 90.0, &mut lang)
        || ui.slider("Right Belt Angle (deg)", -90.0, 90.0, &mut rang)
        || ui.slider("Wind X", -20.0, 20.0, &mut wx)
        || ui.slider("Wind Y", -20.0, 20.0, &mut wy)
        || ui.slider("Drag", 0.0, 5.0, &mut dr)
        || ui.slider("Lift", 0.0, 5.0, &mut lf)
        || ui.checkbox("Wake", &mut wk);
    // Presets for quick setup
    if ui.button("Preset: Horizontal") {
        app.mat_belt_left_x = -3.0; app.mat_belt_left_y = 0.5; app.mat_belt_left_angle_deg = 0.0;
        app.mat_belt_right_x = 3.0; app.mat_belt_right_y = 2.5; app.mat_belt_right_angle_deg = 0.0;
        blx = app.mat_belt_left_x; bly = app.mat_belt_left_y; brx = app.mat_belt_right_x; bry = app.mat_belt_right_y;
    }
    ui.same_line();
    if ui.button("Preset: Left Ramp") {
        app.mat_belt_left_x = -4.0; app.mat_belt_left_y = 0.5; app.mat_belt_left_angle_deg = 20.0;
        app.mat_belt_right_x = 2.5; app.mat_belt_right_y = 2.0; app.mat_belt_right_angle_deg = 0.0;
        blx = app.mat_belt_left_x; bly = app.mat_belt_left_y; brx = app.mat_belt_right_x; bry = app.mat_belt_right_y;
    }
    ui.same_line();
    if ui.button("Preset: Right Ramp") {
        app.mat_belt_left_x = -3.0; app.mat_belt_left_y = 0.5; app.mat_belt_left_angle_deg = 0.0;
        app.mat_belt_right_x = 3.0; app.mat_belt_right_y = 2.5; app.mat_belt_right_angle_deg = -20.0;
        blx = app.mat_belt_left_x; bly = app.mat_belt_left_y; brx = app.mat_belt_right_x; bry = app.mat_belt_right_y;
    }
    if changed {
        // Update cached values
        let speed_changed = (app.mat_conv_speed - cs).abs() > f32::EPSILON;
        let rr_changed = (app.mat_roll_res - rr).abs() > f32::EPSILON;
        let belt_mu_changed = (app.mat_belt_friction - belt_mu).abs() > f32::EPSILON;
        let belt_re_changed = (app.mat_belt_restitution - belt_re).abs() > f32::EPSILON;
        let shp_mu_changed = (app.mat_shape_friction - shp_mu).abs() > f32::EPSILON;
        let shp_re_changed = (app.mat_shape_restitution - shp_re).abs() > f32::EPSILON;
        let belt_ang_changed = (app.mat_belt_left_angle_deg - lang).abs() > f32::EPSILON
            || (app.mat_belt_right_angle_deg - rang).abs() > f32::EPSILON;
        app.mat_conv_speed = cs;
        app.mat_roll_res = rr;
        app.mat_belt_friction = belt_mu;
        app.mat_belt_restitution = belt_re;
        app.mat_shape_friction = shp_mu;
        app.mat_shape_restitution = shp_re;
        let belt_pos_changed = (app.mat_belt_left_x - blx).abs() > f32::EPSILON
            || (app.mat_belt_left_y - bly).abs() > f32::EPSILON
            || (app.mat_belt_right_x - brx).abs() > f32::EPSILON
            || (app.mat_belt_right_y - bry).abs() > f32::EPSILON;
        let belt_geo_changed = (app.mat_belt_half_len - blen).abs() > f32::EPSILON
            || (app.mat_belt_thickness - bth).abs() > f32::EPSILON;
        app.mat_wind_x = wx;
        app.mat_wind_y = wy;
        app.mat_drag = dr;
        app.mat_lift = lf;
        app.mat_wake = wk;
        app.mat_belt_left_x = blx;
        app.mat_belt_left_y = bly;
        app.mat_belt_right_x = brx;
        app.mat_belt_right_y = bry;
        app.mat_belt_left_angle_deg = lang;
        app.mat_belt_right_angle_deg = rang;
        app.mat_belt_half_len = blen;
        app.mat_belt_thickness = bth;
        // Update conveyor belts in-place (material)
        if speed_changed || belt_mu_changed || belt_re_changed {
            if let Some(sid) = app.mat_belt_left {
                unsafe {
                    let mut m = boxdd_sys::ffi::b2Shape_GetSurfaceMaterial(sid);
                    m.tangentSpeed = app.mat_conv_speed;
                    m.friction = app.mat_belt_friction;
                    m.restitution = app.mat_belt_restitution;
                    boxdd_sys::ffi::b2Shape_SetSurfaceMaterial(sid, &m);
                }
            }
            if let Some(sid) = app.mat_belt_right {
                unsafe {
                    let mut m = boxdd_sys::ffi::b2Shape_GetSurfaceMaterial(sid);
                    m.tangentSpeed = -app.mat_conv_speed;
                    m.friction = app.mat_belt_friction;
                    m.restitution = app.mat_belt_restitution;
                    boxdd_sys::ffi::b2Shape_SetSurfaceMaterial(sid, &m);
                }
            }
        }
        // Update belt transforms (position/angle)
        if belt_pos_changed || belt_ang_changed {
            if let Some(bid) = app.mat_belt_left_body {
                unsafe {
                    let ang = app.mat_belt_left_angle_deg * std::f32::consts::PI / 180.0;
                    let (s, c) = ang.sin_cos();
                    let rot = boxdd_sys::ffi::b2Rot { c, s };
                    let pos = boxdd_sys::ffi::b2Vec2 { x: app.mat_belt_left_x, y: app.mat_belt_left_y };
                    boxdd_sys::ffi::b2Body_SetTransform(bid, pos, rot);
                }
            }
            if let Some(bid) = app.mat_belt_right_body {
                unsafe {
                    let ang = app.mat_belt_right_angle_deg * std::f32::consts::PI / 180.0;
                    let (s, c) = ang.sin_cos();
                    let rot = boxdd_sys::ffi::b2Rot { c, s };
                    let pos = boxdd_sys::ffi::b2Vec2 { x: app.mat_belt_right_x, y: app.mat_belt_right_y };
                    boxdd_sys::ffi::b2Body_SetTransform(bid, pos, rot);
                }
            }
        }
        // Update rolling resistance for spawned shapes in-place
        if rr_changed || shp_mu_changed || shp_re_changed {
            for &sid in &app.mat_shapes {
                if unsafe { boxdd_sys::ffi::b2Shape_IsValid(sid) } {
                    unsafe {
                        let mut m = boxdd_sys::ffi::b2Shape_GetSurfaceMaterial(sid);
                        m.rollingResistance = app.mat_roll_res;
                        m.friction = app.mat_shape_friction;
                        m.restitution = app.mat_shape_restitution;
                        boxdd_sys::ffi::b2Shape_SetSurfaceMaterial(sid, &m);
                    }
                }
            }
        }
        // (No separate position-only transform write; handled above to preserve angles)
        if belt_geo_changed {
            let poly = bd::shapes::box_polygon(app.mat_belt_half_len, app.mat_belt_thickness);
            if let Some(sid) = app.mat_belt_left {
                unsafe { boxdd_sys::ffi::b2Shape_SetPolygon(sid, &poly) };
            }
            if let Some(sid) = app.mat_belt_right {
                unsafe { boxdd_sys::ffi::b2Shape_SetPolygon(sid, &poly) };
            }
        }
    }
    if ui.button("Spawn Box Left") {
        spawn_box(app, -3.0, 3.5);
    }
    ui.same_line();
    if ui.button("Spawn Ball Left") {
        spawn_ball(app, -1.5, 3.5);
    }
    if ui.button("Spawn Box Right") {
        spawn_box(app, 1.5, 3.8);
    }
    ui.same_line();
    if ui.button("Spawn Ball Right") {
        spawn_ball(app, 3.0, 3.8);
    }
    ui.text(format!("Materials: spawned {} bodies", app.mat_spawned));
}

pub fn tick(app: &mut super::PhysicsApp) {
    // Apply wind to spawned dynamic shapes (if any)
    if app.mat_shapes.is_empty() {
        return;
    }
    let wind = boxdd_sys::ffi::b2Vec2 { x: app.mat_wind_x, y: app.mat_wind_y };
    let drag = app.mat_drag;
    let lift = app.mat_lift;
    let wake = app.mat_wake;
    for &sid in &app.mat_shapes {
        unsafe { boxdd_sys::ffi::b2Shape_ApplyWindForce(sid, wind, drag, lift, wake) };
    }
}
