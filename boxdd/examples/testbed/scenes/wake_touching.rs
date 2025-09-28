use boxdd as bd;
use dear_imgui as imgui;

// Demonstrates b2Body_WakeTouching: wake up bodies touching a static body.

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // Dedicated static body to act as the waker
    let waker = app
        .world
        .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Static).position([0.0, 0.0]).build());
    app.created_bodies += 1;
    let sdef_static = bd::ShapeDef::builder().density(0.0).build();
    let _ = app
        .world
        .create_polygon_shape_for(waker, &sdef_static, &bd::shapes::box_polygon(5.0, 0.25));
    app.created_shapes += 1;
    app.wt_ground_body = Some(waker);

    // A grid of sleeping boxes resting on the platform
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    for i in 0..4 {
        for j in 0..3 {
            let x = -3.0 + i as f32 * 2.0;
            let y = 1.0 + j as f32 * 1.1;
            let b = app.world.create_body_id(
                bd::BodyBuilder::new()
                    .body_type(bd::BodyType::Dynamic)
                    .position([x, y])
                    .awake(false)
                    .build(),
            );
            app.created_bodies += 1;
            let _ = app
                .world
                .create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.5, 0.5));
            app.created_shapes += 1;
        }
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    if ui.button("Wake Touching (platform)") {
        if let Some(id) = app.wt_ground_body {
            unsafe { boxdd_sys::ffi::b2Body_WakeTouching(id) };
            app.wt_wakes += 1;
        }
    }
    ui.text(format!("Wake Touching: triggered {} times", app.wt_wakes));
}
