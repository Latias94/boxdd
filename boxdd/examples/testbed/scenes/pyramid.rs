use boxdd as bd;

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let columns = app.pyramid_cols.max(1) as usize;
    let rows = app.pyramid_rows.max(1) as usize;
    let box_poly = bd::shapes::box_polygon(0.5, 0.5);
    let sdef = bd::ShapeDef::builder().density(1.0).build();
    for i in 0..rows {
        // Use saturating_sub to avoid usize underflow if rows > columns
        let width = columns.saturating_sub(i);
        for j in 0..width {
            let x = (j as f32) * 1.1 - (width as f32) * 0.55;
            let y = 0.5 + (i as f32) * 1.05 + 2.0;
            let b = app.world.create_body_id(
                bd::BodyBuilder::new()
                    .body_type(bd::BodyType::Dynamic)
                    .position([x, y])
                    .build(),
            );
            app.created_bodies += 1;
            let _ = app.world.create_polygon_shape_for(b, &sdef, &box_poly);
            app.created_shapes += 1;
        }
    }
}

use dear_imgui_rs as imgui;
pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut r = app.pyramid_rows;
    let mut c = app.pyramid_cols;
    if ui.slider("Rows", 1, 30, &mut r) {
        app.pyramid_rows = r;
        let _ = app.reset();
    }
    if ui.slider("Cols", 1, 30, &mut c) {
        app.pyramid_cols = c;
        let _ = app.reset();
    }
}
