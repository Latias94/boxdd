use boxdd as bd;
use dear_imgui_rs as imgui;

fn rect_points(hx: f32, hy: f32) -> [[f32; 2]; 4] {
    [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]]
}

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let sdef = bd::ShapeDef::builder()
        .density(0.0)
        .build()
        .expect("valid testbed definition");
    for (x, y, hx, hy) in [
        (0.0_f32, 2.0, 0.6, 0.6),
        (1.3, 2.4, 0.5, 0.5),
        (3.2, 1.9, 0.4, 0.9),
    ] {
        let body = app
            .world
            .create_body(
                bd::BodyBuilder::from(app.foundation.body_def())
                    .position([x, y])
                    .build()
                    .expect("valid testbed definition"),
            )
            .expect("overlap-query body must be valid");
        app.created_bodies += 1;
        let _ = app
            .world
            .body(body)
            .expect("overlap-query body must stay live")
            .create_polygon(
                &sdef,
                &bd::shapes::box_polygon(hx, hy).expect("overlap-query geometry must be valid"),
            )
            .expect("overlap-query shape must be valid");
        app.created_shapes += 1;
    }
}

pub fn tick(app: &mut super::PhysicsApp, _events: Option<&bd::StepEventsSnapshot>) {
    let state = &mut app.overlap_queries;
    let filter = bd::QueryFilter::default();
    let Ok(aabb) = bd::Aabb::from_center_half_extents(
        [state.center_x, state.center_y],
        [state.half_x, state.half_y],
    ) else {
        state.reset_runtime();
        return;
    };
    let Ok(query) = app.world.query() else {
        state.reset_runtime();
        return;
    };

    let owned_hits = query
        .overlap_aabb(bd::Position::ZERO, aabb, filter)
        .unwrap_or_default();
    state.owned_hits = owned_hits.len();

    state.reused_hit_buffer.clear();
    if query
        .overlap_aabb_into(
            bd::Position::ZERO,
            aabb,
            filter,
            &mut state.reused_hit_buffer,
        )
        .is_ok()
    {
        state.reused_hits = state.reused_hit_buffer.len();
    } else {
        state.reused_hits = 0;
    }

    let mut visited_hits = 0usize;
    state.visit_hits = query
        .visit_overlap_aabb(bd::Position::ZERO, aabb, filter, |_| {
            visited_hits += 1;
            true
        })
        .map(|_| visited_hits)
        .unwrap_or_default();

    let mut stopped_early = false;
    let completed = query
        .visit_overlap_aabb(bd::Position::ZERO, aabb, filter, |_| {
            stopped_early = true;
            false
        })
        .unwrap_or(false);
    state.visit_stopped_early = stopped_early && !completed;

    state.polygon_hits = bd::Transform::from_pos_angle([state.center_x, state.center_y], 0.0)
        .and_then(|transform| {
            bd::ShapeProxy::offset_from_points(
                rect_points(state.half_x * 0.55, state.half_y * 0.55),
                0.01,
                transform,
            )
        })
        .and_then(|proxy| query.overlap_shape(bd::Position::ZERO, proxy, filter))
        .unwrap_or_default()
        .len();
}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let state = &mut app.overlap_queries;
    let mut cx = state.center_x;
    let mut cy = state.center_y;
    let mut hx = state.half_x;
    let mut hy = state.half_y;
    let changed = ui.slider("Center X", -8.0, 8.0, &mut cx)
        || ui.slider("Center Y", -1.0, 8.0, &mut cy)
        || ui.slider("Half X", 0.2, 4.0, &mut hx)
        || ui.slider("Half Y", 0.2, 3.0, &mut hy);
    if changed {
        state.center_x = cx;
        state.center_y = cy;
        state.half_x = hx.max(0.2);
        state.half_y = hy.max(0.2);
    }
    ui.text(format!(
        "AABB overlap: owned={} reused={} visited={}",
        state.owned_hits, state.reused_hits, state.visit_hits
    ));
    ui.text(format!(
        "Offset polygon overlap: {} early_exit={}",
        state.polygon_hits, state.visit_stopped_early
    ));
}
