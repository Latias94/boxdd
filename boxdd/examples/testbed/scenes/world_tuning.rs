use boxdd as bd;
use dear_imgui as imgui;

// World-level tuning demo: sleep/continuous/contact softening, thresholds.

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    // A simple pile to observe effects
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    for i in 0..4 {
        for j in 0..4 {
            let x = -4.0 + i as f32 * 1.2;
            let y = 0.6 + j as f32 * 1.2;
            let b = app
                .world
                .create_body_id(bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic).position([x, y + 4.0]).build());
            app.created_bodies += 1;
            app.world.create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.5, 0.5));
            app.created_shapes += 1;
        }
    }
    // Ground is provided by caller; nothing else to do
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut sl = app.wt_sleeping;
    let mut cc = app.wt_continuous;
    let mut cs = app.wt_softening;
    let mut rt = app.wt_restitution_thres;
    let mut ht = app.wt_hit_thres;
    let mut ws = app.wt_warm_starting;
    let mut max_v = app.wt_max_linear_speed;
    let mut push = app.wt_contact_speed; // requires reset via builder (no runtime getter for tuning)
    let changed_sleep = ui.checkbox("Enable Sleeping", &mut sl);
    let changed_cont = ui.checkbox("Enable Continuous", &mut cc);
    let changed_soft = ui.checkbox("Enable Contact Softening", &mut cs);
    let changed_warm = ui.checkbox("Enable Warm Starting", &mut ws);
    let changed_rt = ui.slider("Restitution Threshold (m/s)", 0.0, 5.0, &mut rt);
    let changed_ht = ui.slider("Hit Event Threshold", 0.0, 50.0, &mut ht);
    let changed_maxv = ui.slider("Maximum Linear Speed", 1.0, 500.0, &mut max_v);
    let changed_push = ui.slider("Contact Push Speed", 0.0, 10.0, &mut push);

    if changed_sleep {
        app.wt_sleeping = sl;
        app.world.enable_sleeping(sl);
    }
    if changed_cont {
        app.wt_continuous = cc;
        app.world.enable_continuous(cc);
    }
    if changed_soft {
        app.wt_softening = cs;
        let _ = app.reset();
    }
    if changed_warm {
        app.wt_warm_starting = ws;
        app.world.enable_warm_starting(ws);
    }
    if changed_rt {
        app.wt_restitution_thres = rt;
        app.world.set_restitution_threshold(rt);
    }
    if changed_ht {
        app.wt_hit_thres = ht;
        app.world.set_hit_event_threshold(ht);
    }
    if changed_maxv {
        app.wt_max_linear_speed = max_v;
        app.world.set_maximum_linear_speed(max_v);
    }
    if changed_push {
        app.wt_contact_speed = push;
        // Rebuild to apply since we don't expose runtime tuning getters
        let _ = app.reset();
    }

    let c = app.world.counters();
    ui.text(format!(
        "Counters: bodies={} shapes={} contacts={} joints={}",
        c.body_count, c.shape_count, c.contact_count, c.joint_count
    ));
}
