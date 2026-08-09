use boxdd as bd;
use dear_imgui_rs as imgui;

// Materials demo: conveyor via tangent speed + rolling resistance showcase.

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let state = &mut app.materials;
    // Two conveyors (static bodies) left/right with opposite tangent speeds
    let conv_mat_l = bd::shapes::SurfaceMaterial::default()
        .with_friction(0.9)
        .expect("conveyor friction must be valid")
        .with_restitution(0.0)
        .expect("conveyor restitution must be valid")
        .with_tangent_speed(state.conveyor_speed)
        .expect("conveyor speed must be finite");
    let conv_mat_r = bd::shapes::SurfaceMaterial::default()
        .with_friction(0.9)
        .expect("conveyor friction must be valid")
        .with_restitution(0.0)
        .expect("conveyor restitution must be valid")
        .with_tangent_speed(-state.conveyor_speed)
        .expect("conveyor speed must be finite");

    let sdef_l = bd::ShapeDef::builder()
        .density(0.0)
        .material(conv_mat_l)
        .build()
        .expect("valid testbed definition");
    let sdef_r = bd::ShapeDef::builder()
        .density(0.0)
        .material(conv_mat_r)
        .build()
        .expect("valid testbed definition");

    // Left belt body at y ~ 0.5
    let left = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Static)
                .position([state.left_belt_x, state.left_belt_y])
                .angle(state.left_belt_angle_deg * std::f32::consts::PI / 180.0)
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let sid_l = app
        .world
        .body(left)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef_l,
            &bd::shapes::box_polygon(state.belt_half_len, state.belt_thickness)
                .expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    state.left_belt_body = Some(left);
    state.left_belt_shape = Some(sid_l);

    // Right belt body at y ~ 2.5
    let right = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Static)
                .position([state.right_belt_x, state.right_belt_y])
                .angle(state.right_belt_angle_deg * std::f32::consts::PI / 180.0)
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let sid_r = app
        .world
        .body(right)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef_r,
            &bd::shapes::box_polygon(state.belt_half_len, state.belt_thickness)
                .expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    state.right_belt_body = Some(right);
    state.right_belt_shape = Some(sid_r);

    // Spawners will use rolling resistance configured via UI
}

fn spawn_box(app: &mut super::PhysicsApp, x: f32, y: f32) {
    let state = &mut app.materials;
    let b = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([x, y])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let mat = bd::shapes::SurfaceMaterial::default()
        .with_friction(0.8)
        .expect("sample friction must be valid")
        .with_restitution(0.1)
        .expect("sample restitution must be valid")
        .with_rolling_resistance(state.rolling_resistance)
        .expect("sample rolling resistance must be valid");
    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .material(mat)
        .build()
        .expect("valid testbed definition");
    let sid = app
        .world
        .body(b)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef,
            &bd::shapes::box_polygon(0.4, 0.25).expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    state.spawned_shapes.push(sid);
    state.spawned_count += 1;
}

fn spawn_ball(app: &mut super::PhysicsApp, x: f32, y: f32) {
    let state = &mut app.materials;
    let b = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([x, y])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let mat = bd::shapes::SurfaceMaterial::default()
        .with_friction(0.4)
        .expect("sample friction must be valid")
        .with_restitution(0.2)
        .expect("sample restitution must be valid")
        .with_rolling_resistance(state.rolling_resistance)
        .expect("sample rolling resistance must be valid");
    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .material(mat)
        .build()
        .expect("valid testbed definition");
    let sid = app
        .world
        .body(b)
        .expect("valid testbed operation")
        .create_circle(
            &sdef,
            &bd::shapes::circle([0.0, 0.0], 0.25).expect("material sample circle must be valid"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    state.spawned_shapes.push(sid);
    state.spawned_count += 1;
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    {
        let state = &mut app.materials;
        let mut cs = state.conveyor_speed;
        let mut rr = state.rolling_resistance;
        let mut wx = state.wind_x;
        let mut wy = state.wind_y;
        let mut dr = state.drag;
        let mut lf = state.lift;
        let mut wk = state.wake_on_wind;
        let mut belt_mu = state.belt_friction;
        let mut belt_re = state.belt_restitution;
        let mut shp_mu = state.shape_friction;
        let mut shp_re = state.shape_restitution;
        let mut blx = state.left_belt_x;
        let mut bly = state.left_belt_y;
        let mut brx = state.right_belt_x;
        let mut bry = state.right_belt_y;
        let mut blen = state.belt_half_len;
        let mut bth = state.belt_thickness;
        let mut lang = state.left_belt_angle_deg;
        let mut rang = state.right_belt_angle_deg;
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
        let mut preset_changed = false;
        if ui.button("Preset: Horizontal") {
            blx = -3.0;
            bly = 0.5;
            lang = 0.0;
            brx = 3.0;
            bry = 2.5;
            rang = 0.0;
            preset_changed = true;
        }
        ui.same_line();
        if ui.button("Preset: Left Ramp") {
            blx = -4.0;
            bly = 0.5;
            lang = 20.0;
            brx = 2.5;
            bry = 2.0;
            rang = 0.0;
            preset_changed = true;
        }
        ui.same_line();
        if ui.button("Preset: Right Ramp") {
            blx = -3.0;
            bly = 0.5;
            lang = 0.0;
            brx = 3.0;
            bry = 2.5;
            rang = -20.0;
            preset_changed = true;
        }
        if changed || preset_changed {
            let speed_changed = (state.conveyor_speed - cs).abs() > f32::EPSILON;
            let rr_changed = (state.rolling_resistance - rr).abs() > f32::EPSILON;
            let belt_mu_changed = (state.belt_friction - belt_mu).abs() > f32::EPSILON;
            let belt_re_changed = (state.belt_restitution - belt_re).abs() > f32::EPSILON;
            let shp_mu_changed = (state.shape_friction - shp_mu).abs() > f32::EPSILON;
            let shp_re_changed = (state.shape_restitution - shp_re).abs() > f32::EPSILON;
            let belt_ang_changed = (state.left_belt_angle_deg - lang).abs() > f32::EPSILON
                || (state.right_belt_angle_deg - rang).abs() > f32::EPSILON;
            let belt_pos_changed = (state.left_belt_x - blx).abs() > f32::EPSILON
                || (state.left_belt_y - bly).abs() > f32::EPSILON
                || (state.right_belt_x - brx).abs() > f32::EPSILON
                || (state.right_belt_y - bry).abs() > f32::EPSILON;
            let belt_geo_changed = (state.belt_half_len - blen).abs() > f32::EPSILON
                || (state.belt_thickness - bth).abs() > f32::EPSILON;
            state.conveyor_speed = cs;
            state.rolling_resistance = rr;
            state.wind_x = wx;
            state.wind_y = wy;
            state.drag = dr;
            state.lift = lf;
            state.wake_on_wind = wk;
            state.belt_friction = belt_mu;
            state.belt_restitution = belt_re;
            state.shape_friction = shp_mu;
            state.shape_restitution = shp_re;
            state.left_belt_x = blx;
            state.left_belt_y = bly;
            state.right_belt_x = brx;
            state.right_belt_y = bry;
            state.left_belt_angle_deg = lang;
            state.right_belt_angle_deg = rang;
            state.belt_half_len = blen;
            state.belt_thickness = bth;
            if speed_changed || belt_mu_changed || belt_re_changed {
                if let Some(sid) = state.left_belt_shape {
                    let m = app
                        .world
                        .shape(sid)
                        .expect("valid testbed operation")
                        .surface_material()
                        .expect("valid testbed operation")
                        .with_tangent_speed(state.conveyor_speed)
                        .expect("conveyor speed must be finite")
                        .with_friction(state.belt_friction)
                        .expect("belt friction must be valid")
                        .with_restitution(state.belt_restitution)
                        .expect("belt restitution must be valid");
                    app.world
                        .shape(sid)
                        .expect("valid testbed operation")
                        .set_surface_material(&m)
                        .expect("valid testbed operation");
                }
                if let Some(sid) = state.right_belt_shape {
                    let m = app
                        .world
                        .shape(sid)
                        .expect("valid testbed operation")
                        .surface_material()
                        .expect("valid testbed operation")
                        .with_tangent_speed(-state.conveyor_speed)
                        .expect("conveyor speed must be finite")
                        .with_friction(state.belt_friction)
                        .expect("belt friction must be valid")
                        .with_restitution(state.belt_restitution)
                        .expect("belt restitution must be valid");
                    app.world
                        .shape(sid)
                        .expect("valid testbed operation")
                        .set_surface_material(&m)
                        .expect("valid testbed operation");
                }
            }
            if belt_pos_changed || belt_ang_changed {
                if let Some(bid) = state.left_belt_body {
                    let ang = state.left_belt_angle_deg * std::f32::consts::PI / 180.0;
                    app.world
                        .body(bid)
                        .expect("valid testbed operation")
                        .set_position_and_rotation([state.left_belt_x, state.left_belt_y], ang)
                        .expect("valid testbed operation");
                }
                if let Some(bid) = state.right_belt_body {
                    let ang = state.right_belt_angle_deg * std::f32::consts::PI / 180.0;
                    app.world
                        .body(bid)
                        .expect("valid testbed operation")
                        .set_position_and_rotation([state.right_belt_x, state.right_belt_y], ang)
                        .expect("valid testbed operation");
                }
            }
            if rr_changed || shp_mu_changed || shp_re_changed {
                for &sid in &state.spawned_shapes {
                    let material = app
                        .world
                        .shape(sid)
                        .expect("valid testbed shape")
                        .surface_material()
                        .expect("valid testbed material")
                        .with_rolling_resistance(state.rolling_resistance)
                        .expect("rolling resistance must be valid")
                        .with_friction(state.shape_friction)
                        .expect("shape friction must be valid")
                        .with_restitution(state.shape_restitution)
                        .expect("shape restitution must be valid");
                    app.world
                        .shape(sid)
                        .expect("valid testbed shape")
                        .set_surface_material(&material)
                        .expect("valid testbed material");
                }
            }
            if belt_geo_changed {
                let poly = bd::shapes::box_polygon(state.belt_half_len, state.belt_thickness)
                    .expect("valid polygon geometry");
                if let Some(sid) = state.left_belt_shape {
                    app.world
                        .shape(sid)
                        .expect("valid testbed operation")
                        .set_polygon(&poly)
                        .expect("valid testbed operation");
                }
                if let Some(sid) = state.right_belt_shape {
                    app.world
                        .shape(sid)
                        .expect("valid testbed operation")
                        .set_polygon(&poly)
                        .expect("valid testbed operation");
                }
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
    ui.text(format!(
        "Materials: spawned {} bodies",
        app.materials.spawned_count
    ));
}

pub fn tick(app: &mut super::PhysicsApp, _events: Option<&bd::StepEventsSnapshot>) {
    let state = &app.materials;
    if state.spawned_shapes.is_empty() {
        return;
    }
    let wind = bd::Vec2::new(state.wind_x, state.wind_y);
    let drag = state.drag;
    let lift = state.lift;
    let wake = state.wake_on_wind;
    for &sid in &state.spawned_shapes {
        app.world
            .shape(sid)
            .expect("valid testbed operation")
            .apply_wind(wind, drag, lift, wake)
            .expect("valid testbed operation");
    }
}
