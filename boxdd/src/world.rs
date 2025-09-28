use crate::Transform;
use crate::body::{Body, BodyDef, BodyType};
use crate::shapes::ShapeDef;
use crate::types::{BodyId, JointId, ShapeId, Vec2};
use boxdd_sys::ffi;
use std::ffi::CString;

type ShapeFilterFn = fn(crate::types::ShapeId, crate::types::ShapeId) -> bool;
type PreSolveFn = fn(
    crate::types::ShapeId,
    crate::types::ShapeId,
    crate::types::Vec2,
    crate::types::Vec2,
) -> bool;

/// Error type for world creation and operations.
#[non_exhaustive]
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
///
/// Chain configuration calls and finish with `build()`. All fields map 1:1 to
/// the upstream `b2WorldDef`.
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
    /// Set gravity vector in meters per second squared.
    pub fn gravity<V: Into<Vec2>>(mut self, g: V) -> Self {
        self.def.0.gravity = ffi::b2Vec2::from(g.into());
        self
    }
    /// Restitution threshold (m/s) under which collisions don't bounce.
    pub fn restitution_threshold(mut self, v: f32) -> Self {
        self.def.0.restitutionThreshold = v;
        self
    }
    /// Impulse magnitude that generates hit events.
    pub fn hit_event_threshold(mut self, v: f32) -> Self {
        self.def.0.hitEventThreshold = v;
        self
    }
    /// Contact solver target stiffness in Hertz.
    pub fn contact_hertz(mut self, v: f32) -> Self {
        self.def.0.contactHertz = v;
        self
    }
    /// Contact damping ratio (non-dimensional).
    pub fn contact_damping_ratio(mut self, v: f32) -> Self {
        self.def.0.contactDampingRatio = v;
        self
    }
    /// Velocity used by continuous collision detection.
    pub fn contact_speed(mut self, v: f32) -> Self {
        self.def.0.contactSpeed = v;
        self
    }
    /// Maximum linear speed clamp for bodies.
    pub fn maximum_linear_speed(mut self, v: f32) -> Self {
        self.def.0.maximumLinearSpeed = v;
        self
    }
    /// Enable/disable sleeping globally.
    pub fn enable_sleep(mut self, flag: bool) -> Self {
        self.def.0.enableSleep = flag;
        self
    }
    /// Enable/disable continuous collision detection globally.
    pub fn enable_continuous(mut self, flag: bool) -> Self {
        self.def.0.enableContinuous = flag;
        self
    }
    /// Enable/disable contact softening.
    pub fn enable_contact_softening(mut self, flag: bool) -> Self {
        self.def.0.enableContactSoftening = flag;
        self
    }
    /// Number of worker threads Box2D may use.
    pub fn worker_count(mut self, n: i32) -> Self {
        self.def.0.workerCount = n;
        self
    }

    #[must_use]
    pub fn build(self) -> WorldDef {
        self.def
    }
}

/// A simulation world; RAII owns the underlying world id and cleans up on drop.
pub struct World {
    id: ffi::b2WorldId,
    // Optional user callbacks stored as heap allocations so we can pass stable
    // pointers as FFI callback context.
    custom_filter: Option<Box<CustomFilterCtx>>,
    pre_solve: Option<Box<PreSolveCtx>>,
}

// Internal callback context holding user closures. These must be Send + Sync
// because Box2D may invoke them from worker threads.
struct CustomFilterCtx {
    cb: Box<dyn Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static>,
}

struct PreSolveCtx {
    cb: Box<
        dyn Fn(
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    >,
}

impl World {
    /// Create a world from a definition.
    pub fn new(def: WorldDef) -> Result<Self, Error> {
        let raw = def.into_raw();
        // SAFETY: FFI call to create a world; returns an id handle
        let world_id = unsafe { ffi::b2CreateWorld(&raw) };
        let ok = unsafe { ffi::b2World_IsValid(world_id) };
        if ok {
            Ok(Self {
                id: world_id,
                custom_filter: None,
                pre_solve: None,
            })
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
    pub fn set_gravity<V: Into<Vec2>>(&mut self, g: V) {
        let gv: ffi::b2Vec2 = g.into().into();
        unsafe { ffi::b2World_SetGravity(self.id, gv) };
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
    pub fn set_body_linear_velocity<V: Into<Vec2>>(&mut self, body: BodyId, v: V) {
        let vv: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2Body_SetLinearVelocity(body, vv) }
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
    /// Enable or disable constraint warm starting at runtime.
    ///
    /// Warm starting seeds the solver with accumulated impulses from the previous
    /// step to improve stability and convergence. Disabling this is only useful
    /// for experiments and will significantly reduce stability in most scenes.
    pub fn enable_warm_starting(&mut self, flag: bool) {
        unsafe { ffi::b2World_EnableWarmStarting(self.raw(), flag) }
    }
    /// Returns true if constraint warm starting is enabled.
    pub fn is_warm_starting_enabled(&self) -> bool {
        unsafe { ffi::b2World_IsWarmStartingEnabled(self.raw()) }
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

    // --- Collision/solve callbacks ---------------------------------------------------------
    /// Register a thread-safe custom filter closure. This is called when a contact pair is
    /// considered for collision if either shape has custom filtering enabled.
    /// Return false to disable the collision.
    pub fn set_custom_filter<F>(&mut self, f: F)
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        // Store the closure so its address is stable and lifetime tied to the world.
        let ctx = Box::new(CustomFilterCtx { cb: Box::new(f) });
        // SAFETY: callback shims only cast the context pointer back to CustomFilterCtx and
        // invoke the Rust closure. They must be extern "C" and thread-safe.
        unsafe extern "C" fn filter_cb(
            a: ffi::b2ShapeId,
            b: ffi::b2ShapeId,
            context: *mut core::ffi::c_void,
        ) -> bool {
            // SAFETY: context is provided by set_custom_filter and points to CustomFilterCtx
            let ctx = unsafe { &*(context as *const CustomFilterCtx) };
            (ctx.cb)(a, b)
        }
        let ctx_ptr: *mut core::ffi::c_void = (&*ctx) as *const CustomFilterCtx as *mut _;
        unsafe { ffi::b2World_SetCustomFilterCallback(self.raw(), Some(filter_cb), ctx_ptr) };
        self.custom_filter = Some(ctx);
    }

    /// Clear the custom filter callback and release associated resources.
    pub fn clear_custom_filter(&mut self) {
        unsafe { ffi::b2World_SetCustomFilterCallback(self.raw(), None, core::ptr::null_mut()) };
        self.custom_filter = None;
    }

    /// Register a thread-safe pre-solve closure. This is called after contact update (when enabled
    /// on shapes) and before the solver. Return false to disable the contact this step.
    pub fn set_pre_solve<F>(&mut self, f: F)
    where
        F: Fn(
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        let ctx = Box::new(PreSolveCtx { cb: Box::new(f) });
        unsafe extern "C" fn presolve_cb(
            a: ffi::b2ShapeId,
            b: ffi::b2ShapeId,
            point: ffi::b2Vec2,
            normal: ffi::b2Vec2,
            context: *mut core::ffi::c_void,
        ) -> bool {
            // SAFETY: context is provided by set_pre_solve and points to PreSolveCtx
            let ctx = unsafe { &*(context as *const PreSolveCtx) };
            (ctx.cb)(
                a,
                b,
                crate::types::Vec2::from(point),
                crate::types::Vec2::from(normal),
            )
        }
        let ctx_ptr: *mut core::ffi::c_void = (&*ctx) as *const PreSolveCtx as *mut _;
        unsafe { ffi::b2World_SetPreSolveCallback(self.raw(), Some(presolve_cb), ctx_ptr) };
        self.pre_solve = Some(ctx);
    }

    /// Clear the pre-solve callback and release associated resources.
    pub fn clear_pre_solve(&mut self) {
        unsafe { ffi::b2World_SetPreSolveCallback(self.raw(), None, core::ptr::null_mut()) };
        self.pre_solve = None;
    }

    /// Compatibility helper: set or clear the custom filter using a plain function pointer.
    pub fn set_custom_filter_callback(&mut self, cb: Option<ShapeFilterFn>) {
        match cb {
            Some(func) => self.set_custom_filter(func),
            None => self.clear_custom_filter(),
        }
    }

    /// Compatibility helper: set or clear the pre-solve using a plain function pointer.
    pub fn set_pre_solve_callback(&mut self, cb: Option<PreSolveFn>) {
        match cb {
            Some(func) => self.set_pre_solve(func),
            None => self.clear_pre_solve(),
        }
    }

    // Convenience joints built from world anchors and axis using body ids
    pub fn create_revolute_joint_world<VA: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_world: VA,
    ) -> crate::joints::Joint<'_> {
        let aw: ffi::b2Vec2 = anchor_world.into().into();
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, aw);
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

    pub fn create_revolute_joint_world_id<VA: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_world: VA,
    ) -> JointId {
        let aw: ffi::b2Vec2 = anchor_world.into().into();
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let la = crate::core::math::world_to_local_point(ta, aw);
        let lb = crate::core::math::world_to_local_point(tb, aw);
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

    pub fn create_prismatic_joint_world<VA: Into<Vec2>, VB: Into<Vec2>, AX: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::Joint<'_> {
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis: ffi::b2Vec2 = axis_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build();
        let def = crate::joints::PrismaticJointDef::new(base);
        self.create_prismatic_joint(&def)
    }

    pub fn create_prismatic_joint_world_id<VA: Into<Vec2>, VB: Into<Vec2>, AX: Into<Vec2>>(
        &mut self,
        body_a: ffi::b2BodyId,
        body_b: ffi::b2BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> JointId {
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis: ffi::b2Vec2 = axis_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build();
        let def = crate::joints::PrismaticJointDef::new(base);
        self.create_prismatic_joint_id(&def)
    }

    pub fn create_wheel_joint_world<VA: Into<Vec2>, VB: Into<Vec2>, AX: Into<Vec2>>(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::Joint<'_> {
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis: ffi::b2Vec2 = axis_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build();
        let def = crate::joints::WheelJointDef::new(base);
        self.create_wheel_joint(&def)
    }

    pub fn create_wheel_joint_world_id<VA: Into<Vec2>, VB: Into<Vec2>, AX: Into<Vec2>>(
        &mut self,
        body_a: ffi::b2BodyId,
        body_b: ffi::b2BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> JointId {
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis: ffi::b2Vec2 = axis_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        let base = crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
            )
            .build();
        let def = crate::joints::WheelJointDef::new(base);
        self.create_wheel_joint_id(&def)
    }

    /// Helper: build a joint base from two world anchor points.
    /// Build `JointBase` from two world anchor points.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let a = world.create_body_id(BodyBuilder::new().position([-1.0,2.0]).build());
    /// let b = world.create_body_id(BodyBuilder::new().position([ 1.0,2.0]).build());
    /// let sdef = ShapeDef::builder().density(1.0).build();
    /// world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5,0.5));
    /// world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5,0.5));
    /// let base = world.joint_base_from_world_points(a, b, world.body_position(a), world.body_position(b));
    /// # let _ = base;
    /// ```
    pub fn joint_base_from_world_points<VA: Into<Vec2>, VB: Into<Vec2>>(
        &self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
    ) -> crate::joints::JointBase {
        // Build JointBase from two world anchor points.
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
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
    /// Build `JointBase` from world anchors and a shared world axis (X-axis of local frames).
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let a = world.create_body_id(BodyBuilder::new().position([0.0,2.0]).build());
    /// let b = world.create_body_id(BodyBuilder::new().position([1.0,2.0]).build());
    /// let sdef = ShapeDef::builder().density(1.0).build();
    /// world.create_polygon_shape_for(a, &sdef, &shapes::box_polygon(0.5,0.5));
    /// world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5,0.5));
    /// let axis = Vec2::new(1.0, 0.0);
    /// let base = world.joint_base_from_world_with_axis(a, b, world.body_position(a), world.body_position(b), axis);
    /// # let _ = base;
    /// ```
    pub fn joint_base_from_world_with_axis<VA: Into<Vec2>, VB: Into<Vec2>, AX: Into<Vec2>>(
        &self,
        body_a: BodyId,
        body_b: BodyId,
        anchor_a_world: VA,
        anchor_b_world: VB,
        axis_world: AX,
    ) -> crate::joints::JointBase {
        // Build JointBase from world anchors and a shared world axis (X-axis of local frames).
        let ta = unsafe { ffi::b2Body_GetTransform(body_a) };
        let tb = unsafe { ffi::b2Body_GetTransform(body_b) };
        let wa: ffi::b2Vec2 = anchor_a_world.into().into();
        let wb: ffi::b2Vec2 = anchor_b_world.into().into();
        let axis: ffi::b2Vec2 = axis_world.into().into();
        let la = crate::core::math::world_to_local_point(ta, wa);
        let lb = crate::core::math::world_to_local_point(tb, wb);
        let ra = crate::core::math::world_axis_to_local_rot(ta, axis);
        let rb = crate::core::math::world_axis_to_local_rot(tb, axis);
        crate::joints::JointBaseBuilder::new()
            .bodies_by_id(body_a, body_b)
            .local_frames_raw(
                ffi::b2Transform { p: la, q: ra },
                ffi::b2Transform { p: lb, q: rb },
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
        if unsafe { ffi::b2Shape_IsValid(shape) } {
            unsafe { ffi::b2DestroyShape(shape, update_body_mass) };
        }
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
        if unsafe { ffi::b2Chain_IsValid(chain) } {
            unsafe { ffi::b2DestroyChain(chain) };
        }
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
        let wrote =
            unsafe { ffi::b2Shape_GetSensorData(shape, ids.as_mut_ptr(), cap) }.max(0) as usize;
        unsafe { ids.set_len(wrote.min(cap as usize)) };
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
