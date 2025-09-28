use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    let sensor_def = bd::ShapeDef::builder()
        .density(0.0)
        .sensor(true)
        .enable_sensor_events(true)
        .build();
    let _ = app.world.create_segment_shape_for(
        ground,
        &sensor_def,
        &bd::shapes::segment([-5.0_f32, 1.0], [5.0, 1.0]),
    );
    app.created_shapes += 1;

    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();
    let a = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([-0.5_f32, 4.0])
            .build(),
    );
    app.created_bodies += 1;
    let b = app.world.create_body_id(
        bd::BodyBuilder::new()
            .body_type(bd::BodyType::Dynamic)
            .position([0.5_f32, 6.0])
            .build(),
    );
    app.created_bodies += 1;
    let _ = app
        .world
        .create_polygon_shape_for(a, &sdef, &bd::shapes::box_polygon(0.4, 0.4));
    app.created_shapes += 1;
    let _ = app
        .world
        .create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.4, 0.4));
    app.created_shapes += 1;

    app.world.enable_continuous(true);
    app.world.set_hit_event_threshold(app.events_threshold);
}

pub fn tick(app: &mut super::PhysicsApp) {
    app.ev_moves += app.world.body_events().len();
    let se = app.world.sensor_events();
    app.ev_sens_beg += se.begin.len();
    app.ev_sens_end += se.end.len();
    let ce = app.world.contact_events();
    app.ev_con_beg += ce.begin.len();
    app.ev_con_end += ce.end.len();
    app.ev_con_hit += ce.hit.len();
    app.ev_joint += app.world.joint_events().len();
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut th = app.events_threshold;
    if ui.slider("Hit Threshold", 0.0, 2.0, &mut th) {
        app.events_threshold = th;
        let _ = app.reset();
    }
    if ui.button("Reset Event Counters") {
        app.ev_moves = 0;
        app.ev_sens_beg = 0;
        app.ev_sens_end = 0;
        app.ev_con_beg = 0;
        app.ev_con_end = 0;
        app.ev_con_hit = 0;
        app.ev_joint = 0;
    }
    ui.text(format!(
        "Events: move={} sensor(b={},e={}) contact(b={},e={},hit={}) joints={}",
        app.ev_moves,
        app.ev_sens_beg,
        app.ev_sens_end,
        app.ev_con_beg,
        app.ev_con_end,
        app.ev_con_hit,
        app.ev_joint
    ));
}
