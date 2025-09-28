use boxdd as bd;
use dear_imgui as imgui;

fn gen_points(n: usize, radius: f32) -> Vec<[f32; 2]> {
    // Deterministic pseudo-random points roughly on a circle with slight jitter
    let mut pts = Vec::with_capacity(n);
    let two_pi = core::f32::consts::PI * 2.0;
    for i in 0..n {
        let t = (i as f32) / (n as f32);
        let angle = two_pi * t + (t * 37.0).sin() * 0.15;
        let r = radius * (1.0 + 0.15 * (t * 19.0).cos());
        pts.push([r * angle.cos(), r * angle.sin()]);
    }
    pts
}

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Spawn a dynamic convex hull built from N points
    let n = app.hull_points.clamp(3, 32) as usize;
    let pts = gen_points(n, 1.2);
    if let Some(poly) = bd::shapes::polygon_from_points(pts.iter().copied(), 0.02) {
        let body = app.world.create_body_id(
            bd::BodyBuilder::new()
                .body_type(bd::BodyType::Dynamic)
                .position([0.0_f32, 6.0])
                .build(),
        );
        app.created_bodies += 1;
        let _ = app
            .world
            .create_polygon_shape_for(body, &bd::ShapeDef::builder().density(1.0).build(), &poly);
        app.created_shapes += 1;
    }
}

#[allow(dead_code)]
pub fn tick(_app: &mut super::PhysicsApp) {}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut n = app.hull_points;
    if ui.slider("Points", 3, 32, &mut n) {
        app.hull_points = n.clamp(3, 32);
        let _ = app.reset();
    }
    ui.text("Builds a convex hull from generated points.");
}
