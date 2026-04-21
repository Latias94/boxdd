use boxdd as bd;
use dear_imgui_rs as imgui;

#[allow(dead_code)]
fn rect_points(hx: f32, hy: f32) -> [[f32; 2]; 4] {
    [
        [-hx, -hy],
        [hx, -hy],
        [hx, hy],
        [-hx, hy],
    ]
}

pub fn build(_app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {}

#[allow(dead_code)]
pub fn tick(app: &mut super::PhysicsApp) {
    let proxy_a = bd::ShapeProxy::new(rect_points(app.sd_a_hx, app.sd_a_hy), app.sd_a_radius)
        .expect("rect proxy must stay within the Box2D shape-proxy point limit");
    let proxy_b = bd::ShapeProxy::new(rect_points(app.sd_b_hx, app.sd_b_hy), app.sd_b_radius)
        .expect("rect proxy must stay within the Box2D shape-proxy point limit");
    let mut cache = bd::SimplexCache::default();
    let out = bd::shape_distance(
        bd::DistanceInput::new(
            proxy_a,
            proxy_b,
            bd::Transform::from_pos_angle([app.sd_a_x, app.sd_a_y], app.sd_a_angle),
            bd::Transform::from_pos_angle([app.sd_b_x, app.sd_b_y], app.sd_b_angle),
        )
        .with_radii(true),
        &mut cache,
    );
    app.sd_distance = out.distance;
    app.sd_point_ax = out.point_a.x;
    app.sd_point_ay = out.point_a.y;
    app.sd_point_bx = out.point_b.x;
    app.sd_point_by = out.point_b.y;
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    // Controls for shape A and shape B
    let mut ax = app.sd_a_x; let mut ay = app.sd_a_y; let mut aa = app.sd_a_angle;
    let mut ahx = app.sd_a_hx; let mut ahy = app.sd_a_hy; let mut ar = app.sd_a_radius;
    let mut bx = app.sd_b_x; let mut by = app.sd_b_y; let mut ba = app.sd_b_angle;
    let mut bhx = app.sd_b_hx; let mut bhy = app.sd_b_hy; let mut br = app.sd_b_radius;
    let changed =
        ui.slider("A.x", -10.0, 10.0, &mut ax) ||
        ui.slider("A.y", -10.0, 10.0, &mut ay) ||
        ui.slider("A.angle", -std::f32::consts::PI, std::f32::consts::PI, &mut aa) ||
        ui.slider("A.hx", 0.05, 5.0, &mut ahx) ||
        ui.slider("A.hy", 0.05, 5.0, &mut ahy) ||
        ui.slider("A.radius", 0.0, 0.5, &mut ar) ||
        ui.slider("B.x", -10.0, 10.0, &mut bx) ||
        ui.slider("B.y", -10.0, 10.0, &mut by) ||
        ui.slider("B.angle", -std::f32::consts::PI, std::f32::consts::PI, &mut ba) ||
        ui.slider("B.hx", 0.05, 5.0, &mut bhx) ||
        ui.slider("B.hy", 0.05, 5.0, &mut bhy) ||
        ui.slider("B.radius", 0.0, 0.5, &mut br);
    if changed {
        app.sd_a_x = ax; app.sd_a_y = ay; app.sd_a_angle = aa;
        app.sd_a_hx = ahx; app.sd_a_hy = ahy; app.sd_a_radius = ar.max(0.0);
        app.sd_b_x = bx; app.sd_b_y = by; app.sd_b_angle = ba;
        app.sd_b_hx = bhx; app.sd_b_hy = bhy; app.sd_b_radius = br.max(0.0);
    }
    ui.text(format!(
        "Distance = {:.4}, A=({:.3},{:.3}) B=({:.3},{:.3})",
        app.sd_distance, app.sd_point_ax, app.sd_point_ay, app.sd_point_bx, app.sd_point_by
    ));
}

