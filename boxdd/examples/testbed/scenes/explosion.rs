use boxdd as bd;
use bd::world_extras::{ExplosionDef, WorldExplosionExt};
use dear_imgui as imgui;

pub fn build(_app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {}

pub fn tick(_app: &mut super::PhysicsApp) {}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    let mut px = app.ex_center_x;
    let mut py = app.ex_center_y;
    let mut r = app.ex_radius;
    let mut f = app.ex_falloff;
    let mut imp = app.ex_impulse;
    let changed = ui.slider("Center X", -50.0, 50.0, &mut px)
        || ui.slider("Center Y", -10.0, 50.0, &mut py)
        || ui.slider("Radius", 0.0, 20.0, &mut r)
        || ui.slider("Falloff", 0.0, 20.0, &mut f)
        || ui.slider("Impulse/Len", -20.0, 20.0, &mut imp);
    if changed {
        app.ex_center_x = px;
        app.ex_center_y = py;
        app.ex_radius = r.max(0.0);
        app.ex_falloff = f.max(0.0);
        app.ex_impulse = imp;
    }
    if ui.button("Explode") {
        let def = ExplosionDef::new()
            .position([app.ex_center_x, app.ex_center_y])
            .radius(app.ex_radius)
            .falloff(app.ex_falloff)
            .impulse_per_length(app.ex_impulse);
        app.world.explode(&def);
    }
    ui.text("Applies Box2D world explosion to nearby shapes.");
}
