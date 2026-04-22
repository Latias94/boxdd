use boxdd as bd;
use dear_imgui_rs as imgui;

// Basic manifold viewer: two convex polygons with adjustable pose and size.
// We sample world contact hit events to display an estimated normal and point.

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let state = &app.manifold;
    let kdef = bd::BodyBuilder::new()
        .body_type(bd::BodyType::Kinematic)
        .build();

    let sdef = bd::ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .enable_hit_events(true)
        .build();

    // Body A
    let a = app.world.create_body_id(
        bd::BodyBuilder::from(kdef.clone())
            .position([state.a_x, state.a_y])
            .angle(state.a_angle)
            .build(),
    );
    app.created_bodies += 1;
    app.world
        .create_polygon_shape_for(a, &sdef, &bd::shapes::box_polygon(state.a_half_x, state.a_half_y));
    app.created_shapes += 1;

    // Body B
    let b = app.world.create_body_id(
        bd::BodyBuilder::from(kdef)
            .position([state.b_x, state.b_y])
            .angle(state.b_angle)
            .build(),
    );
    app.created_bodies += 1;
    app.world
        .create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(state.b_half_x, state.b_half_y));
    app.created_shapes += 1;
}

pub fn tick(app: &mut super::PhysicsApp) {
    let state = &mut app.manifold;
    let xf_a = bd::Transform::from_pos_angle([state.a_x, state.a_y], state.a_angle);
    let xf_b = bd::Transform::from_pos_angle([state.b_x, state.b_y], state.b_angle);

    let poly_a = bd::shapes::box_polygon(state.a_half_x, state.a_half_y);
    let m = match state.mode {
        0 => {
            let poly_b = bd::shapes::box_polygon(state.b_half_x, state.b_half_y);
            bd::collide_polygons(poly_a, xf_a, poly_b, xf_b)
        }
        1 => {
            let circle_b = bd::shapes::circle([0.0, 0.0], state.b_radius);
            bd::collide_polygon_and_circle(poly_a, xf_a, circle_b, xf_b)
        }
        2 => {
            let half_len = state.b_half_x.max(0.1);
            let cap_b = bd::shapes::capsule([-half_len, 0.0], [half_len, 0.0], state.b_radius);
            bd::collide_polygon_and_capsule(poly_a, xf_a, cap_b, xf_b)
        }
        3 => {
            let seg_half = state.segment_half_len.max(0.1);
            let seg = bd::shapes::segment([-seg_half, 0.0], [seg_half, 0.0]);
            bd::collide_segment_and_polygon(seg, xf_b, poly_a, xf_a)
        }
        4 => {
            let seg_half = state.segment_half_len.max(0.1);
            let seg = bd::shapes::segment([-seg_half, 0.0], [seg_half, 0.0]);
            let cap = bd::shapes::capsule(
                [-state.b_half_x.max(0.1), 0.0],
                [state.b_half_x.max(0.1), 0.0],
                state.b_radius,
            );
            bd::collide_segment_and_capsule(seg, xf_b, cap, xf_b)
        }
        _ => bd::Manifold::default(),
    };

    let points = m.points();
    state.contact_count = points.len();
    state.normal_x = m.normal.x;
    state.normal_y = m.normal.y;
    if let Some(point) = points.first() {
        state.point1_x = point.point.x;
        state.point1_y = point.point.y;
        state.separation1 = point.separation;
        state.impulse1 = point.normal_impulse;
        state.total_impulse1 = point.total_normal_impulse;
    } else {
        state.point1_x = 0.0;
        state.point1_y = 0.0;
        state.separation1 = 0.0;
        state.impulse1 = 0.0;
        state.total_impulse1 = 0.0;
    }
    if let Some(point) = points.get(1) {
        state.point2_x = point.point.x;
        state.point2_y = point.point.y;
        state.separation2 = point.separation;
        state.impulse2 = point.normal_impulse;
        state.total_impulse2 = point.total_normal_impulse;
    } else {
        state.point2_x = 0.0;
        state.point2_y = 0.0;
        state.separation2 = 0.0;
        state.impulse2 = 0.0;
        state.total_impulse2 = 0.0;
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut changed = false;
    // Mode selection
    let mut mode = app.manifold.mode;
    changed |= ui.slider("Mode (0=Poly,1=Circle,2=Capsule,3=Seg,4=SegCapsule)", 0, 4, &mut mode);
    if mode != app.manifold.mode {
        app.manifold.mode = mode;
        changed = true;
    }
    // Body A
    let mut ax = app.manifold.a_x;
    let mut ay = app.manifold.a_y;
    let mut aa = app.manifold.a_angle;
    let mut ahx = app.manifold.a_half_x;
    let mut ahy = app.manifold.a_half_y;
    changed |= ui.slider("A: X", -10.0, 10.0, &mut ax);
    changed |= ui.slider("A: Y", -1.0, 12.0, &mut ay);
    changed |= ui.slider("A: Angle (rad)", -3.2, 3.2, &mut aa);
    changed |= ui.slider("A: Half X", 0.1, 2.0, &mut ahx);
    changed |= ui.slider("A: Half Y", 0.1, 2.0, &mut ahy);
    // Body B
    let mut bx = app.manifold.b_x;
    let mut by = app.manifold.b_y;
    let mut ba = app.manifold.b_angle;
    let mut bhx = app.manifold.b_half_x;
    let mut bhy = app.manifold.b_half_y;
    changed |= ui.slider("B: X", -10.0, 10.0, &mut bx);
    changed |= ui.slider("B: Y", -1.0, 12.0, &mut by);
    changed |= ui.slider("B: Angle (rad)", -3.2, 3.2, &mut ba);
    if app.manifold.mode == 0 {
        changed |= ui.slider("B: Half X", 0.1, 2.0, &mut bhx);
        changed |= ui.slider("B: Half Y", 0.1, 2.0, &mut bhy);
    } else if app.manifold.mode == 1 {
        let mut rr = app.manifold.b_radius;
        changed |= ui.slider("B (circle): radius", 0.05, 1.5, &mut rr);
        app.manifold.b_radius = rr;
    } else if app.manifold.mode == 2 {
        let mut rr = app.manifold.b_radius;
        changed |= ui.slider("B (capsule): half length (uses B Half X)", 0.1, 2.0, &mut bhx);
        changed |= ui.slider("B (capsule): radius", 0.05, 0.8, &mut rr);
        app.manifold.b_radius = rr;
    } else {
        // segment modes use segment half-length and B rotation as orientation
        let mut sh = app.manifold.segment_half_len;
        changed |= ui.slider("B (segment): half length", 0.1, 3.0, &mut sh);
        app.manifold.segment_half_len = sh;
    }
    // Show metrics toggle
    let mut show = app.manifold.show_metrics;
    if ui.checkbox("Show metrics", &mut show) {
        app.manifold.show_metrics = show;
    }
    if app.manifold.show_metrics {
        ui.text(format!(
            "P0: sep={:.4} imp={:.3} total={:.3}",
            app.manifold.separation1, app.manifold.impulse1, app.manifold.total_impulse1
        ));
        if app.manifold.contact_count > 1 {
            ui.text(format!(
                "P1: sep={:.4} imp={:.3} total={:.3}",
                app.manifold.separation2, app.manifold.impulse2, app.manifold.total_impulse2
            ));
        }
    }

    if changed {
        {
            let state = &mut app.manifold;
            state.a_x = ax;
            state.a_y = ay;
            state.a_angle = aa;
            state.a_half_x = ahx;
            state.a_half_y = ahy;
            state.b_x = bx;
            state.b_y = by;
            state.b_angle = ba;
            state.b_half_x = bhx;
            state.b_half_y = bhy;
        }
        let _ = app.reset();
    }
    ui.text(format!(
        "Manifold: hits={} normal=({:.2},{:.2}) point=({:.2},{:.2})",
        app.manifold.contact_count,
        app.manifold.normal_x,
        app.manifold.normal_y,
        app.manifold.point1_x,
        app.manifold.point1_y
    ));
}

pub fn debug_overlay(app: &super::PhysicsApp, ui: &imgui::Ui) {
    let state = &app.manifold;
    let dl = ui.get_foreground_draw_list();
    let ds = ui.io().display_size();
    let origin = [ds[0] * 0.5, ds[1] * 0.5];
    let scale = app.pixels_per_meter;
    let w2s = |x: f32, y: f32| [origin[0] + x * scale, ds[1] - (origin[1] + y * scale)];

    let point = w2s(state.point1_x, state.point1_y);
    dl.add_circle(point, 5.0, 0xffff55ffu32)
        .thickness(2.0)
        .build();
    if state.contact_count > 1 {
        let point2 = w2s(state.point2_x, state.point2_y);
        dl.add_circle(point2, 5.0, 0xff55ffffu32)
            .thickness(2.0)
            .build();
    }

    let normal_tip = w2s(
        state.point1_x + state.normal_x * 0.7,
        state.point1_y + state.normal_y * 0.7,
    );
    dl.add_line(point, normal_tip, 0xffffff00u32)
        .thickness(2.0)
        .build();
}
