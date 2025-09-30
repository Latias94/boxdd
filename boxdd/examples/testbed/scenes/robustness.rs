use boxdd as bd;
use dear_imgui_rs as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    app.world.enable_continuous(true);
    app.world.set_hit_event_threshold(0.05);
    let wall = app.world.create_body_id(bd::BodyBuilder::new().build());
    app.created_bodies += 1;
    for i in 0..20 {
        let y1 = i as f32 * 0.5;
        let y2 = y1 + 0.5;
        let seg = bd::shapes::segment([6.0_f32, y1], [6.0_f32, y2]);
        let _ = app
            .world
            .create_segment_shape_for(wall, &bd::ShapeDef::builder().enable_hit_events(true).build(), &seg);
        app.created_shapes += 1;
    }
    let bullet = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([0.0_f32, 1.5])
            .bullet(true)
            .build(),
    );
    app.created_bodies += 1;
    let _ = app.world.create_circle_shape_for(
        bullet,
        &bd::ShapeDef::builder()
            .density(1.0)
            .enable_hit_events(true)
            .build(),
        &bd::shapes::circle([0.0_f32, 0.0], 0.25),
    );
    app.created_shapes += 1;
    app.world
        .set_body_linear_velocity(bullet, [app.robust_bullet_speed, 0.0_f32]);
    // Slender stack to the left for solver stability
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    for i in 0..10 {
        let id = app.world.create_body_id(
            bd::BodyBuilder::new()
                .body_type(bd::BodyType::Dynamic)
                .position([-10.0_f32, 0.5 + i as f32 * 2.1])
                .build(),
        );
        app.created_bodies += 1;
        let _ = app
            .world
            .create_polygon_shape_for(id, &sdef, &bd::shapes::box_polygon(0.1, 1.0));
        app.created_shapes += 1;
    }
}

pub fn tick(app: &mut super::PhysicsApp) {
    app.robust_hit_count += app.world.contact_events().hit.len();
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut sp = app.robust_bullet_speed;
    if ui.slider("Bullet Speed", 1.0, 200.0, &mut sp) {
        app.robust_bullet_speed = sp;
        let _ = app.reset();
    }
    ui.text(format!("Hit events so far: {}", app.robust_hit_count));
}
