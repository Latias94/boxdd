use crate::body::{Body, BodyDef, BodyType};
use crate::shapes::ShapeDef;
use crate::types::{BodyId, JointId, ShapeId, Vec2};
use crate::Transform;
use boxdd_sys::ffi;
use std::ffi::CString;

/// Error type for world creation and operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to create Box2D world")]
    CreateFailed,
}

/// World definition builder for constructing a simulation world.
#[derive(Clone, Debug)]
pub struct WorldDef(ffi::b2WorldDef);

impl Default for WorldDef {
    fn default() -> Self {
        // SAFETY: FFI call to obtain a plain value struct
        let def = unsafe { ffi::b2DefaultWorldDef() };
        Self(def)
    }
}

impl WorldDef {
    pub fn builder() -> WorldBuilder {
        WorldBuilder::from(Self::default())
    }

    pub fn into_raw(self) -> ffi::b2WorldDef {
        self.0
    }
}

/// Fluent builder for `WorldDef`.
#[derive(Clone, Debug)]
pub struct WorldBuilder {
    def: WorldDef,
}

impl From<WorldDef> for WorldBuilder {
    fn from(def: WorldDef) -> Self {
        Self { def }
    }
}

impl WorldBuilder {
    pub fn gravity<V: Into<ffi::b2Vec2>>(mut self, g: V) -> Self {
        self.def.0.gravity = g.into();
        self
    }
    pub fn restitution_threshold(mut self, v: f32) -> Self {
        self.def.0.restitutionThreshold = v;
        self
    }
    pub fn hit_event_threshold(mut self, v: f32) -> Self {
        self.def.0.hitEventThreshold = v;
        self
    }
    pub fn contact_hertz(mut self, v: f32) -> Self {
        self.def.0.contactHertz = v;
        self
    }
    pub fn contact_damping_ratio(mut self, v: f32) -> Self {
        self.def.0.contactDampingRatio = v;
        self
    }
    pub fn contact_speed(mut self, v: f32) -> Self {
        self.def.0.contactSpeed = v;
        self
    }
    pub fn maximum_linear_speed(mut self, v: f32) -> Self {
        self.def.0.maximumLinearSpeed = v;
        self
    }
    pub fn enable_sleep(mut self, flag: bool) -> Self {
        self.def.0.enableSleep = flag;
        self
    }
    pub fn enable_continuous(mut self, flag: bool) -> Self {
        self.def.0.enableContinuous = flag;
        self
    }
    pub fn enable_contact_softening(mut self, flag: bool) -> Self {
        self.def.0.enableContactSoftening = flag;
        self
    }
    pub fn worker_count(mut self, n: i32) -> Self {
        self.def.0.workerCount = n;
        self
    }

    pub fn build(self) -> WorldDef {
        self.def
    }
}

/// A simulation world; RAII owns the underlying world id and cleans up on drop.
pub struct World {
    id: ffi::b2WorldId,
}

impl World {
    /// Create a world from a definition.
    pub fn new(def: WorldDef) -> Result<Self, Error> {
        let raw = def.into_raw();
        // SAFETY: FFI call to create a world; returns an id handle
        let world_id = unsafe { ffi::b2CreateWorld(&raw) };
        let ok = unsafe { ffi::b2World_IsValid(world_id) };
        if ok {
            Ok(Self { id: world_id })
        } else {
            Err(Error::CreateFailed)
        }
    }

    /// Step the simulation by `time_step` seconds using `sub_steps` sub-steps.
    pub fn step(&mut self, time_step: f32, sub_steps: i32) {
        // SAFETY: valid world id managed by RAII
        unsafe { ffi::b2World_Step(self.id, time_step, sub_steps) };
    }

    /// Set gravity vector.
    pub fn set_gravity(&mut self, g: Vec2) {
        unsafe { ffi::b2World_SetGravity(self.id, g.into()) };
    }

    /// Get current gravity vector.
    pub fn gravity(&self) -> Vec2 {
        Vec2::from(unsafe { ffi::b2World_GetGravity(self.id) })
    }

    /// Expose raw id for advanced use-cases.
    pub fn raw(&self) -> ffi::b2WorldId {
        self.id
    }

    /// World counters snapshot (sizes, tree heights, etc.).
    pub fn counters(&self) -> Counters {
        let c = unsafe { ffi::b2World_GetCounters(self.id) };
        Counters::from(c)
    }

    /// Get a body's transform safely from its id.
    pub fn body_transform(&self, body: BodyId) -> Transform {
        Transform::from(unsafe { ffi::b2Body_GetTransform(body) })
    }
    /// Get a body's world position.
    pub fn body_position(&self, body: BodyId) -> Vec2 {
        Vec2::from(unsafe { ffi::b2Body_GetPosition(body) })
    }
    /// Set a body's linear velocity by id.
    pub fn set_body_linear_velocity<V: Into<ffi::b2Vec2>>(&mut self, body: BodyId, v: V) {
        unsafe { ffi::b2Body_SetLinearVelocity(body, v.into()) }
    }
    /// Set a body's angular velocity by id.
    pub fn set_body_angular_velocity(&mut self, body: BodyId, w: f32) {
        unsafe { ffi::b2Body_SetAngularVelocity(body, w) }
    }
    /// Set a body's type by id.
    pub fn set_body_type(&mut self, body: BodyId, t: BodyType) {
        unsafe { ffi::b2Body_SetType(body, t.into()) }
    }
    /// Enable a body by id.
    pub fn enable_body(&mut self, body: BodyId) {
        unsafe { ffi::b2Body_Enable(body) }
    }
    /// Disable a body by id.
    pub fn disable_body(&mut self, body: BodyId) {
        unsafe { ffi::b2Body_Disable(body) }
    }
    /// Set a body's name by id.
    pub fn set_body_name(&mut self, body: BodyId, name: &str) {
        if let Ok(cs) = CString::new(name) {
            unsafe { ffi::b2Body_SetName(body, cs.as_ptr()) }
        }
    }
    /// Get number of awake bodies.
    pub fn awake_body_count(&self) -> i32 {
        unsafe { ffi::b2World_GetAwakeBodyCount(self.id) }
    }

    /// Create a body owned by this world.
    pub fn create_body<'w>(&'w mut self, def: BodyDef) -> Body<'w> {
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.id, &raw) };
        Body::new(id)
    }

    /// ID-style body creation. Prefer when you don't want RAII wrappers.
    pub fn create_body_id(&mut self, def: BodyDef) -> BodyId {
        let raw = def.0;
        unsafe { ffi::b2CreateBody(self.id, &raw) }
    }

    /// Destroy a body by id.
    pub fn destroy_body_id(&mut self, id: BodyId) {
        unsafe { ffi::b2DestroyBody(id) };
    }

    // Runtime configuration helpers mirroring WorldDef fields
    pub fn enable_sleeping(&mut self, flag: bool) {
        unsafe { ffi::b2World_EnableSleeping(self.raw(), flag) }
    }
    pub fn is_sleeping_enabled(&self) -> bool {
        unsafe { ffi::b2World_IsSleepingEnabled(self.raw()) }
    }
    pub fn enable_continuous(&mut self, flag: bool) {
        unsafe { ffi::b2World_EnableContinuous(self.raw(), flag) }
    }
    pub fn is_continuous_enabled(&self) -> bool {
        unsafe { ffi::b2World_IsContinuousEnabled(self.raw()) }
    }
    pub fn set_restitution_threshold(&mut self, value: f32) {
        unsafe { ffi::b2World_SetRestitutionThreshold(self.raw(), value) }
    }
    pub fn restitution_threshold(&self) -> f32 {
        unsafe { ffi::b2World_GetRestitutionThreshold(self.raw()) }
    }
    pub fn set_hit_event_threshold(&mut self, value: f32) {
        unsafe { ffi::b2World_SetHitEventThreshold(self.raw(), value) }
    }
    pub fn hit_event_threshold(&self) -> f32 {
        unsafe { ffi::b2World_GetHitEventThreshold(self.raw()) }
    }
    pub fn set_contact_tuning(&mut self, hertz: f32, damping_ratio: f32, push_speed: f32) {
        unsafe { ffi::b2World_SetContactTuning(self.raw(), hertz, damping_ratio, push_speed) }
    }
    pub fn set_maximum_linear_speed(&mut self, v: f32) {
        unsafe { ffi::b2World_SetMaximumLinearSpeed(self.raw(), v) }
    }
    pub fn maximum_linear_speed(&self) -> f32 {
        unsafe { ffi::b2World_GetMaximumLinearSpeed(self.raw()) }
    }

    // Convenience joints built from world anchors and axis using body ids
    pub fn create_revolute_joint_world<VA: Into<ffi::b2Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_world: VA,
    ) -> crate::joints::Joint<'_> {
        let aw = anchor_world.into();
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let la = {
            let dx = aw.x - ta.p.x;
            let dy = aw.y - ta.p.y;
            let c = ta.q.c;
            let s = ta.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let lb = {
            let dx = aw.x - tb.p.x;
            let dy = aw.y - tb.p.y;
            let c = tb.q.c;
            let s = tb.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
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
        let def = crate::joints::RevoluteJointDef::new(base);
        self.create_revolute_joint(&def)
    }

    pub fn create_revolute_joint_world_id<VA: Into<ffi::b2Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_world: VA,
    ) -> JointId {
        let aw = anchor_world.into();
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let la = {
            let dx = aw.x - ta.p.x;
            let dy = aw.y - ta.p.y;
            let c = ta.q.c;
            let s = ta.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let lb = {
            let dx = aw.x - tb.p.x;
            let dy = aw.y - tb.p.y;
            let c = tb.q.c;
            let s = tb.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
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
        let def = crate::joints::RevoluteJointDef::new(base);
        self.create_revolute_joint_id(&def)
    }

    pub fn create_prismatic_joint_world<
        VA: Into<ffi::b2Vec2>,
        VB: Into<ffi::b2Vec2>,
        AX: Into<ffi::b2Vec2>,
    >(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::Joint<'_> {
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa = anchor_a_world.into();
        let wb = anchor_b_world.into();
        let axis = axis_world.into();
        let la = {
            let dx = wa.x - ta.p.x;
            let dy = wa.y - ta.p.y;
            let c = ta.q.c;
            let s = ta.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let lb = {
            let dx = wb.x - tb.p.x;
            let dy = wb.y - tb.p.y;
            let c = tb.q.c;
            let s = tb.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let angle_w = axis.y.atan2(axis.x);
        let angle_a = ta.q.s.atan2(ta.q.c);
        let angle_b = tb.q.s.atan2(tb.q.c);
        let (sa, ca) = (angle_w - angle_a).sin_cos();
        let (sb, cb) = (angle_w - angle_b).sin_cos();
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: ca, s: sa },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: cb, s: sb },
                },
            )
            .build();
        let def = crate::joints::PrismaticJointDef::new(base);
        self.create_prismatic_joint(&def)
    }

    pub fn create_prismatic_joint_world_id<
        VA: Into<ffi::b2Vec2>,
        VB: Into<ffi::b2Vec2>,
        AX: Into<ffi::b2Vec2>,
    >(
        &mut self,
        body_a: ffi::b2BodyId,
        body_b: ffi::b2BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> JointId {
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa = anchor_a_world.into();
        let wb = anchor_b_world.into();
        let axis = axis_world.into();
        let la = {
            let dx = wa.x - ta.p.x;
            let dy = wa.y - ta.p.y;
            let c = ta.q.c;
            let s = ta.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let lb = {
            let dx = wb.x - tb.p.x;
            let dy = wb.y - tb.p.y;
            let c = tb.q.c;
            let s = tb.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let angle_w = axis.y.atan2(axis.x);
        let angle_a = ta.q.s.atan2(ta.q.c);
        let angle_b = tb.q.s.atan2(tb.q.c);
        let (sa, ca) = (angle_w - angle_a).sin_cos();
        let (sb, cb) = (angle_w - angle_b).sin_cos();
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: ca, s: sa },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: cb, s: sb },
                },
            )
            .build();
        let def = crate::joints::PrismaticJointDef::new(base);
        self.create_prismatic_joint_id(&def)
    }

    pub fn create_wheel_joint_world<
        VA: Into<ffi::b2Vec2>,
        VB: Into<ffi::b2Vec2>,
        AX: Into<ffi::b2Vec2>,
    >(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::Joint<'_> {
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa = anchor_a_world.into();
        let wb = anchor_b_world.into();
        let axis = axis_world.into();
        let la = {
            let dx = wa.x - ta.p.x;
            let dy = wa.y - ta.p.y;
            let c = ta.q.c;
            let s = ta.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let lb = {
            let dx = wb.x - tb.p.x;
            let dy = wb.y - tb.p.y;
            let c = tb.q.c;
            let s = tb.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let angle_w = axis.y.atan2(axis.x);
        let angle_a = ta.q.s.atan2(ta.q.c);
        let angle_b = tb.q.s.atan2(tb.q.c);
        let (sa, ca) = (angle_w - angle_a).sin_cos();
        let (sb, cb) = (angle_w - angle_b).sin_cos();
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: ca, s: sa },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: cb, s: sb },
                },
            )
            .build();
        let def = crate::joints::WheelJointDef::new(base);
        self.create_wheel_joint(&def)
    }

    pub fn create_wheel_joint_world_id<
        VA: Into<ffi::b2Vec2>,
        VB: Into<ffi::b2Vec2>,
        AX: Into<ffi::b2Vec2>,
    >(
        &mut self,
        body_a: ffi::b2BodyId,
        body_b: ffi::b2BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> JointId {
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa = anchor_a_world.into();
        let wb = anchor_b_world.into();
        let axis = axis_world.into();
        let la = {
            let dx = wa.x - ta.p.x;
            let dy = wa.y - ta.p.y;
            let c = ta.q.c;
            let s = ta.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let lb = {
            let dx = wb.x - tb.p.x;
            let dy = wb.y - tb.p.y;
            let c = tb.q.c;
            let s = tb.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let angle_w = axis.y.atan2(axis.x);
        let angle_a = ta.q.s.atan2(ta.q.c);
        let angle_b = tb.q.s.atan2(tb.q.c);
        let (sa, ca) = (angle_w - angle_a).sin_cos();
        let (sb, cb) = (angle_w - angle_b).sin_cos();
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: ca, s: sa },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: cb, s: sb },
                },
            )
            .build();
        let def = crate::joints::WheelJointDef::new(base);
        self.create_wheel_joint_id(&def)
    }

    /// Helper: build a joint base from two world anchor points.
    pub fn joint_base_from_world_points<VA: Into<ffi::b2Vec2>, VB: Into<ffi::b2Vec2>>(
        &self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
    ) -> crate::joints::JointBase {
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa = anchor_a_world.into();
        let wb = anchor_b_world.into();
        let la = {
            let dx = wa.x - ta.p.x;
            let dy = wa.y - ta.p.y;
            let c = ta.q.c;
            let s = ta.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let lb = {
            let dx = wb.x - tb.p.x;
            let dy = wb.y - tb.p.y;
            let c = tb.q.c;
            let s = tb.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
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
            .build()
    }

    /// Helper: build a joint base from two world anchors and a shared world axis (X-axis of joint frames).
    pub fn joint_base_from_world_with_axis<
        VA: Into<ffi::b2Vec2>,
        VB: Into<ffi::b2Vec2>,
        AX: Into<ffi::b2Vec2>,
    >(
        &self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::JointBase {
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa = anchor_a_world.into();
        let wb = anchor_b_world.into();
        let axis = axis_world.into();
        let la = {
            let dx = wa.x - ta.p.x;
            let dy = wa.y - ta.p.y;
            let c = ta.q.c;
            let s = ta.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let lb = {
            let dx = wb.x - tb.p.x;
            let dy = wb.y - tb.p.y;
            let c = tb.q.c;
            let s = tb.q.s;
            ffi::b2Vec2 {
                x: c * dx + s * dy,
                y: -s * dx + c * dy,
            }
        };
        let angle_w = axis.y.atan2(axis.x);
        let angle_a = ta.q.s.atan2(ta.q.c);
        let angle_b = tb.q.s.atan2(tb.q.c);
        let (sa, ca) = (angle_w - angle_a).sin_cos();
        let (sb, cb) = (angle_w - angle_b).sin_cos();
        crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform {
                    p: la,
                    q: ffi::b2Rot { c: ca, s: sa },
                },
                ffi::b2Transform {
                    p: lb,
                    q: ffi::b2Rot { c: cb, s: sb },
                },
            )
            .build()
    }

    // ID-based shape helpers (world-anchored)
    pub fn create_circle_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        c: &ffi::b2Circle,
    ) -> ShapeId {
        unsafe { ffi::b2CreateCircleShape(body, &def.0, c) }
    }
    pub fn create_segment_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        s: &ffi::b2Segment,
    ) -> ShapeId {
        unsafe { ffi::b2CreateSegmentShape(body, &def.0, s) }
    }
    pub fn create_capsule_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        c: &ffi::b2Capsule,
    ) -> ShapeId {
        unsafe { ffi::b2CreateCapsuleShape(body, &def.0, c) }
    }
    pub fn create_polygon_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        p: &ffi::b2Polygon,
    ) -> ShapeId {
        unsafe { ffi::b2CreatePolygonShape(body, &def.0, p) }
    }
    pub fn destroy_shape_id(&mut self, shape: ShapeId, update_body_mass: bool) {
        unsafe { ffi::b2DestroyShape(shape, update_body_mass) };
    }

    // Chain API (ID-style)
    pub fn create_chain_for_id(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> ffi::b2ChainId {
        unsafe { ffi::b2CreateChain(body, &def.def) }
    }
    pub fn destroy_chain_id(&mut self, chain: ffi::b2ChainId) {
        unsafe { ffi::b2DestroyChain(chain) };
    }

    // Sensor helpers (ID-style)
    /// Get the maximum capacity required to retrieve sensor overlaps for a shape id.
    pub fn shape_sensor_capacity(&self, shape: ShapeId) -> i32 {
        unsafe { ffi::b2Shape_GetSensorCapacity(shape) }
    }
    /// Get overlapped shapes for a sensor shape id. Returns empty if not a sensor.
    pub fn shape_sensor_overlaps(&self, shape: ShapeId) -> Vec<ShapeId> {
        let cap = self.shape_sensor_capacity(shape);
        if cap <= 0 {
            return Vec::new();
        }
        let mut ids: Vec<ShapeId> = Vec::with_capacity(cap as usize);
        let wrote = unsafe { ffi::b2Shape_GetSensorData(shape, ids.as_mut_ptr(), cap) };
        unsafe { ids.set_len(wrote.max(0) as usize) };
        ids
    }
    /// Get overlapped shapes for a sensor shape id, filtered to valid (non-destroyed) ids.
    pub fn shape_sensor_overlaps_valid(&self, shape: ShapeId) -> Vec<ShapeId> {
        self.shape_sensor_overlaps(shape)
            .into_iter()
            .filter(|&sid| unsafe { ffi::b2Shape_IsValid(sid) })
            .collect()
    }
}

impl Drop for World {
    fn drop(&mut self) {
        // SAFETY: destroy the world id when this wrapper drops
        unsafe { ffi::b2DestroyWorld(self.id) };
    }
}

/// Simulation counters providing size and internal stats.
#[derive(Clone, Debug)]
pub struct Counters {
    pub body_count: i32,
    pub shape_count: i32,
    pub contact_count: i32,
    pub joint_count: i32,
    pub island_count: i32,
    pub stack_used: i32,
    pub static_tree_height: i32,
    pub tree_height: i32,
    pub byte_count: i32,
    pub task_count: i32,
    pub color_counts: [i32; 24],
}

impl From<ffi::b2Counters> for Counters {
    fn from(c: ffi::b2Counters) -> Self {
        Self {
            body_count: c.bodyCount,
            shape_count: c.shapeCount,
            contact_count: c.contactCount,
            joint_count: c.jointCount,
            island_count: c.islandCount,
            stack_used: c.stackUsed,
            static_tree_height: c.staticTreeHeight,
            tree_height: c.treeHeight,
            byte_count: c.byteCount,
            task_count: c.taskCount,
            color_counts: c.colorCounts,
        }
    }
}
