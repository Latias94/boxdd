use boxdd as bd;
use dear_imgui_rs as imgui;

fn rect_points(hx: f32, hy: f32) -> [[f32; 2]; 4] {
    [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]]
}

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let sdef = bd::ShapeDef::builder().density(0.0).build();
    for (x, y, hx, hy) in [
        (0.0_f32, 2.0, 0.6, 0.6),
        (1.3, 2.4, 0.5, 0.5),
        (3.2, 1.9, 0.4, 0.9),
    ] {
        let body = app
            .world
            .create_body_id(bd::BodyBuilder::new().position([x, y]).build());
        app.created_bodies += 1;
        let _ =
            app.world
                .create_polygon_shape_for(body, &sdef, &bd::shapes::box_polygon(hx, hy));
        app.created_shapes += 1;
    }
}

pub fn tick(app: &mut super::PhysicsApp) {
    let filter = bd::QueryFilter::default();
    let aabb = bd::Aabb::from_center_half_extents(
        [app.q_center_x, app.q_center_y],
        [app.q_half_x, app.q_half_y],
    );

    let owned_hits = app.world.overlap_aabb(aabb, filter);
    app.q_overlaps = owned_hits.len();

    let mut reused_hits = Vec::new();
    app.world.overlap_aabb_into(aabb, filter, &mut reused_hits);
    app.q_reused_hits = reused_hits.len();

    let mut visited_hits = 0usize;
    let _ = app.world.visit_overlap_aabb(aabb, filter, |_| {
        visited_hits += 1;
        true
    });
    app.q_visit_hits = visited_hits;

    let mut stopped_early = false;
    let completed = app.world.visit_overlap_aabb(aabb, filter, |_| {
        stopped_early = true;
        false
    });
    app.q_visit_stopped_early = stopped_early && !completed;

    app.q_polygon_hits = app
        .world
        .overlap_polygon_points_with_offset(
            rect_points(app.q_half_x * 0.55, app.q_half_y * 0.55),
            0.01,
            [app.q_center_x, app.q_center_y],
            0.0_f32,
            filter,
        )
        .len();
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut cx = app.q_center_x;
    let mut cy = app.q_center_y;
    let mut hx = app.q_half_x;
    let mut hy = app.q_half_y;
    let changed = ui.slider("Center X", -8.0, 8.0, &mut cx)
        || ui.slider("Center Y", -1.0, 8.0, &mut cy)
        || ui.slider("Half X", 0.2, 4.0, &mut hx)
        || ui.slider("Half Y", 0.2, 3.0, &mut hy);
    if changed {
        app.q_center_x = cx;
        app.q_center_y = cy;
        app.q_half_x = hx.max(0.2);
        app.q_half_y = hy.max(0.2);
    }
    ui.text(format!(
        "AABB overlap: owned={} reused={} visited={}",
        app.q_overlaps, app.q_reused_hits, app.q_visit_hits
    ));
    ui.text(format!(
        "Offset polygon overlap: {} early_exit={}",
        app.q_polygon_hits, app.q_visit_stopped_early
    ));
}
