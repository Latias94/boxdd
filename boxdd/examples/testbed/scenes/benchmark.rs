use boxdd as bd;
use dear_imgui as imgui;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    let n = app.bench_bodies.max(10) as usize;
    let cols = (n as f32).sqrt().ceil() as usize;
    let rows = n.div_ceil(cols);
    let mut spawned = 0usize;
    for r in 0..rows {
        for c in 0..cols {
            if spawned >= n {
                break;
            }
            let x = -((cols as f32) * 0.6) * 0.5 + (c as f32) * 0.6;
            let y = 0.5 + (r as f32) * 0.6 + 2.0;
            let b = app.world.create_body_id(
                bd::BodyBuilder::new()
                    .body_type(bd::BodyType::Dynamic)
                    .position([x, y])
                    .build(),
            );
            app.created_bodies += 1;
            let _ = app
                .world
                .create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(0.25, 0.25));
            app.created_shapes += 1;
            spawned += 1;
        }
    }
}

pub fn tick(_app: &mut super::PhysicsApp) {}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut n = app.bench_bodies;
    if ui.slider("Bodies", 10, 2000, &mut n) {
        app.bench_bodies = n.max(10);
        let _ = app.reset();
    }
    ui.text("Simple stacked boxes for perf.");
}
