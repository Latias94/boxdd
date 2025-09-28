use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    for i in 0..6 {
        let b = app.world.create_body_id(
            bd::BodyBuilder::new()
                .body_type(bd::BodyType::Dynamic)
                .position([-2.0_f32, 0.6 + i as f32 * 0.55])
                .build(),
        );
        app.created_bodies += 1;
        let _ = app
            .world
            .create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.25, 0.25));
        app.created_shapes += 1;
    }
    let pend = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([1.0_f32, 3.0])
            .build(),
    );
    app.created_bodies += 1;
    let _ = app
        .world
        .create_polygon_shape_for(pend, &sdef, &bd::shapes::box_polygon(0.1, 0.5));
    app.created_shapes += 1;
    let base = app
        .world
        .joint_base_from_world_points(ground, pend, [1.0_f32, 3.5], [1.0_f32, 3.5]);
    let rdef = bd::RevoluteJointDef::new(base)
        .enable_motor(true)
        .max_motor_torque(1.0)
        .motor_speed(2.0);
    let _ = app.world.create_revolute_joint_id(&rdef);
    app.created_joints += 1;
}

pub fn tick(_app: &mut super::PhysicsApp) {}

pub fn ui_params(_app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    ui.text("Single-threaded (worker_count=1), continuous on.");
}
