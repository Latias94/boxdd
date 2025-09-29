//! Serializable snapshots for configs and selected runtime state.
//!
//! This module is only compiled when the `serde` feature is enabled.

#![cfg(feature = "serde")]

use crate::{body::BodyType, types::Vec2, world::World};
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
    pub fn take(world: &World, id: ffi::b2BodyId) -> Self {
        let rot = unsafe { ffi::b2Body_GetRotation(id) };
        Self {
            body_type: match unsafe { ffi::b2Body_GetType(id) } {
                x if x == ffi::b2BodyType_b2_staticBody => BodyType::Static,
                x if x == ffi::b2BodyType_b2_kinematicBody => BodyType::Kinematic,
                _ => BodyType::Dynamic,
            },
            position: world.body_position(id),
            angle: rot.s.atan2(rot.c),
            linear_velocity: Vec2::from(unsafe { ffi::b2Body_GetLinearVelocity(id) }),
            angular_velocity: unsafe { ffi::b2Body_GetAngularVelocity(id) },
            linear_damping: unsafe { ffi::b2Body_GetLinearDamping(id) },
            angular_damping: unsafe { ffi::b2Body_GetAngularDamping(id) },
            gravity_scale: unsafe { ffi::b2Body_GetGravityScale(id) },
        }
    }

    pub fn apply(&self, world: &mut World, id: ffi::b2BodyId) {
        world.set_body_type(id, self.body_type);
        // set transform (position + angle)
        let (s, c) = self.angle.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Vec2 = self.position.into();
        unsafe { ffi::b2Body_SetTransform(id, pos, rot) };
        world.set_body_linear_velocity(id, self.linear_velocity);
        world.set_body_angular_velocity(id, self.angular_velocity);
        unsafe { ffi::b2Body_SetLinearDamping(id, self.linear_damping) };
        unsafe { ffi::b2Body_SetAngularDamping(id, self.angular_damping) };
        unsafe { ffi::b2Body_SetGravityScale(id, self.gravity_scale) };
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
        let cfg = WorldConfigSnapshot::take(world);
        // Build body list from registry (only tracks bodies created via this wrapper)
        let body_ids = world.body_ids();
        let mut bodies = Vec::new();

        for &bid in &body_ids {
            // BodyDef from runtime
            let def = body_def_from_runtime(bid);
            // Optional name
            let name = unsafe {
                let p = ffi::b2Body_GetName(bid);
                if p.is_null() {
                    None
                } else {
                    Some(std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned())
                }
            };
            // Shapes
            let shapes = shapes_from_body(world, bid);
            bodies.push(BodyRecord { def, name, shapes });
        }

        // Gather joints by walking per body and deduping (without Hash/Eq)
        let mut joint_list: Vec<ffi::b2JointId> = Vec::new();
        for &bid in &body_ids {
            let count = unsafe { ffi::b2Body_GetJointCount(bid) }.max(0) as usize;
            if count == 0 {
                continue;
            }
            let mut arr: Vec<ffi::b2JointId> = Vec::with_capacity(count);
            let wrote = unsafe { ffi::b2Body_GetJoints(bid, arr.as_mut_ptr(), count as i32) }.max(0)
                as usize;
            unsafe { arr.set_len(wrote.min(count)) };
            for j in arr {
                if !joint_list.iter().any(|&x| eq_joint(x, j)) {
                    joint_list.push(j);
                }
            }
        }

        let mut joints = Vec::new();
        for j in joint_list {
            let a = unsafe { ffi::b2Joint_GetBodyA(j) };
            let b = unsafe { ffi::b2Joint_GetBodyB(j) };
            let ia = find_body_index(&body_ids, a);
            let ib = find_body_index(&body_ids, b);
            let (Some(ia), Some(ib)) = (ia, ib) else {
                continue;
            };
            let t = unsafe { ffi::b2Joint_GetType(j) };
            let kind = match t {
                x if x == ffi::b2JointType_b2_distanceJoint => JointKind::Distance,
                x if x == ffi::b2JointType_b2_filterJoint => JointKind::Filter,
                x if x == ffi::b2JointType_b2_motorJoint => JointKind::Motor,
                x if x == ffi::b2JointType_b2_prismaticJoint => JointKind::Prismatic,
                x if x == ffi::b2JointType_b2_revoluteJoint => JointKind::Revolute,
                x if x == ffi::b2JointType_b2_weldJoint => JointKind::Weld,
                _ => JointKind::Wheel,
            };
            let la = unsafe { ffi::b2Joint_GetLocalFrameA(j) };
            let lb = unsafe { ffi::b2Joint_GetLocalFrameB(j) };
            // Capture per-type parameters using FFI getters
            let params = match kind {
                JointKind::Distance => Some(JointParams::Distance {
                    length: unsafe { ffi::b2DistanceJoint_GetLength(j) },
                    spring_enabled: unsafe { ffi::b2DistanceJoint_IsSpringEnabled(j) },
                    spring_hertz: unsafe { ffi::b2DistanceJoint_GetSpringHertz(j) },
                    spring_damping_ratio: unsafe { ffi::b2DistanceJoint_GetSpringDampingRatio(j) },
                    limit_enabled: unsafe { ffi::b2DistanceJoint_IsLimitEnabled(j) },
                    min_length: unsafe { ffi::b2DistanceJoint_GetMinLength(j) },
                    max_length: unsafe { ffi::b2DistanceJoint_GetMaxLength(j) },
                    motor_enabled: unsafe { ffi::b2DistanceJoint_IsMotorEnabled(j) },
                    motor_speed: unsafe { ffi::b2DistanceJoint_GetMotorSpeed(j) },
                    max_motor_force: unsafe { ffi::b2DistanceJoint_GetMaxMotorForce(j) },
                }),
                JointKind::Prismatic => Some(JointParams::Prismatic {
                    spring_enabled: unsafe { ffi::b2PrismaticJoint_IsSpringEnabled(j) },
                    spring_hertz: unsafe { ffi::b2PrismaticJoint_GetSpringHertz(j) },
                    spring_damping_ratio: unsafe { ffi::b2PrismaticJoint_GetSpringDampingRatio(j) },
                    target_translation: unsafe { ffi::b2PrismaticJoint_GetTargetTranslation(j) },
                    limit_enabled: unsafe { ffi::b2PrismaticJoint_IsLimitEnabled(j) },
                    lower: unsafe { ffi::b2PrismaticJoint_GetLowerLimit(j) },
                    upper: unsafe { ffi::b2PrismaticJoint_GetUpperLimit(j) },
                    motor_enabled: unsafe { ffi::b2PrismaticJoint_IsMotorEnabled(j) },
                    motor_speed: unsafe { ffi::b2PrismaticJoint_GetMotorSpeed(j) },
                    max_motor_force: unsafe { ffi::b2PrismaticJoint_GetMaxMotorForce(j) },
                }),
                JointKind::Revolute => Some(JointParams::Revolute {
                    spring_enabled: unsafe { ffi::b2RevoluteJoint_IsSpringEnabled(j) },
                    spring_hertz: unsafe { ffi::b2RevoluteJoint_GetSpringHertz(j) },
                    spring_damping_ratio: unsafe { ffi::b2RevoluteJoint_GetSpringDampingRatio(j) },
                    target_angle: unsafe { ffi::b2RevoluteJoint_GetTargetAngle(j) },
                    limit_enabled: unsafe { ffi::b2RevoluteJoint_IsLimitEnabled(j) },
                    lower: unsafe { ffi::b2RevoluteJoint_GetLowerLimit(j) },
                    upper: unsafe { ffi::b2RevoluteJoint_GetUpperLimit(j) },
                    motor_enabled: unsafe { ffi::b2RevoluteJoint_IsMotorEnabled(j) },
                    motor_speed: unsafe { ffi::b2RevoluteJoint_GetMotorSpeed(j) },
                    max_motor_torque: unsafe { ffi::b2RevoluteJoint_GetMaxMotorTorque(j) },
                }),
                JointKind::Weld => Some(JointParams::Weld {
                    linear_hertz: unsafe { ffi::b2WeldJoint_GetLinearHertz(j) },
                    linear_damping_ratio: unsafe { ffi::b2WeldJoint_GetLinearDampingRatio(j) },
                    angular_hertz: unsafe { ffi::b2WeldJoint_GetAngularHertz(j) },
                    angular_damping_ratio: unsafe { ffi::b2WeldJoint_GetAngularDampingRatio(j) },
                }),
                JointKind::Wheel => Some(JointParams::Wheel {
                    spring_enabled: unsafe { ffi::b2WheelJoint_IsSpringEnabled(j) },
                    spring_hertz: unsafe { ffi::b2WheelJoint_GetSpringHertz(j) },
                    spring_damping_ratio: unsafe { ffi::b2WheelJoint_GetSpringDampingRatio(j) },
                    limit_enabled: unsafe { ffi::b2WheelJoint_IsLimitEnabled(j) },
                    lower: unsafe { ffi::b2WheelJoint_GetLowerLimit(j) },
                    upper: unsafe { ffi::b2WheelJoint_GetUpperLimit(j) },
                    motor_enabled: unsafe { ffi::b2WheelJoint_IsMotorEnabled(j) },
                    motor_speed: unsafe { ffi::b2WheelJoint_GetMotorSpeed(j) },
                    max_motor_torque: unsafe { ffi::b2WheelJoint_GetMaxMotorTorque(j) },
                }),
                JointKind::Motor => Some(JointParams::Motor {
                    linear_velocity: Vec2::from(unsafe { ffi::b2MotorJoint_GetLinearVelocity(j) }),
                    angular_velocity: unsafe { ffi::b2MotorJoint_GetAngularVelocity(j) },
                    max_velocity_force: unsafe { ffi::b2MotorJoint_GetMaxVelocityForce(j) },
                    max_velocity_torque: unsafe { ffi::b2MotorJoint_GetMaxVelocityTorque(j) },
                    linear_hertz: unsafe { ffi::b2MotorJoint_GetLinearHertz(j) },
                    linear_damping_ratio: unsafe { ffi::b2MotorJoint_GetLinearDampingRatio(j) },
                    angular_hertz: unsafe { ffi::b2MotorJoint_GetAngularHertz(j) },
                    angular_damping_ratio: unsafe { ffi::b2MotorJoint_GetAngularDampingRatio(j) },
                    max_spring_force: unsafe { ffi::b2MotorJoint_GetMaxSpringForce(j) },
                    max_spring_torque: unsafe { ffi::b2MotorJoint_GetMaxSpringTorque(j) },
                }),
                JointKind::Filter => Some(JointParams::Filter {}),
            };

            joints.push(JointRecord {
                kind,
                body_a: ia,
                body_b: ib,
                local_a: crate::Transform::from(la),
                local_b: crate::Transform::from(lb),
                params,
            });
        }

        // Chains via world registry (ID-style creation only)
        let mut chains: Vec<ChainRecord> = Vec::new();
        for cr in world.chain_records() {
            if let Some(bi) = find_body_index(&body_ids, cr.body) {
                let materials: Option<ChainMaterials> = if cr.materials.is_empty() {
                    None
                } else if cr.materials.len() == 1 {
                    Some(ChainMaterials::Single(crate::shapes::SurfaceMaterial(
                        cr.materials[0],
                    )))
                } else {
                    Some(ChainMaterials::Multiple(
                        cr.materials
                            .iter()
                            .cloned()
                            .map(crate::shapes::SurfaceMaterial)
                            .collect(),
                    ))
                };
                chains.push(ChainRecord {
                    body: bi,
                    is_loop: cr.is_loop,
                    filter: crate::filter::Filter::from(cr.filter),
                    enable_sensor_events: cr.enable_sensor_events,
                    points: cr.points.iter().cloned().map(Vec2::from).collect(),
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
                        let c = ffi::b2Circle {
                            center: (*center).into(),
                            radius: *radius,
                        };
                        let _ = world.create_circle_shape_for(id, def, &c);
                    }
                    ShapeGeom::Segment { p1, p2 } => {
                        let s = ffi::b2Segment {
                            point1: (*p1).into(),
                            point2: (*p2).into(),
                        };
                        let _ = world.create_segment_shape_for(id, def, &s);
                    }
                    ShapeGeom::Capsule { c1, c2, radius } => {
                        let cap = ffi::b2Capsule {
                            center1: (*c1).into(),
                            center2: (*c2).into(),
                            radius: *radius,
                        };
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

fn body_def_from_runtime(id: ffi::b2BodyId) -> crate::body::BodyDef {
    let btype = unsafe { ffi::b2Body_GetType(id) };
    let bt = if btype == ffi::b2BodyType_b2_staticBody {
        BodyType::Static
    } else if btype == ffi::b2BodyType_b2_kinematicBody {
        BodyType::Kinematic
    } else {
        BodyType::Dynamic
    };
    let pos = Vec2::from(unsafe { ffi::b2Body_GetPosition(id) });
    let rot = unsafe { ffi::b2Body_GetRotation(id) };
    let angle = rot.s.atan2(rot.c);
    let linvel = Vec2::from(unsafe { ffi::b2Body_GetLinearVelocity(id) });
    let angvel = unsafe { ffi::b2Body_GetAngularVelocity(id) };
    let lin_damp = unsafe { ffi::b2Body_GetLinearDamping(id) };
    let ang_damp = unsafe { ffi::b2Body_GetAngularDamping(id) };
    let gscale = unsafe { ffi::b2Body_GetGravityScale(id) };
    // Defaults for flags not queryable via getters
    crate::body::BodyBuilder::new()
        .body_type(bt)
        .position(pos)
        .angle(angle)
        .linear_velocity(linvel)
        .angular_velocity(angvel)
        .linear_damping(lin_damp)
        .angular_damping(ang_damp)
        .gravity_scale(gscale)
        .build()
}

fn shapes_from_body(world: &World, body: ffi::b2BodyId) -> Vec<ShapeInstance> {
    let mut out = Vec::new();
    let count = unsafe { ffi::b2Body_GetShapeCount(body) }.max(0) as usize;
    if count == 0 {
        return out;
    }
    let mut arr: Vec<ffi::b2ShapeId> = Vec::with_capacity(count);
    let wrote =
        unsafe { ffi::b2Body_GetShapes(body, arr.as_mut_ptr(), count as i32) }.max(0) as usize;
    unsafe { arr.set_len(wrote.min(count)) };
    for sid in arr {
        let st = unsafe { ffi::b2Shape_GetType(sid) };
        // Build ShapeDef from runtime properties
        let mat = unsafe { ffi::b2Shape_GetSurfaceMaterial(sid) };
        let mut builder = crate::shapes::ShapeDef::builder()
            .material(crate::shapes::SurfaceMaterial(mat))
            .density(unsafe { ffi::b2Shape_GetDensity(sid) })
            .filter_ex(crate::filter::Filter::from(unsafe {
                ffi::b2Shape_GetFilter(sid)
            }));
        let is_sensor = unsafe { ffi::b2Shape_IsSensor(sid) };
        if is_sensor {
            builder = builder.sensor(true);
        }
        // Additional flags captured at creation for ID-style shapes
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
        let geom = if st == ffi::b2ShapeType_b2_circleShape {
            let c = unsafe { ffi::b2Shape_GetCircle(sid) };
            ShapeGeom::Circle {
                center: Vec2::from(c.center),
                radius: c.radius,
            }
        } else if st == ffi::b2ShapeType_b2_segmentShape {
            let s = unsafe { ffi::b2Shape_GetSegment(sid) };
            ShapeGeom::Segment {
                p1: Vec2::from(s.point1),
                p2: Vec2::from(s.point2),
            }
        } else if st == ffi::b2ShapeType_b2_capsuleShape {
            let c = unsafe { ffi::b2Shape_GetCapsule(sid) };
            ShapeGeom::Capsule {
                c1: Vec2::from(c.center1),
                c2: Vec2::from(c.center2),
                radius: c.radius,
            }
        } else if st == ffi::b2ShapeType_b2_polygonShape {
            let p = unsafe { ffi::b2Shape_GetPolygon(sid) };
            let mut verts: Vec<Vec2> = Vec::new();
            let n = (p.count as usize).min(8);
            for i in 0..n {
                verts.push(Vec2::from(p.vertices[i]));
            }
            ShapeGeom::Polygon {
                vertices: verts,
                radius: p.radius,
            }
        } else {
            // Unsupported shape type (chain segments etc.)
            continue;
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
fn eq_joint(a: ffi::b2JointId, b: ffi::b2JointId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

#[inline]
fn eq_body(a: ffi::b2BodyId, b: ffi::b2BodyId) -> bool {
    a.index1 == b.index1 && a.world0 == b.world0 && a.generation == b.generation
}

fn find_body_index(list: &[ffi::b2BodyId], target: ffi::b2BodyId) -> Option<u32> {
    for (i, &x) in list.iter().enumerate() {
        if eq_body(x, target) {
            return Some(i as u32);
        }
    }
    None
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
