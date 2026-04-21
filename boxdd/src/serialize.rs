//! Serializable snapshots for configs and selected runtime state.
//!
//! This module is only compiled when the `serialize` feature is enabled.

use crate::{
    body::BodyType,
    joints::JointType,
    shapes::ShapeType,
    types::{BodyId, JointId, Vec2},
    world::World,
};
use boxdd_sys::ffi;
// no Hash/Eq on FFI ids; use simple field comparisons and linear scans

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorldConfigSnapshot {
    pub gravity: Vec2,
    pub enable_sleep: bool,
    pub enable_continuous: bool,
    pub enable_warm_starting: bool,
    pub restitution_threshold: f32,
    pub hit_event_threshold: f32,
    pub contact_hertz: f32,
    pub contact_damping_ratio: f32,
    pub contact_speed: f32,
    pub maximum_linear_speed: f32,
}

impl WorldConfigSnapshot {
    pub fn take(world: &World) -> Self {
        Self {
            gravity: world.gravity(),
            enable_sleep: world.is_sleeping_enabled(),
            enable_continuous: world.is_continuous_enabled(),
            enable_warm_starting: world.is_warm_starting_enabled(),
            restitution_threshold: world.restitution_threshold(),
            hit_event_threshold: world.hit_event_threshold(),
            // contact tuning: there is only setter, snapshot via world config defaults
            // We cannot read contact_hertz/damping/push individually; store defaults.
            // Use reasonable defaults; users can override when applying.
            contact_hertz: 30.0,
            contact_damping_ratio: 1.0,
            contact_speed: 100.0,
            maximum_linear_speed: world.maximum_linear_speed(),
        }
    }

    pub fn apply(&self, world: &mut World) {
        world.set_gravity(self.gravity);
        world.enable_sleeping(self.enable_sleep);
        world.enable_continuous(self.enable_continuous);
        world.enable_warm_starting(self.enable_warm_starting);
        world.set_restitution_threshold(self.restitution_threshold);
        world.set_hit_event_threshold(self.hit_event_threshold);
        world.set_contact_tuning(
            self.contact_hertz,
            self.contact_damping_ratio,
            self.contact_speed,
        );
        world.set_maximum_linear_speed(self.maximum_linear_speed);
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BodySnapshot {
    pub body_type: BodyType,
    pub position: Vec2,
    pub angle: f32,
    pub linear_velocity: Vec2,
    pub angular_velocity: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub gravity_scale: f32,
}

impl BodySnapshot {
    pub fn take(world: &World, id: BodyId) -> Self {
        crate::core::debug_checks::assert_body_valid(id);
        Self {
            body_type: crate::body::body_type_impl(id),
            position: world.body_position(id),
            angle: crate::body::body_rotation_impl(id).angle(),
            linear_velocity: crate::body::body_linear_velocity_impl(id),
            angular_velocity: crate::body::body_angular_velocity_impl(id),
            linear_damping: crate::body::body_linear_damping_impl(id),
            angular_damping: crate::body::body_angular_damping_impl(id),
            gravity_scale: crate::body::body_gravity_scale_impl(id),
        }
    }

    pub fn apply(&self, world: &mut World, id: BodyId) {
        crate::core::debug_checks::assert_body_valid(id);
        world.set_body_type(id, self.body_type);
        world.set_body_position_and_rotation(id, self.position, self.angle);
        world.set_body_linear_velocity(id, self.linear_velocity);
        world.set_body_angular_velocity(id, self.angular_velocity);
        crate::body::body_set_linear_damping_impl(id, self.linear_damping);
        crate::body::body_set_angular_damping_impl(id, self.angular_damping);
        crate::body::body_set_gravity_scale_impl(id, self.gravity_scale);
    }
}

// =============== Full Scene Snapshot (experimental, minimal joints) ===============

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SceneSnapshot {
    pub world: WorldConfigSnapshot,
    pub bodies: Vec<BodyRecord>,
    pub joints: Vec<JointRecord>,
    #[serde(default)]
    pub chains: Vec<ChainRecord>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BodyRecord {
    pub def: crate::body::BodyDef,
    #[serde(default)]
    pub name: Option<String>,
    pub shapes: Vec<ShapeInstance>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ShapeInstance {
    pub def: crate::shapes::ShapeDef,
    #[serde(default)]
    pub sensor: bool,
    pub geom: ShapeGeom,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ShapeGeom {
    Circle { center: Vec2, radius: f32 },
    Segment { p1: Vec2, p2: Vec2 },
    Capsule { c1: Vec2, c2: Vec2, radius: f32 },
    Polygon { vertices: Vec<Vec2>, radius: f32 },
}

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum JointKind {
    Distance,
    Filter,
    Motor,
    Prismatic,
    Revolute,
    Weld,
    Wheel,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct JointRecord {
    pub kind: JointKind,
    pub body_a: u32,
    pub body_b: u32,
    pub local_a: crate::Transform,
    pub local_b: crate::Transform,
    #[serde(default)]
    pub params: Option<JointParams>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum JointParams {
    Distance {
        length: f32,
        spring_enabled: bool,
        spring_hertz: f32,
        spring_damping_ratio: f32,
        limit_enabled: bool,
        min_length: f32,
        max_length: f32,
        motor_enabled: bool,
        motor_speed: f32,
        max_motor_force: f32,
    },
    Prismatic {
        spring_enabled: bool,
        spring_hertz: f32,
        spring_damping_ratio: f32,
        target_translation: f32,
        limit_enabled: bool,
        lower: f32,
        upper: f32,
        motor_enabled: bool,
        motor_speed: f32,
        max_motor_force: f32,
    },
    Revolute {
        spring_enabled: bool,
        spring_hertz: f32,
        spring_damping_ratio: f32,
        target_angle: f32,
        limit_enabled: bool,
        lower: f32,
        upper: f32,
        motor_enabled: bool,
        motor_speed: f32,
        max_motor_torque: f32,
    },
    Weld {
        linear_hertz: f32,
        linear_damping_ratio: f32,
        angular_hertz: f32,
        angular_damping_ratio: f32,
    },
    Wheel {
        spring_enabled: bool,
        spring_hertz: f32,
        spring_damping_ratio: f32,
        limit_enabled: bool,
        lower: f32,
        upper: f32,
        motor_enabled: bool,
        motor_speed: f32,
        max_motor_torque: f32,
    },
    Motor {
        linear_velocity: Vec2,
        angular_velocity: f32,
        max_velocity_force: f32,
        max_velocity_torque: f32,
        linear_hertz: f32,
        linear_damping_ratio: f32,
        angular_hertz: f32,
        angular_damping_ratio: f32,
        max_spring_force: f32,
        max_spring_torque: f32,
    },
    Filter {},
}

impl SceneSnapshot {
    pub fn take(world: &World) -> Self {
        crate::core::callback_state::assert_not_in_callback();
        let cfg = WorldConfigSnapshot::take(world);
        // Build body list from registry (only tracks bodies created via this wrapper)
        let body_ids = world.body_ids();
        let mut bodies = Vec::new();

        for &bid in &body_ids {
            crate::core::debug_checks::assert_body_valid(bid);
            // BodyDef from runtime
            let def = body_def_from_runtime(world, bid);
            // Optional name
            let name = world.body_name(bid);
            // Shapes
            let shapes = shapes_from_body(world, bid);
            bodies.push(BodyRecord { def, name, shapes });
        }

        // Gather joints by walking per body and deduping (without Hash/Eq)
        let mut joint_list: Vec<JointId> = Vec::new();
        for &bid in &body_ids {
            for j in world.body_joints(bid) {
                if !joint_list.iter().any(|&x| eq_joint(x, j)) {
                    joint_list.push(j);
                }
            }
        }

        let mut joints = Vec::new();
        for j in joint_list {
            if !crate::joints::joint_is_valid_impl(j) {
                continue;
            }
            let a = world.joint_body_a_id(j);
            let b = world.joint_body_b_id(j);
            let ia = find_body_index(&body_ids, a);
            let ib = find_body_index(&body_ids, b);
            let (Some(ia), Some(ib)) = (ia, ib) else {
                continue;
            };
            let kind = joint_kind_from_runtime(world.joint_type(j));
            let params = joint_params_from_runtime(world, j, kind);

            joints.push(JointRecord {
                kind,
                body_a: ia,
                body_b: ib,
                local_a: world.joint_local_frame_a(j),
                local_b: world.joint_local_frame_b(j),
                params,
            });
        }

        // Chains via registry (captured at creation time).
        let mut chains: Vec<ChainRecord> = Vec::new();
        for cr in world.chain_records() {
            if let Some(bi) = find_body_index(&body_ids, cr.body) {
                let materials = match cr.materials {
                    crate::world::ChainMaterialsRecord::Default => None,
                    crate::world::ChainMaterialsRecord::Single(material) => {
                        Some(ChainMaterials::Single(material))
                    }
                    crate::world::ChainMaterialsRecord::Multiple(materials) => {
                        Some(ChainMaterials::Multiple(materials))
                    }
                };
                chains.push(ChainRecord {
                    body: bi,
                    is_loop: cr.is_loop,
                    filter: cr.filter,
                    enable_sensor_events: cr.enable_sensor_events,
                    points: cr.points,
                    materials,
                });
            }
        }

        Self {
            world: cfg,
            bodies,
            joints,
            chains,
        }
    }

    pub fn rebuild(&self) -> World {
        // Build world with gravity from config then apply runtime knobs
        let mut world = World::new(
            crate::world::WorldDef::builder()
                .gravity(self.world.gravity)
                .build(),
        )
        .expect("create world");
        self.world.apply(&mut world);

        // Create bodies and shapes
        let mut map: Vec<ffi::b2BodyId> = Vec::with_capacity(self.bodies.len());
        for br in &self.bodies {
            let id = world.create_body_id(br.def.clone());
            if let Some(name) = &br.name {
                world.set_body_name(id, name);
            }
            for sh in &br.shapes {
                let def = &sh.def;
                match &sh.geom {
                    ShapeGeom::Circle { center, radius } => {
                        let c = crate::shapes::Circle::new(*center, *radius);
                        let _ = world.create_circle_shape_for(id, def, &c);
                    }
                    ShapeGeom::Segment { p1, p2 } => {
                        let s = crate::shapes::Segment::new(*p1, *p2);
                        let _ = world.create_segment_shape_for(id, def, &s);
                    }
                    ShapeGeom::Capsule { c1, c2, radius } => {
                        let cap = crate::shapes::Capsule::new(*c1, *c2, *radius);
                        let _ = world.create_capsule_shape_for(id, def, &cap);
                    }
                    ShapeGeom::Polygon { vertices, radius } => {
                        // Build polygon via helper from points
                        if let Some(poly) =
                            crate::shapes::helpers::polygon_from_points(vertices.clone(), *radius)
                        {
                            let _ = world.create_polygon_shape_for(id, def, &poly);
                        }
                    }
                }
            }
            map.push(id);
        }

        // Create chains (captured via ID-style chain creation records).
        for cr in &self.chains {
            let body = map.get(cr.body as usize).copied();
            let Some(body) = body else {
                continue;
            };
            let mut b = crate::shapes::chain::ChainDef::builder()
                .points(cr.points.iter().copied())
                .is_loop(cr.is_loop)
                .filter(cr.filter)
                .enable_sensor_events(cr.enable_sensor_events);
            match &cr.materials {
                None => {}
                Some(ChainMaterials::Single(m)) => {
                    b = b.single_material(m);
                }
                Some(ChainMaterials::Multiple(ms)) => {
                    b = b.materials(ms);
                }
            }
            let def = b.build();
            let _ = world.create_chain_for_id(body, &def);
        }

        // Create joints (base frames only; type-specific parameters defaulted)
        for jr in &self.joints {
            let a = map.get(jr.body_a as usize).copied();
            let b = map.get(jr.body_b as usize).copied();
            let (Some(aid), Some(bid)) = (a, b) else {
                continue;
            };
            let base = crate::joints::JointBaseBuilder::new()
                .bodies_by_id(aid, bid)
                .local_frames_raw(jr.local_a.into(), jr.local_b.into())
                .build();
            match jr.kind {
                JointKind::Distance => {
                    let def = crate::joints::DistanceJointDef::new(base);
                    let id = world.create_distance_joint_id(&def);
                    if let Some(JointParams::Distance {
                        length,
                        spring_enabled,
                        spring_hertz,
                        spring_damping_ratio,
                        limit_enabled,
                        min_length,
                        max_length,
                        motor_enabled,
                        motor_speed,
                        max_motor_force,
                    }) = &jr.params
                    {
                        world.distance_set_length(id, *length);
                        world.distance_enable_spring(id, *spring_enabled);
                        world.distance_set_spring_hertz(id, *spring_hertz);
                        world.distance_set_spring_damping_ratio(id, *spring_damping_ratio);
                        world.distance_enable_limit(id, *limit_enabled);
                        world.distance_set_length_range(id, *min_length, *max_length);
                        world.distance_enable_motor(id, *motor_enabled);
                        world.distance_set_motor_speed(id, *motor_speed);
                        world.distance_set_max_motor_force(id, *max_motor_force);
                    }
                }
                JointKind::Filter => {
                    let def = crate::joints::FilterJointDef::new(base);
                    let _ = world.create_filter_joint_id(&def);
                }
                JointKind::Motor => {
                    let def = crate::joints::MotorJointDef::new(base);
                    let id = world.create_motor_joint_id(&def);
                    if let Some(JointParams::Motor {
                        linear_velocity,
                        angular_velocity,
                        max_velocity_force,
                        max_velocity_torque,
                        linear_hertz,
                        linear_damping_ratio,
                        angular_hertz,
                        angular_damping_ratio,
                        max_spring_force,
                        max_spring_torque,
                    }) = &jr.params
                    {
                        world.motor_set_linear_velocity(id, *linear_velocity);
                        world.motor_set_angular_velocity(id, *angular_velocity);
                        world.motor_set_max_velocity_force(id, *max_velocity_force);
                        world.motor_set_max_velocity_torque(id, *max_velocity_torque);
                        world.motor_set_linear_hertz(id, *linear_hertz);
                        world.motor_set_linear_damping_ratio(id, *linear_damping_ratio);
                        world.motor_set_angular_hertz(id, *angular_hertz);
                        world.motor_set_angular_damping_ratio(id, *angular_damping_ratio);
                        world.motor_set_max_spring_force(id, *max_spring_force);
                        world.motor_set_max_spring_torque(id, *max_spring_torque);
                    }
                }
                JointKind::Prismatic => {
                    let def = crate::joints::PrismaticJointDef::new(base);
                    let id = world.create_prismatic_joint_id(&def);
                    if let Some(JointParams::Prismatic {
                        spring_enabled,
                        spring_hertz,
                        spring_damping_ratio,
                        target_translation,
                        limit_enabled,
                        lower,
                        upper,
                        motor_enabled,
                        motor_speed,
                        max_motor_force,
                    }) = &jr.params
                    {
                        world.prismatic_enable_spring(id, *spring_enabled);
                        world.prismatic_set_spring_hertz(id, *spring_hertz);
                        world.prismatic_set_spring_damping_ratio(id, *spring_damping_ratio);
                        world.prismatic_set_target_translation(id, *target_translation);
                        world.prismatic_enable_limit(id, *limit_enabled);
                        world.prismatic_set_limits(id, *lower, *upper);
                        world.prismatic_enable_motor(id, *motor_enabled);
                        world.prismatic_set_motor_speed(id, *motor_speed);
                        world.prismatic_set_max_motor_force(id, *max_motor_force);
                    }
                }
                JointKind::Revolute => {
                    let def = crate::joints::RevoluteJointDef::new(base);
                    let id = world.create_revolute_joint_id(&def);
                    if let Some(JointParams::Revolute {
                        spring_enabled,
                        spring_hertz,
                        spring_damping_ratio,
                        target_angle,
                        limit_enabled,
                        lower,
                        upper,
                        motor_enabled,
                        motor_speed,
                        max_motor_torque,
                    }) = &jr.params
                    {
                        world.revolute_enable_spring(id, *spring_enabled);
                        world.revolute_set_spring_hertz(id, *spring_hertz);
                        world.revolute_set_spring_damping_ratio(id, *spring_damping_ratio);
                        world.revolute_set_target_angle(id, *target_angle);
                        world.revolute_enable_limit(id, *limit_enabled);
                        world.revolute_set_limits(id, *lower, *upper);
                        world.revolute_enable_motor(id, *motor_enabled);
                        world.revolute_set_motor_speed(id, *motor_speed);
                        world.revolute_set_max_motor_torque(id, *max_motor_torque);
                    }
                }
                JointKind::Weld => {
                    let def = crate::joints::WeldJointDef::new(base);
                    let id = world.create_weld_joint_id(&def);
                    if let Some(JointParams::Weld {
                        linear_hertz,
                        linear_damping_ratio,
                        angular_hertz,
                        angular_damping_ratio,
                    }) = &jr.params
                    {
                        world.weld_set_linear_hertz(id, *linear_hertz);
                        world.weld_set_linear_damping_ratio(id, *linear_damping_ratio);
                        world.weld_set_angular_hertz(id, *angular_hertz);
                        world.weld_set_angular_damping_ratio(id, *angular_damping_ratio);
                    }
                }
                JointKind::Wheel => {
                    let def = crate::joints::WheelJointDef::new(base);
                    let id = world.create_wheel_joint_id(&def);
                    if let Some(JointParams::Wheel {
                        spring_enabled,
                        spring_hertz,
                        spring_damping_ratio,
                        limit_enabled,
                        lower,
                        upper,
                        motor_enabled,
                        motor_speed,
                        max_motor_torque,
                    }) = &jr.params
                    {
                        world.wheel_enable_spring(id, *spring_enabled);
                        world.wheel_set_spring_hertz(id, *spring_hertz);
                        world.wheel_set_spring_damping_ratio(id, *spring_damping_ratio);
                        world.wheel_enable_limit(id, *limit_enabled);
                        world.wheel_set_limits(id, *lower, *upper);
                        world.wheel_enable_motor(id, *motor_enabled);
                        world.wheel_set_motor_speed(id, *motor_speed);
                        world.wheel_set_max_motor_torque(id, *max_motor_torque);
                    }
                }
            }
        }

        world
    }
}

fn body_def_from_runtime(world: &World, id: BodyId) -> crate::body::BodyDef {
    crate::core::debug_checks::assert_body_valid(id);
    // Defaults for flags not queryable via getters
    crate::body::BodyBuilder::new()
        .body_type(crate::body::body_type_impl(id))
        .position(world.body_position(id))
        .angle(crate::body::body_rotation_impl(id).angle())
        .linear_velocity(crate::body::body_linear_velocity_impl(id))
        .angular_velocity(crate::body::body_angular_velocity_impl(id))
        .linear_damping(crate::body::body_linear_damping_impl(id))
        .angular_damping(crate::body::body_angular_damping_impl(id))
        .gravity_scale(crate::body::body_gravity_scale_impl(id))
        .build()
}

fn shapes_from_body(world: &World, body: BodyId) -> Vec<ShapeInstance> {
    crate::core::debug_checks::assert_body_valid(body);
    let mut out = Vec::new();
    for sid in world.body_shapes(body) {
        // Build ShapeDef from runtime properties
        let mut builder = crate::shapes::ShapeDef::builder()
            .material(world.shape_surface_material(sid))
            .density(crate::shapes::shape_density_impl(sid))
            .filter(crate::shapes::shape_filter_impl(sid));
        let is_sensor = crate::shapes::shape_is_sensor_impl(sid);
        if is_sensor {
            builder = builder.sensor(true);
        }
        // Additional flags captured at creation (some flags have no runtime getters).
        #[cfg(feature = "serialize")]
        if let Some(flags) = world.shape_flags(sid) {
            if flags.enable_custom_filtering {
                builder = builder.enable_custom_filtering(true);
            }
            if flags.enable_sensor_events {
                builder = builder.enable_sensor_events(true);
            }
            if flags.enable_contact_events {
                builder = builder.enable_contact_events(true);
            }
            if flags.enable_hit_events {
                builder = builder.enable_hit_events(true);
            }
            if flags.enable_pre_solve_events {
                builder = builder.enable_pre_solve_events(true);
            }
            if flags.invoke_contact_creation {
                builder = builder.invoke_contact_creation(true);
            }
        }
        let sdef = builder.build();
        // geometry
        let geom = match crate::shapes::shape_type_impl(sid) {
            ShapeType::Circle => {
                let c = crate::shapes::shape_circle_impl(sid);
                ShapeGeom::Circle {
                    center: c.center,
                    radius: c.radius,
                }
            }
            ShapeType::Segment => {
                let s = crate::shapes::shape_segment_impl(sid);
                ShapeGeom::Segment {
                    p1: s.point1,
                    p2: s.point2,
                }
            }
            ShapeType::Capsule => {
                let c = crate::shapes::shape_capsule_impl(sid);
                ShapeGeom::Capsule {
                    c1: c.center1,
                    c2: c.center2,
                    radius: c.radius,
                }
            }
            ShapeType::Polygon => {
                let p = crate::shapes::shape_polygon_impl(sid);
                ShapeGeom::Polygon {
                    vertices: p.vertices().to_vec(),
                    radius: p.radius(),
                }
            }
            ShapeType::ChainSegment => {
                // Unsupported shape type (chain segments etc.)
                continue;
            }
        };
        out.push(ShapeInstance {
            def: sdef,
            sensor: is_sensor,
            geom,
        });
    }
    out
}

#[inline]
fn eq_joint(a: JointId, b: JointId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

#[inline]
fn eq_body(a: BodyId, b: BodyId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

fn find_body_index(list: &[BodyId], target: BodyId) -> Option<u32> {
    for (i, &x) in list.iter().enumerate() {
        if eq_body(x, target) {
            return Some(i as u32);
        }
    }
    None
}

fn joint_kind_from_runtime(kind: JointType) -> JointKind {
    match kind {
        JointType::Distance => JointKind::Distance,
        JointType::Filter => JointKind::Filter,
        JointType::Motor => JointKind::Motor,
        JointType::Prismatic => JointKind::Prismatic,
        JointType::Revolute => JointKind::Revolute,
        JointType::Weld => JointKind::Weld,
        JointType::Wheel => JointKind::Wheel,
    }
}

fn joint_params_from_runtime(
    world: &World,
    joint: JointId,
    kind: JointKind,
) -> Option<JointParams> {
    match kind {
        JointKind::Distance => Some(JointParams::Distance {
            length: world.distance_length(joint),
            spring_enabled: world.distance_spring_enabled(joint),
            spring_hertz: world.distance_spring_hertz(joint),
            spring_damping_ratio: world.distance_spring_damping_ratio(joint),
            limit_enabled: world.distance_limit_enabled(joint),
            min_length: world.distance_min_length(joint),
            max_length: world.distance_max_length(joint),
            motor_enabled: world.distance_motor_enabled(joint),
            motor_speed: world.distance_motor_speed(joint),
            max_motor_force: world.distance_max_motor_force(joint),
        }),
        JointKind::Prismatic => Some(JointParams::Prismatic {
            spring_enabled: world.prismatic_spring_enabled(joint),
            spring_hertz: world.prismatic_spring_hertz(joint),
            spring_damping_ratio: world.prismatic_spring_damping_ratio(joint),
            target_translation: world.prismatic_target_translation(joint),
            limit_enabled: world.prismatic_limit_enabled(joint),
            lower: world.prismatic_lower_limit(joint),
            upper: world.prismatic_upper_limit(joint),
            motor_enabled: world.prismatic_motor_enabled(joint),
            motor_speed: world.prismatic_motor_speed(joint),
            max_motor_force: world.prismatic_max_motor_force(joint),
        }),
        JointKind::Revolute => Some(JointParams::Revolute {
            spring_enabled: world.revolute_spring_enabled(joint),
            spring_hertz: world.revolute_spring_hertz(joint),
            spring_damping_ratio: world.revolute_spring_damping_ratio(joint),
            target_angle: world.revolute_target_angle(joint),
            limit_enabled: world.revolute_limit_enabled(joint),
            lower: world.revolute_lower_limit(joint),
            upper: world.revolute_upper_limit(joint),
            motor_enabled: world.revolute_motor_enabled(joint),
            motor_speed: world.revolute_motor_speed(joint),
            max_motor_torque: world.revolute_max_motor_torque(joint),
        }),
        JointKind::Weld => Some(JointParams::Weld {
            linear_hertz: world.weld_linear_hertz(joint),
            linear_damping_ratio: world.weld_linear_damping_ratio(joint),
            angular_hertz: world.weld_angular_hertz(joint),
            angular_damping_ratio: world.weld_angular_damping_ratio(joint),
        }),
        JointKind::Wheel => Some(JointParams::Wheel {
            spring_enabled: world.wheel_spring_enabled(joint),
            spring_hertz: world.wheel_spring_hertz(joint),
            spring_damping_ratio: world.wheel_spring_damping_ratio(joint),
            limit_enabled: world.wheel_limit_enabled(joint),
            lower: world.wheel_lower_limit(joint),
            upper: world.wheel_upper_limit(joint),
            motor_enabled: world.wheel_motor_enabled(joint),
            motor_speed: world.wheel_motor_speed(joint),
            max_motor_torque: world.wheel_max_motor_torque(joint),
        }),
        JointKind::Motor => Some(JointParams::Motor {
            linear_velocity: world.motor_linear_velocity(joint),
            angular_velocity: world.motor_angular_velocity(joint),
            max_velocity_force: world.motor_max_velocity_force(joint),
            max_velocity_torque: world.motor_max_velocity_torque(joint),
            linear_hertz: world.motor_linear_hertz(joint),
            linear_damping_ratio: world.motor_linear_damping_ratio(joint),
            angular_hertz: world.motor_angular_hertz(joint),
            angular_damping_ratio: world.motor_angular_damping_ratio(joint),
            max_spring_force: world.motor_max_spring_force(joint),
            max_spring_torque: world.motor_max_spring_torque(joint),
        }),
        JointKind::Filter => Some(JointParams::Filter {}),
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChainRecord {
    pub body: u32,
    pub is_loop: bool,
    pub filter: crate::filter::Filter,
    pub enable_sensor_events: bool,
    pub points: Vec<Vec2>,
    #[serde(default)]
    pub materials: Option<ChainMaterials>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ChainMaterials {
    Single(crate::shapes::SurfaceMaterial),
    Multiple(Vec<crate::shapes::SurfaceMaterial>),
}
