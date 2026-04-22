// Central physics app + scene routing for the ImGui testbed
use boxdd as bd;
use dear_imgui_rs as imgui;

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
pub mod overlap_queries {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/testbed/scenes/overlap_queries.rs"
    ));
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
pub mod query_casts {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/testbed/scenes/query_casts.rs"
    ));
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
    OverlapQueries,
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
    QueryCasts,
    Doohickey,
    Issues,
}

pub struct OverlapQueryState {
    pub center_x: f32,
    pub center_y: f32,
    pub half_x: f32,
    pub half_y: f32,
    pub owned_hits: usize,
    pub reused_hits: usize,
    pub visit_hits: usize,
    pub polygon_hits: usize,
    pub visit_stopped_early: bool,
    pub reused_hit_buffer: Vec<bd::types::ShapeId>,
}

impl Default for OverlapQueryState {
    fn default() -> Self {
        Self {
            center_x: 0.0,
            center_y: 2.0,
            half_x: 2.0,
            half_y: 1.0,
            owned_hits: 0,
            reused_hits: 0,
            visit_hits: 0,
            polygon_hits: 0,
            visit_stopped_early: false,
            reused_hit_buffer: Vec::new(),
        }
    }
}

impl OverlapQueryState {
    fn reset_runtime(&mut self) {
        self.owned_hits = 0;
        self.reused_hits = 0;
        self.visit_hits = 0;
        self.polygon_hits = 0;
        self.visit_stopped_early = false;
        self.reused_hit_buffer.clear();
    }
}

pub struct QueryCastState {
    pub mode: i32,
    pub ray_origin_x: f32,
    pub ray_origin_y: f32,
    pub ray_dx: f32,
    pub ray_dy: f32,
    pub ray_hits: usize,
    pub ray_hit_buffer: Vec<bd::RayResult>,
    pub shape_pos_y: f32,
    pub shape_angle: f32,
    pub shape_tx: f32,
    pub shape_ty: f32,
    pub shape_radius: f32,
    pub shape_hits: usize,
    pub shape_min_fraction: f32,
    pub shape_hit_buffer: Vec<bd::RayResult>,
    pub toi_start_x: f32,
    pub toi_start_y: f32,
    pub toi_angle: f32,
    pub toi_dx: f32,
    pub toi_dy: f32,
    pub toi_radius: f32,
    pub toi_state: bd::ToiState,
    pub toi_fraction: f32,
}

impl Default for QueryCastState {
    fn default() -> Self {
        Self {
            mode: 0,
            ray_origin_x: -10.0,
            ray_origin_y: 10.0,
            ray_dx: 25.0,
            ray_dy: -12.0,
            ray_hits: 0,
            ray_hit_buffer: Vec::new(),
            shape_pos_y: 6.0,
            shape_angle: 0.0,
            shape_tx: 0.0,
            shape_ty: -6.0,
            shape_radius: 0.02,
            shape_hits: 0,
            shape_min_fraction: 1.0,
            shape_hit_buffer: Vec::new(),
            toi_start_x: -2.0,
            toi_start_y: 6.0,
            toi_angle: 0.0,
            toi_dx: 4.0,
            toi_dy: -4.5,
            toi_radius: 0.02,
            toi_state: bd::ToiState::Unknown,
            toi_fraction: 1.0,
        }
    }
}

impl QueryCastState {
    fn reset_runtime(&mut self) {
        self.ray_hits = 0;
        self.ray_hit_buffer.clear();
        self.shape_hits = 0;
        self.shape_min_fraction = 1.0;
        self.shape_hit_buffer.clear();
        self.toi_state = bd::ToiState::Unknown;
        self.toi_fraction = 1.0;
    }
}

pub struct BodiesLabState {
    pub mode: usize,
    pub set_velocity_x: f32,
    pub set_velocity_y: f32,
    pub set_velocity_body: Option<bd::types::BodyId>,
    pub kinematic_speed: f32,
    pub kinematic_platform: Option<bd::types::BodyId>,
    pub wake_touch_count: usize,
    pub wake_touch_ground_body: Option<bd::types::BodyId>,
}

impl Default for BodiesLabState {
    fn default() -> Self {
        Self {
            mode: 0,
            set_velocity_x: 0.0,
            set_velocity_y: -20.0,
            set_velocity_body: None,
            kinematic_speed: 2.0,
            kinematic_platform: None,
            wake_touch_count: 0,
            wake_touch_ground_body: None,
        }
    }
}

impl BodiesLabState {
    fn reset_runtime(&mut self) {
        self.set_velocity_body = None;
        self.kinematic_platform = None;
        self.wake_touch_count = 0;
        self.wake_touch_ground_body = None;
    }
}

pub struct WorldLabState {
    pub mode: usize,
    pub sleeping: bool,
    pub continuous: bool,
    pub contact_softening: bool,
    pub restitution_threshold: f32,
    pub hit_event_threshold: f32,
    pub warm_starting: bool,
    pub maximum_linear_speed: f32,
    pub contact_speed: f32,
    pub explosion_center_x: f32,
    pub explosion_center_y: f32,
    pub explosion_radius: f32,
    pub explosion_falloff: f32,
    pub explosion_impulse: f32,
}

impl Default for WorldLabState {
    fn default() -> Self {
        Self {
            mode: 0,
            sleeping: true,
            continuous: false,
            contact_softening: false,
            restitution_threshold: 1.0,
            hit_event_threshold: 0.0,
            warm_starting: true,
            maximum_linear_speed: 100.0,
            contact_speed: 2.0,
            explosion_center_x: 0.0,
            explosion_center_y: 3.0,
            explosion_radius: 3.0,
            explosion_falloff: 0.1,
            explosion_impulse: 2.0,
        }
    }
}

pub struct ManifoldState {
    pub a_x: f32,
    pub a_y: f32,
    pub a_angle: f32,
    pub a_half_x: f32,
    pub a_half_y: f32,
    pub b_x: f32,
    pub b_y: f32,
    pub b_angle: f32,
    pub b_half_x: f32,
    pub b_half_y: f32,
    pub contact_count: usize,
    pub normal_x: f32,
    pub normal_y: f32,
    pub point1_x: f32,
    pub point1_y: f32,
    pub point2_x: f32,
    pub point2_y: f32,
    pub mode: i32,
    pub b_radius: f32,
    pub show_metrics: bool,
    pub separation1: f32,
    pub separation2: f32,
    pub impulse1: f32,
    pub impulse2: f32,
    pub total_impulse1: f32,
    pub total_impulse2: f32,
    pub segment_half_len: f32,
}

impl Default for ManifoldState {
    fn default() -> Self {
        Self {
            a_x: -0.8,
            a_y: 3.0,
            a_angle: 0.0,
            a_half_x: 0.7,
            a_half_y: 0.5,
            b_x: 0.8,
            b_y: 3.2,
            b_angle: 0.2,
            b_half_x: 0.6,
            b_half_y: 0.4,
            contact_count: 0,
            normal_x: 0.0,
            normal_y: 0.0,
            point1_x: 0.0,
            point1_y: 0.0,
            point2_x: 0.0,
            point2_y: 0.0,
            mode: 0,
            b_radius: 0.5,
            show_metrics: false,
            separation1: 0.0,
            separation2: 0.0,
            impulse1: 0.0,
            impulse2: 0.0,
            total_impulse1: 0.0,
            total_impulse2: 0.0,
            segment_half_len: 1.0,
        }
    }
}

impl ManifoldState {
    fn reset_runtime(&mut self) {
        self.contact_count = 0;
        self.normal_x = 0.0;
        self.normal_y = 0.0;
        self.point1_x = 0.0;
        self.point1_y = 0.0;
        self.point2_x = 0.0;
        self.point2_y = 0.0;
        self.separation1 = 0.0;
        self.separation2 = 0.0;
        self.impulse1 = 0.0;
        self.impulse2 = 0.0;
        self.total_impulse1 = 0.0;
        self.total_impulse2 = 0.0;
    }
}

pub struct MaterialsState {
    pub conveyor_speed: f32,
    pub rolling_resistance: f32,
    pub spawned_count: usize,
    pub wind_x: f32,
    pub wind_y: f32,
    pub drag: f32,
    pub lift: f32,
    pub wake_on_wind: bool,
    pub spawned_shapes: Vec<bd::types::ShapeId>,
    pub left_belt_shape: Option<bd::types::ShapeId>,
    pub right_belt_shape: Option<bd::types::ShapeId>,
    pub belt_friction: f32,
    pub belt_restitution: f32,
    pub shape_friction: f32,
    pub shape_restitution: f32,
    pub left_belt_body: Option<bd::types::BodyId>,
    pub right_belt_body: Option<bd::types::BodyId>,
    pub belt_half_len: f32,
    pub belt_thickness: f32,
    pub left_belt_x: f32,
    pub left_belt_y: f32,
    pub right_belt_x: f32,
    pub right_belt_y: f32,
    pub left_belt_angle_deg: f32,
    pub right_belt_angle_deg: f32,
}

impl Default for MaterialsState {
    fn default() -> Self {
        Self {
            conveyor_speed: 1.5,
            rolling_resistance: 0.5,
            spawned_count: 0,
            wind_x: 2.0,
            wind_y: 0.0,
            drag: 1.0,
            lift: 0.0,
            wake_on_wind: false,
            spawned_shapes: Vec::new(),
            left_belt_shape: None,
            right_belt_shape: None,
            belt_friction: 0.9,
            belt_restitution: 0.0,
            shape_friction: 0.6,
            shape_restitution: 0.2,
            left_belt_body: None,
            right_belt_body: None,
            belt_half_len: 4.5,
            belt_thickness: 0.2,
            left_belt_x: -3.0,
            left_belt_y: 0.5,
            right_belt_x: 3.0,
            right_belt_y: 2.5,
            left_belt_angle_deg: 0.0,
            right_belt_angle_deg: 0.0,
        }
    }
}

impl MaterialsState {
    fn reset_runtime(&mut self) {
        self.spawned_count = 0;
        self.spawned_shapes.clear();
        self.left_belt_shape = None;
        self.right_belt_shape = None;
        self.left_belt_body = None;
        self.right_belt_body = None;
    }
}

// Reusable per-frame buffers shared across scenes so the interactive testbed
// reflects the same hot-path guidance as the public examples and docs.
pub struct TestbedScratch {
    pub body_events: Vec<bd::BodyMoveEvent>,
    pub sensor_events: bd::SensorEvents,
    pub contact_events: bd::ContactEvents,
    pub joint_events: Vec<bd::JointEvent>,
    pub ray_hits: Vec<bd::RayResult>,
}

impl Default for TestbedScratch {
    fn default() -> Self {
        Self {
            body_events: Vec::new(),
            sensor_events: bd::SensorEvents::default(),
            contact_events: bd::ContactEvents::default(),
            joint_events: Vec::new(),
            ray_hits: Vec::new(),
        }
    }
}

impl TestbedScratch {
    fn reset_runtime(&mut self) {
        self.body_events.clear();
        self.sensor_events.begin.clear();
        self.sensor_events.end.clear();
        self.contact_events.begin.clear();
        self.contact_events.end.clear();
        self.contact_events.hit.clear();
        self.joint_events.clear();
        self.ray_hits.clear();
    }
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
    // Stats
    pub created_bodies: usize,
    pub created_shapes: usize,
    pub created_joints: usize,
    pub scratch: TestbedScratch,
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
    pub overlap_queries: OverlapQueryState,
    pub query_casts: QueryCastState,
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
    // Continuous: ghost bumps
    pub gb_speed: f32,
    pub gb_bump_h: f32,
    pub gb_hits: usize,
    // Continuous: restitution threshold
    pub cr_threshold: f32,
    pub cr_restitution: f32,
    pub cr_drop_y: f32,
    // Joints: filter
    pub fj_disable_collide: bool,
    pub fj_hits: usize,
    pub bodies_lab: BodiesLabState,
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
    pub manifold: ManifoldState,
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
    // Weld joint
    pub wj_hz: f32,
    pub wj_dr: f32,
    pub world_lab: WorldLabState,
    pub materials: MaterialsState,
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
}

type SceneBuildFn = fn(&mut PhysicsApp, bd::types::BodyId);
type SceneTickFn = fn(&mut PhysicsApp);
type SceneUiFn = fn(&mut PhysicsApp, &imgui::Ui);
type SceneOverlayFn = fn(&PhysicsApp, &imgui::Ui);

struct SceneSpec {
    id: Scene,
    name: &'static str,
    build: SceneBuildFn,
    tick: Option<SceneTickFn>,
    ui: SceneUiFn,
    overlay: Option<SceneOverlayFn>,
}

const SCENE_SPECS: [SceneSpec; 30] = [
    SceneSpec {
        id: Scene::Pyramid,
        name: "Pyramid",
        build: pyramid::build,
        tick: None,
        ui: pyramid::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Bridge,
        name: "Bridge",
        build: bridge::build,
        tick: None,
        ui: bridge::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Car,
        name: "Car",
        build: car::build,
        tick: None,
        ui: car::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::ChainWalkway,
        name: "Chain Walkway",
        build: chain_walkway::build,
        tick: None,
        ui: chain_walkway::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Sensors,
        name: "Sensors",
        build: sensors::build,
        tick: None,
        ui: sensors::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Contacts,
        name: "Contacts",
        build: contacts::build,
        tick: None,
        ui: contacts::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::ContinuousLab,
        name: "Continuous: Lab",
        build: continuous_lab::build,
        tick: Some(continuous_lab::tick),
        ui: continuous_lab::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Stacking,
        name: "Stacking",
        build: stacking::build,
        tick: None,
        ui: stacking::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Shapes,
        name: "Shapes Variety",
        build: shapes::build,
        tick: Some(shapes::tick),
        ui: shapes::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Robustness,
        name: "Robustness",
        build: robustness::build,
        tick: Some(robustness::tick),
        ui: robustness::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Events,
        name: "Events Summary",
        build: events::build,
        tick: Some(events::tick),
        ui: events::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Benchmark,
        name: "Benchmark",
        build: benchmark::build,
        tick: Some(benchmark::tick),
        ui: benchmark::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Determinism,
        name: "Determinism",
        build: determinism::build,
        tick: Some(determinism::tick),
        ui: determinism::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::OverlapQueries,
        name: "Queries: Overlap",
        build: overlap_queries::build,
        tick: Some(overlap_queries::tick),
        ui: overlap_queries::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::CharacterMover,
        name: "Character Mover",
        build: character_mover::build,
        tick: Some(character_mover::tick),
        ui: character_mover::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::JointsLab,
        name: "Joints: Lab (Unified)",
        build: joints_lab::build,
        tick: Some(joints_lab::tick),
        ui: joints_lab::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::SoftBody,
        name: "Soft Body (Donut)",
        build: soft_body::build,
        tick: Some(soft_body::tick),
        ui: soft_body::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::ConvexHull,
        name: "Convex Hull",
        build: convex_hull::build,
        tick: Some(convex_hull::tick),
        ui: convex_hull::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::BodiesLab,
        name: "Bodies: Lab",
        build: bodies_lab::build,
        tick: Some(bodies_lab::tick),
        ui: bodies_lab::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::ShapeDistance,
        name: "Collision: Shape Distance",
        build: shape_distance::build,
        tick: Some(shape_distance::tick),
        ui: shape_distance::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::JointSeparation,
        name: "Joints: Separation",
        build: joint_separation::build,
        tick: Some(joint_separation::tick),
        ui: joint_separation::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Manifold,
        name: "Collision: Manifold (basic)",
        build: manifold::build,
        tick: Some(manifold::tick),
        ui: manifold::ui_params,
        overlay: Some(manifold::debug_overlay),
    },
    SceneSpec {
        id: Scene::QueryCasts,
        name: "Queries: Casts & TOI",
        build: query_casts::build,
        tick: Some(query_casts::tick),
        ui: query_casts::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::WorldTuning,
        name: "World: Lab",
        build: world_lab::build,
        tick: None,
        ui: world_lab::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::MotionLocks,
        name: "Joints: Motion Locks",
        build: motion_locks::build,
        tick: None,
        ui: motion_locks::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::BreakableJoint,
        name: "Joints: Breakable",
        build: breakable_joint::build,
        tick: Some(breakable_joint::tick),
        ui: breakable_joint::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Materials,
        name: "Materials: Conveyor & Rolling",
        build: materials::build,
        tick: Some(materials::tick),
        ui: materials::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::ShapeEditing,
        name: "Shape Editing",
        build: shape_editing::build,
        tick: None,
        ui: shape_editing::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Doohickey,
        name: "Doohickey",
        build: doohickey::build,
        tick: Some(doohickey::tick),
        ui: doohickey::ui_params,
        overlay: None,
    },
    SceneSpec {
        id: Scene::Issues,
        name: "Issues",
        build: issues::build,
        tick: Some(issues::tick),
        ui: issues::ui_params,
        overlay: None,
    },
];

fn scene_spec(scene: Scene) -> &'static SceneSpec {
    SCENE_SPECS
        .iter()
        .find(|spec| spec.id == scene)
        .expect("scene registry must cover all Scene variants")
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
            scratch: TestbedScratch::default(),
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
            overlap_queries: OverlapQueryState::default(),
            query_casts: QueryCastState::default(),
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
            gb_speed: 40.0,
            gb_bump_h: 0.2,
            gb_hits: 0,
            cr_threshold: 1.0,
            cr_restitution: 0.7,
            cr_drop_y: 8.0,
            fj_disable_collide: true,
            fj_hits: 0,
            bodies_lab: BodiesLabState::default(),
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
            manifold: ManifoldState::default(),
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
            wj_hz: 8.0,
            wj_dr: 0.7,
            world_lab: WorldLabState::default(),
            materials: MaterialsState::default(),
            se_body: None,
            se_shape: None,
            se_mode: 0,
            se_hx: 0.8,
            se_hy: 0.5,
            se_radius: 0.2,
            jl_mode: 0,
            cl_mode: 0,
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
                b = b.enable_continuous(true);
            }
            Scene::WorldTuning => {
                b = b
                    .enable_sleep(self.world_lab.sleeping)
                    .enable_continuous(self.world_lab.continuous)
                    .enable_contact_softening(self.world_lab.contact_softening)
                    .maximum_linear_speed(self.world_lab.maximum_linear_speed)
                    .contact_speed(self.world_lab.contact_speed)
                    .restitution_threshold(self.world_lab.restitution_threshold)
                    .hit_event_threshold(self.world_lab.hit_event_threshold);
            }
            _ => {}
        }
        self.world = bd::World::new(b.build())?;
        if matches!(self.scene, Scene::WorldTuning) {
            self.world.enable_warm_starting(self.world_lab.warm_starting);
        }
        self.ev_moves = 0;
        self.ev_sens_beg = 0;
        self.ev_sens_end = 0;
        self.ev_con_beg = 0;
        self.ev_con_end = 0;
        self.ev_con_hit = 0;
        self.ev_joint = 0;
        self.robust_hit_count = 0;
        self.scratch.reset_runtime();
        self.overlap_queries.reset_runtime();
        self.cm_fraction = 1.0;
        self.query_casts.reset_runtime();
        self.gb_hits = 0;
        self.manifold.reset_runtime();
        self.pb_ball_count = 0;
        self.ml_body = None;
        self.bj_broken = 0;
        self.bodies_lab.reset_runtime();
        self.materials.reset_runtime();
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
            let c = self.world.counters();
            self.cnt_bodies = c.body_count;
            self.cnt_shapes = c.shape_count;
            self.cnt_contacts = c.contact_count;
            self.cnt_joints = c.joint_count;
            self.cnt_islands = c.island_count;
            self.cnt_awake = self.world.awake_body_count();
        }
        if let Some(tick) = scene_spec(self.scene).tick {
            tick(self);
        }
    }

    pub fn ui(&mut self, ui: &imgui::Ui) {
        ui.window("BoxDD Testbed")
            .size([380.0, 700.0], imgui::Condition::FirstUseEver)
            .position([16.0, 16.0], imgui::Condition::FirstUseEver)
            .size_constraints([340.0, 420.0], [540.0, 1080.0])
            .build(|| {
                self.ui_transport_controls(ui);
                ui.separator();
                self.ui_scene_section(ui);
                self.ui_global_section(ui);
                self.ui_debug_draw_section(ui);
                self.ui_stats_section(ui);
            });
    }

    fn panel_section_flags(default_open: bool) -> imgui::TreeNodeFlags {
        let mut flags = imgui::TreeNodeFlags::FRAMED | imgui::TreeNodeFlags::SPAN_AVAIL_WIDTH;
        if default_open {
            flags |= imgui::TreeNodeFlags::DEFAULT_OPEN;
        }
        flags
    }

    fn ui_transport_controls(&mut self, ui: &imgui::Ui) {
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
        ui.text(format!("Scene: {}", scene_spec(self.scene).name));
    }

    fn ui_scene_section(&mut self, ui: &imgui::Ui) {
        if !ui.collapsing_header("Scene", Self::panel_section_flags(true)) {
            return;
        }

        let mut idx = self.scene_index();
        if let Some(_c) = ui.begin_combo("Scene##scene_select", scene_spec(self.scene).name) {
            for (i, spec) in SCENE_SPECS.iter().enumerate() {
                let selected = i == idx;
                if ui.selectable_config(spec.name).selected(selected).build() {
                    idx = i;
                    self.scene = self.scene_from_index(idx);
                    let _ = self.reset();
                }
            }
        }

        ui.text("Parameters");
        (scene_spec(self.scene).ui)(self, ui);
    }

    fn ui_global_section(&mut self, ui: &imgui::Ui) {
        if !ui.collapsing_header("Global", Self::panel_section_flags(true)) {
            return;
        }

        let mut ppm = self.pixels_per_meter;
        if ui.slider("Pixels / Meter", 5.0, 120.0, &mut ppm) {
            self.pixels_per_meter = ppm.max(1.0);
        }

        let mut ts = self.time_scale;
        if ui.slider("Time Scale", 0.1, 2.0, &mut ts) {
            self.time_scale = ts;
        }

        let mut g = self.gravity_y;
        if ui.slider("Gravity Y", -30.0, 10.0, &mut g) {
            self.gravity_y = g;
            let _ = self.reset();
        }

        let mut ss = self.sub_steps;
        if ui.slider("Substeps", 1, 32, &mut ss) {
            self.sub_steps = ss;
        }

        ui.checkbox("Running", &mut self.running);
    }

    fn ui_debug_draw_section(&mut self, ui: &imgui::Ui) {
        if !ui.collapsing_header("Debug Draw", Self::panel_section_flags(false)) {
            return;
        }

        ui.checkbox("Shapes", &mut self.dd_draw_shapes);
        ui.same_line();
        ui.checkbox("Joints", &mut self.dd_draw_joints);
        ui.checkbox("Joint Extras", &mut self.dd_draw_joint_extras);
        ui.same_line();
        ui.checkbox("AABBs", &mut self.dd_draw_bounds);
        ui.checkbox("Mass/COM", &mut self.dd_draw_mass);
        ui.same_line();
        ui.checkbox("Body Names", &mut self.dd_draw_body_names);
        ui.checkbox("Contacts", &mut self.dd_draw_contacts);
        ui.same_line();
        ui.checkbox("Contact Features", &mut self.dd_draw_contact_features);
        ui.checkbox("Contact Normals", &mut self.dd_draw_contact_normals);
        ui.same_line();
        ui.checkbox("Contact Forces", &mut self.dd_draw_contact_forces);
        ui.checkbox("Friction Forces", &mut self.dd_draw_friction_forces);
        ui.same_line();
        ui.checkbox("Islands", &mut self.dd_draw_islands);

        let mut fs = self.dd_force_scale;
        let mut js = self.dd_joint_scale;
        let _ = ui.slider("Force Scale", 0.0, 10.0, &mut fs);
        let _ = ui.slider("Joint Scale", 0.1, 5.0, &mut js);
        self.dd_force_scale = fs;
        self.dd_joint_scale = js;
    }

    fn ui_stats_section(&self, ui: &imgui::Ui) {
        if !ui.collapsing_header("Stats", Self::panel_section_flags(true)) {
            return;
        }

        let c = self.world.counters();
        ui.text(format!(
            "Live: bodies={} shapes={} contacts={} joints={}",
            c.body_count, c.shape_count, c.contact_count, c.joint_count
        ));
        ui.text(format!(
            "Build: created bodies={} shapes={} joints={}",
            self.created_bodies, self.created_shapes, self.created_joints
        ));
        ui.text(format!(
            "Step: {:.2} ms, awake={}, islands={}",
            self.step_ms, self.cnt_awake, self.cnt_islands
        ));
    }

    pub fn debug_overlay(&self, ui: &imgui::Ui) {
        if let Some(overlay) = scene_spec(self.scene).overlay {
            overlay(self, ui);
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
        (scene_spec(self.scene).build)(self, ground);
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
        SCENE_SPECS
            .iter()
            .position(|spec| spec.id == self.scene)
            .expect("scene registry must cover all Scene variants")
    }

    pub fn scene_from_index(&self, i: usize) -> Scene {
        SCENE_SPECS
            .get(i)
            .map(|spec| spec.id)
            .unwrap_or(Scene::Pyramid)
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
