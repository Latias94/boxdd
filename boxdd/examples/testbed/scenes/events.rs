use boxdd as bd;
use dear_imgui_rs as imgui;

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    let sensor_def = bd::ShapeDef::builder()
        .density(0.0)
        .sensor(true)
        .enable_sensor_events(true)
        .build()
        .expect("valid testbed definition");
    let _ = app
        .world
        .body(ground)
        .expect("valid testbed operation")
        .create_segment(
            &sensor_def,
            &bd::shapes::segment([-5.0_f32, 1.0], [5.0, 1.0])
                .expect("event sensor segment must be valid"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;

    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build()
        .expect("valid testbed definition");
    let a = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([-0.5_f32, 4.0])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let b = app
        .world
        .create_body(
            bd::BodyBuilder::from(app.foundation.body_def())
                .body_type(bd::BodyType::Dynamic)
                .position([0.5_f32, 6.0])
                .build()
                .expect("valid testbed definition"),
        )
        .expect("valid testbed operation");
    app.created_bodies += 1;
    let _ = app
        .world
        .body(a)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef,
            &bd::shapes::box_polygon(0.4, 0.4).expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;
    let _ = app
        .world
        .body(b)
        .expect("valid testbed operation")
        .create_polygon(
            &sdef,
            &bd::shapes::box_polygon(0.4, 0.4).expect("valid polygon geometry"),
        )
        .expect("valid testbed operation");
    app.created_shapes += 1;

    app.world
        .enable_continuous(true)
        .expect("valid testbed operation");
    app.world
        .set_hit_event_threshold(app.events_threshold)
        .expect("valid testbed operation");
}

pub fn tick(app: &mut super::PhysicsApp, events: Option<&bd::StepEventsSnapshot>) {
    let Some(events) = events else {
        return;
    };
    app.ev_moves += events.body.len();
    app.ev_sens_beg += events.sensor.begin.len();
    app.ev_sens_end += events.sensor.end.len();
    app.ev_con_beg += events.contact.begin.len();
    app.ev_con_end += events.contact.end.len();
    app.ev_con_hit += events.contact.hit.len();
    app.ev_joint += events.joint.len();
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
