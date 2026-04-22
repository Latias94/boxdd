use boxdd as bd;
use dear_imgui_rs as imgui;

// Query casts: world ray/shape casts plus standalone TOI.

fn rect_points(hx: f32, hy: f32) -> [[f32; 2]; 4] {
    [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]]
}

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    match app.query_casts.mode {
        0 => {
            let sdef = bd::ShapeDef::builder().density(0.0).build();
            let block = app
                .world
                .create_body_id(bd::BodyBuilder::new().position([0.0_f32, 2.5]).build());
            app.created_bodies += 1;
            let _ = app
                .world
                .create_polygon_shape_for(block, &sdef, &bd::shapes::box_polygon(0.5, 0.5));
            app.created_shapes += 1;

            let wall = app
                .world
                .create_body_id(bd::BodyBuilder::new().position([2.2_f32, 1.6]).build());
            app.created_bodies += 1;
            let _ = app
                .world
                .create_polygon_shape_for(wall, &sdef, &bd::shapes::box_polygon(0.4, 0.9));
            app.created_shapes += 1;
        }
        1 => {
            let sdef = bd::ShapeDef::builder().density(0.0).build();
            let _ = app.world.create_polygon_shape_for(
                ground,
                &sdef,
                &bd::shapes::box_polygon(0.75, 0.25),
            );
            app.created_shapes += 1;

            let obs = app
                .world
                .create_body_id(bd::BodyBuilder::new().position([1.5_f32, 1.0]).build());
            app.created_bodies += 1;
            let _ = app
                .world
                .create_polygon_shape_for(obs, &sdef, &bd::shapes::box_polygon(0.4, 0.8));
            app.created_shapes += 1;
        }
        2 => {
            let pillar = app
                .world
                .create_body_id(bd::BodyBuilder::new().position([0.0_f32, 1.0]).build());
            app.created_bodies += 1;
            let _ = app.world.create_polygon_shape_for(
                pillar,
                &bd::ShapeDef::builder().density(0.0).build(),
                &bd::shapes::box_polygon(0.5, 1.0),
            );
            app.created_shapes += 1;
        }
        _ => {}
    }
}

pub fn tick(app: &mut super::PhysicsApp) {
    let state = &mut app.query_casts;
    match state.mode {
        0 => {
            state.ray_hit_buffer.clear();
            app.world.cast_ray_all_into(
                [state.ray_origin_x, state.ray_origin_y],
                [state.ray_dx, state.ray_dy],
                bd::QueryFilter::default(),
                &mut state.ray_hit_buffer,
            );
            state.ray_hits = state.ray_hit_buffer.len();
        }
        1 => {
            let rect = rect_points(0.5, 0.25);
            state.shape_hit_buffer.clear();
            app.world.cast_shape_points_with_offset_into(
                rect,
                state.shape_radius,
                [0.0_f32, state.shape_pos_y],
                state.shape_angle,
                [state.shape_tx, state.shape_ty],
                bd::QueryFilter::default(),
                &mut state.shape_hit_buffer,
            );
            state.shape_hits = state.shape_hit_buffer.len();
            state.shape_min_fraction = state
                .shape_hit_buffer
                .iter()
                .map(|h| h.fraction)
                .fold(1.0, f32::min);
        }
        2 => {
            let pillar = bd::ShapeProxy::new(rect_points(0.5, 1.0), state.toi_radius)
                .expect("pillar proxy must stay within the Box2D shape-proxy point limit");
            let mover = bd::ShapeProxy::new(rect_points(0.4, 0.4), state.toi_radius)
                .expect("mover proxy must stay within the Box2D shape-proxy point limit");
            let out = bd::time_of_impact(bd::ToiInput::new(
                pillar,
                mover,
                bd::Sweep::new(
                    [0.0_f32, 0.0],
                    [0.0, 1.0],
                    [0.0, 1.0],
                    bd::Rot::IDENTITY,
                    bd::Rot::IDENTITY,
                ),
                bd::Sweep::new(
                    [0.0_f32, 0.0],
                    [state.toi_start_x, state.toi_start_y],
                    [state.toi_start_x + state.toi_dx, state.toi_start_y + state.toi_dy],
                    bd::Rot::from_radians(state.toi_angle),
                    bd::Rot::from_radians(state.toi_angle),
                ),
            ));
            state.toi_state = out.state;
            state.toi_fraction = out.fraction;
        }
        _ => {}
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let names = ["Ray Cast", "Shape Cast", "TOI"];
    let idx = app.query_casts.mode.clamp(0, 2) as usize;
    if let Some(_c) = ui.begin_combo("Mode", names[idx]) {
        for (i, &name) in names.iter().enumerate() {
            let selected = i == idx;
            if ui.selectable_config(name).selected(selected).build() {
                app.query_casts.mode = i as i32;
                let _ = app.reset();
                return;
            }
        }
    }
    match app.query_casts.mode {
        0 => {
            let mut ox = app.query_casts.ray_origin_x;
            let mut oy = app.query_casts.ray_origin_y;
            let mut dx = app.query_casts.ray_dx;
            let mut dy = app.query_casts.ray_dy;
            let changed = ui.slider("Origin X", -50.0, 50.0, &mut ox)
                || ui.slider("Origin Y", -10.0, 50.0, &mut oy)
                || ui.slider("Dir X", -100.0, 100.0, &mut dx)
                || ui.slider("Dir Y", -100.0, 100.0, &mut dy);
            if changed {
                let state = &mut app.query_casts;
                state.ray_origin_x = ox;
                state.ray_origin_y = oy;
                state.ray_dx = dx;
                state.ray_dy = dy;
            }
            ui.text(format!("Ray cast hits={}", app.query_casts.ray_hits));
        }
        1 => {
            let mut y = app.query_casts.shape_pos_y;
            let mut ang = app.query_casts.shape_angle;
            let mut dx = app.query_casts.shape_tx;
            let mut dy = app.query_casts.shape_ty;
            let mut r = app.query_casts.shape_radius;
            let changed = ui.slider("Pos Y", 0.0, 10.0, &mut y)
                || ui.slider("Angle (rad)", -std::f32::consts::PI, std::f32::consts::PI, &mut ang)
                || ui.slider("Cast dX", -5.0, 5.0, &mut dx)
                || ui.slider("Cast dY", -10.0, 0.0, &mut dy)
                || ui.slider("Radius", 0.0, 0.25, &mut r);
            if changed {
                let state = &mut app.query_casts;
                state.shape_pos_y = y;
                state.shape_angle = ang;
                state.shape_tx = dx;
                state.shape_ty = dy;
                state.shape_radius = r.max(0.0);
            }
            ui.text(format!(
                "Shape Cast: hits={} min_fraction={:.3}",
                app.query_casts.shape_hits, app.query_casts.shape_min_fraction
            ));
        }
        2 => {
            let mut sx = app.query_casts.toi_start_x;
            let mut sy = app.query_casts.toi_start_y;
            let mut ang = app.query_casts.toi_angle;
            let mut dx = app.query_casts.toi_dx;
            let mut dy = app.query_casts.toi_dy;
            let mut r = app.query_casts.toi_radius;
            let changed = ui.slider("Start X", -5.0, 5.0, &mut sx)
                || ui.slider("Start Y", 0.0, 10.0, &mut sy)
                || ui.slider("Angle (rad)", -std::f32::consts::PI, std::f32::consts::PI, &mut ang)
                || ui.slider("dX", -10.0, 10.0, &mut dx)
                || ui.slider("dY", -10.0, 10.0, &mut dy)
                || ui.slider("Radius", 0.0, 0.25, &mut r);
            if changed {
                let state = &mut app.query_casts;
                state.toi_start_x = sx;
                state.toi_start_y = sy;
                state.toi_angle = ang;
                state.toi_dx = dx;
                state.toi_dy = dy;
                state.toi_radius = r.max(0.0);
            }
            ui.text(format!(
                "TOI: state={:?} fraction={:.3}",
                app.query_casts.toi_state, app.query_casts.toi_fraction
            ));
        }
        _ => {}
    }
}
