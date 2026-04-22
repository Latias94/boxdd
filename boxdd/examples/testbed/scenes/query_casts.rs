use boxdd as bd;
use dear_imgui_rs as imgui;

// Query casts: world ray/shape casts plus standalone TOI.

fn rect_points(hx: f32, hy: f32) -> [[f32; 2]; 4] {
    [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]]
}

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    match app.cast_mode {
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
    match app.cast_mode {
        0 => {
            let hits = app.world.cast_ray_all(
                [app.rw_origin_x, app.rw_origin_y],
                [app.rw_dx, app.rw_dy],
                bd::QueryFilter::default(),
            );
            app.rw_hits = hits.len();
        }
        1 => {
            let rect = rect_points(0.5, 0.25);
            let hits = app.world.cast_shape_points_with_offset(
                rect,
                app.sc_radius,
                [0.0_f32, app.sc_pos_y],
                app.sc_angle,
                [app.sc_tx, app.sc_ty],
                bd::QueryFilter::default(),
            );
            app.sc_hits = hits.len();
            app.sc_min_fraction = hits.iter().map(|h| h.fraction).fold(1.0, f32::min);
        }
        2 => {
            let pillar = bd::ShapeProxy::new(rect_points(0.5, 1.0), app.toi_radius)
                .expect("pillar proxy must stay within the Box2D shape-proxy point limit");
            let mover = bd::ShapeProxy::new(rect_points(0.4, 0.4), app.toi_radius)
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
                    [app.toi_start_x, app.toi_start_y],
                    [app.toi_start_x + app.toi_dx, app.toi_start_y + app.toi_dy],
                    bd::Rot::from_radians(app.toi_angle),
                    bd::Rot::from_radians(app.toi_angle),
                ),
            ));
            app.toi_state = out.state;
            app.toi_fraction = out.fraction;
        }
        _ => {}
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let names = ["Ray Cast", "Shape Cast", "TOI"];
    let mut idx = app.cast_mode.clamp(0, 2) as usize;
    if let Some(_c) = ui.begin_combo("Mode", names[idx]) {
        for (i, &name) in names.iter().enumerate() {
            let selected = i == idx;
            if ui.selectable_config(name).selected(selected).build() {
                idx = i;
                app.cast_mode = i as i32;
                let _ = app.reset();
            }
        }
    }
    match app.cast_mode {
        0 => {
            let mut ox = app.rw_origin_x;
            let mut oy = app.rw_origin_y;
            let mut dx = app.rw_dx;
            let mut dy = app.rw_dy;
            let changed = ui.slider("Origin X", -50.0, 50.0, &mut ox)
                || ui.slider("Origin Y", -10.0, 50.0, &mut oy)
                || ui.slider("Dir X", -100.0, 100.0, &mut dx)
                || ui.slider("Dir Y", -100.0, 100.0, &mut dy);
            if changed {
                app.rw_origin_x = ox;
                app.rw_origin_y = oy;
                app.rw_dx = dx;
                app.rw_dy = dy;
            }
            ui.text(format!("Ray cast hits={}", app.rw_hits));
        }
        1 => {
            let mut y = app.sc_pos_y;
            let mut ang = app.sc_angle;
            let mut dx = app.sc_tx;
            let mut dy = app.sc_ty;
            let mut r = app.sc_radius;
            let changed = ui.slider("Pos Y", 0.0, 10.0, &mut y)
                || ui.slider("Angle (rad)", -std::f32::consts::PI, std::f32::consts::PI, &mut ang)
                || ui.slider("Cast dX", -5.0, 5.0, &mut dx)
                || ui.slider("Cast dY", -10.0, 0.0, &mut dy)
                || ui.slider("Radius", 0.0, 0.25, &mut r);
            if changed {
                app.sc_pos_y = y;
                app.sc_angle = ang;
                app.sc_tx = dx;
                app.sc_ty = dy;
                app.sc_radius = r.max(0.0);
            }
            ui.text(format!(
                "Shape Cast: hits={} min_fraction={:.3}",
                app.sc_hits, app.sc_min_fraction
            ));
        }
        2 => {
            let mut sx = app.toi_start_x;
            let mut sy = app.toi_start_y;
            let mut ang = app.toi_angle;
            let mut dx = app.toi_dx;
            let mut dy = app.toi_dy;
            let mut r = app.toi_radius;
            let changed = ui.slider("Start X", -5.0, 5.0, &mut sx)
                || ui.slider("Start Y", 0.0, 10.0, &mut sy)
                || ui.slider("Angle (rad)", -std::f32::consts::PI, std::f32::consts::PI, &mut ang)
                || ui.slider("dX", -10.0, 10.0, &mut dx)
                || ui.slider("dY", -10.0, 10.0, &mut dy)
                || ui.slider("Radius", 0.0, 0.25, &mut r);
            if changed {
                app.toi_start_x = sx;
                app.toi_start_y = sy;
                app.toi_angle = ang;
                app.toi_dx = dx;
                app.toi_dy = dy;
                app.toi_radius = r.max(0.0);
            }
            ui.text(format!(
                "TOI: state={:?} fraction={:.3}",
                app.toi_state, app.toi_fraction
            ));
        }
        _ => {}
    }
}
