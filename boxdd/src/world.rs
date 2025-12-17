use crate::Transform;
use crate::body::{Body, BodyDef, BodyType};
use crate::core::world_core::{CustomFilterCtx, PreSolveCtx, WorldCore};
use crate::shapes::{ShapeDef, SurfaceMaterial};
use crate::types::{BodyId, ChainId, JointId, MassData, ShapeId, Vec2};
use boxdd_sys::ffi;
use std::ffi::CString;
use std::rc::Rc;
use std::sync::Arc;

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
#[doc(alias = "world_def")]
#[doc(alias = "worlddef")]
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

// serde for WorldDef via config representation
#[cfg(feature = "serde")]
impl serde::Serialize for WorldDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            gravity: crate::types::Vec2,
            restitution_threshold: f32,
            hit_event_threshold: f32,
            contact_hertz: f32,
            contact_damping_ratio: f32,
            contact_speed: f32,
            maximum_linear_speed: f32,
            enable_sleep: bool,
            enable_continuous: bool,
            enable_contact_softening: bool,
            worker_count: i32,
        }
        let r = Repr {
            gravity: crate::types::Vec2::from(self.0.gravity),
            restitution_threshold: self.0.restitutionThreshold,
            hit_event_threshold: self.0.hitEventThreshold,
            contact_hertz: self.0.contactHertz,
            contact_damping_ratio: self.0.contactDampingRatio,
            contact_speed: self.0.contactSpeed,
            maximum_linear_speed: self.0.maximumLinearSpeed,
            enable_sleep: self.0.enableSleep,
            enable_continuous: self.0.enableContinuous,
            enable_contact_softening: self.0.enableContactSoftening,
            worker_count: self.0.workerCount,
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for WorldDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            #[serde(default)]
            gravity: Option<crate::types::Vec2>,
            #[serde(default)]
            restitution_threshold: Option<f32>,
            #[serde(default)]
            hit_event_threshold: Option<f32>,
            #[serde(default)]
            contact_hertz: Option<f32>,
            #[serde(default)]
            contact_damping_ratio: Option<f32>,
            #[serde(default)]
            contact_speed: Option<f32>,
            #[serde(default)]
            maximum_linear_speed: Option<f32>,
            #[serde(default)]
            enable_sleep: Option<bool>,
            #[serde(default)]
            enable_continuous: Option<bool>,
            #[serde(default)]
            enable_contact_softening: Option<bool>,
            #[serde(default)]
            worker_count: Option<i32>,
        }
        let r = Repr::deserialize(deserializer)?;
        let mut b = WorldDef::default();
        if let Some(g) = r.gravity {
            b.0.gravity = ffi::b2Vec2::from(g);
        }
        if let Some(v) = r.restitution_threshold {
            b.0.restitutionThreshold = v;
        }
        if let Some(v) = r.hit_event_threshold {
            b.0.hitEventThreshold = v;
        }
        if let Some(v) = r.contact_hertz {
            b.0.contactHertz = v;
        }
        if let Some(v) = r.contact_damping_ratio {
            b.0.contactDampingRatio = v;
        }
        if let Some(v) = r.contact_speed {
            b.0.contactSpeed = v;
        }
        if let Some(v) = r.maximum_linear_speed {
            b.0.maximumLinearSpeed = v;
        }
        if let Some(v) = r.enable_sleep {
            b.0.enableSleep = v;
        }
        if let Some(v) = r.enable_continuous {
            b.0.enableContinuous = v;
        }
        if let Some(v) = r.enable_contact_softening {
            b.0.enableContactSoftening = v;
        }
        if let Some(v) = r.worker_count {
            b.0.workerCount = v;
        }
        Ok(b)
    }
}

/// Fluent builder for `WorldDef`.
///
/// Chain configuration calls and finish with `build()`. All fields map 1:1 to
/// the upstream `b2WorldDef`.
#[doc(alias = "world_builder")]
#[doc(alias = "worldbuilder")]
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

/// A simulation world.
///
/// Note: the underlying Box2D world is owned by an internal reference-counted core, so it will
/// be destroyed when the last owned handle (`OwnedBody`/`OwnedShape`/`OwnedJoint`/`OwnedChain`)
/// is dropped.
pub struct World {
    core: Arc<WorldCore>,
    // Box2D's external API is not thread-safe; prevent `World: Send/Sync`.
    _not_send_sync: core::marker::PhantomData<Rc<()>>,
}

/// A cheap, cloneable handle to a world that keeps it alive via the internal reference-counted core.
///
/// Unlike `&World`, this does not borrow the world, which makes it convenient to store inside other
/// objects (e.g. debug draw implementations). It is still `!Send`/`!Sync` to match Box2D's thread
/// safety guarantees.
#[derive(Clone)]
pub struct WorldHandle {
    core: Arc<WorldCore>,
    _not_send_sync: core::marker::PhantomData<Rc<()>>,
}

impl WorldHandle {
    /// Expose raw world id for advanced use-cases.
    pub fn raw(&self) -> ffi::b2WorldId {
        self.core.id
    }
}

#[cfg(feature = "serialize")]
pub use crate::core::serialize_registry::{ChainCreateRecord, ShapeFlagsRecord};

impl World {
    /// Create a world from a definition.
    pub fn new(def: WorldDef) -> Result<Self, Error> {
        let _guard = crate::core::box2d_lock::lock();
        let raw = def.into_raw();
        // SAFETY: FFI call to create a world; returns an id handle
        let world_id = unsafe { ffi::b2CreateWorld(&raw) };
        let ok = unsafe { ffi::b2World_IsValid(world_id) };
        if ok {
            Ok(Self {
                core: WorldCore::new(world_id),
                _not_send_sync: core::marker::PhantomData,
            })
        } else {
            Err(Error::CreateFailed)
        }
    }

    /// Step the simulation by `time_step` seconds using `sub_steps` sub-steps.
    pub fn step(&mut self, time_step: f32, sub_steps: i32) {
        crate::core::callback_state::assert_not_in_callback();
        // Prepare panic forwarding for callbacks invoked during the FFI call.
        self.core
            .callback_panicked
            .store(false, std::sync::atomic::Ordering::Relaxed);
        *self
            .core
            .callback_panic
            .lock()
            .expect("callback_panic mutex poisoned") = None;
        // SAFETY: valid world id managed by RAII
        unsafe { ffi::b2World_Step(self.raw(), time_step, sub_steps) };

        // Flush deferred destroys scheduled from callbacks.
        self.core.process_deferred_destroys();

        if self
            .core
            .callback_panicked
            .load(std::sync::atomic::Ordering::Relaxed)
            && let Some(payload) = self
                .core
                .callback_panic
                .lock()
                .expect("callback_panic mutex poisoned")
                .take()
        {
            std::panic::resume_unwind(payload);
        }
    }

    /// Flush deferred destroys scheduled from Box2D callbacks.
    ///
    /// Most users don't need to call this because `World::step` and debug draw helpers flush
    /// automatically. This is useful if you drop `Owned*` handles during callbacks but want to
    /// reclaim resources without stepping the simulation again.
    pub fn flush_deferred_destroys(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        self.core.process_deferred_destroys();
    }

    /// Set gravity vector.
    pub fn set_gravity<V: Into<Vec2>>(&mut self, g: V) {
        crate::core::callback_state::assert_not_in_callback();
        let gv: ffi::b2Vec2 = g.into().into();
        unsafe { ffi::b2World_SetGravity(self.raw(), gv) };
    }

    pub fn try_set_gravity<V: Into<Vec2>>(&mut self, g: V) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        let gv: ffi::b2Vec2 = g.into().into();
        unsafe { ffi::b2World_SetGravity(self.raw(), gv) };
        Ok(())
    }

    /// Get current gravity vector.
    pub fn gravity(&self) -> Vec2 {
        crate::core::callback_state::assert_not_in_callback();
        Vec2::from(unsafe { ffi::b2World_GetGravity(self.raw()) })
    }

    pub fn try_gravity(&self) -> crate::error::ApiResult<Vec2> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(Vec2::from(unsafe { ffi::b2World_GetGravity(self.raw()) }))
    }

    /// Expose raw id for advanced use-cases.
    pub fn raw(&self) -> ffi::b2WorldId {
        self.core.id
    }

    pub(crate) fn core_arc(&self) -> Arc<WorldCore> {
        Arc::clone(&self.core)
    }

    /// Create a cheap, cloneable handle to this world.
    pub fn handle(&self) -> WorldHandle {
        WorldHandle {
            core: Arc::clone(&self.core),
            _not_send_sync: core::marker::PhantomData,
        }
    }

    /// Number of outstanding owned handles (`OwnedBody`/`OwnedShape`/`OwnedJoint`/`OwnedChain`).
    pub fn owned_handle_count(&self) -> usize {
        Arc::strong_count(&self.core).saturating_sub(1)
    }

    pub fn owned_handle_counts(&self) -> OwnedHandleCounts {
        let (bodies, shapes, joints, chains) = self.core.owned_counts();
        OwnedHandleCounts {
            bodies,
            shapes,
            joints,
            chains,
        }
    }

    /// Attempt to destroy the world by consuming `self`.
    ///
    /// This returns an error if there are still owned handles alive, because they keep the world
    /// core reference-counted and prevent destruction.
    pub fn shutdown(self) -> Result<(), (Self, OutstandingOwnedHandles)> {
        let strong = Arc::strong_count(&self.core);
        if strong == 1 {
            Ok(())
        } else {
            let (bodies, shapes, joints, chains) = self.core.owned_counts();
            Err((
                self,
                OutstandingOwnedHandles {
                    strong_count: strong,
                    counts: OwnedHandleCounts {
                        bodies,
                        shapes,
                        joints,
                        chains,
                    },
                },
            ))
        }
    }

    /// Enumerate known body ids created via this wrapper. Invalid/destroyed ids are filtered out.
    #[cfg(feature = "serialize")]
    pub fn body_ids(&self) -> Vec<BodyId> {
        crate::core::callback_state::assert_not_in_callback();
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .body_ids()
    }

    /// Return chain creation records captured at creation time.
    #[cfg(feature = "serialize")]
    pub fn chain_records(&self) -> Vec<ChainCreateRecord> {
        crate::core::callback_state::assert_not_in_callback();
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .chain_records()
    }

    /// Return recorded shape flags for shapes created via this wrapper.
    #[cfg(feature = "serialize")]
    pub fn shape_flags(&self, sid: ffi::b2ShapeId) -> Option<ShapeFlagsRecord> {
        self.core
            .registries
            .lock()
            .expect("registries mutex poisoned")
            .shape_flags(sid)
    }

    /// World counters snapshot (sizes, tree heights, etc.).
    pub fn counters(&self) -> Counters {
        crate::core::callback_state::assert_not_in_callback();
        let c = unsafe { ffi::b2World_GetCounters(self.raw()) };
        Counters::from(c)
    }

    pub fn try_counters(&self) -> crate::error::ApiResult<Counters> {
        crate::core::callback_state::check_not_in_callback()?;
        let c = unsafe { ffi::b2World_GetCounters(self.raw()) };
        Ok(Counters::from(c))
    }

    /// Get a body's transform safely from its id.
    pub fn body_transform(&self, body: BodyId) -> Transform {
        crate::core::debug_checks::assert_body_valid(body);
        Transform::from(unsafe { ffi::b2Body_GetTransform(body) })
    }

    pub fn try_body_transform(&self, body: BodyId) -> crate::error::ApiResult<Transform> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(Transform::from(unsafe { ffi::b2Body_GetTransform(body) }))
    }

    /// Get a body's world position.
    pub fn body_position(&self, body: BodyId) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        Vec2::from(unsafe { ffi::b2Body_GetPosition(body) })
    }

    pub fn try_body_position(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(Vec2::from(unsafe { ffi::b2Body_GetPosition(body) }))
    }

    pub fn body_local_point<V: Into<Vec2>>(&self, body: BodyId, world_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let p: ffi::b2Vec2 = world_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalPoint(body, p) })
    }

    pub fn try_body_local_point<V: Into<Vec2>>(
        &self,
        body: BodyId,
        world_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let p: ffi::b2Vec2 = world_point.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetLocalPoint(body, p) }))
    }

    pub fn body_world_point<V: Into<Vec2>>(&self, body: BodyId, local_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let p: ffi::b2Vec2 = local_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldPoint(body, p) })
    }

    pub fn try_body_world_point<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let p: ffi::b2Vec2 = local_point.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetWorldPoint(body, p) }))
    }

    pub fn body_local_vector<V: Into<Vec2>>(&self, body: BodyId, world_vector: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let v: ffi::b2Vec2 = world_vector.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalVector(body, v) })
    }

    pub fn try_body_local_vector<V: Into<Vec2>>(
        &self,
        body: BodyId,
        world_vector: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let v: ffi::b2Vec2 = world_vector.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetLocalVector(body, v) }))
    }

    pub fn body_world_vector<V: Into<Vec2>>(&self, body: BodyId, local_vector: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let v: ffi::b2Vec2 = local_vector.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldVector(body, v) })
    }

    pub fn try_body_world_vector<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_vector: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let v: ffi::b2Vec2 = local_vector.into().into();
        Ok(Vec2::from(unsafe { ffi::b2Body_GetWorldVector(body, v) }))
    }

    pub fn body_local_point_velocity<V: Into<Vec2>>(&self, body: BodyId, local_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let p: ffi::b2Vec2 = local_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetLocalPointVelocity(body, p) })
    }

    pub fn try_body_local_point_velocity<V: Into<Vec2>>(
        &self,
        body: BodyId,
        local_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let p: ffi::b2Vec2 = local_point.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLocalPointVelocity(body, p)
        }))
    }

    pub fn body_world_point_velocity<V: Into<Vec2>>(&self, body: BodyId, world_point: V) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        let p: ffi::b2Vec2 = world_point.into().into();
        Vec2::from(unsafe { ffi::b2Body_GetWorldPointVelocity(body, p) })
    }

    pub fn try_body_world_point_velocity<V: Into<Vec2>>(
        &self,
        body: BodyId,
        world_point: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        let p: ffi::b2Vec2 = world_point.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetWorldPointVelocity(body, p)
        }))
    }

    pub fn body_mass(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_GetMass(body) }
    }

    pub fn try_body_mass(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(unsafe { ffi::b2Body_GetMass(body) })
    }

    pub fn body_rotational_inertia(&self, body: BodyId) -> f32 {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_GetRotationalInertia(body) }
    }

    pub fn try_body_rotational_inertia(&self, body: BodyId) -> crate::error::ApiResult<f32> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(unsafe { ffi::b2Body_GetRotationalInertia(body) })
    }

    pub fn body_local_center_of_mass(&self, body: BodyId) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        Vec2::from(unsafe { ffi::b2Body_GetLocalCenterOfMass(body) })
    }

    pub fn try_body_local_center_of_mass(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetLocalCenterOfMass(body)
        }))
    }

    pub fn body_world_center_of_mass(&self, body: BodyId) -> Vec2 {
        crate::core::debug_checks::assert_body_valid(body);
        Vec2::from(unsafe { ffi::b2Body_GetWorldCenterOfMass(body) })
    }

    pub fn try_body_world_center_of_mass(&self, body: BodyId) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(Vec2::from(unsafe {
            ffi::b2Body_GetWorldCenterOfMass(body)
        }))
    }

    pub fn body_mass_data(&self, body: BodyId) -> MassData {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_GetMassData(body) }
    }

    pub fn try_body_mass_data(&self, body: BodyId) -> crate::error::ApiResult<MassData> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(unsafe { ffi::b2Body_GetMassData(body) })
    }

    pub fn set_body_mass_data(&mut self, body: BodyId, mass_data: MassData) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_SetMassData(body, mass_data) };
    }

    pub fn try_set_body_mass_data(
        &mut self,
        body: BodyId,
        mass_data: MassData,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_SetMassData(body, mass_data) };
        Ok(())
    }

    pub fn body_apply_mass_from_shapes(&mut self, body: BodyId) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_ApplyMassFromShapes(body) };
    }

    pub fn try_body_apply_mass_from_shapes(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_ApplyMassFromShapes(body) };
        Ok(())
    }

    pub fn set_body_target_transform(
        &mut self,
        body: BodyId,
        target: Transform,
        time_step: f32,
        wake: bool,
    ) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_SetTargetTransform(body, target.into(), time_step, wake) };
    }

    pub fn try_set_body_target_transform(
        &mut self,
        body: BodyId,
        target: Transform,
        time_step: f32,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_SetTargetTransform(body, target.into(), time_step, wake) };
        Ok(())
    }

    /// Set a body's world position and rotation (angle in radians) by id.
    pub fn set_body_position_and_rotation<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        p: V,
        angle_radians: f32,
    ) {
        crate::core::debug_checks::assert_body_valid(body);
        let (s, c) = angle_radians.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Vec2 = p.into().into();
        unsafe { ffi::b2Body_SetTransform(body, pos, rot) };
    }

    pub fn try_set_body_position_and_rotation<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        p: V,
        angle_radians: f32,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        let (s, c) = angle_radians.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Vec2 = p.into().into();
        unsafe { ffi::b2Body_SetTransform(body, pos, rot) };
        Ok(())
    }

    /// Set a body's linear velocity by id.
    pub fn set_body_linear_velocity<V: Into<Vec2>>(&mut self, body: BodyId, v: V) {
        crate::core::debug_checks::assert_body_valid(body);
        let vv: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2Body_SetLinearVelocity(body, vv) }
    }

    pub fn try_set_body_linear_velocity<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        v: V,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        let vv: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2Body_SetLinearVelocity(body, vv) }
        Ok(())
    }

    /// Set a body's angular velocity by id.
    pub fn set_body_angular_velocity(&mut self, body: BodyId, w: f32) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_SetAngularVelocity(body, w) }
    }

    pub fn try_set_body_angular_velocity(
        &mut self,
        body: BodyId,
        w: f32,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_SetAngularVelocity(body, w) }
        Ok(())
    }

    /// Get the current motion locks for a body.
    pub fn body_motion_locks(&self, body: BodyId) -> ffi::b2MotionLocks {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_GetMotionLocks(body) }
    }

    pub fn try_body_motion_locks(
        &self,
        body: BodyId,
    ) -> crate::error::ApiResult<ffi::b2MotionLocks> {
        crate::core::debug_checks::check_body_valid(body)?;
        Ok(unsafe { ffi::b2Body_GetMotionLocks(body) })
    }

    /// Set motion locks (translation/rotation constraints) for a body.
    pub fn set_body_motion_locks(&mut self, body: BodyId, locks: ffi::b2MotionLocks) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_SetMotionLocks(body, locks) }
    }

    pub fn try_set_body_motion_locks(
        &mut self,
        body: BodyId,
        locks: ffi::b2MotionLocks,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_SetMotionLocks(body, locks) }
        Ok(())
    }

    /// Apply a linear impulse to the center of mass of a body.
    pub fn body_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        impulse: V,
        wake: bool,
    ) {
        crate::core::debug_checks::assert_body_valid(body);
        let i: ffi::b2Vec2 = impulse.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(body, i, wake) };
    }

    pub fn try_body_apply_linear_impulse_to_center<V: Into<Vec2>>(
        &mut self,
        body: BodyId,
        impulse: V,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        let i: ffi::b2Vec2 = impulse.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(body, i, wake) };
        Ok(())
    }

    /// Apply an angular impulse to a body.
    pub fn body_apply_angular_impulse(&mut self, body: BodyId, impulse: f32, wake: bool) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_ApplyAngularImpulse(body, impulse, wake) };
    }

    pub fn try_body_apply_angular_impulse(
        &mut self,
        body: BodyId,
        impulse: f32,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_ApplyAngularImpulse(body, impulse, wake) };
        Ok(())
    }

    /// Clear accumulated forces and torque on a body (usually only needed before stepping).
    pub fn body_clear_forces(&mut self, body: BodyId) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_ClearForces(body) };
    }

    pub fn try_body_clear_forces(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_ClearForces(body) };
        Ok(())
    }

    /// Wake all touching bodies.
    pub fn body_wake_touching(&mut self, body: BodyId) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_WakeTouching(body) };
    }

    pub fn try_body_wake_touching(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_WakeTouching(body) };
        Ok(())
    }

    /// Set a body's type by id.
    pub fn set_body_type(&mut self, body: BodyId, t: BodyType) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_SetType(body, t.into()) }
    }

    pub fn try_set_body_type(&mut self, body: BodyId, t: BodyType) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_SetType(body, t.into()) }
        Ok(())
    }

    /// Enable a body by id.
    pub fn enable_body(&mut self, body: BodyId) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_Enable(body) }
    }

    pub fn try_enable_body(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_Enable(body) }
        Ok(())
    }

    /// Disable a body by id.
    pub fn disable_body(&mut self, body: BodyId) {
        crate::core::debug_checks::assert_body_valid(body);
        unsafe { ffi::b2Body_Disable(body) }
    }

    pub fn try_disable_body(&mut self, body: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        unsafe { ffi::b2Body_Disable(body) }
        Ok(())
    }

    /// Set a body's name by id.
    pub fn set_body_name(&mut self, body: BodyId, name: &str) {
        crate::core::debug_checks::assert_body_valid(body);
        let cs = CString::new(name).expect("body name contains an interior NUL byte");
        unsafe { ffi::b2Body_SetName(body, cs.as_ptr()) }
    }

    pub fn try_set_body_name(&mut self, body: BodyId, name: &str) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(body)?;
        let cs = CString::new(name).map_err(|_| crate::error::ApiError::NulByteInString)?;
        unsafe { ffi::b2Body_SetName(body, cs.as_ptr()) }
        Ok(())
    }

    /// Get number of awake bodies.
    pub fn awake_body_count(&self) -> i32 {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_GetAwakeBodyCount(self.raw()) }
    }

    pub fn try_awake_body_count(&self) -> crate::error::ApiResult<i32> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(unsafe { ffi::b2World_GetAwakeBodyCount(self.raw()) })
    }

    /// Create a body owned by this world.
    pub fn create_body<'w>(&'w mut self, def: BodyDef) -> Body<'w> {
        crate::core::callback_state::assert_not_in_callback();
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        Body::new(self.core_arc(), id)
    }

    pub fn try_create_body<'w>(&'w mut self, def: BodyDef) -> crate::error::ApiResult<Body<'w>> {
        crate::core::callback_state::check_not_in_callback()?;
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        Ok(Body::new(self.core_arc(), id))
    }

    /// Create a RAII-owned body. Dropping the returned handle destroys the body.
    pub fn create_body_owned(&mut self, def: BodyDef) -> crate::body::OwnedBody {
        crate::core::callback_state::assert_not_in_callback();
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        crate::body::OwnedBody::new(self.core_arc(), id)
    }

    pub fn try_create_body_owned(
        &mut self,
        def: BodyDef,
    ) -> crate::error::ApiResult<crate::body::OwnedBody> {
        crate::core::callback_state::check_not_in_callback()?;
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        Ok(crate::body::OwnedBody::new(self.core_arc(), id))
    }

    /// ID-style body creation. Prefer when you want to store/pass ids without borrowing the world.
    pub fn create_body_id(&mut self, def: BodyDef) -> BodyId {
        crate::core::callback_state::assert_not_in_callback();
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        id
    }

    pub fn try_create_body_id(&mut self, def: BodyDef) -> crate::error::ApiResult<BodyId> {
        crate::core::callback_state::check_not_in_callback()?;
        let raw = def.0;
        let id = unsafe { ffi::b2CreateBody(self.raw(), &raw) };
        #[cfg(feature = "serialize")]
        {
            self.core.record_body(id);
        }
        Ok(id)
    }

    /// Destroy a body by id.
    pub fn destroy_body_id(&mut self, id: BodyId) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Body_IsValid(id) } {
            #[cfg(feature = "serialize")]
            self.core.cleanup_before_destroy_body(id);
            unsafe { ffi::b2DestroyBody(id) };
        }
    }

    pub fn try_destroy_body_id(&mut self, id: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(id)?;
        #[cfg(feature = "serialize")]
        self.core.cleanup_before_destroy_body(id);
        unsafe { ffi::b2DestroyBody(id) };
        Ok(())
    }

    /// Borrow a scoped body handle by id (returns `None` if the id is invalid).
    pub fn body<'w>(&'w mut self, id: BodyId) -> Option<Body<'w>> {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Body_IsValid(id) } {
            Some(Body::new(self.core_arc(), id))
        } else {
            None
        }
    }

    /// Borrow a scoped joint handle by id (returns `None` if the id is invalid).
    pub fn joint<'w>(&'w mut self, id: JointId) -> Option<crate::joints::Joint<'w>> {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Joint_IsValid(id) } {
            Some(crate::joints::Joint {
                id,
                _world: core::marker::PhantomData,
            })
        } else {
            None
        }
    }

    /// Borrow a scoped shape handle by id (returns `None` if the id is invalid).
    pub fn shape<'w>(&'w mut self, id: ShapeId) -> Option<crate::shapes::Shape<'w>> {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Shape_IsValid(id) } {
            Some(crate::shapes::Shape::new(self.core_arc(), id))
        } else {
            None
        }
    }

    /// Borrow a scoped chain handle by id (returns `None` if the id is invalid).
    pub fn chain<'w>(&'w mut self, id: ChainId) -> Option<crate::shapes::chain::Chain<'w>> {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Chain_IsValid(id) } {
            Some(crate::shapes::chain::Chain::new(self.core_arc(), id))
        } else {
            None
        }
    }

    // Runtime configuration helpers mirroring WorldDef fields
    pub fn enable_sleeping(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableSleeping(self.raw(), flag) }
    }
    pub fn is_sleeping_enabled(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_IsSleepingEnabled(self.raw()) }
    }
    pub fn enable_continuous(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableContinuous(self.raw(), flag) }
    }
    pub fn is_continuous_enabled(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_IsContinuousEnabled(self.raw()) }
    }
    /// Enable or disable constraint warm starting at runtime.
    ///
    /// Warm starting seeds the solver with accumulated impulses from the previous
    /// step to improve stability and convergence. Disabling this is only useful
    /// for experiments and will significantly reduce stability in most scenes.
    pub fn enable_warm_starting(&mut self, flag: bool) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_EnableWarmStarting(self.raw(), flag) }
    }
    /// Returns true if constraint warm starting is enabled.
    pub fn is_warm_starting_enabled(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_IsWarmStartingEnabled(self.raw()) }
    }
    pub fn set_restitution_threshold(&mut self, value: f32) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_SetRestitutionThreshold(self.raw(), value) }
    }
    pub fn restitution_threshold(&self) -> f32 {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_GetRestitutionThreshold(self.raw()) }
    }
    pub fn set_hit_event_threshold(&mut self, value: f32) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_SetHitEventThreshold(self.raw(), value) }
    }
    pub fn hit_event_threshold(&self) -> f32 {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_GetHitEventThreshold(self.raw()) }
    }
    pub fn set_contact_tuning(&mut self, hertz: f32, damping_ratio: f32, push_speed: f32) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_SetContactTuning(self.raw(), hertz, damping_ratio, push_speed) }
    }
    pub fn set_maximum_linear_speed(&mut self, v: f32) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_SetMaximumLinearSpeed(self.raw(), v) }
    }
    pub fn maximum_linear_speed(&self) -> f32 {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_GetMaximumLinearSpeed(self.raw()) }
    }

    // --- Collision/solve callbacks ---------------------------------------------------------
    /// Register a thread-safe custom filter closure. This is called when a contact pair is
    /// considered for collision if either shape has custom filtering enabled.
    /// Return false to disable the collision.
    ///
    /// Note: Box2D runs this callback while the world is locked. Calling into `boxdd` APIs from
    /// inside the closure will panic (to avoid re-entrant mutation of Box2D internals).
    pub fn set_custom_filter<F>(&mut self, f: F)
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        crate::core::callback_state::assert_not_in_callback();
        // Store the closure so its address is stable and lifetime tied to the world.
        let ctx = Box::new(CustomFilterCtx {
            core: Arc::downgrade(&self.core),
            cb: Box::new(f),
        });
        // SAFETY: callback shims only cast the context pointer back to CustomFilterCtx and
        // invoke the Rust closure. They must be extern "C" and thread-safe.
        unsafe extern "C" fn filter_cb(
            a: ffi::b2ShapeId,
            b: ffi::b2ShapeId,
            context: *mut core::ffi::c_void,
        ) -> bool {
            // SAFETY: context is provided by set_custom_filter and points to CustomFilterCtx
            let ctx = unsafe { &*(context as *const CustomFilterCtx) };
            let core = match ctx.core.upgrade() {
                Some(c) => c,
                None => return true,
            };
            if core
                .callback_panicked
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                return true;
            }
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                (ctx.cb)(a, b)
            })) {
                Ok(v) => v,
                Err(payload) => {
                    if !core
                        .callback_panicked
                        .swap(true, std::sync::atomic::Ordering::SeqCst)
                    {
                        *core
                            .callback_panic
                            .lock()
                            .expect("callback_panic mutex poisoned") = Some(payload);
                    }
                    true
                }
            }
        }
        let ctx_ptr: *mut core::ffi::c_void = (&*ctx) as *const CustomFilterCtx as *mut _;
        unsafe { ffi::b2World_SetCustomFilterCallback(self.raw(), Some(filter_cb), ctx_ptr) };
        *self
            .core
            .custom_filter
            .lock()
            .expect("custom_filter mutex poisoned") = Some(ctx);
    }

    /// Clear the custom filter callback and release associated resources.
    pub fn clear_custom_filter(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_SetCustomFilterCallback(self.raw(), None, core::ptr::null_mut()) };
        *self
            .core
            .custom_filter
            .lock()
            .expect("custom_filter mutex poisoned") = None;
    }

    /// Register a thread-safe pre-solve closure. This is called after contact update (when enabled
    /// on shapes) and before the solver. Return false to disable the contact this step.
    ///
    /// Note: Box2D runs this callback while the world is locked. Calling into `boxdd` APIs from
    /// inside the closure will panic (to avoid re-entrant mutation of Box2D internals).
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
        crate::core::callback_state::assert_not_in_callback();
        let ctx = Box::new(PreSolveCtx {
            core: Arc::downgrade(&self.core),
            cb: Box::new(f),
        });
        unsafe extern "C" fn presolve_cb(
            a: ffi::b2ShapeId,
            b: ffi::b2ShapeId,
            point: ffi::b2Vec2,
            normal: ffi::b2Vec2,
            context: *mut core::ffi::c_void,
        ) -> bool {
            // SAFETY: context is provided by set_pre_solve and points to PreSolveCtx
            let ctx = unsafe { &*(context as *const PreSolveCtx) };
            let core = match ctx.core.upgrade() {
                Some(c) => c,
                None => return true,
            };
            if core
                .callback_panicked
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                return true;
            }
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                (ctx.cb)(
                    a,
                    b,
                    crate::types::Vec2::from(point),
                    crate::types::Vec2::from(normal),
                )
            })) {
                Ok(v) => v,
                Err(payload) => {
                    if !core
                        .callback_panicked
                        .swap(true, std::sync::atomic::Ordering::SeqCst)
                    {
                        *core
                            .callback_panic
                            .lock()
                            .expect("callback_panic mutex poisoned") = Some(payload);
                    }
                    true
                }
            }
        }
        let ctx_ptr: *mut core::ffi::c_void = (&*ctx) as *const PreSolveCtx as *mut _;
        unsafe { ffi::b2World_SetPreSolveCallback(self.raw(), Some(presolve_cb), ctx_ptr) };
        *self
            .core
            .pre_solve
            .lock()
            .expect("pre_solve mutex poisoned") = Some(ctx);
    }

    /// Clear the pre-solve callback and release associated resources.
    pub fn clear_pre_solve(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2World_SetPreSolveCallback(self.raw(), None, core::ptr::null_mut()) };
        *self
            .core
            .pre_solve
            .lock()
            .expect("pre_solve mutex poisoned") = None;
    }

    /// Compatibility helper: set or clear the custom filter using a plain function pointer.
    pub fn set_custom_filter_callback(&mut self, cb: Option<ShapeFilterFn>) {
        crate::core::callback_state::assert_not_in_callback();
        match cb {
            Some(func) => self.set_custom_filter(func),
            None => self.clear_custom_filter(),
        }
    }

    /// Compatibility helper: set or clear the pre-solve using a plain function pointer.
    pub fn set_pre_solve_callback(&mut self, cb: Option<PreSolveFn>) {
        crate::core::callback_state::assert_not_in_callback();
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
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
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
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
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
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
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
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
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
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
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
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
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
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
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
        crate::core::debug_checks::assert_body_valid(body_a);
        crate::core::debug_checks::assert_body_valid(body_b);
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
        crate::core::debug_checks::assert_body_valid(body);
        let sid = unsafe { ffi::b2CreateCircleShape(body, &def.0, c) };
        #[cfg(feature = "serialize")]
        self.record_shape_flags(sid, &def.0);
        sid
    }
    pub fn create_circle_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        c: &ffi::b2Circle,
    ) -> crate::shapes::OwnedShape {
        let sid = self.create_circle_shape_for(body, def, c);
        crate::shapes::OwnedShape::new(self.core_arc(), sid)
    }
    pub fn create_segment_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        s: &ffi::b2Segment,
    ) -> ShapeId {
        crate::core::debug_checks::assert_body_valid(body);
        let sid = unsafe { ffi::b2CreateSegmentShape(body, &def.0, s) };
        #[cfg(feature = "serialize")]
        self.record_shape_flags(sid, &def.0);
        sid
    }
    pub fn create_segment_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        s: &ffi::b2Segment,
    ) -> crate::shapes::OwnedShape {
        let sid = self.create_segment_shape_for(body, def, s);
        crate::shapes::OwnedShape::new(self.core_arc(), sid)
    }
    pub fn create_capsule_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        c: &ffi::b2Capsule,
    ) -> ShapeId {
        crate::core::debug_checks::assert_body_valid(body);
        let sid = unsafe { ffi::b2CreateCapsuleShape(body, &def.0, c) };
        #[cfg(feature = "serialize")]
        self.record_shape_flags(sid, &def.0);
        sid
    }
    pub fn create_capsule_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        c: &ffi::b2Capsule,
    ) -> crate::shapes::OwnedShape {
        let sid = self.create_capsule_shape_for(body, def, c);
        crate::shapes::OwnedShape::new(self.core_arc(), sid)
    }
    pub fn create_polygon_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        p: &ffi::b2Polygon,
    ) -> ShapeId {
        crate::core::debug_checks::assert_body_valid(body);
        let sid = unsafe { ffi::b2CreatePolygonShape(body, &def.0, p) };
        #[cfg(feature = "serialize")]
        self.record_shape_flags(sid, &def.0);
        sid
    }
    pub fn create_polygon_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        p: &ffi::b2Polygon,
    ) -> crate::shapes::OwnedShape {
        let sid = self.create_polygon_shape_for(body, def, p);
        crate::shapes::OwnedShape::new(self.core_arc(), sid)
    }
    pub fn destroy_shape_id(&mut self, shape: ShapeId, update_body_mass: bool) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Shape_IsValid(shape) } {
            unsafe { ffi::b2DestroyShape(shape, update_body_mass) };
        }
        #[cfg(feature = "serialize")]
        {
            self.core.remove_shape_flags(shape);
        }
    }

    // Chain API (ID-style)
    pub fn create_chain_for_id(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> ChainId {
        crate::core::debug_checks::assert_body_valid(body);
        crate::shapes::chain::assert_chain_def_valid(def);
        let cid = unsafe { ffi::b2CreateChain(body, &def.def) };
        #[cfg(feature = "serialize")]
        {
            let meta = crate::core::serialize_registry::ChainCreateMeta::from_def(body, def);
            self.core.record_chain(cid, meta);
        }
        cid
    }

    pub fn try_create_chain_for_id(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> crate::error::ApiResult<ChainId> {
        crate::core::debug_checks::check_body_valid(body)?;
        crate::shapes::chain::check_chain_def_valid(def)?;
        let cid = unsafe { ffi::b2CreateChain(body, &def.def) };
        #[cfg(feature = "serialize")]
        {
            let meta = crate::core::serialize_registry::ChainCreateMeta::from_def(body, def);
            self.core.record_chain(cid, meta);
        }
        Ok(cid)
    }

    pub fn create_chain_for_owned(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> crate::shapes::chain::OwnedChain {
        let cid = self.create_chain_for_id(body, def);
        crate::shapes::chain::OwnedChain::new(self.core_arc(), cid)
    }

    pub fn try_create_chain_for_owned(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> crate::error::ApiResult<crate::shapes::chain::OwnedChain> {
        let cid = self.try_create_chain_for_id(body, def)?;
        Ok(crate::shapes::chain::OwnedChain::new(self.core_arc(), cid))
    }

    pub fn destroy_chain_id(&mut self, chain: ChainId) {
        crate::core::debug_checks::assert_chain_valid(chain);
        if unsafe { ffi::b2Chain_IsValid(chain) } {
            unsafe { ffi::b2DestroyChain(chain) };
        }
        #[cfg(feature = "serialize")]
        {
            self.core.remove_chain(chain);
        }
    }

    pub fn try_destroy_chain_id(&mut self, chain: ChainId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_chain_valid(chain)?;
        unsafe { ffi::b2DestroyChain(chain) };
        #[cfg(feature = "serialize")]
        {
            self.core.remove_chain(chain);
        }
        Ok(())
    }

    // Shape helpers (ID-style)
    pub fn shape_set_circle(&mut self, shape: ShapeId, c: &ffi::b2Circle) {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_SetCircle(shape, c) }
    }

    pub fn try_shape_set_circle(
        &mut self,
        shape: ShapeId,
        c: &ffi::b2Circle,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        unsafe { ffi::b2Shape_SetCircle(shape, c) }
        Ok(())
    }

    pub fn shape_set_segment(&mut self, shape: ShapeId, s: &ffi::b2Segment) {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_SetSegment(shape, s) }
    }

    pub fn try_shape_set_segment(
        &mut self,
        shape: ShapeId,
        s: &ffi::b2Segment,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        unsafe { ffi::b2Shape_SetSegment(shape, s) }
        Ok(())
    }

    pub fn shape_set_capsule(&mut self, shape: ShapeId, c: &ffi::b2Capsule) {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_SetCapsule(shape, c) }
    }

    pub fn try_shape_set_capsule(
        &mut self,
        shape: ShapeId,
        c: &ffi::b2Capsule,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        unsafe { ffi::b2Shape_SetCapsule(shape, c) }
        Ok(())
    }

    pub fn shape_set_polygon(&mut self, shape: ShapeId, p: &ffi::b2Polygon) {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_SetPolygon(shape, p) }
    }

    pub fn try_shape_set_polygon(
        &mut self,
        shape: ShapeId,
        p: &ffi::b2Polygon,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        unsafe { ffi::b2Shape_SetPolygon(shape, p) }
        Ok(())
    }

    pub fn shape_surface_material(&self, shape: ShapeId) -> SurfaceMaterial {
        crate::core::debug_checks::assert_shape_valid(shape);
        SurfaceMaterial(unsafe { ffi::b2Shape_GetSurfaceMaterial(shape) })
    }

    pub fn try_shape_surface_material(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<SurfaceMaterial> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(SurfaceMaterial(unsafe {
            ffi::b2Shape_GetSurfaceMaterial(shape)
        }))
    }

    pub fn shape_set_surface_material(&mut self, shape: ShapeId, material: &SurfaceMaterial) {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_SetSurfaceMaterial(shape, &material.0) }
    }

    pub fn try_shape_set_surface_material(
        &mut self,
        shape: ShapeId,
        material: &SurfaceMaterial,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        unsafe { ffi::b2Shape_SetSurfaceMaterial(shape, &material.0) }
        Ok(())
    }

    pub fn shape_body_id(&self, shape: ShapeId) -> BodyId {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_GetBody(shape) }
    }

    pub fn try_shape_body_id(&self, shape: ShapeId) -> crate::error::ApiResult<BodyId> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(unsafe { ffi::b2Shape_GetBody(shape) })
    }

    /// Return the closest point on a shape to `target` (in world coordinates).
    pub fn shape_closest_point<V: Into<Vec2>>(&self, shape: ShapeId, target: V) -> Vec2 {
        crate::core::debug_checks::assert_shape_valid(shape);
        let t: ffi::b2Vec2 = target.into().into();
        Vec2::from(unsafe { ffi::b2Shape_GetClosestPoint(shape, t) })
    }

    pub fn try_shape_closest_point<V: Into<Vec2>>(
        &self,
        shape: ShapeId,
        target: V,
    ) -> crate::error::ApiResult<Vec2> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        let t: ffi::b2Vec2 = target.into().into();
        Ok(Vec2::from(unsafe {
            ffi::b2Shape_GetClosestPoint(shape, t)
        }))
    }

    /// Apply wind force/torque approximation to a shape.
    pub fn shape_apply_wind<V: Into<Vec2>>(
        &mut self,
        shape: ShapeId,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) {
        crate::core::debug_checks::assert_shape_valid(shape);
        let w: ffi::b2Vec2 = wind.into().into();
        unsafe { ffi::b2Shape_ApplyWind(shape, w, drag, lift, wake) }
    }

    pub fn try_shape_apply_wind<V: Into<Vec2>>(
        &mut self,
        shape: ShapeId,
        wind: V,
        drag: f32,
        lift: f32,
        wake: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        let w: ffi::b2Vec2 = wind.into().into();
        unsafe { ffi::b2Shape_ApplyWind(shape, w, drag, lift, wake) }
        Ok(())
    }

    // Sensor helpers (ID-style)
    /// Get the maximum capacity required to retrieve sensor overlaps for a shape id.
    pub fn shape_sensor_capacity(&self, shape: ShapeId) -> i32 {
        crate::core::debug_checks::assert_shape_valid(shape);
        unsafe { ffi::b2Shape_GetSensorCapacity(shape) }
    }

    pub fn try_shape_sensor_capacity(&self, shape: ShapeId) -> crate::error::ApiResult<i32> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        Ok(unsafe { ffi::b2Shape_GetSensorCapacity(shape) })
    }

    /// Get overlapped shapes for a sensor shape id. Returns empty if not a sensor.
    pub fn shape_sensor_overlaps(&self, shape: ShapeId) -> Vec<ShapeId> {
        crate::core::debug_checks::assert_shape_valid(shape);
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

    pub fn try_shape_sensor_overlaps(
        &self,
        shape: ShapeId,
    ) -> crate::error::ApiResult<Vec<ShapeId>> {
        crate::core::debug_checks::check_shape_valid(shape)?;
        let cap = unsafe { ffi::b2Shape_GetSensorCapacity(shape) };
        if cap <= 0 {
            return Ok(Vec::new());
        }
        let mut ids: Vec<ShapeId> = Vec::with_capacity(cap as usize);
        let wrote =
            unsafe { ffi::b2Shape_GetSensorData(shape, ids.as_mut_ptr(), cap) }.max(0) as usize;
        unsafe { ids.set_len(wrote.min(cap as usize)) };
        Ok(ids)
    }
    /// Get overlapped shapes for a sensor shape id, filtered to valid (non-destroyed) ids.
    pub fn shape_sensor_overlaps_valid(&self, shape: ShapeId) -> Vec<ShapeId> {
        self.shape_sensor_overlaps(shape)
            .into_iter()
            .filter(|&sid| unsafe { ffi::b2Shape_IsValid(sid) })
            .collect()
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnedHandleCounts {
    pub bodies: usize,
    pub shapes: usize,
    pub joints: usize,
    pub chains: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct OutstandingOwnedHandles {
    /// `Arc` strong count of the internal world core, including the `World` itself.
    pub strong_count: usize,
    pub counts: OwnedHandleCounts,
}

impl OutstandingOwnedHandles {
    pub fn total(&self) -> usize {
        self.counts.bodies + self.counts.shapes + self.counts.joints + self.counts.chains
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
#[cfg(feature = "serialize")]
impl World {
    fn record_shape_flags(&mut self, sid: ffi::b2ShapeId, def: &ffi::b2ShapeDef) {
        self.core.record_shape_flags(sid, def);
    }
}
