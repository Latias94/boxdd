use boxdd as bd;
use dear_imgui as imgui;

// CCD demo: a long thin capsule sliding along a slanted static segment.

pub fn build(app: &mut super::PhysicsApp, ground: bd::types::BodyId) {
    // Static slanted segment from left to right
    let ang = app.ss_slope_deg.to_radians();
    let len = 14.0_f32;
    let dx = 0.5 * len * ang.cos();
    let dy = 0.5 * len * ang.sin();
    let p1 = [-dx, 0.0 + dy];
    let p2 = [dx, 0.0 - dy];
    let sdef = bd::ShapeDef::builder().density(0.0).build();
    let _ = app.world.create_segment_shape_for(ground, &sdef, &bd::shapes::segment(p1, p2));
    app.created_shapes += 1;

    // Thin capsule body above the segment, moving parallel to segment
    let dxv = p2[0] - p1[0];
    let dyv = p2[1] - p1[1];
    let lenv = (dxv * dxv + dyv * dyv).sqrt().max(1e-6);
    let dirx = dxv / lenv;
    let diry = dyv / lenv;
    let start = bd::Vec2::new(-dirx * 5.0, -diry * 5.0 + 4.0);
    let b = app
        .world
        .create_body_id(
            bd::BodyBuilder::new()
                .body_type(bd::BodyType::Dynamic)
                .position([start.x, start.y])
                .bullet(true)
                .build(),
        );
    app.created_bodies += 1;
    let sdef_dyn = bd::ShapeDef::builder().density(1.0).build();
    let cap = bd::shapes::capsule([-1.2, 0.0], [1.2, 0.0], 0.08);
    let _ = app.world.create_capsule_shape_for(b, &sdef_dyn, &cap);
    app.created_shapes += 1;
    // Kick along the segment direction
    unsafe {
        boxdd_sys::ffi::b2Body_SetLinearVelocity(
            b,
            boxdd_sys::ffi::b2Vec2 { x: dirx * app.ss_speed, y: diry * app.ss_speed },
        )
    };
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut slope = app.ss_slope_deg;
    let mut speed = app.ss_speed;
    let changed = ui.slider("Slope (deg)", -45.0, 45.0, &mut slope)
        || ui.slider("Speed", 1.0, 80.0, &mut speed);
    if changed {
        app.ss_slope_deg = slope;
        app.ss_speed = speed;
        let _ = app.reset();
    }
    ui.text("Segment Slide: thin body should track segment via CCD");
}
