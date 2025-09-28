//! Joint builders and creation helpers.
//!
//! Two creation styles are available:
//! - RAII wrappers: `World::create_*_joint(&def) -> Joint` returning a scoped wrapper that destroys
//!   the underlying joint on drop.
//! - ID style: `World::create_*_joint_id(&def) -> b2JointId` returning the raw id for storage.
//!
//! The `World` convenience builders (`revolute`, `prismatic`, `wheel`, `distance`, `weld`,
//! `motor_joint`, `filter_joint`) help compose joints in world space and build local frames
//! from world anchors/axes.
use std::marker::PhantomData;

use crate::body::Body;
use crate::types::{BodyId, JointId};
use crate::world::World;
use boxdd_sys::ffi;

/// A joint owned by a world; drops by destroying the underlying joint.
pub struct Joint<'w> {
    pub(crate) id: ffi::b2JointId,
    _world: PhantomData<&'w World>,
}

impl<'w> Joint<'w> {
    pub fn id(&self) -> JointId {
        self.id
    }
    pub fn linear_separation(&self) -> f32 {
        unsafe { ffi::b2Joint_GetLinearSeparation(self.id) }
    }
    pub fn angular_separation(&self) -> f32 {
        unsafe { ffi::b2Joint_GetAngularSeparation(self.id) }
    }
}

impl<'w> Drop for Joint<'w> {
    fn drop(&mut self) {
        if unsafe { ffi::b2Joint_IsValid(self.id) } {
            unsafe { ffi::b2DestroyJoint(self.id, true) };
        }
    }
}

/// Base joint definition builder for common properties.
///
/// This configures `b2JointDef` fields shared by all joint types. Typically
/// you construct a specific joint def (e.g. `RevoluteJointDef`) with this as
/// its `base`.
#[derive(Clone, Debug)]
pub struct JointBase(pub(crate) ffi::b2JointDef);

impl Default for JointBase {
    fn default() -> Self {
        // No default constructor provided for b2JointDef; zero is OK for POD and we'll set fields explicitly.
        // Use identity frames by default.
        let mut base: ffi::b2JointDef = unsafe { core::mem::zeroed() };
        base.drawScale = 1.0;
        Self(base)
    }
}

#[derive(Clone, Debug)]
pub struct JointBaseBuilder {
    base: JointBase,
}

impl JointBaseBuilder {
    /// Create a new base with identity local frames.
    pub fn new() -> Self {
        Self {
            base: JointBase::default(),
        }
    }
    /// Attach two bodies using RAII wrappers.
    pub fn bodies<'w>(mut self, a: &Body<'w>, b: &Body<'w>) -> Self {
        self.base.0.bodyIdA = a.id;
        self.base.0.bodyIdB = b.id;
        self
    }
    /// Attach two bodies by raw ids.
    pub fn bodies_by_id(mut self, a: BodyId, b: BodyId) -> Self {
        self.base.0.bodyIdA = a;
        self.base.0.bodyIdB = b;
        self
    }
    /// Set local frames from positions and angles (radians).
    pub fn local_frames<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        pos_a: VA,
        angle_a: f32,
        pos_b: VB,
        angle_b: f32,
    ) -> Self {
        let (sa, ca) = angle_a.sin_cos();
        let (sb, cb) = angle_b.sin_cos();
        self.base.0.localFrameA = ffi::b2Transform {
            p: ffi::b2Vec2::from(pos_a.into()),
            q: ffi::b2Rot { c: ca, s: sa },
        };
        self.base.0.localFrameB = ffi::b2Transform {
            p: ffi::b2Vec2::from(pos_b.into()),
            q: ffi::b2Rot { c: cb, s: sb },
        };
        self
    }
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.base.0.collideConnected = flag;
        self
    }
    /// Force threshold for joint events.
    pub fn force_threshold(mut self, v: f32) -> Self {
        self.base.0.forceThreshold = v;
        self
    }
    /// Torque threshold for joint events.
    pub fn torque_threshold(mut self, v: f32) -> Self {
        self.base.0.torqueThreshold = v;
        self
    }
    /// Advanced constraint tuning frequency in Hertz.
    pub fn constraint_hertz(mut self, v: f32) -> Self {
        self.base.0.constraintHertz = v;
        self
    }
    /// Advanced constraint damping ratio.
    pub fn constraint_damping_ratio(mut self, v: f32) -> Self {
        self.base.0.constraintDampingRatio = v;
        self
    }
    pub fn draw_scale(mut self, v: f32) -> Self {
        self.base.0.drawScale = v;
        self
    }
    pub fn local_frames_raw(mut self, a: ffi::b2Transform, b: ffi::b2Transform) -> Self {
        self.base.0.localFrameA = a;
        self.base.0.localFrameB = b;
        self
    }
    /// Set local anchor positions from world points (rotation remains identity).
    pub fn local_points_from_world<'w, V: Into<crate::types::Vec2>>(
        mut self,
        body_a: &Body<'w>,
        world_a: V,
        body_b: &Body<'w>,
        world_b: V,
    ) -> Self {
        let ta = body_a.transform();
        let tb = body_b.transform();
        let wa: ffi::b2Vec2 = world_a.into().into();
        let wb: ffi::b2Vec2 = world_b.into().into();
        let la = crate::core::math::world_to_local_point(ta.into(), wa);
        let lb = crate::core::math::world_to_local_point(tb.into(), wb);
        let ident = ffi::b2Transform {
            p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            q: ffi::b2Rot { c: 1.0, s: 0.0 },
        };
        let mut fa = ident;
        let mut fb = ident;
        fa.p = la;
        fb.p = lb;
        self.base.0.localFrameA = fa;
        self.base.0.localFrameB = fb;
        self
    }
    pub fn build(self) -> JointBase {
        self.base
    }

    /// Set local frames using world anchors and a shared world axis (X-axis of joint frame).
    /// This computes localFrameA/B.rotation so that their X-axis aligns with the given world axis,
    /// and localFrameA/B.position to the given world anchor points.
    pub fn frames_from_world_with_axis<'w, VA, VB, AX>(
        mut self,
        body_a: &Body<'w>,
        anchor_a_world: VA,
        axis_world: AX,
        body_b: &Body<'w>,
        anchor_b_world: VB,
    ) -> Self
    where
        VA: Into<crate::types::Vec2>,
        VB: Into<crate::types::Vec2>,
        AX: Into<crate::types::Vec2>,
    {
        let ta = body_a.transform();
        let tb = body_b.transform();
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis_w: ffi::b2Vec2 = axis_world.into().into();
        // Local frames: positions from anchors, rotations from world axis
        let la = crate::core::math::world_to_local_point(ta.into(), wa);
        let lb = crate::core::math::world_to_local_point(tb.into(), wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta.into(), axis_w);
        let rb = crate::core::math::world_axis_to_local_rot(tb.into(), axis_w);
        self.base.0.localFrameA = ffi::b2Transform { p: la, q: ra };
        self.base.0.localFrameB = ffi::b2Transform { p: lb, q: rb };
        self
    }
}

// Distance joint
#[derive(Clone, Debug)]
pub struct DistanceJointDef(pub(crate) ffi::b2DistanceJointDef);

impl DistanceJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2DistanceJointDef = unsafe { ffi::b2DefaultDistanceJointDef() };
        def.base = base.0;
        Self(def)
    }
    pub fn length(mut self, v: f32) -> Self {
        self.0.length = v;
        self
    }
    pub fn enable_spring(mut self, flag: bool) -> Self {
        self.0.enableSpring = flag;
        self
    }
    pub fn lower_spring_force(mut self, v: f32) -> Self {
        self.0.lowerSpringForce = v;
        self
    }
    pub fn upper_spring_force(mut self, v: f32) -> Self {
        self.0.upperSpringForce = v;
        self
    }
    pub fn hertz(mut self, v: f32) -> Self {
        self.0.hertz = v;
        self
    }
    pub fn damping_ratio(mut self, v: f32) -> Self {
        self.0.dampingRatio = v;
        self
    }
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.0.enableLimit = flag;
        self
    }
    pub fn min_length(mut self, v: f32) -> Self {
        self.0.minLength = v;
        self
    }
    pub fn max_length(mut self, v: f32) -> Self {
        self.0.maxLength = v;
        self
    }
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.0.enableMotor = flag;
        self
    }
    pub fn max_motor_force(mut self, v: f32) -> Self {
        self.0.maxMotorForce = v;
        self
    }
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.0.motorSpeed = v;
        self
    }

    /// Convenience: compute length from two world points.
    pub fn length_from_world_points<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Self {
        let a: ffi::b2Vec2 = a.into().into();
        let b: ffi::b2Vec2 = b.into().into();
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        self.0.length = (dx * dx + dy * dy).sqrt();
        self
    }
}

// Revolute joint
#[derive(Clone, Debug)]
pub struct RevoluteJointDef(pub(crate) ffi::b2RevoluteJointDef);

impl RevoluteJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2RevoluteJointDef = unsafe { ffi::b2DefaultRevoluteJointDef() };
        def.base = base.0;
        Self(def)
    }
    pub fn target_angle(mut self, v: f32) -> Self {
        self.0.targetAngle = v;
        self
    }
    pub fn enable_spring(mut self, flag: bool) -> Self {
        self.0.enableSpring = flag;
        self
    }
    pub fn hertz(mut self, v: f32) -> Self {
        self.0.hertz = v;
        self
    }
    pub fn damping_ratio(mut self, v: f32) -> Self {
        self.0.dampingRatio = v;
        self
    }
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.0.enableLimit = flag;
        self
    }
    pub fn lower_angle(mut self, v: f32) -> Self {
        self.0.lowerAngle = v;
        self
    }
    pub fn upper_angle(mut self, v: f32) -> Self {
        self.0.upperAngle = v;
        self
    }
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.0.enableMotor = flag;
        self
    }
    pub fn max_motor_torque(mut self, v: f32) -> Self {
        self.0.maxMotorTorque = v;
        self
    }
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.0.motorSpeed = v;
        self
    }

    /// Convenience: set angular limits in degrees.
    pub fn limit_deg(mut self, lower_deg: f32, upper_deg: f32) -> Self {
        let to_rad = core::f32::consts::PI / 180.0;
        self.0.lowerAngle = lower_deg * to_rad;
        self.0.upperAngle = upper_deg * to_rad;
        self.0.enableLimit = true;
        self
    }
    /// Convenience: motor speed in degrees/sec.
    pub fn motor_speed_deg(mut self, speed_deg_per_s: f32) -> Self {
        self.0.motorSpeed = speed_deg_per_s * (core::f32::consts::PI / 180.0);
        self
    }
}

// Prismatic joint
#[derive(Clone, Debug)]
pub struct PrismaticJointDef(pub(crate) ffi::b2PrismaticJointDef);

impl PrismaticJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2PrismaticJointDef = unsafe { ffi::b2DefaultPrismaticJointDef() };
        def.base = base.0;
        Self(def)
    }
    pub fn enable_spring(mut self, flag: bool) -> Self {
        self.0.enableSpring = flag;
        self
    }
    pub fn hertz(mut self, v: f32) -> Self {
        self.0.hertz = v;
        self
    }
    pub fn damping_ratio(mut self, v: f32) -> Self {
        self.0.dampingRatio = v;
        self
    }
    pub fn target_translation(mut self, v: f32) -> Self {
        self.0.targetTranslation = v;
        self
    }
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.0.enableLimit = flag;
        self
    }
    pub fn lower_translation(mut self, v: f32) -> Self {
        self.0.lowerTranslation = v;
        self
    }
    pub fn upper_translation(mut self, v: f32) -> Self {
        self.0.upperTranslation = v;
        self
    }
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.0.enableMotor = flag;
        self
    }
    pub fn max_motor_force(mut self, v: f32) -> Self {
        self.0.maxMotorForce = v;
        self
    }
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.0.motorSpeed = v;
        self
    }
}

// Wheel joint
#[derive(Clone, Debug)]
pub struct WheelJointDef(pub(crate) ffi::b2WheelJointDef);

impl WheelJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2WheelJointDef = unsafe { ffi::b2DefaultWheelJointDef() };
        def.base = base.0;
        Self(def)
    }
    pub fn enable_spring(mut self, flag: bool) -> Self {
        self.0.enableSpring = flag;
        self
    }
    pub fn hertz(mut self, v: f32) -> Self {
        self.0.hertz = v;
        self
    }
    pub fn damping_ratio(mut self, v: f32) -> Self {
        self.0.dampingRatio = v;
        self
    }
    pub fn enable_limit(mut self, flag: bool) -> Self {
        self.0.enableLimit = flag;
        self
    }
    pub fn lower_translation(mut self, v: f32) -> Self {
        self.0.lowerTranslation = v;
        self
    }
    pub fn upper_translation(mut self, v: f32) -> Self {
        self.0.upperTranslation = v;
        self
    }
    pub fn enable_motor(mut self, flag: bool) -> Self {
        self.0.enableMotor = flag;
        self
    }
    pub fn max_motor_torque(mut self, v: f32) -> Self {
        self.0.maxMotorTorque = v;
        self
    }
    pub fn motor_speed(mut self, v: f32) -> Self {
        self.0.motorSpeed = v;
        self
    }
    /// Convenience: motor speed in degrees/sec.
    pub fn motor_speed_deg(mut self, speed_deg_per_s: f32) -> Self {
        self.0.motorSpeed = speed_deg_per_s * (core::f32::consts::PI / 180.0);
        self
    }
}

// Weld joint
#[derive(Clone, Debug)]
pub struct WeldJointDef(pub(crate) ffi::b2WeldJointDef);

impl WeldJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2WeldJointDef = unsafe { ffi::b2DefaultWeldJointDef() };
        def.base = base.0;
        Self(def)
    }
    pub fn linear_hertz(mut self, v: f32) -> Self {
        self.0.linearHertz = v;
        self
    }
    pub fn angular_hertz(mut self, v: f32) -> Self {
        self.0.angularHertz = v;
        self
    }
    pub fn linear_damping_ratio(mut self, v: f32) -> Self {
        self.0.linearDampingRatio = v;
        self
    }
    pub fn angular_damping_ratio(mut self, v: f32) -> Self {
        self.0.angularDampingRatio = v;
        self
    }
}

// Motor joint
#[derive(Clone, Debug)]
pub struct MotorJointDef(pub(crate) ffi::b2MotorJointDef);

impl MotorJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2MotorJointDef = unsafe { ffi::b2DefaultMotorJointDef() };
        def.base = base.0;
        Self(def)
    }
    pub fn linear_velocity<V: Into<crate::types::Vec2>>(mut self, v: V) -> Self {
        self.0.linearVelocity = ffi::b2Vec2::from(v.into());
        self
    }
    pub fn max_velocity_force(mut self, v: f32) -> Self {
        self.0.maxVelocityForce = v;
        self
    }
    pub fn angular_velocity(mut self, v: f32) -> Self {
        self.0.angularVelocity = v;
        self
    }
    pub fn max_velocity_torque(mut self, v: f32) -> Self {
        self.0.maxVelocityTorque = v;
        self
    }
    pub fn linear_hertz(mut self, v: f32) -> Self {
        self.0.linearHertz = v;
        self
    }
    pub fn linear_damping_ratio(mut self, v: f32) -> Self {
        self.0.linearDampingRatio = v;
        self
    }
    pub fn max_spring_force(mut self, v: f32) -> Self {
        self.0.maxSpringForce = v;
        self
    }
    pub fn angular_hertz(mut self, v: f32) -> Self {
        self.0.angularHertz = v;
        self
    }
    pub fn angular_damping_ratio(mut self, v: f32) -> Self {
        self.0.angularDampingRatio = v;
        self
    }
    pub fn max_spring_torque(mut self, v: f32) -> Self {
        self.0.maxSpringTorque = v;
        self
    }
}

// Filter joint (no params beyond base)
#[derive(Clone, Debug)]
pub struct FilterJointDef(pub(crate) ffi::b2FilterJointDef);

impl FilterJointDef {
    pub fn new(base: JointBase) -> Self {
        let mut def: ffi::b2FilterJointDef = unsafe { ffi::b2DefaultFilterJointDef() };
        def.base = base.0;
        Self(def)
    }
}

impl World {
    pub fn create_wheel_joint<'w>(&'w mut self, def: &WheelJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateWheelJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_weld_joint<'w>(&'w mut self, def: &WeldJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateWeldJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_motor_joint<'w>(&'w mut self, def: &MotorJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateMotorJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_filter_joint<'w>(&'w mut self, def: &FilterJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateFilterJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
}

/// Examples
///
/// Distance joint (ID-style, no ffi)
/// ```no_run
/// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, DistanceJointDef, Vec2};
/// let mut world = World::new(WorldDef::builder().gravity([0.0, -9.8]).build()).unwrap();
/// let a = world.create_body_id(BodyBuilder::new().position([-1.0, 2.0]).build());
/// let b = world.create_body_id(BodyBuilder::new().position([ 1.0, 2.0]).build());
/// let sdef = ShapeDef::builder().density(1.0).build();
/// let _sa = world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5, 0.5));
/// let _sb = world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));
/// // Build local frames from world anchors via helper
/// let wa = Vec2::new(-1.0, 2.0); // anchor on A (world)
/// let wb = Vec2::new( 1.0, 2.0); // anchor on B (world)
/// let base = world.joint_base_from_world_points(a, b, wa, wb);
/// let ddef = DistanceJointDef::new(base)
///     .enable_spring(true)
///     .hertz(4.0)
///     .damping_ratio(0.7)
///     .length_from_world_points(wa, wb);
/// let _jid = world.create_distance_joint_id(&ddef);
/// ```
///
/// Revolute joint with limits and motor (ID-style, no ffi)
/// ```no_run
/// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, RevoluteJointDef, Vec2};
/// let mut world = World::new(WorldDef::builder().gravity([0.0, -9.8]).build()).unwrap();
/// let a = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
/// let b = world.create_body_id(BodyBuilder::new().position([1.0, 2.0]).build());
/// let sdef = ShapeDef::builder().density(1.0).build();
/// let _sa = world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5, 0.5));
/// let _sb = world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));
/// // World anchor at A's position
/// let wa = world.body_position(a);
/// let base = world.joint_base_from_world_points(a, b, wa, wa);
/// let rdef = RevoluteJointDef::new(base)
///     .limit_deg(-30.0, 30.0)
///     .enable_motor(true)
///     .motor_speed_deg(90.0)
///     .max_motor_torque(10.0);
/// let _jid = world.create_revolute_joint_id(&rdef);
/// ```

// Convenience builders
/// Builder for a revolute (hinge) joint in world space.
///
/// Configure anchors/limits/motor and finish with `build()` to create the joint.
///
/// Example
/// ```no_run
/// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
/// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
/// let a = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
/// let b = world.create_body_id(BodyBuilder::new().position([1.0, 2.0]).build());
/// let sdef = ShapeDef::builder().density(1.0).build();
/// world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5,0.5));
/// world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5,0.5));
/// let mut joint = world.revolute(a, b)
///     .anchor_world(world.body_position(a))
///     .with_limit_and_motor_deg(-30.0, 30.0, 10.0, 90.0)
///     .build();
/// # let _ = joint.id();
/// ```
pub struct RevoluteJointBuilder<'w> {
    world: &'w mut World,
    body_a: ffi::b2BodyId,
    body_b: ffi::b2BodyId,
    anchor_world: Option<ffi::b2Vec2>,
    def: RevoluteJointDef,
}

impl<'w> RevoluteJointBuilder<'w> {
    /// Set world-space anchor (defaults to body A position).
    pub fn anchor_world<V: Into<crate::types::Vec2>>(mut self, a: V) -> Self {
        self.anchor_world = Some(ffi::b2Vec2::from(a.into()));
        self
    }
    pub fn limit(mut self, lower_rad: f32, upper_rad: f32) -> Self {
        self.def = self
            .def
            .enable_limit(true)
            .lower_angle(lower_rad)
            .upper_angle(upper_rad);
        self
    }
    pub fn limit_deg(mut self, lower_deg: f32, upper_deg: f32) -> Self {
        self.def = self.def.limit_deg(lower_deg, upper_deg);
        self
    }
    pub fn motor(mut self, max_torque: f32, speed_rad_per_s: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_torque(max_torque)
            .motor_speed(speed_rad_per_s);
        self
    }
    pub fn motor_deg(mut self, max_torque: f32, speed_deg_per_s: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_torque(max_torque)
            .motor_speed_deg(speed_deg_per_s);
        self
    }
    /// Enable spring with given `hertz` and `damping_ratio`.
    pub fn spring(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .enable_spring(true)
            .hertz(hertz)
            .damping_ratio(damping_ratio);
        self
    }
    pub fn target_angle(mut self, radians: f32) -> Self {
        self.def = self.def.target_angle(radians);
        self
    }
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }

    // Combos
    pub fn with_limit_and_motor(
        mut self,
        lower_rad: f32,
        upper_rad: f32,
        max_torque: f32,
        speed_rad_per_s: f32,
    ) -> Self {
        self = self.limit(lower_rad, upper_rad);
        self = self.motor(max_torque, speed_rad_per_s);
        self
    }
    pub fn with_limit_and_motor_deg(
        mut self,
        lower_deg: f32,
        upper_deg: f32,
        max_torque: f32,
        speed_deg_per_s: f32,
    ) -> Self {
        self = self.limit_deg(lower_deg, upper_deg);
        self = self.motor_deg(max_torque, speed_deg_per_s);
        self
    }
    pub fn with_limit_and_spring(
        mut self,
        lower_rad: f32,
        upper_rad: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(lower_rad, upper_rad);
        self = self.spring(hertz, damping_ratio);
        self
    }
    pub fn with_motor_and_spring(
        mut self,
        max_torque: f32,
        speed_rad_per_s: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.motor(max_torque, speed_rad_per_s);
        self = self.spring(hertz, damping_ratio);
        self
    }
    pub fn with_motor_and_spring_deg(
        mut self,
        max_torque: f32,
        speed_deg_per_s: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.motor_deg(max_torque, speed_deg_per_s);
        self = self.spring(hertz, damping_ratio);
        self
    }
    pub fn with_limit_motor_spring(
        mut self,
        lower_rad: f32,
        upper_rad: f32,
        max_torque: f32,
        speed_rad_per_s: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(lower_rad, upper_rad);
        self = self.motor(max_torque, speed_rad_per_s);
        self = self.spring(hertz, damping_ratio);
        self
    }
    pub fn with_limit_motor_spring_deg(
        mut self,
        lower_deg: f32,
        upper_deg: f32,
        max_torque: f32,
        speed_deg_per_s: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit_deg(lower_deg, upper_deg);
        self = self.motor_deg(max_torque, speed_deg_per_s);
        self = self.spring(hertz, damping_ratio);
        self
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        // Default anchor = body A position
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_world.unwrap_or(ta.p);
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, aw);
        let base = JointBaseBuilder::new()
            .bodies_by_id(self.body_a, self.body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
            )
            .build();
        self.def.0.base = base.0;
        self.world.create_revolute_joint(&self.def)
    }
}
pub struct PrismaticJointBuilder<'w> {
    world: &'w mut World,
    body_a: ffi::b2BodyId,
    body_b: ffi::b2BodyId,
    anchor_a_world: Option<ffi::b2Vec2>,
    anchor_b_world: Option<ffi::b2Vec2>,
    axis_world: Option<ffi::b2Vec2>,
    def: PrismaticJointDef,
}

impl<'w> PrismaticJointBuilder<'w> {
    /// Set world-space anchors for A and B.
    pub fn anchors_world<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Self {
        self.anchor_a_world = Some(ffi::b2Vec2::from(a.into()));
        self.anchor_b_world = Some(ffi::b2Vec2::from(b.into()));
        self
    }
    /// Set world-space axis this joint slides along.
    pub fn axis_world<V: Into<crate::types::Vec2>>(mut self, axis: V) -> Self {
        self.axis_world = Some(ffi::b2Vec2::from(axis.into()));
        self
    }
    pub fn limit(mut self, lower: f32, upper: f32) -> Self {
        self.def = self
            .def
            .enable_limit(true)
            .lower_translation(lower)
            .upper_translation(upper);
        self
    }
    pub fn motor(mut self, max_force: f32, speed: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_force(max_force)
            .motor_speed(speed);
        self
    }
    pub fn spring(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .enable_spring(true)
            .hertz(hertz)
            .damping_ratio(damping_ratio);
        self
    }
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }

    /// Convenience: enable limit and motor together.
    pub fn with_limit_and_motor(
        mut self,
        lower: f32,
        upper: f32,
        max_force: f32,
        speed: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.motor(max_force, speed);
        self
    }
    /// Convenience: enable limit and spring together.
    pub fn with_limit_and_spring(
        mut self,
        lower: f32,
        upper: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Convenience: enable motor and spring together.
    pub fn with_motor_and_spring(
        mut self,
        max_force: f32,
        speed: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.motor(max_force, speed);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Convenience: enable limit, motor, and spring together.
    pub fn with_limit_motor_spring(
        mut self,
        lower: f32,
        upper: f32,
        max_force: f32,
        speed: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.motor(max_force, speed);
        self = self.spring(hertz, damping_ratio);
        self
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        // Defaults: anchors = body positions, axis = x
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let axis = self.axis_world.unwrap_or(ffi::b2Vec2 { x: 1.0, y: 0.0 });
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        let base = JointBaseBuilder::new()
            .bodies_by_id(self.body_a, self.body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build();
        self.def.0.base = base.0;
        self.world.create_prismatic_joint(&self.def)
    }
}

pub struct WheelJointBuilder<'w> {
    world: &'w mut World,
    body_a: BodyId,
    body_b: BodyId,
    anchor_a_world: Option<ffi::b2Vec2>,
    anchor_b_world: Option<ffi::b2Vec2>,
    axis_world: Option<ffi::b2Vec2>,
    def: WheelJointDef,
}

impl<'w> WheelJointBuilder<'w> {
    /// Set world-space anchors for A and B.
    pub fn anchors_world<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Self {
        self.anchor_a_world = Some(ffi::b2Vec2::from(a.into()));
        self.anchor_b_world = Some(ffi::b2Vec2::from(b.into()));
        self
    }
    /// Set wheel axis in world space.
    pub fn axis_world<V: Into<crate::types::Vec2>>(mut self, axis: V) -> Self {
        self.axis_world = Some(ffi::b2Vec2::from(axis.into()));
        self
    }
    pub fn limit(mut self, lower: f32, upper: f32) -> Self {
        self.def = self
            .def
            .enable_limit(true)
            .lower_translation(lower)
            .upper_translation(upper);
        self
    }
    pub fn motor(mut self, max_torque: f32, speed: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_torque(max_torque)
            .motor_speed(speed);
        self
    }
    pub fn motor_deg(mut self, max_torque: f32, speed_deg: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_torque(max_torque)
            .motor_speed_deg(speed_deg);
        self
    }
    pub fn spring(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .enable_spring(true)
            .hertz(hertz)
            .damping_ratio(damping_ratio);
        self
    }
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }

    /// Convenience: enable limit and motor together.
    pub fn with_limit_and_motor(
        mut self,
        lower: f32,
        upper: f32,
        max_torque: f32,
        speed: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.motor(max_torque, speed);
        self
    }
    /// Convenience: enable limit and motor together (motor speed in degrees/sec).
    pub fn with_limit_and_motor_deg(
        mut self,
        lower: f32,
        upper: f32,
        max_torque: f32,
        speed_deg: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.motor_deg(max_torque, speed_deg);
        self
    }
    /// Convenience: enable limit and spring together.
    pub fn with_limit_and_spring(
        mut self,
        lower: f32,
        upper: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Convenience: enable motor and spring together.
    pub fn with_motor_and_spring(
        mut self,
        max_torque: f32,
        speed: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.motor(max_torque, speed);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Convenience: enable motor and spring together (motor speed in degrees/sec).
    pub fn with_motor_and_spring_deg(
        mut self,
        max_torque: f32,
        speed_deg: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.motor_deg(max_torque, speed_deg);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Convenience: enable limit, motor, and spring together.
    pub fn with_limit_motor_spring(
        mut self,
        lower: f32,
        upper: f32,
        max_torque: f32,
        speed: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.motor(max_torque, speed);
        self = self.spring(hertz, damping_ratio);
        self
    }
    /// Convenience: enable limit, motor (deg/s), and spring together.
    pub fn with_limit_motor_spring_deg(
        mut self,
        lower: f32,
        upper: f32,
        max_torque: f32,
        speed_deg: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(lower, upper);
        self = self.motor_deg(max_torque, speed_deg);
        self = self.spring(hertz, damping_ratio);
        self
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        // Defaults: anchors = body positions, axis = x
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let axis = self.axis_world.unwrap_or(ffi::b2Vec2 { x: 1.0, y: 0.0 });
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        let base = JointBaseBuilder::new()
            .bodies_by_id(self.body_a, self.body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build();
        self.def.0.base = base.0;
        self.world.create_wheel_joint(&self.def)
    }
}

impl World {
    pub fn revolute<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> RevoluteJointBuilder<'w> {
        RevoluteJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_world: None,
            def: RevoluteJointDef::new(JointBase::default()),
        }
    }
    pub fn prismatic<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> PrismaticJointBuilder<'w> {
        PrismaticJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: PrismaticJointDef::new(JointBase::default()),
        }
    }
    pub fn wheel<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WheelJointBuilder<'w> {
        WheelJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: WheelJointDef::new(JointBase::default()),
        }
    }
}

// Distance joint convenience builder
pub struct DistanceJointBuilder<'w> {
    world: &'w mut World,
    body_a: BodyId,
    body_b: BodyId,
    anchor_a_world: Option<ffi::b2Vec2>,
    anchor_b_world: Option<ffi::b2Vec2>,
    def: DistanceJointDef,
}

impl<'w> DistanceJointBuilder<'w> {
    /// Set world-space anchors for A and B.
    pub fn anchors_world<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Self {
        self.anchor_a_world = Some(ffi::b2Vec2::from(a.into()));
        self.anchor_b_world = Some(ffi::b2Vec2::from(b.into()));
        self
    }
    /// Set desired distance (meters).
    pub fn length(mut self, len: f32) -> Self {
        self.def = self.def.length(len);
        self
    }
    /// Compute desired distance from two world points.
    pub fn length_from_world_points<VA: Into<crate::types::Vec2>, VB: Into<crate::types::Vec2>>(
        mut self,
        a: VA,
        b: VB,
    ) -> Self {
        self.def = self
            .def
            .length_from_world_points(ffi::b2Vec2::from(a.into()), ffi::b2Vec2::from(b.into()));
        self
    }
    pub fn limit(mut self, min_len: f32, max_len: f32) -> Self {
        self.def = self
            .def
            .enable_limit(true)
            .min_length(min_len)
            .max_length(max_len);
        self
    }
    pub fn motor(mut self, max_force: f32, speed: f32) -> Self {
        self.def = self
            .def
            .enable_motor(true)
            .max_motor_force(max_force)
            .motor_speed(speed);
        self
    }
    pub fn spring(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .enable_spring(true)
            .hertz(hertz)
            .damping_ratio(damping_ratio);
        self
    }
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }

    pub fn with_limit_and_motor(
        mut self,
        min_len: f32,
        max_len: f32,
        max_force: f32,
        speed: f32,
    ) -> Self {
        self = self.limit(min_len, max_len);
        self = self.motor(max_force, speed);
        self
    }
    pub fn with_limit_and_spring(
        mut self,
        min_len: f32,
        max_len: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(min_len, max_len);
        self = self.spring(hertz, damping_ratio);
        self
    }
    pub fn with_motor_and_spring(
        mut self,
        max_force: f32,
        speed: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.motor(max_force, speed);
        self = self.spring(hertz, damping_ratio);
        self
    }
    pub fn with_limit_motor_spring(
        mut self,
        min_len: f32,
        max_len: f32,
        max_force: f32,
        speed: f32,
        hertz: f32,
        damping_ratio: f32,
    ) -> Self {
        self = self.limit(min_len, max_len);
        self = self.motor(max_force, speed);
        self = self.spring(hertz, damping_ratio);
        self
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        // Compute frames from anchors; default to body positions
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_a_world.unwrap_or(ta.p);
        let bw = self.anchor_b_world.unwrap_or(tb.p);
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, bw);
        let base = JointBaseBuilder::new()
            .bodies_by_id(self.body_a, self.body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
            )
            .build();
        self.def.0.base = base.0;
        self.world.create_distance_joint(&self.def)
    }
}

// Weld joint convenience builder
pub struct WeldJointBuilder<'w> {
    world: &'w mut World,
    body_a: BodyId,
    body_b: BodyId,
    anchor_world: Option<ffi::b2Vec2>,
    def: WeldJointDef,
}

impl<'w> WeldJointBuilder<'w> {
    /// Set world-space anchor (defaults to body A position).
    pub fn anchor_world<V: Into<crate::types::Vec2>>(mut self, a: V) -> Self {
        self.anchor_world = Some(ffi::b2Vec2::from(a.into()));
        self
    }
    pub fn linear_stiffness(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .linear_hertz(hertz)
            .linear_damping_ratio(damping_ratio);
        self
    }
    pub fn angular_stiffness(mut self, hertz: f32, damping_ratio: f32) -> Self {
        self.def = self
            .def
            .angular_hertz(hertz)
            .angular_damping_ratio(damping_ratio);
        self
    }
    pub fn with_stiffness(
        mut self,
        linear_hz: f32,
        linear_dr: f32,
        angular_hz: f32,
        angular_dr: f32,
    ) -> Self {
        self = self.linear_stiffness(linear_hz, linear_dr);
        self = self.angular_stiffness(angular_hz, angular_dr);
        self
    }
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        let ta = unsafe { ffi::b2Body_GetTransform(self.body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(self.body_b) };
        let aw = self.anchor_world.unwrap_or(ta.p);
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, aw);
        let base = JointBaseBuilder::new()
            .bodies_by_id(self.body_a, self.body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: 1.0, s: 0.0 },
                },
            )
            .build();
        self.def.0.base = base.0;
        self.world.create_weld_joint(&self.def)
    }
}

// Motor joint convenience builder
pub struct MotorJointBuilder<'w> {
    world: &'w mut World,
    body_a: BodyId,
    body_b: BodyId,
    def: MotorJointDef,
}

impl<'w> MotorJointBuilder<'w> {
    pub fn linear_velocity<V: Into<crate::types::Vec2>>(mut self, v: V) -> Self {
        self.def = self.def.linear_velocity(ffi::b2Vec2::from(v.into()));
        self
    }
    pub fn angular_velocity(mut self, w: f32) -> Self {
        self.def = self.def.angular_velocity(w);
        self
    }
    pub fn max_velocity_force(mut self, f: f32) -> Self {
        self.def = self.def.max_velocity_force(f);
        self
    }
    pub fn max_velocity_torque(mut self, t: f32) -> Self {
        self.def = self.def.max_velocity_torque(t);
        self
    }
    pub fn linear_spring(mut self, hz: f32, dr: f32) -> Self {
        self.def = self.def.linear_hertz(hz).linear_damping_ratio(dr);
        self
    }
    pub fn angular_spring(mut self, hz: f32, dr: f32) -> Self {
        self.def = self.def.angular_hertz(hz).angular_damping_ratio(dr);
        self
    }
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }

    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        // Default frames: identity (base only needs bodies)
        let base = JointBaseBuilder::new()
            .bodies_by_id(self.body_a, self.body_b)
            .build();
        self.def.0.base = base.0;
        self.world.create_motor_joint(&self.def)
    }
}

// Filter joint convenience builder (minimal)
/// Builder for a filter joint that disables collision between two bodies
/// while keeping them in the same island.
pub struct FilterJointBuilder<'w> {
    world: &'w mut World,
    body_a: BodyId,
    body_b: BodyId,
    def: FilterJointDef,
}

impl<'w> FilterJointBuilder<'w> {
    /// Whether the attached bodies should collide with each other.
    pub fn collide_connected(mut self, flag: bool) -> Self {
        self.def.0.base.collideConnected = flag;
        self
    }
    #[must_use]
    pub fn build(mut self) -> Joint<'w> {
        let base = JointBaseBuilder::new()
            .bodies_by_id(self.body_a, self.body_b)
            .build();
        self.def.0.base = base.0;
        self.world.create_filter_joint(&self.def)
    }
}

impl World {
    pub fn distance<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> DistanceJointBuilder<'w> {
        DistanceJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            def: DistanceJointDef::new(JointBase::default()),
        }
    }
    pub fn weld<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WeldJointBuilder<'w> {
        WeldJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_world: None,
            def: WeldJointDef::new(JointBase::default()),
        }
    }
    pub fn motor_joint<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> MotorJointBuilder<'w> {
        MotorJointBuilder {
            world: self,
            body_a,
            body_b,
            def: MotorJointDef::new(JointBase::default()),
        }
    }
    pub fn filter_joint<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> FilterJointBuilder<'w> {
        FilterJointBuilder {
            world: self,
            body_a,
            body_b,
            def: FilterJointDef::new(JointBase::default()),
        }
    }
}

impl World {
    pub fn create_distance_joint<'w>(&'w mut self, def: &DistanceJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateDistanceJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_distance_joint_id(&mut self, def: &DistanceJointDef) -> JointId {
        unsafe { ffi::b2CreateDistanceJoint(self.raw(), &def.0) }
    }
    pub fn create_revolute_joint<'w>(&'w mut self, def: &RevoluteJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreateRevoluteJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_revolute_joint_id(&mut self, def: &RevoluteJointDef) -> JointId {
        unsafe { ffi::b2CreateRevoluteJoint(self.raw(), &def.0) }
    }
    pub fn create_prismatic_joint<'w>(&'w mut self, def: &PrismaticJointDef) -> Joint<'w> {
        let id = unsafe { ffi::b2CreatePrismaticJoint(self.raw(), &def.0) };
        Joint {
            id,
            _world: PhantomData,
        }
    }
    pub fn create_prismatic_joint_id(&mut self, def: &PrismaticJointDef) -> JointId {
        unsafe { ffi::b2CreatePrismaticJoint(self.raw(), &def.0) }
    }
    pub fn create_wheel_joint_id(&mut self, def: &WheelJointDef) -> JointId {
        unsafe { ffi::b2CreateWheelJoint(self.raw(), &def.0) }
    }
    pub fn create_weld_joint_id(&mut self, def: &WeldJointDef) -> JointId {
        unsafe { ffi::b2CreateWeldJoint(self.raw(), &def.0) }
    }
    pub fn create_motor_joint_id(&mut self, def: &MotorJointDef) -> JointId {
        unsafe { ffi::b2CreateMotorJoint(self.raw(), &def.0) }
    }
    pub fn create_filter_joint_id(&mut self, def: &FilterJointDef) -> JointId {
        unsafe { ffi::b2CreateFilterJoint(self.raw(), &def.0) }
    }
    pub fn destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) {
        if unsafe { ffi::b2Joint_IsValid(id) } {
            unsafe { ffi::b2DestroyJoint(id, wake_bodies) };
        }
    }
}

// Runtime joint control APIs (by joint type)
impl World {
    // Distance joint
    #[inline]
    pub fn distance_set_length(&mut self, id: JointId, length: f32) {
        unsafe { ffi::b2DistanceJoint_SetLength(id, length) }
    }
    #[inline]
    pub fn distance_enable_spring(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2DistanceJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn distance_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2DistanceJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn distance_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2DistanceJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn distance_enable_limit(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2DistanceJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn distance_set_length_range(&mut self, id: JointId, min_length: f32, max_length: f32) {
        unsafe { ffi::b2DistanceJoint_SetLengthRange(id, min_length, max_length) }
    }
    #[inline]
    pub fn distance_enable_motor(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2DistanceJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn distance_set_motor_speed(&mut self, id: JointId, speed: f32) {
        unsafe { ffi::b2DistanceJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn distance_set_max_motor_force(&mut self, id: JointId, force: f32) {
        unsafe { ffi::b2DistanceJoint_SetMaxMotorForce(id, force) }
    }

    // Prismatic joint
    #[inline]
    pub fn prismatic_enable_spring(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2PrismaticJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn prismatic_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2PrismaticJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn prismatic_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2PrismaticJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn prismatic_set_target_translation(&mut self, id: JointId, translation: f32) {
        unsafe { ffi::b2PrismaticJoint_SetTargetTranslation(id, translation) }
    }
    #[inline]
    pub fn prismatic_enable_limit(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2PrismaticJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn prismatic_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        unsafe { ffi::b2PrismaticJoint_SetLimits(id, lower, upper) }
    }
    #[inline]
    pub fn prismatic_enable_motor(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2PrismaticJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn prismatic_set_motor_speed(&mut self, id: JointId, speed: f32) {
        unsafe { ffi::b2PrismaticJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn prismatic_set_max_motor_force(&mut self, id: JointId, force: f32) {
        unsafe { ffi::b2PrismaticJoint_SetMaxMotorForce(id, force) }
    }

    // Revolute joint
    #[inline]
    pub fn revolute_enable_spring(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2RevoluteJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn revolute_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2RevoluteJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn revolute_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2RevoluteJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn revolute_set_target_angle(&mut self, id: JointId, angle: f32) {
        unsafe { ffi::b2RevoluteJoint_SetTargetAngle(id, angle) }
    }
    #[inline]
    pub fn revolute_enable_limit(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2RevoluteJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn revolute_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        unsafe { ffi::b2RevoluteJoint_SetLimits(id, lower, upper) }
    }
    #[inline]
    pub fn revolute_enable_motor(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2RevoluteJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn revolute_set_motor_speed(&mut self, id: JointId, speed: f32) {
        unsafe { ffi::b2RevoluteJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn revolute_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        unsafe { ffi::b2RevoluteJoint_SetMaxMotorTorque(id, torque) }
    }

    // Weld joint
    #[inline]
    pub fn weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2WeldJoint_SetLinearHertz(id, hertz) }
    }
    #[inline]
    pub fn weld_set_linear_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2WeldJoint_SetLinearDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2WeldJoint_SetAngularHertz(id, hertz) }
    }
    #[inline]
    pub fn weld_set_angular_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2WeldJoint_SetAngularDampingRatio(id, damping_ratio) }
    }

    // Wheel joint
    #[inline]
    pub fn wheel_enable_spring(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2WheelJoint_EnableSpring(id, enable) }
    }
    #[inline]
    pub fn wheel_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2WheelJoint_SetSpringHertz(id, hertz) }
    }
    #[inline]
    pub fn wheel_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        unsafe { ffi::b2WheelJoint_SetSpringDampingRatio(id, damping_ratio) }
    }
    #[inline]
    pub fn wheel_enable_limit(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2WheelJoint_EnableLimit(id, enable) }
    }
    #[inline]
    pub fn wheel_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        unsafe { ffi::b2WheelJoint_SetLimits(id, lower, upper) }
    }
    #[inline]
    pub fn wheel_enable_motor(&mut self, id: JointId, enable: bool) {
        unsafe { ffi::b2WheelJoint_EnableMotor(id, enable) }
    }
    #[inline]
    pub fn wheel_set_motor_speed(&mut self, id: JointId, speed: f32) {
        unsafe { ffi::b2WheelJoint_SetMotorSpeed(id, speed) }
    }
    #[inline]
    pub fn wheel_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        unsafe { ffi::b2WheelJoint_SetMaxMotorTorque(id, torque) }
    }

    // Motor joint
    #[inline]
    pub fn motor_set_linear_velocity<V: Into<crate::types::Vec2>>(&mut self, id: JointId, v: V) {
        let vv: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2MotorJoint_SetLinearVelocity(id, vv) }
    }
    #[inline]
    pub fn motor_set_angular_velocity(&mut self, id: JointId, w: f32) {
        unsafe { ffi::b2MotorJoint_SetAngularVelocity(id, w) }
    }
    #[inline]
    pub fn motor_set_max_velocity_force(&mut self, id: JointId, f: f32) {
        unsafe { ffi::b2MotorJoint_SetMaxVelocityForce(id, f) }
    }
    #[inline]
    pub fn motor_set_max_velocity_torque(&mut self, id: JointId, t: f32) {
        unsafe { ffi::b2MotorJoint_SetMaxVelocityTorque(id, t) }
    }
    #[inline]
    pub fn motor_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2MotorJoint_SetLinearHertz(id, hertz) }
    }
    #[inline]
    pub fn motor_set_linear_damping_ratio(&mut self, id: JointId, damping: f32) {
        unsafe { ffi::b2MotorJoint_SetLinearDampingRatio(id, damping) }
    }
    #[inline]
    pub fn motor_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        unsafe { ffi::b2MotorJoint_SetAngularHertz(id, hertz) }
    }
    #[inline]
    pub fn motor_set_angular_damping_ratio(&mut self, id: JointId, damping: f32) {
        unsafe { ffi::b2MotorJoint_SetAngularDampingRatio(id, damping) }
    }
    #[inline]
    pub fn motor_set_max_spring_force(&mut self, id: JointId, f: f32) {
        unsafe { ffi::b2MotorJoint_SetMaxSpringForce(id, f) }
    }
    #[inline]
    pub fn motor_set_max_spring_torque(&mut self, id: JointId, t: f32) {
        unsafe { ffi::b2MotorJoint_SetMaxSpringTorque(id, t) }
    }
}
