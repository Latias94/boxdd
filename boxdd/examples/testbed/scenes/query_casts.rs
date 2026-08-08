use boxdd as bd;
use dear_imgui_rs as imgui;

// Query casts: world ray/shape casts plus standalone TOI.

fn rect_points(hx: f32, hy: f32) -> [[f32; 2]; 4] {
    [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]]
}

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    match app.query_casts.mode {
        0 => {
            let sdef = bd::ShapeDef::builder()
                .density(0.0)
                .build()
                .expect("valid testbed definition");
            let block = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .position([0.0_f32, 2.5])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("query-cast block body must be valid");
            app.created_bodies += 1;
            let _ = app
                .world
                .body(block)
                .expect("query-cast block body must stay live")
                .create_polygon(
                    &sdef,
                    &bd::shapes::box_polygon(0.5, 0.5)
                        .expect("query-cast block geometry must be valid"),
                )
                .expect("query-cast block shape must be valid");
            app.created_shapes += 1;

            let wall = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .position([2.2_f32, 1.6])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("query-cast wall body must be valid");
            app.created_bodies += 1;
            let _ = app
                .world
                .body(wall)
                .expect("query-cast wall body must stay live")
                .create_polygon(
                    &sdef,
                    &bd::shapes::box_polygon(0.4, 0.9)
                        .expect("query-cast wall geometry must be valid"),
                )
                .expect("query-cast wall shape must be valid");
            app.created_shapes += 1;
        }
        1 => {
            let sdef = bd::ShapeDef::builder()
                .density(0.0)
                .build()
                .expect("valid testbed definition");
            let _ = app
                .world
                .body(ground)
                .expect("testbed ground body must stay live")
                .create_polygon(
                    &sdef,
                    &bd::shapes::box_polygon(0.75, 0.25)
                        .expect("query-cast ground geometry must be valid"),
                )
                .expect("query-cast ground shape must be valid");
            app.created_shapes += 1;

            let obs = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .position([1.5_f32, 1.0])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("query-cast obstacle body must be valid");
            app.created_bodies += 1;
            let _ = app
                .world
                .body(obs)
                .expect("query-cast obstacle body must stay live")
                .create_polygon(
                    &sdef,
                    &bd::shapes::box_polygon(0.4, 0.8)
                        .expect("query-cast obstacle geometry must be valid"),
                )
                .expect("query-cast obstacle shape must be valid");
            app.created_shapes += 1;
        }
        2 => {
            let pillar = app
                .world
                .create_body(
                    bd::BodyBuilder::from(app.foundation.body_def())
                        .position([0.0_f32, 1.0])
                        .build()
                        .expect("valid testbed definition"),
                )
                .expect("query-cast pillar body must be valid");
            app.created_bodies += 1;
            let _ = app
                .world
                .body(pillar)
                .expect("query-cast pillar body must stay live")
                .create_polygon(
                    &bd::ShapeDef::builder()
                        .density(0.0)
                        .build()
                        .expect("valid testbed definition"),
                    &bd::shapes::box_polygon(0.5, 1.0)
                        .expect("query-cast pillar geometry must be valid"),
                )
                .expect("query-cast pillar shape must be valid");
            app.created_shapes += 1;
        }
        _ => {}
    }
}

pub fn tick(app: &mut super::PhysicsApp, _events: Option<&bd::StepEventsSnapshot>) {
    let state = &mut app.query_casts;
    match state.mode {
        0 => {
            let Ok(query) = app.world.query() else {
                state.ray_hits = 0;
                return;
            };
            state.ray_hit_buffer.clear();
            if query
                .cast_ray_all_into(
                    bd::Position::new(
                        bd::WorldScalar::from(state.ray_origin_x),
                        bd::WorldScalar::from(state.ray_origin_y),
                    ),
                    [state.ray_dx, state.ray_dy],
                    bd::QueryFilter::default(),
                    &mut state.ray_hit_buffer,
                )
                .is_ok()
            {
                state.ray_hits = state.ray_hit_buffer.len();
            } else {
                state.ray_hits = 0;
            }
        }
        1 => {
            let Ok(query) = app.world.query() else {
                state.shape_hits = 0;
                return;
            };
            let rect = rect_points(0.5, 0.25);
            let proxy =
                bd::Transform::from_pos_angle([0.0_f32, state.shape_pos_y], state.shape_angle)
                    .and_then(|transform| {
                        bd::ShapeProxy::offset_from_points(rect, state.shape_radius, transform)
                    });
            let Ok(proxy) = proxy else {
                state.shape_hits = 0;
                state.shape_min_fraction = 1.0;
                return;
            };
            state.shape_hit_buffer.clear();
            if query
                .cast_shape_into(
                    bd::Position::ZERO,
                    proxy,
                    [state.shape_tx, state.shape_ty],
                    bd::QueryFilter::default(),
                    &mut state.shape_hit_buffer,
                )
                .is_ok()
            {
                state.shape_hits = state.shape_hit_buffer.len();
                state.shape_min_fraction = state
                    .shape_hit_buffer
                    .iter()
                    .map(|h| h.fraction)
                    .fold(1.0, f32::min);
            } else {
                state.shape_hits = 0;
                state.shape_min_fraction = 1.0;
            }
        }
        2 => {
            let pillar = bd::ShapeProxy::new(rect_points(0.5, 1.0), state.toi_radius)
                .expect("pillar proxy must stay within the Box2D shape-proxy point limit");
            let mover = bd::ShapeProxy::new(rect_points(0.4, 0.4), state.toi_radius)
                .expect("mover proxy must stay within the Box2D shape-proxy point limit");
            let Ok(pillar_sweep) = bd::Sweep::new(
                [0.0_f32, 0.0],
                [0.0, 1.0],
                [0.0, 1.0],
                bd::Rot::IDENTITY,
                bd::Rot::IDENTITY,
            ) else {
                return;
            };
            let Ok(toi_rotation) = bd::Rot::from_radians(state.toi_angle) else {
                return;
            };
            let Ok(mover_sweep) = bd::Sweep::new(
                [0.0_f32, 0.0],
                [state.toi_start_x, state.toi_start_y],
                [
                    state.toi_start_x + state.toi_dx,
                    state.toi_start_y + state.toi_dy,
                ],
                toi_rotation,
                toi_rotation,
            ) else {
                return;
            };
            let Ok(input) = bd::ToiInput::new(pillar, mover, pillar_sweep, mover_sweep) else {
                return;
            };
            let Ok(out) = bd::time_of_impact(input) else {
                return;
            };
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
                || ui.slider(
                    "Angle (rad)",
                    -std::f32::consts::PI,
                    std::f32::consts::PI,
                    &mut ang,
                )
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
                || ui.slider(
                    "Angle (rad)",
                    -std::f32::consts::PI,
                    std::f32::consts::PI,
                    &mut ang,
                )
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
