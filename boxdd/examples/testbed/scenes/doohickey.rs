use boxdd as bd;
use dear_imgui_rs as imgui;

// Doohickey: two wheels, two bars, revolute motors and a prismatic slider

pub fn build(app: &mut super::PhysicsApp, _ground: bd::types::BodyId) {
    let scale = 1.0_f32;

    // Common builders
    let bdef_dyn = bd::BodyBuilder::new().body_type(bd::BodyType::Dynamic);
    let mat = bd::shapes::SurfaceMaterial::default().rolling_resistance(0.1);
    let sdef_rr = bd::ShapeDef::builder().material(mat).build();

    let circle = bd::shapes::circle([0.0_f32, 0.0], 1.0 * scale);
    let bar = bd::shapes::capsule([-3.5 * scale, 0.0], [3.5 * scale, 0.0], 0.15 * scale);

    // Body positions
    let p_w1 = [-5.0 * scale, 3.0 * scale];
    let p_w2 = [5.0 * scale, 3.0 * scale];
    let p_b1 = [-1.5 * scale, 3.0 * scale];
    let p_b2 = [1.5 * scale, 3.0 * scale];

    // Wheels
    let w1 = app.world.create_body_id(bdef_dyn.clone().position(p_w1).build());
    app.world.create_circle_shape_for(w1, &sdef_rr, &circle);
    app.created_bodies += 1;
    app.created_shapes += 1;
    let w2 = app.world.create_body_id(bdef_dyn.clone().position(p_w2).build());
    app.world.create_circle_shape_for(w2, &sdef_rr, &circle);
    app.created_bodies += 1;
    app.created_shapes += 1;

    // Bars
    let b1 = app.world.create_body_id(bdef_dyn.clone().position(p_b1).build());
    app.world.create_capsule_shape_for(b1, &sdef_rr, &bar);
    app.created_bodies += 1;
    app.created_shapes += 1;
    let b2 = app.world.create_body_id(bdef_dyn.clone().position(p_b2).build());
    app.world.create_capsule_shape_for(b2, &sdef_rr, &bar);
    app.created_bodies += 1;
    app.created_shapes += 1;

    // Revolute joints at wheel centers (motors enabled)
    {
        let base = app
            .world
            .joint_base_from_world_points(w1, b1, p_w1, p_w1);
        let rdef = bd::RevoluteJointDef::new(base)
            .enable_motor(true)
            .max_motor_torque(app.revolute_torque)
            .motor_speed(app.revolute_speed);
        let _ = app.world.create_revolute_joint_id(&rdef);
        app.created_joints += 1;
    }
    {
        let base = app
            .world
            .joint_base_from_world_points(w2, b2, p_w2, p_w2);
        let rdef = bd::RevoluteJointDef::new(base)
            .enable_motor(true)
            .max_motor_torque(app.revolute_torque)
            .motor_speed(app.revolute_speed);
        let _ = app.world.create_revolute_joint_id(&rdef);
        app.created_joints += 1;
    }

    // Prismatic slider between bars along X axis
    let anchor_a = [p_b1[0] + 2.0 * scale, p_b1[1]];
    let anchor_b = [p_b2[0] - 2.0 * scale, p_b2[1]];
    let axis = [1.0_f32, 0.0];
    let base = app
        .world
        .joint_base_from_world_with_axis(b1, b2, anchor_a, anchor_b, axis);
    let pdef = bd::PrismaticJointDef::new(base)
        .enable_limit(true)
        .lower_translation(app.prism_lower)
        .upper_translation(app.prism_upper)
        .enable_motor(true)
        .max_motor_force(app.prism_force)
        .motor_speed(app.prism_speed)
        .enable_spring(true)
        .hertz(1.0)
        .damping_ratio(0.5);
    let _ = app.world.create_prismatic_joint_id(&pdef);
    app.created_joints += 1;
}

#[allow(dead_code)]
pub fn tick(_app: &mut super::PhysicsApp) {}

pub fn ui_params(app: &mut super::PhysicsApp, ui: &imgui::Ui) {
    ui.text("Doohickey: wheels + bars with motors and prismatic slider");
    // Revolute motors
    let mut rs = app.revolute_speed;
    let mut rt = app.revolute_torque;
    let mut pl = app.prism_lower;
    let mut pu = app.prism_upper;
    let mut ps = app.prism_speed;
    let mut pf = app.prism_force;
    let changed = ui.slider("Rev Motor Speed", -20.0, 20.0, &mut rs)
        || ui.slider("Rev Max Torque", 0.0, 500.0, &mut rt)
        || ui.slider("Prismatic Lower", -5.0, 0.0, &mut pl)
        || ui.slider("Prismatic Upper", 0.0, 5.0, &mut pu)
        || ui.slider("Prismatic Motor Speed", -20.0, 20.0, &mut ps)
        || ui.slider("Prismatic Max Force", 0.0, 500.0, &mut pf);
    if changed {
        app.revolute_speed = rs;
        app.revolute_torque = rt.max(0.0);
        if pl > pu { std::mem::swap(&mut pl, &mut pu); }
        app.prism_lower = pl;
        app.prism_upper = pu;
        app.prism_speed = ps;
        app.prism_force = pf.max(0.0);
        let _ = app.reset();
    }
}
