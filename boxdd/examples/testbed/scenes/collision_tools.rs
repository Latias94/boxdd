use boxdd as bd;
use dear_imgui as imgui;

// Collision Tools: unify Ray, Overlap, Shape Cast, TOI into one scene with a mode toggle.

#[allow(dead_code)]
fn rect_points(hx: f32, hy: f32) -> [[f32; 2]; 4] { [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]] }
#[allow(dead_code)]
fn box_pts(h: f32) -> [[f32; 2]; 4] { [[-h, -h], [h, -h], [h, h], [-h, h]] }

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    match app.ct_mode {
        0 => { /* Ray: no special world setup */ }
        1 => { /* Overlap: no special world setup */ }
        2 => {
            // Shape Cast: add a couple of static obstacles to cast into
            let sdef = bd::ShapeDef::builder().density(0.0).build();
            let _ = app
                .world
                .create_polygon_shape_for(ground, &sdef, &bd::shapes::box_polygon(0.75, 0.25));
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
        3 => {
            // TOI (via shape cast fraction): add a static pillar to hit
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

#[allow(dead_code)]
pub fn tick(app: &mut super::PhysicsApp) {
    match app.ct_mode {
        0 => {
            // Ray world
            let hits = app.world.cast_ray_all(
                [app.rw_origin_x, app.rw_origin_y],
                [app.rw_dx, app.rw_dy],
                bd::QueryFilter::default(),
            );
            app.rw_hits = hits.len();
        }
        1 => {
            // Overlap AABB
            let aabb = bd::Aabb::from_center_half_extents(
                [app.ow_center_x, app.ow_center_y],
                [app.ow_half_x, app.ow_half_y],
            );
            let ids = app.world.overlap_aabb(aabb, bd::QueryFilter::default());
            app.ow_hits = ids.len();
        }
        2 => {
            // Shape cast rectangle
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
        3 => {
            // TOI-like metric via shape cast fraction on a box
            let a = box_pts(0.4);
            let hits = app.world.cast_shape_points_with_offset(
                a,
                app.toi_radius,
                [app.toi_start_x, app.toi_start_y],
                app.toi_angle,
                [app.toi_dx, app.toi_dy],
                bd::QueryFilter::default(),
            );
            app.toi_hits = hits.len();
            app.toi_min_fraction = hits.iter().map(|h| h.fraction).fold(1.0, f32::min);
        }
        _ => {}
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    // Mode switcher
    let names = ["Ray", "Overlap", "Shape Cast", "TOI"];
    let mut idx = app.ct_mode.clamp(0, 3) as usize;
    if let Some(_c) = ui.begin_combo("Mode", names[idx]) {
        for (i, &name) in names.iter().enumerate() {
            let selected = i == idx;
            if ui.selectable_config(name).selected(selected).build() {
                idx = i;
                app.ct_mode = i as i32;
                let _ = app.reset();
            }
        }
    }
    match app.ct_mode {
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
            ui.text(format!("Ray World: hits={}", app.rw_hits));
        }
        1 => {
            let mut cx = app.ow_center_x;
            let mut cy = app.ow_center_y;
            let mut hx = app.ow_half_x;
            let mut hy = app.ow_half_y;
            let changed = ui.slider("Center X", -25.0, 25.0, &mut cx)
                || ui.slider("Center Y", -5.0, 25.0, &mut cy)
                || ui.slider("Half X", 0.1, 10.0, &mut hx)
                || ui.slider("Half Y", 0.1, 10.0, &mut hy);
            if changed {
                app.ow_center_x = cx;
                app.ow_center_y = cy;
                app.ow_half_x = hx.max(0.1);
                app.ow_half_y = hy.max(0.1);
            }
            ui.text(format!("Overlap World: matches={}", app.ow_hits));
        }
        2 => {
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
        3 => {
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
                "TOI (shape-cast) hits={} min_fraction={:.3}",
                app.toi_hits, app.toi_min_fraction
            ));
        }
        _ => {}
    }
}
