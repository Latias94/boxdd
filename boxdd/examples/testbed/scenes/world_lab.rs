use boxdd as bd;
use bd::world_extras::ExplosionDef;
use dear_imgui_rs as imgui;

// World Lab: Tuning + Explosion

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    match app.world_lab.mode {
        0 => {
            // small pile to observe tuning effects
            let sdef = bd::ShapeDef::builder().density(1.0).build();
            for i in 0..4 {
                for j in 0..4 {
                    let x = -4.0 + i as f32 * 1.2;
                    let y = 0.6 + j as f32 * 1.2;
                    let b = app
                        .world
                        .create_body_id(
                            bd::BodyBuilder::new()
                                .body_type(bd::BodyType::Dynamic)
                                .position([x, y + 4.0])
                                .build(),
                        );
                    app.created_bodies += 1;
                    app.world.create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.5, 0.5));
                    app.created_shapes += 1;
                }
            }
        }
        1 => {
            // Explosion mode doesn't require extra setup; use existing bodies
        }
        _ => {}
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let names = ["Tuning", "Explosion"];
    let mut m = app.world_lab.mode;
    if ui.combo_simple_string("World Lab", &mut m, &names) && m != app.world_lab.mode {
        app.world_lab.mode = m;
        let _ = app.reset();
        return;
    }
    match app.world_lab.mode {
        0 => {
            let mut sl = app.world_lab.sleeping;
            let mut cc = app.world_lab.continuous;
            let mut cs = app.world_lab.contact_softening;
            let mut rt = app.world_lab.restitution_threshold;
            let mut ht = app.world_lab.hit_event_threshold;
            let mut ws = app.world_lab.warm_starting;
            let mut max_v = app.world_lab.maximum_linear_speed;
            let mut push = app.world_lab.contact_speed;
            let changed_sleep = ui.checkbox("Enable Sleeping", &mut sl);
            let changed_cont = ui.checkbox("Enable Continuous", &mut cc);
            let changed_soft = ui.checkbox("Enable Contact Softening", &mut cs);
            let changed_warm = ui.checkbox("Enable Warm Starting", &mut ws);
            let changed_rt = ui.slider("Restitution Threshold (m/s)", 0.0, 5.0, &mut rt);
            let changed_ht = ui.slider("Hit Event Threshold", 0.0, 50.0, &mut ht);
            let changed_maxv = ui.slider("Maximum Linear Speed", 1.0, 500.0, &mut max_v);
            let changed_push = ui.slider("Contact Push Speed", 0.0, 10.0, &mut push);
            let mut reset_needed = false;
            if changed_sleep {
                app.world_lab.sleeping = sl;
                app.world.enable_sleeping(sl);
            }
            if changed_cont {
                app.world_lab.continuous = cc;
                app.world.enable_continuous(cc);
            }
            if changed_soft {
                app.world_lab.contact_softening = cs;
                reset_needed = true;
            }
            if changed_warm {
                app.world_lab.warm_starting = ws;
                app.world.enable_warm_starting(ws);
            }
            if changed_rt {
                app.world_lab.restitution_threshold = rt;
                app.world.set_restitution_threshold(rt);
            }
            if changed_ht {
                app.world_lab.hit_event_threshold = ht;
                app.world.set_hit_event_threshold(ht);
            }
            if changed_maxv {
                app.world_lab.maximum_linear_speed = max_v;
                app.world.set_maximum_linear_speed(max_v);
            }
            if changed_push {
                app.world_lab.contact_speed = push;
                reset_needed = true;
            }
            if reset_needed {
                let _ = app.reset();
            }
        }
        1 => {
            let mut px = app.world_lab.explosion_center_x;
            let mut py = app.world_lab.explosion_center_y;
            let mut r = app.world_lab.explosion_radius;
            let mut f = app.world_lab.explosion_falloff;
            let mut imp = app.world_lab.explosion_impulse;
            let changed = ui.slider("Center X", -50.0, 50.0, &mut px)
                || ui.slider("Center Y", -10.0, 50.0, &mut py)
                || ui.slider("Radius", 0.0, 20.0, &mut r)
                || ui.slider("Falloff", 0.0, 20.0, &mut f)
                || ui.slider("Impulse/Len", -20.0, 20.0, &mut imp);
            if changed {
                app.world_lab.explosion_center_x = px;
                app.world_lab.explosion_center_y = py;
                app.world_lab.explosion_radius = r.max(0.0);
                app.world_lab.explosion_falloff = f.max(0.0);
                app.world_lab.explosion_impulse = imp;
            }
            if ui.button("Explode") {
                let def = ExplosionDef::new()
                    .position([
                        app.world_lab.explosion_center_x,
                        app.world_lab.explosion_center_y,
                    ])
                    .radius(app.world_lab.explosion_radius)
                    .falloff(app.world_lab.explosion_falloff)
                    .impulse_per_length(app.world_lab.explosion_impulse);
                app.world.explode(&def);
            }
        }
        _ => {}
    }
}

