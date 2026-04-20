use boxdd as bd;
use dear_imgui_rs as imgui;

// Basic manifold viewer: two convex polygons with adjustable pose and size.
// We sample world contact hit events to display an estimated normal and point.

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
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
            .position([app.mf_a_x, app.mf_a_y])
            .angle(app.mf_a_angle)
            .build(),
    );
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(a, &sdef, &bd::shapes::box_polygon(app.mf_a_hx, app.mf_a_hy));
    app.created_shapes += 1;

    // Body B
    let b = app.world.create_body_id(
        bd::BodyBuilder::from(kdef)
            .position([app.mf_b_x, app.mf_b_y])
            .angle(app.mf_b_angle)
            .build(),
    );
    app.created_bodies += 1;
    app.world.create_polygon_shape_for(b, &sdef, &bd::shapes::box_polygon(app.mf_b_hx, app.mf_b_hy));
    app.created_shapes += 1;
}

pub fn tick(app: &mut super::PhysicsApp) {
    let xf_a = bd::Transform::from_pos_angle([app.mf_a_x, app.mf_a_y], app.mf_a_angle);
    let xf_b = bd::Transform::from_pos_angle([app.mf_b_x, app.mf_b_y], app.mf_b_angle);

    let poly_a = bd::shapes::box_polygon(app.mf_a_hx, app.mf_a_hy);
    let m = match app.mf_mode {
        0 => {
            let poly_b = bd::shapes::box_polygon(app.mf_b_hx, app.mf_b_hy);
            bd::collide_polygons(poly_a, xf_a, poly_b, xf_b)
        }
        1 => {
            let circle_b = bd::shapes::circle([0.0, 0.0], app.mf_b_radius);
            bd::collide_polygon_and_circle(poly_a, xf_a, circle_b, xf_b)
        }
        2 => {
            let half_len = app.mf_b_hx.max(0.1);
            let cap_b = bd::shapes::capsule([-half_len, 0.0], [half_len, 0.0], app.mf_b_radius);
            bd::collide_polygon_and_capsule(poly_a, xf_a, cap_b, xf_b)
        }
        3 => {
            let seg_half = app.mf_seg_half.max(0.1);
            let seg = bd::shapes::segment([-seg_half, 0.0], [seg_half, 0.0]);
            bd::collide_segment_and_polygon(seg, xf_b, poly_a, xf_a)
        }
        4 => {
            let seg_half = app.mf_seg_half.max(0.1);
            let seg = bd::shapes::segment([-seg_half, 0.0], [seg_half, 0.0]);
            let cap = bd::shapes::capsule(
                [-app.mf_b_hx.max(0.1), 0.0],
                [app.mf_b_hx.max(0.1), 0.0],
                app.mf_b_radius,
            );
            bd::collide_segment_and_capsule(seg, xf_b, cap, xf_b)
        }
        _ => bd::Manifold::default(),
    };

    let points = m.points();
    app.mf_contacts = points.len();
    app.mf_normal_x = m.normal.x;
    app.mf_normal_y = m.normal.y;
    if let Some(point) = points.first() {
        app.mf_point_x = point.point.x;
        app.mf_point_y = point.point.y;
        app.mf_sep1 = point.separation;
        app.mf_impulse1 = point.normal_impulse;
        app.mf_total_impulse1 = point.total_normal_impulse;
    } else {
        app.mf_point_x = 0.0;
        app.mf_point_y = 0.0;
        app.mf_sep1 = 0.0;
        app.mf_impulse1 = 0.0;
        app.mf_total_impulse1 = 0.0;
    }
    if let Some(point) = points.get(1) {
        app.mf_point2_x = point.point.x;
        app.mf_point2_y = point.point.y;
        app.mf_sep2 = point.separation;
        app.mf_impulse2 = point.normal_impulse;
        app.mf_total_impulse2 = point.total_normal_impulse;
    } else {
        app.mf_point2_x = 0.0;
        app.mf_point2_y = 0.0;
        app.mf_sep2 = 0.0;
        app.mf_impulse2 = 0.0;
        app.mf_total_impulse2 = 0.0;
    }
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut changed = false;
    // Mode selection
    let mut mode = app.mf_mode;
    changed |= ui.slider("Mode (0=Poly,1=Circle,2=Capsule,3=Seg,4=SegCapsule)", 0, 4, &mut mode);
    if mode != app.mf_mode { app.mf_mode = mode; changed = true; }
    // Body A
    let mut ax = app.mf_a_x;
    let mut ay = app.mf_a_y;
    let mut aa = app.mf_a_angle;
    let mut ahx = app.mf_a_hx;
    let mut ahy = app.mf_a_hy;
    changed |= ui.slider("A: X", -10.0, 10.0, &mut ax);
    changed |= ui.slider("A: Y", -1.0, 12.0, &mut ay);
    changed |= ui.slider("A: Angle (rad)", -3.2, 3.2, &mut aa);
    changed |= ui.slider("A: Half X", 0.1, 2.0, &mut ahx);
    changed |= ui.slider("A: Half Y", 0.1, 2.0, &mut ahy);
    // Body B
    let mut bx = app.mf_b_x;
    let mut by = app.mf_b_y;
    let mut ba = app.mf_b_angle;
    let mut bhx = app.mf_b_hx;
    let mut bhy = app.mf_b_hy;
    changed |= ui.slider("B: X", -10.0, 10.0, &mut bx);
    changed |= ui.slider("B: Y", -1.0, 12.0, &mut by);
    changed |= ui.slider("B: Angle (rad)", -3.2, 3.2, &mut ba);
    if app.mf_mode == 0 {
        changed |= ui.slider("B: Half X", 0.1, 2.0, &mut bhx);
        changed |= ui.slider("B: Half Y", 0.1, 2.0, &mut bhy);
    } else if app.mf_mode == 1 {
        let mut rr = app.mf_b_radius;
        changed |= ui.slider("B (circle): radius", 0.05, 1.5, &mut rr);
        app.mf_b_radius = rr;
    } else if app.mf_mode == 2 {
        let mut rr = app.mf_b_radius;
        changed |= ui.slider("B (capsule): half length (uses B Half X)", 0.1, 2.0, &mut bhx);
        changed |= ui.slider("B (capsule): radius", 0.05, 0.8, &mut rr);
        app.mf_b_radius = rr;
    } else {
        // segment modes use segment half-length and B rotation as orientation
        let mut sh = app.mf_seg_half;
        changed |= ui.slider("B (segment): half length", 0.1, 3.0, &mut sh);
        app.mf_seg_half = sh;
    }
    // Show metrics toggle
    let mut show = app.mf_show_metrics;
    if ui.checkbox("Show metrics", &mut show) {
        app.mf_show_metrics = show;
    }
    if app.mf_show_metrics {
        ui.text(format!(
            "P0: sep={:.4} imp={:.3} total={:.3}",
            app.mf_sep1, app.mf_impulse1, app.mf_total_impulse1
        ));
        if app.mf_contacts > 1 {
            ui.text(format!(
                "P1: sep={:.4} imp={:.3} total={:.3}",
                app.mf_sep2, app.mf_impulse2, app.mf_total_impulse2
            ));
        }
    }

    if changed {
        app.mf_a_x = ax; app.mf_a_y = ay; app.mf_a_angle = aa; app.mf_a_hx = ahx; app.mf_a_hy = ahy;
        app.mf_b_x = bx; app.mf_b_y = by; app.mf_b_angle = ba; app.mf_b_hx = bhx; app.mf_b_hy = bhy;
        let _ = app.reset();
    }
    ui.text(format!(
        "Manifold: hits={} normal=({:.2},{:.2}) point=({:.2},{:.2})",
        app.mf_contacts, app.mf_normal_x, app.mf_normal_y, app.mf_point_x, app.mf_point_y
    ));
}
