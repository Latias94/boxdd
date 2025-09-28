use boxdd as bd;
use dear_imgui as imgui;
use boxdd_sys::ffi;

#[allow(dead_code)]
fn rect_points(hx: f32, hy: f32) -> [ffi::b2Vec2; 4] {
    [
        ffi::b2Vec2 { x: -hx, y: -hy },
        ffi::b2Vec2 { x: hx, y: -hy },
        ffi::b2Vec2 { x: hx, y: hy },
        ffi::b2Vec2 { x: -hx, y: hy },
    ]
}

pub fn build(_app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {}

#[allow(dead_code)]
pub fn tick(app: &mut super::PhysicsApp) {
    // Build two proxies and compute GJK distance
    let a_pts = rect_points(app.sd_a_hx, app.sd_a_hy);
    let b_pts = rect_points(app.sd_b_hx, app.sd_b_hy);
    let proxy_a = unsafe { ffi::b2MakeProxy(a_pts.as_ptr(), a_pts.len() as i32, app.sd_a_radius) };
    let proxy_b = unsafe { ffi::b2MakeProxy(b_pts.as_ptr(), b_pts.len() as i32, app.sd_b_radius) };
    let (sa, ca) = app.sd_a_angle.sin_cos();
    let (sb, cb) = app.sd_b_angle.sin_cos();
    let ta = ffi::b2Transform {
        p: bd::Vec2::new(app.sd_a_x, app.sd_a_y).into(),
        q: ffi::b2Rot { c: ca, s: sa },
    };
    let tb = ffi::b2Transform {
        p: bd::Vec2::new(app.sd_b_x, app.sd_b_y).into(),
        q: ffi::b2Rot { c: cb, s: sb },
    };
    let input = ffi::b2DistanceInput {
        proxyA: proxy_a,
        proxyB: proxy_b,
        transformA: ta,
        transformB: tb,
        useRadii: true,
    };
    let mut cache = ffi::b2SimplexCache { count: 0, indexA: [0; 3], indexB: [0; 3] };
    let out = unsafe { ffi::b2ShapeDistance(&input, &mut cache, core::ptr::null_mut(), 0) };
    app.sd_distance = out.distance;
    app.sd_point_ax = out.pointA.x;
    app.sd_point_ay = out.pointA.y;
    app.sd_point_bx = out.pointB.x;
    app.sd_point_by = out.pointB.y;
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

