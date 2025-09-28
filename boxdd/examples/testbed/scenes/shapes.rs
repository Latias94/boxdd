use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Circle, box, capsule on ground with mixed materials
    let s_circle = bd::ShapeDef::builder()
        .density(1.0)
        .material(bd::SurfaceMaterial::default().friction(0.8).restitution(0.7))
        .build();
    let s_poly = bd::ShapeDef::builder()
        .density(1.0)
        .material(bd::SurfaceMaterial::default().friction(0.4).restitution(0.1))
        .build();
    let s_caps = bd::ShapeDef::builder()
        .density(1.0)
        .material(bd::SurfaceMaterial::default().friction(0.05).restitution(0.0))
        .build();
    let b_circle = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([-4.0_f32, 6.0])
            .build(),
    );
    app.created_bodies += 1;
    let b_poly = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([0.0_f32, 6.0])
            .build(),
    );
    app.created_bodies += 1;
    let b_caps = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([4.0_f32, 6.0])
            .build(),
    );
    app.created_bodies += 1;
    let _ = app.world.create_circle_shape_for(
        b_circle,
        &s_circle,
        &bd::shapes::circle([0.0_f32, 0.0], 0.5),
    );
    app.created_shapes += 1;
    let _ = app.world.create_polygon_shape_for(
        b_poly,
        &s_poly,
        &bd::shapes::box_polygon(0.6, 0.4),
    );
    app.created_shapes += 1;
    let _ = app.world.create_capsule_shape_for(
        b_caps,
        &s_caps,
        &bd::shapes::capsule([-0.6_f32, 0.0], [0.6, 0.0], 0.2),
    );
    app.created_shapes += 1;
}

pub fn tick(_app: &mut super::PhysicsApp) {}

pub fn ui_params(_app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    ui.text("Spawns: circle, box, capsule (materials)");
}
