// Central physics app + scene routing for the ImGui testbed
use boxdd as bd;
use boxdd_sys::ffi;
use dear_imgui as imgui;

// Re-export per-scene modules for callers
pub mod shapes {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/shapes.rs"));
}
pub mod events {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/events.rs"));
}
pub mod robustness {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/robustness.rs"));
}
pub mod benchmark {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/benchmark.rs"));
}
pub mod determinism {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/determinism.rs"));
}
pub mod queries_casts {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/queries_casts.rs"));
}
pub mod character_mover {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/character_mover.rs"));
}
// Unified labs and tools
pub mod shape_distance {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/shape_distance.rs"));
}
pub mod joint_separation {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/joint_separation.rs"));
}
pub mod pyramid {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/pyramid.rs"));
}
pub mod stacking {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/stacking.rs"));
}
pub mod bridge {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/bridge.rs"));
}
pub mod car {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/car.rs"));
}
pub mod chain_walkway {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/chain_walkway.rs"));
}
pub mod sensors {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/sensors.rs"));
}
pub mod contacts {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/contacts.rs"));
}
// Continuous lab combines bullet/ghost/restitution/pinball/segment slide
pub mod continuous_lab {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/continuous_lab.rs"));
}
pub mod joints_lab {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/joints_lab.rs"));
}
pub mod soft_body {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/soft_body.rs"));
}
pub mod convex_hull {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/convex_hull.rs"));
}
// Bodies lab combines set velocity / kinematic / wake touching
pub mod bodies_lab {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/bodies_lab.rs"));
}
pub mod manifold {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/manifold.rs"));
}
// World lab combines tuning + explosion
pub mod world_lab {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/world_lab.rs"));
}
pub mod motion_locks {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/motion_locks.rs"));
}
pub mod breakable_joint {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/breakable_joint.rs"));
}
// world_tuning module replaced by world_lab routing
pub mod materials {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/materials.rs"));
}
pub mod shape_editing {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/shape_editing.rs"));
}
pub mod collision_tools {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/collision_tools.rs"));
}
// Extra samples ported from top-level examples
pub mod doohickey {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/doohickey.rs"));
}
pub mod issues {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/testbed/scenes/issues.rs"));
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Scene {
    Pyramid,
    Bridge,
    Car,
    ChainWalkway,
    Sensors,
    Contacts,
    ContinuousLab,
    Stacking,
    Shapes,
    Robustness,
    Events,
    Benchmark,
    Determinism,
    QueriesCasts,
    CharacterMover,
    JointsLab,
    SoftBody,
    ConvexHull,
    BodiesLab,
    ShapeDistance,
    JointSeparation,
    Manifold,
    MotionLocks,
    BreakableJoint,
    WorldTuning,
    Materials,
    ShapeEditing,
    CollisionTools,
    Doohickey,
    Issues,
}

pub struct PhysicsApp {
    pub world: bd::World,
    pub scene: Scene,
    pub gravity_y: f32,
    pub sub_steps: i32,
    pub running: bool,
    pub pixels_per_meter: f32,
    // Time scaling for stepping (1.0 = real-time at base dt)
    pub time_scale: f32,
    // Debug draw options (mirrors DebugDrawOptions)
    pub dd_draw_shapes: bool,
    pub dd_draw_joints: bool,
    pub dd_draw_joint_extras: bool,
    pub dd_draw_bounds: bool,
    pub dd_draw_mass: bool,
    pub dd_draw_body_names: bool,
    pub dd_draw_contacts: bool,
    pub dd_draw_graph_colors: bool,
    pub dd_draw_contact_features: bool,
    pub dd_draw_contact_normals: bool,
    pub dd_draw_contact_forces: bool,
    pub dd_draw_friction_forces: bool,
    pub dd_draw_islands: bool,
    pub dd_force_scale: f32,
    pub dd_joint_scale: f32,
    // Collision tools unified
    pub ct_mode: i32,
    // Stats
    pub created_bodies: usize,
    pub created_shapes: usize,
    pub created_joints: usize,
    // Step stats
    pub step_ms: f32,
    pub cnt_bodies: i32,
    pub cnt_shapes: i32,
    pub cnt_joints: i32,
    pub cnt_contacts: i32,
    pub cnt_islands: i32,
    pub cnt_awake: i32,
    // Scene params
    pub pyramid_rows: i32,
    pub pyramid_cols: i32,
    pub bridge_planks: i32,
    pub car_motor_speed: f32,
    pub car_motor_torque: f32,
    pub car_hz: f32,
    pub car_dr: f32,
    pub revolute_lower_deg: f32,
    pub revolute_upper_deg: f32,
    pub revolute_speed: f32,
    pub revolute_torque: f32,
    pub prism_lower: f32,
    pub prism_upper: f32,
    pub prism_speed: f32,
    pub prism_force: f32,
    pub chain_boxes: i32,
    pub chain_amp: f32,
    pub chain_freq: f32,
    pub chain_mode: i32,
    // Sensors
    pub sensor_band_y: f32,
    pub sensor_half_thickness: f32,
    pub sensor_mover_start_y: f32,
    pub sensor_radius: f32,
    // Issues scene
    pub issues_visitors: i32,
    // Contacts
    pub contact_box_half: f32,
    pub contact_speed: f32,
    pub contact_gap: f32,
    // Bullet
    pub bullet_dt: f32,
    pub bullet_substeps: i32,
    pub bullet_speed: f32,
    pub bullet_radius: f32,
    pub bullet_threshold: f32,
    // Events scene
    pub events_threshold: f32,
    pub ev_moves: usize,
    pub ev_sens_beg: usize,
    pub ev_sens_end: usize,
    pub ev_con_beg: usize,
    pub ev_con_end: usize,
    pub ev_con_hit: usize,
    pub ev_joint: usize,
    // Robustness
    pub robust_bullet_speed: f32,
    pub robust_hit_count: usize,
    // Benchmark
    pub bench_bodies: i32,
    // Queries & casts quick stats
    pub q_ray_origin_y: f32,
    pub q_ray_length: f32,
    pub q_overlaps: usize,
    pub q_ray_hits: usize,
    // Character mover
    pub cm_c1_y: f32,
    pub cm_c2_y: f32,
    pub cm_radius: f32,
    pub cm_move_x: f32,
    pub cm_fraction: f32,
    // Distance joint scene
    pub dist_length: f32,
    pub dist_hz: f32,
    pub dist_dr: f32,
    // Motor joint scene
    pub motor_lin_x: f32,
    pub motor_lin_y: f32,
    pub motor_ang_w: f32,
    pub motor_max_force: f32,
    pub motor_max_torque: f32,
    pub motor_lin_hz: f32,
    pub motor_lin_dr: f32,
    pub motor_ang_hz: f32,
    pub motor_ang_dr: f32,
    // Wheel joint scene
    pub wheel_enable_limit: bool,
    pub wheel_lower: f32,
    pub wheel_upper: f32,
    pub wheel_enable_motor: bool,
    pub wheel_motor_speed_deg: f32,
    pub wheel_motor_torque: f32,
    pub wheel_enable_spring: bool,
    pub wheel_hz: f32,
    pub wheel_dr: f32,
    // Soft body (donut)
    pub soft_scale: f32,
    // Convex hull
    pub hull_points: i32,
    // Collision: shape cast
    pub sc_pos_y: f32,
    pub sc_angle: f32,
    pub sc_tx: f32,
    pub sc_ty: f32,
    pub sc_radius: f32,
    pub sc_hits: usize,
    pub sc_min_fraction: f32,
    // Collision: TOI (shape cast proxy)
    pub toi_start_x: f32,
    pub toi_start_y: f32,
    pub toi_angle: f32,
    pub toi_dx: f32,
    pub toi_dy: f32,
    pub toi_radius: f32,
    pub toi_hits: usize,
    pub toi_min_fraction: f32,
    // Continuous: ghost bumps
    pub gb_speed: f32,
    pub gb_bump_h: f32,
    pub gb_hits: usize,
    // Continuous: restitution threshold
    pub cr_threshold: f32,
    pub cr_restitution: f32,
    pub cr_drop_y: f32,
    // Collision: ray world
    pub rw_origin_x: f32,
    pub rw_origin_y: f32,
    pub rw_dx: f32,
    pub rw_dy: f32,
    pub rw_hits: usize,
    // Collision: overlap world
    pub ow_center_x: f32,
    pub ow_center_y: f32,
    pub ow_half_x: f32,
    pub ow_half_y: f32,
    pub ow_hits: usize,
    // World: explosion
    pub ex_center_x: f32,
    pub ex_center_y: f32,
    pub ex_radius: f32,
    pub ex_falloff: f32,
    pub ex_impulse: f32,
    // Joints: filter
    pub fj_disable_collide: bool,
    pub fj_hits: usize,
    // Bodies: set velocity
    pub bsv_vx: f32,
    pub bsv_vy: f32,
    pub bsv_body: Option<bd::types::BodyId>,
    // Bodies: kinematic
    pub bk_speed: f32,
    pub bk_platform: Option<bd::types::BodyId>,
    // Collision: shape distance (rect-rect)
    pub sd_a_x: f32,
    pub sd_a_y: f32,
    pub sd_a_angle: f32,
    pub sd_a_hx: f32,
    pub sd_a_hy: f32,
    pub sd_a_radius: f32,
    pub sd_b_x: f32,
    pub sd_b_y: f32,
    pub sd_b_angle: f32,
    pub sd_b_hx: f32,
    pub sd_b_hy: f32,
    pub sd_b_radius: f32,
    pub sd_distance: f32,
    pub sd_point_ax: f32,
    pub sd_point_ay: f32,
    pub sd_point_bx: f32,
    pub sd_point_by: f32,
    // Joints: separation
    pub js_count: i32,
    pub js_joint_ids: Vec<bd::types::JointId>,
    pub js_min_lin: f32,
    pub js_max_lin: f32,
    pub js_min_ang: f32,
    pub js_max_ang: f32,
    // Manifold (simple viewer)
    pub mf_a_x: f32,
    pub mf_a_y: f32,
    pub mf_a_angle: f32,
    pub mf_a_hx: f32,
    pub mf_a_hy: f32,
    pub mf_b_x: f32,
    pub mf_b_y: f32,
    pub mf_b_angle: f32,
    pub mf_b_hx: f32,
    pub mf_b_hy: f32,
    pub mf_contacts: usize,
    pub mf_normal_x: f32,
    pub mf_normal_y: f32,
    pub mf_point_x: f32,
    pub mf_point_y: f32,
    pub mf_point2_x: f32,
    pub mf_point2_y: f32,
    pub mf_mode: i32, // 0=Poly-Poly, 1=Circle-Poly, 2=Capsule-Poly
    pub mf_b_radius: f32,
    pub mf_show_metrics: bool,
    pub mf_sep1: f32,
    pub mf_sep2: f32,
    pub mf_impulse1: f32,
    pub mf_impulse2: f32,
    pub mf_total_impulse1: f32,
    pub mf_total_impulse2: f32,
    pub mf_seg_half: f32,
    // Pinball
    pub pb_restitution: f32,
    pub pb_ball_radius: f32,
    pub pb_ball_count: usize,
    pub pb_flippers: bool,
    pub pb_left_flipper: Option<bd::types::BodyId>,
    pub pb_right_flipper: Option<bd::types::BodyId>,
    pub pb_flipper_torque: f32,
    pub pb_flip_impulse: f32,
    pub pb_left_joint: Option<bd::types::JointId>,
    pub pb_right_joint: Option<bd::types::JointId>,
    pub pb_hold_left: bool,
    pub pb_hold_right: bool,
    pub pb_flip_speed_deg: f32,
    pub pb_left_lower_deg: f32,
    pub pb_left_upper_deg: f32,
    pub pb_right_lower_deg: f32,
    pub pb_right_upper_deg: f32,
    // Segment Slide
    pub ss_slope_deg: f32,
    pub ss_speed: f32,
    // Motion Locks
    pub ml_lock_x: bool,
    pub ml_lock_y: bool,
    pub ml_lock_rot: bool,
    pub ml_body: Option<bd::types::BodyId>,
    // Breakable joint
    pub bj_force_thres: f32,
    pub bj_torque_thres: f32,
    pub bj_broken: usize,
    // Wake touching
    pub wt_wakes: usize,
    pub wt_ground_body: Option<bd::types::BodyId>,
    // Weld joint
    pub wj_hz: f32,
    pub wj_dr: f32,
    // World tuning
    pub wt_sleeping: bool,
    pub wt_continuous: bool,
    pub wt_softening: bool,
    pub wt_restitution_thres: f32,
    pub wt_hit_thres: f32,
    pub wt_warm_starting: bool,
    pub wt_max_linear_speed: f32,
    pub wt_contact_speed: f32,
    // Materials
    pub mat_conv_speed: f32,
    pub mat_roll_res: f32,
    pub mat_spawned: usize,
    pub mat_wind_x: f32,
    pub mat_wind_y: f32,
    pub mat_drag: f32,
    pub mat_lift: f32,
    pub mat_wake: bool,
    pub mat_shapes: Vec<bd::types::ShapeId>,
    pub mat_belt_left: Option<bd::types::ShapeId>,
    pub mat_belt_right: Option<bd::types::ShapeId>,
    pub mat_belt_friction: f32,
    pub mat_belt_restitution: f32,
    pub mat_shape_friction: f32,
    pub mat_shape_restitution: f32,
    pub mat_belt_left_body: Option<bd::types::BodyId>,
    pub mat_belt_right_body: Option<bd::types::BodyId>,
    pub mat_belt_half_len: f32,
    pub mat_belt_thickness: f32,
    pub mat_belt_left_x: f32,
    pub mat_belt_left_y: f32,
    pub mat_belt_right_x: f32,
    pub mat_belt_right_y: f32,
    pub mat_belt_left_angle_deg: f32,
    pub mat_belt_right_angle_deg: f32,
    // Shape editing
    pub se_body: Option<bd::types::BodyId>,
    pub se_shape: Option<bd::types::ShapeId>,
    pub se_mode: i32,
    pub se_hx: f32,
    pub se_hy: f32,
    pub se_radius: f32,
    // Joints Lab
    pub jl_mode: usize,
    // Continuous Lab
    pub cl_mode: usize,
    // Bodies Lab
    pub bl_mode: usize,
    // World Lab
    pub wl_mode: usize,
}

impl PhysicsApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let gravity_y = -9.8;
        let scene = Scene::Pyramid;
        let mut app = Self {
            world: bd::World::new(bd::WorldDef::builder().gravity([0.0, gravity_y]).build())?,
            scene,
            gravity_y,
            sub_steps: 4,
            running: true,
            created_bodies: 0,
            created_shapes: 0,
            created_joints: 0,
            step_ms: 0.0,
            cnt_bodies: 0,
            cnt_shapes: 0,
            cnt_joints: 0,
            cnt_contacts: 0,
            cnt_islands: 0,
            cnt_awake: 0,
            pixels_per_meter: 30.0,
            time_scale: 1.0,
            dd_draw_shapes: true,
            dd_draw_joints: true,
            dd_draw_joint_extras: false,
            dd_draw_bounds: false,
            dd_draw_mass: false,
            dd_draw_body_names: false,
            dd_draw_contacts: false,
            dd_draw_graph_colors: false,
            dd_draw_contact_features: false,
            dd_draw_contact_normals: false,
            dd_draw_contact_forces: false,
            dd_draw_friction_forces: false,
            dd_draw_islands: false,
            dd_force_scale: 1.0,
            dd_joint_scale: 1.0,
            ct_mode: 0,
            pyramid_rows: 10,
            pyramid_cols: 10,
            bridge_planks: 20,
            car_motor_speed: 15.0,
            car_motor_torque: 40.0,
            car_hz: 4.0,
            car_dr: 0.7,
            revolute_lower_deg: -45.0,
            revolute_upper_deg: 45.0,
            revolute_speed: 2.0,
            revolute_torque: 50.0,
            prism_lower: 0.0,
            prism_upper: 4.0,
            prism_speed: 2.0,
            prism_force: 100.0,
            chain_boxes: 10,
            chain_amp: 0.4,
            chain_freq: 0.6,
            chain_mode: 0,
            sensor_band_y: 1.5,
            sensor_half_thickness: 0.3,
            sensor_mover_start_y: 3.0,
            sensor_radius: 0.25,
            issues_visitors: 10,
            contact_box_half: 0.5,
            contact_speed: 2.0,
            contact_gap: 1.5,
            bullet_dt: 1.0 / 240.0,
            bullet_substeps: 16,
            bullet_speed: 60.0,
            bullet_radius: 0.3,
            bullet_threshold: 0.001,
            events_threshold: 0.2,
            ev_moves: 0,
            ev_sens_beg: 0,
            ev_sens_end: 0,
            ev_con_beg: 0,
            ev_con_end: 0,
            ev_con_hit: 0,
            ev_joint: 0,
            robust_bullet_speed: 80.0,
            robust_hit_count: 0,
            bench_bodies: 200,
            q_ray_origin_y: 10.0,
            q_ray_length: 100.0,
            q_overlaps: 0,
            q_ray_hits: 0,
            cm_c1_y: 1.0,
            cm_c2_y: 1.8,
            cm_radius: 0.25,
            cm_move_x: 2.0,
            cm_fraction: 1.0,
            dist_length: 4.0,
            dist_hz: 5.0,
            dist_dr: 0.7,
            motor_lin_x: 0.0,
            motor_lin_y: 0.0,
            motor_ang_w: 0.0,
            motor_max_force: 200.0,
            motor_max_torque: 200.0,
            motor_lin_hz: 0.0,
            motor_lin_dr: 0.0,
            motor_ang_hz: 0.0,
            motor_ang_dr: 0.0,
            wheel_enable_limit: true,
            wheel_lower: -0.5,
            wheel_upper: 0.5,
            wheel_enable_motor: true,
            wheel_motor_speed_deg: 120.0,
            wheel_motor_torque: 100.0,
            wheel_enable_spring: true,
            wheel_hz: 6.0,
            wheel_dr: 0.7,
            soft_scale: 1.0,
            hull_points: 16,
            sc_pos_y: 6.0,
            sc_angle: 0.0,
            sc_tx: 0.0,
            sc_ty: -6.0,
            sc_radius: 0.02,
            sc_hits: 0,
            sc_min_fraction: 1.0,
            toi_start_x: -2.0,
            toi_start_y: 6.0,
            toi_angle: 0.0,
            toi_dx: 4.0,
            toi_dy: -4.5,
            toi_radius: 0.02,
            toi_hits: 0,
            toi_min_fraction: 1.0,
            gb_speed: 40.0,
            gb_bump_h: 0.2,
            gb_hits: 0,
            cr_threshold: 1.0,
            cr_restitution: 0.7,
            cr_drop_y: 8.0,
            rw_origin_x: -10.0,
            rw_origin_y: 10.0,
            rw_dx: 25.0,
            rw_dy: -12.0,
            rw_hits: 0,
            ow_center_x: 0.0,
            ow_center_y: 2.0,
            ow_half_x: 2.0,
            ow_half_y: 1.0,
            ow_hits: 0,
            ex_center_x: 0.0,
            ex_center_y: 3.0,
            ex_radius: 3.0,
            ex_falloff: 0.1,
            ex_impulse: 2.0,
            fj_disable_collide: true,
            fj_hits: 0,
            bsv_vx: 0.0,
            bsv_vy: -20.0,
            bsv_body: None,
            bk_speed: 2.0,
            bk_platform: None,
            sd_a_x: -2.0,
            sd_a_y: 2.0,
            sd_a_angle: 0.0,
            sd_a_hx: 0.6,
            sd_a_hy: 0.4,
            sd_a_radius: 0.0,
            sd_b_x: 2.0,
            sd_b_y: 2.0,
            sd_b_angle: 0.0,
            sd_b_hx: 0.6,
            sd_b_hy: 0.4,
            sd_b_radius: 0.0,
            sd_distance: 0.0,
            sd_point_ax: 0.0,
            sd_point_ay: 0.0,
            sd_point_bx: 0.0,
            sd_point_by: 0.0,
            js_count: 4,
            js_joint_ids: Vec::new(),
            js_min_lin: 0.0,
            js_max_lin: 0.0,
            js_min_ang: 0.0,
            js_max_ang: 0.0,
            mf_a_x: -0.8,
            mf_a_y: 3.0,
            mf_a_angle: 0.0,
            mf_a_hx: 0.7,
            mf_a_hy: 0.5,
            mf_b_x: 0.8,
            mf_b_y: 3.2,
            mf_b_angle: 0.2,
            mf_b_hx: 0.6,
            mf_b_hy: 0.4,
            mf_contacts: 0,
            mf_normal_x: 0.0,
            mf_normal_y: 0.0,
            mf_point_x: 0.0,
            mf_point_y: 0.0,
            mf_point2_x: 0.0,
            mf_point2_y: 0.0,
            mf_mode: 0,
            mf_b_radius: 0.5,
            mf_show_metrics: false,
            mf_sep1: 0.0,
            mf_sep2: 0.0,
            mf_impulse1: 0.0,
            mf_impulse2: 0.0,
            mf_total_impulse1: 0.0,
            mf_total_impulse2: 0.0,
            mf_seg_half: 1.0,
            pb_restitution: 0.8,
            pb_ball_radius: 0.25,
            pb_ball_count: 0,
            pb_flippers: false,
            pb_left_flipper: None,
            pb_right_flipper: None,
            pb_flipper_torque: 60.0,
            pb_flip_impulse: 2.0,
            pb_left_joint: None,
            pb_right_joint: None,
            pb_hold_left: false,
            pb_hold_right: false,
            pb_flip_speed_deg: 360.0,
            pb_left_lower_deg: -57.0,
            pb_left_upper_deg: 23.0,
            pb_right_lower_deg: -23.0,
            pb_right_upper_deg: 57.0,
            ss_slope_deg: 20.0,
            ss_speed: 25.0,
            ml_lock_x: false,
            ml_lock_y: false,
            ml_lock_rot: false,
            ml_body: None,
            bj_force_thres: 50.0,
            bj_torque_thres: 0.0,
            bj_broken: 0,
            wt_wakes: 0,
            wt_ground_body: None,
            wj_hz: 8.0,
            wj_dr: 0.7,
            wt_sleeping: true,
            wt_continuous: false,
            wt_softening: false,
            wt_restitution_thres: 1.0,
            wt_hit_thres: 0.0,
            wt_warm_starting: true,
            wt_max_linear_speed: 100.0,
            wt_contact_speed: 2.0,
            mat_conv_speed: 1.5,
            mat_roll_res: 0.5,
            mat_spawned: 0,
            mat_wind_x: 2.0,
            mat_wind_y: 0.0,
            mat_drag: 1.0,
            mat_lift: 0.0,
            mat_wake: false,
            mat_shapes: Vec::new(),
            mat_belt_left: None,
            mat_belt_right: None,
            mat_belt_friction: 0.9,
            mat_belt_restitution: 0.0,
            mat_shape_friction: 0.6,
            mat_shape_restitution: 0.2,
            mat_belt_left_body: None,
            mat_belt_right_body: None,
            mat_belt_half_len: 4.5,
            mat_belt_thickness: 0.2,
            mat_belt_left_x: -3.0,
            mat_belt_left_y: 0.5,
            mat_belt_right_x: 3.0,
            mat_belt_right_y: 2.5,
            mat_belt_left_angle_deg: 0.0,
            mat_belt_right_angle_deg: 0.0,
            se_body: None,
            se_shape: None,
            se_mode: 0,
            se_hx: 0.8,
            se_hy: 0.5,
            se_radius: 0.2,
            jl_mode: 0,
            cl_mode: 0,
            bl_mode: 0,
            wl_mode: 0,
        };
        app.build_scene();
        Ok(app)
    }

    pub fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut b = bd::WorldDef::builder().gravity([0.0, self.gravity_y]);
        match self.scene {
            Scene::Events => {
                b = b.enable_continuous(true).hit_event_threshold(self.events_threshold);
            }
            Scene::Determinism => {
                b = b.worker_count(1).enable_continuous(true);
            }
            Scene::WorldTuning => {
                b = b
                    .enable_sleep(self.wt_sleeping)
                    .enable_continuous(self.wt_continuous)
                    .enable_contact_softening(self.wt_softening)
                    .maximum_linear_speed(self.wt_max_linear_speed)
                    .contact_speed(self.wt_contact_speed)
                    .restitution_threshold(self.wt_restitution_thres)
                    .hit_event_threshold(self.wt_hit_thres);
            }
            _ => {}
        }
        self.world = bd::World::new(b.build())?;
        if matches!(self.scene, Scene::WorldTuning) {
            self.world.enable_warm_starting(self.wt_warm_starting);
        }
        self.ev_moves = 0;
        self.ev_sens_beg = 0;
        self.ev_sens_end = 0;
        self.ev_con_beg = 0;
        self.ev_con_end = 0;
        self.ev_con_hit = 0;
        self.ev_joint = 0;
        self.robust_hit_count = 0;
        self.q_overlaps = 0;
        self.q_ray_hits = 0;
        self.cm_fraction = 1.0;
        self.sc_hits = 0;
        self.sc_min_fraction = 1.0;
        self.toi_hits = 0;
        self.toi_min_fraction = 1.0;
        self.gb_hits = 0;
        // Reset new scene stats
        self.mf_contacts = 0;
        self.mf_normal_x = 0.0;
        self.mf_normal_y = 0.0;
        self.mf_point_x = 0.0;
        self.mf_point_y = 0.0;
        self.pb_ball_count = 0;
        self.ml_body = None;
        self.bj_broken = 0;
        self.wt_wakes = 0;
        self.wt_ground_body = None;
        self.mf_point2_x = 0.0;
        self.mf_point2_y = 0.0;
        self.mat_spawned = 0;
        self.mat_shapes.clear();
        self.build_scene();
        Ok(())
    }

    pub fn update(&mut self) {
        if self.running {
            let t0 = std::time::Instant::now();
            let (base_dt, sub) = match self.scene {
                Scene::ContinuousLab if self.cl_mode == 0 => (self.bullet_dt, self.bullet_substeps),
                _ => (1.0 / 60.0, self.sub_steps),
            };
            let dt = (base_dt * self.time_scale.max(0.0)).max(0.0);
            self.world.step(dt, sub);
            self.step_ms = t0.elapsed().as_secs_f32() * 1000.0;
            // World counters
            unsafe {
                let c = ffi::b2World_GetCounters(self.world.raw());
                self.cnt_bodies = c.bodyCount;
                self.cnt_shapes = c.shapeCount;
                self.cnt_contacts = c.contactCount;
                self.cnt_joints = c.jointCount;
                self.cnt_islands = c.islandCount;
                self.cnt_awake = ffi::b2World_GetAwakeBodyCount(self.world.raw());
            }
        }
        match self.scene {
            Scene::Events => events::tick(self),
            Scene::Robustness => robustness::tick(self),
            Scene::QueriesCasts => queries_casts::tick(self),
            Scene::CharacterMover => character_mover::tick(self),
            Scene::Shapes => shapes::tick(self),
            Scene::Benchmark => benchmark::tick(self),
            Scene::Determinism => determinism::tick(self),
            Scene::ContinuousLab => continuous_lab::tick(self),
            Scene::Manifold => manifold::tick(self),
            Scene::BreakableJoint => breakable_joint::tick(self),
            Scene::Materials => materials::tick(self),
            Scene::JointsLab => joints_lab::tick(self),
            Scene::Issues => issues::tick(self),
            _ => {}
        }
    }

    pub fn ui(&mut self, ui: &imgui::Ui) {
        ui.window("BoxDD Testbed").build(|| {
            if ui.button(if self.running { "Pause" } else { "Play" }) {
                self.running = !self.running;
            }
            ui.same_line();
            if ui.button("Step") {
                self.step_once();
            }
            ui.same_line();
            if ui.button("Reset") {
                let _ = self.reset();
            }
            ui.separator();

            // Global tunables
            ui.text("Global");
            let mut ppm = self.pixels_per_meter;
            if ui.slider("Pixels / Meter", 5.0, 120.0, &mut ppm) {
                self.pixels_per_meter = ppm.max(1.0);
            }
            let mut ts = self.time_scale;
            if ui.slider("Time Scale", 0.1, 2.0, &mut ts) { self.time_scale = ts; }
            ui.separator();

            let names = [
                "Pyramid",
                "Bridge",
                "Car",
                "Chain Walkway",
                "Sensors",
                "Contacts",
                "Continuous: Lab",
                "Stacking",
                "Shapes Variety",
                "Robustness",
                "Events Summary",
                "Benchmark",
                "Determinism",
                "Queries & Casts",
                "Character Mover",
                "Joints: Lab (Unified)",
                "Soft Body (Donut)",
                "Convex Hull",
                "Bodies: Lab",
                "Collision: Shape Distance",
                "Joints: Separation",
                "Collision: Manifold (basic)",
                "Collision Tools (Ray/Overlap/Cast/TOI)",
                "World: Lab",
                "Joints: Motion Locks",
                "Joints: Breakable",
                "Materials: Conveyor & Rolling",
                "Shape Editing",
                "Doohickey",
                "Issues",
            ];
            let mut idx = self.scene_index();
            if let Some(_c) = ui.begin_combo("Scene", names[idx]) {
                for (i, &name) in names.iter().enumerate() {
                    let selected = i == idx;
                    if ui.selectable_config(name).selected(selected).build() {
                        idx = i;
                        self.scene = self.scene_from_index(idx);
                        let _ = self.reset();
                    }
                }
            }
            ui.separator();
            let mut g = self.gravity_y;
            if ui.slider("Gravity Y", -30.0, 10.0, &mut g) {
                self.gravity_y = g;
                let _ = self.reset();
            }
            let mut ss = self.sub_steps;
            if ui.slider("Substeps", 1, 32, &mut ss) {
                self.sub_steps = ss;
            }
            let mut run = self.running;
            if ui.checkbox("Running", &mut run) {
                self.running = run;
            }
            let c = self.world.counters();
            ui.text(format!(
                "Counters: bodies={} shapes={} contacts={} joints={}",
                c.body_count, c.shape_count, c.contact_count, c.joint_count
            ));
            ui.separator();
            ui.text("Scene Params");
            match self.scene {
                Scene::Pyramid => pyramid::ui_params(self, ui),
                Scene::Stacking => stacking::ui_params(self, ui),
                Scene::Bridge => bridge::ui_params(self, ui),
                Scene::Car => car::ui_params(self, ui),
                Scene::ChainWalkway => chain_walkway::ui_params(self, ui),
                Scene::Sensors => sensors::ui_params(self, ui),
                Scene::Contacts => contacts::ui_params(self, ui),
                Scene::ContinuousLab => continuous_lab::ui_params(self, ui),
                Scene::Shapes => shapes::ui_params(self, ui),
                Scene::Events => events::ui_params(self, ui),
                Scene::Robustness => robustness::ui_params(self, ui),
                Scene::Benchmark => benchmark::ui_params(self, ui),
                Scene::Determinism => determinism::ui_params(self, ui),
                Scene::QueriesCasts => queries_casts::ui_params(self, ui),
                Scene::CharacterMover => character_mover::ui_params(self, ui),
                Scene::JointsLab => joints_lab::ui_params(self, ui),
                Scene::SoftBody => soft_body::ui_params(self, ui),
                Scene::ConvexHull => convex_hull::ui_params(self, ui),
                Scene::BodiesLab => bodies_lab::ui_params(self, ui),
                Scene::ShapeDistance => shape_distance::ui_params(self, ui),
                Scene::JointSeparation => joint_separation::ui_params(self, ui),
                Scene::Manifold => manifold::ui_params(self, ui),
                Scene::CollisionTools => collision_tools::ui_params(self, ui),
                Scene::WorldTuning => world_lab::ui_params(self, ui),
                Scene::MotionLocks => motion_locks::ui_params(self, ui),
                Scene::BreakableJoint => breakable_joint::ui_params(self, ui),
                Scene::Materials => materials::ui_params(self, ui),
                Scene::ShapeEditing => shape_editing::ui_params(self, ui),
                Scene::Doohickey => doohickey::ui_params(self, ui),
                Scene::Issues => issues::ui_params(self, ui),
            }
            ui.separator();
            ui.text("Debug Draw");
            ui.checkbox("Shapes", &mut self.dd_draw_shapes);
            ui.same_line(); ui.checkbox("Joints", &mut self.dd_draw_joints);
            ui.checkbox("Joint Extras", &mut self.dd_draw_joint_extras);
            ui.same_line(); ui.checkbox("AABBs", &mut self.dd_draw_bounds);
            ui.checkbox("Mass/COM", &mut self.dd_draw_mass);
            ui.same_line(); ui.checkbox("Body Names", &mut self.dd_draw_body_names);
            ui.checkbox("Contacts", &mut self.dd_draw_contacts);
            ui.same_line(); ui.checkbox("Contact Features", &mut self.dd_draw_contact_features);
            ui.checkbox("Contact Normals", &mut self.dd_draw_contact_normals);
            ui.same_line(); ui.checkbox("Contact Forces", &mut self.dd_draw_contact_forces);
            ui.checkbox("Friction Forces", &mut self.dd_draw_friction_forces);
            ui.same_line(); ui.checkbox("Islands", &mut self.dd_draw_islands);
            let mut fs = self.dd_force_scale; let mut js = self.dd_joint_scale;
            let _ = ui.slider("Force Scale", 0.0, 10.0, &mut fs);
            let _ = ui.slider("Joint Scale", 0.1, 5.0, &mut js);
            self.dd_force_scale = fs; self.dd_joint_scale = js;
            ui.separator();
            ui.text(format!(
                "Stats: step={:.2} ms, awake={}, bodies={}, shapes={}, joints={}, contacts={}, islands={}",
                self.step_ms, self.cnt_awake, self.cnt_bodies, self.cnt_shapes, self.cnt_joints, self.cnt_contacts, self.cnt_islands
            ));
        });
    }

    /// Draw small scene-specific overlays on top of world debug draw.
    pub fn debug_overlay(&self, ui: &imgui::Ui) {
        // Currently only used by the Manifold scene.
        if self.scene == Scene::Manifold {
                let dl = ui.get_foreground_draw_list();
                let ds = ui.io().display_size();
                let origin = [ds[0] * 0.5, ds[1] * 0.5];
                let s = self.pixels_per_meter; // pixels per meter (shared with debug draw)
                let w2s = |x: f32, y: f32| [origin[0] + x * s, ds[1] - (origin[1] + y * s)];

                // Contact point
                let p = w2s(self.mf_point_x, self.mf_point_y);
                let col = 0xffff55ffu32; // magenta point
                dl.add_circle(p, 5.0, col).thickness(2.0).build();
                // Second point if available
                if self.mf_contacts > 1 {
                    let p2 = w2s(self.mf_point2_x, self.mf_point2_y);
                    dl.add_circle(p2, 5.0, 0xff55ffffu32).thickness(2.0).build();
                }

                // Normal arrow (from contact point)
                let nx = self.mf_normal_x;
                let ny = self.mf_normal_y;
                let len = 0.7_f32; // meters
                let q = w2s(self.mf_point_x + nx * len, self.mf_point_y + ny * len);
                dl.add_line(p, q, 0xffffff00u32).thickness(2.0).build();
            }
    }

    pub fn build_scene(&mut self) {
        self.created_bodies = 0;
        self.created_shapes = 0;
        self.created_joints = 0;
        let ground = self.world.create_body_id(bd::BodyBuilder::new().build());
        self.created_bodies += 1;
        let _g = self.world.create_polygon_shape_for(
            ground,
            &bd::ShapeDef::builder().density(0.0).build(),
            &bd::shapes::box_polygon(50.0, 1.0),
        );
        self.created_shapes += 1;
        match self.scene {
            Scene::Pyramid => pyramid::build(self, ground),
            Scene::Stacking => stacking::build(self, ground),
            Scene::Bridge => bridge::build(self, ground),
            Scene::Car => car::build(self, ground),
            Scene::ChainWalkway => chain_walkway::build(self, ground),
            Scene::Sensors => sensors::build(self, ground),
            Scene::Contacts => contacts::build(self, ground),
            Scene::ContinuousLab => continuous_lab::build(self, ground),
            Scene::Shapes => shapes::build(self, ground),
            Scene::Events => events::build(self, ground),
            Scene::Robustness => robustness::build(self, ground),
            Scene::Benchmark => benchmark::build(self, ground),
            Scene::Determinism => determinism::build(self, ground),
            Scene::QueriesCasts => queries_casts::build(self, ground),
            Scene::CharacterMover => character_mover::build(self, ground),
            Scene::JointsLab => joints_lab::build(self, ground),
            Scene::SoftBody => soft_body::build(self, ground),
            Scene::ConvexHull => convex_hull::build(self, ground),
            Scene::CollisionTools => collision_tools::build(self, ground),
            Scene::BodiesLab => bodies_lab::build(self, ground),
            Scene::ShapeDistance => shape_distance::build(self, ground),
            Scene::JointSeparation => joint_separation::build(self, ground),
            Scene::Manifold => manifold::build(self, ground),
            Scene::MotionLocks => motion_locks::build(self, ground),
            Scene::BreakableJoint => breakable_joint::build(self, ground),
            Scene::WorldTuning => world_lab::build(self, ground),
            Scene::Materials => materials::build(self, ground),
            Scene::ShapeEditing => shape_editing::build(self, ground),
            Scene::Doohickey => doohickey::build(self, ground),
            Scene::Issues => issues::build(self, ground),
        }
    }

    pub fn step_once(&mut self) {
        let (base_dt, sub) = match self.scene {
            Scene::ContinuousLab if self.cl_mode == 0 => (self.bullet_dt, self.bullet_substeps),
            _ => (1.0 / 60.0, self.sub_steps),
        };
        let dt = (base_dt * self.time_scale.max(0.0)).max(0.0);
        self.world.step(dt, sub);
    }

    pub fn scene_index(&self) -> usize {
        match self.scene {
            Scene::Pyramid => 0,
            Scene::Bridge => 1,
            Scene::Car => 2,
            Scene::ChainWalkway => 3,
            Scene::Sensors => 4,
            Scene::Contacts => 5,
            Scene::ContinuousLab => 6,
            Scene::Stacking => 7,
            Scene::Shapes => 8,
            Scene::Robustness => 9,
            Scene::Events => 10,
            Scene::Benchmark => 11,
            Scene::Determinism => 12,
            Scene::QueriesCasts => 13,
            Scene::CharacterMover => 14,
            Scene::JointsLab => 15,
            Scene::SoftBody => 16,
            Scene::ConvexHull => 17,
            Scene::BodiesLab => 18,
            Scene::ShapeDistance => 19,
            Scene::JointSeparation => 20,
            Scene::Manifold => 21,
            Scene::CollisionTools => 22,
            Scene::WorldTuning => 23,
            Scene::MotionLocks => 24,
            Scene::BreakableJoint => 25,
            Scene::Materials => 26,
            Scene::ShapeEditing => 27,
            Scene::Doohickey => 28,
            Scene::Issues => 29,
        }
    }

    pub fn scene_from_index(&self, i: usize) -> Scene {
        match i {
            0 => Scene::Pyramid,
            1 => Scene::Bridge,
            2 => Scene::Car,
            3 => Scene::ChainWalkway,
            4 => Scene::Sensors,
            5 => Scene::Contacts,
            6 => Scene::ContinuousLab,
            7 => Scene::Stacking,
            8 => Scene::Shapes,
            9 => Scene::Robustness,
            10 => Scene::Events,
            11 => Scene::Benchmark,
            12 => Scene::Determinism,
            13 => Scene::QueriesCasts,
            14 => Scene::CharacterMover,
            15 => Scene::JointsLab,
            16 => Scene::SoftBody,
            17 => Scene::ConvexHull,
            18 => Scene::BodiesLab,
            19 => Scene::ShapeDistance,
            20 => Scene::JointSeparation,
            21 => Scene::Manifold,
            22 => Scene::CollisionTools,
            23 => Scene::WorldTuning,
            24 => Scene::MotionLocks,
            25 => Scene::BreakableJoint,
            26 => Scene::Materials,
            27 => Scene::ShapeEditing,
            28 => Scene::Doohickey,
            29 => Scene::Issues,
            _ => Scene::Pyramid,
        }
    }

    pub fn debug_draw_options(&self) -> bd::DebugDrawOptions {
        bd::DebugDrawOptions {
            force_scale: self.dd_force_scale,
            joint_scale: self.dd_joint_scale,
            draw_shapes: self.dd_draw_shapes,
            draw_joints: self.dd_draw_joints,
            draw_joint_extras: self.dd_draw_joint_extras,
            draw_bounds: self.dd_draw_bounds,
            draw_mass: self.dd_draw_mass,
            draw_body_names: self.dd_draw_body_names,
            draw_contacts: self.dd_draw_contacts,
            draw_graph_colors: self.dd_draw_graph_colors,
            draw_contact_features: self.dd_draw_contact_features,
            draw_contact_normals: self.dd_draw_contact_normals,
            draw_contact_forces: self.dd_draw_contact_forces,
            draw_friction_forces: self.dd_draw_friction_forces,
            draw_islands: self.dd_draw_islands,
            ..Default::default()
        }
    }
}
