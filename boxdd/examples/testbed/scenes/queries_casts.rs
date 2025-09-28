use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Ground is enough; add a dynamic box to hit
    let b = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([0.0_f32, 2.0])
            .build(),
    );
    app.created_bodies += 1;
    let _ = app
        .world
        .create_polygon_shape_for(b, &bd::ShapeDef::builder().density(1.0).build(), &bd::shapes::box_polygon(0.5, 0.5));
    app.created_shapes += 1;
}

pub fn tick(app: &mut super::PhysicsApp) {
    use bd::{Aabb, Vec2};
    let ids = app.world.overlap_aabb(
        Aabb { lower: Vec2::new(-1.0, -1.0), upper: Vec2::new(1.0, 1.0) },
        bd::QueryFilter::default(),
    );
    app.q_overlaps = ids.len();
    let hits = app.world.cast_ray_all(
        [0.0_f32, app.q_ray_origin_y],
        [0.0, -app.q_ray_length],
        bd::QueryFilter::default(),
    );
    app.q_ray_hits = hits.len();
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut oy = app.q_ray_origin_y;
    let mut len = app.q_ray_length;
    if ui.slider("Ray Origin Y", 1.0, 50.0, &mut oy) {
        app.q_ray_origin_y = oy;
    }
    if ui.slider("Ray Length", 1.0, 200.0, &mut len) {
        app.q_ray_length = len;
    }
    ui.text(format!(
        "Queries: overlaps={} ray_hits={}",
        app.q_overlaps, app.q_ray_hits
    ));
}
