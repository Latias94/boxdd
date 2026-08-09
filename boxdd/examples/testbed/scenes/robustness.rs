use boxdd as bd;
use dear_imgui_rs as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    app.world
        .enable_continuous(true)
        .expect("valid testbed operation");
    app.world
        .set_hit_event_threshold(0.05)
        .expect("valid testbed operation");
    let wall = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    for i in 0..20 {
        let y1 = i as f32 * 0.5;
        let y2 = y1 + 0.5;
        let seg = bd::shapes::segment([6.0_f32, y1], [6.0_f32, y2])
            .expect("robustness wall segment must be valid");
        let _ = app
            .world
            .body(wall)
            .expect("valid testbed operation")
            .create_segment(
                &bd::ShapeDef::builder()
                    .enable_hit_events(true)
                    .build()
                    .expect("valid testbed definition"),
                &seg,
            )
            .expect("valid testbed operation");
        app.created_shapes += 1;
    }
    let bullet = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([0.0_f32, 1.5])
                .bullet(true)
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let _ = app
        .world
        .body(bullet)
        .expect("valid testbed operation")
        .create_circle(
            &bd::ShapeDef::builder()
                .density(1.0)
                .enable_hit_events(true)
                .build()
                .expect("valid testbed definition"),
            &bd::shapes::circle([0.0_f32, 0.0], 0.25)
                .expect("robustness bullet circle must be valid"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    app.world
        .body(bullet)
        .expect("valid testbed operation")
        .set_linear_velocity([app.robust_bullet_speed, 0.0_f32])
        .expect("valid testbed operation");
    // Slender stack to the left for solver stability
    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .build()
        .expect("valid testbed definition");
    for i in 0..10 {
        let id = app
            .world
            .create_body(
                bd::BodyBuilder::from(app.foundation.body_def())
                    .body_type(bd::BodyType::Dynamic)
                    .position([-10.0_f32, 0.5 + i as f32 * 2.1])
                    .build()
                    .expect("valid testbed definition"),
            )
            .expect("valid testbed operation");
        app.created_bodies += 1;
        let _ = app
            .world
            .body(id)
            .expect("valid testbed operation")
            .create_polygon(
                &sdef,
                &bd::shapes::box_polygon(0.1, 1.0).expect("valid polygon geometry"),
            )
            .expect("valid testbed operation");
        app.created_shapes += 1;
    }
}

pub fn tick(app: &mut super::PhysicsApp, events: Option<&bd::StepEventsSnapshot>) {
    if let Some(events) = events {
        app.robust_hit_count += events.contact.hit.len();
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut sp = app.robust_bullet_speed;
    if ui.slider("Bullet Speed", 1.0, 200.0, &mut sp) {
        app.robust_bullet_speed = sp;
        let _ = app.reset();
    }
    ui.text(format!("Hit events so far: {}", app.robust_hit_count));
}
