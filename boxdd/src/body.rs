use std::marker::PhantomData;

use crate::types::{BodyId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;

/// Body types.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BodyType {
    Static,
    Kinematic,
    Dynamic,
}

impl From<BodyType> for ffi::b2BodyType {
    fn from(t: BodyType) -> Self {
        match t {
            BodyType::Static => ffi::b2BodyType_b2_staticBody,
            BodyType::Kinematic => ffi::b2BodyType_b2_kinematicBody,
            BodyType::Dynamic => ffi::b2BodyType_b2_dynamicBody,
        }
    }
}

/// Body definition wrapper with builder API.
#[derive(Clone, Debug)]
pub struct BodyDef(pub(crate) ffi::b2BodyDef);

impl Default for BodyDef {
    fn default() -> Self {
        let def = unsafe { ffi::b2DefaultBodyDef() };
        Self(def)
    }
}

/// Fluent builder for `BodyDef`.
#[doc(alias = "body_builder")]
#[doc(alias = "bodybuilder")]
///
/// Chain methods to configure a body and finish with `build()`. This maps
/// to the upstream `b2BodyDef` fields.
#[derive(Clone, Debug)]
pub struct BodyBuilder {
    def: BodyDef,
}

impl BodyBuilder {
    /// Start a new builder with default `BodyDef`.
    pub fn new() -> Self {
        Self {
            def: BodyDef::default(),
        }
    }
    /// Set the body type (static, kinematic, dynamic).
    pub fn body_type(mut self, t: BodyType) -> Self {
        self.def.0.type_ = t.into();
        self
    }
    /// Initial world-space position.
    pub fn position<V: Into<Vec2>>(mut self, p: V) -> Self {
        self.def.0.position = ffi::b2Vec2::from(p.into());
        self
    }
    /// Initial rotation in radians.
    pub fn angle(mut self, radians: f32) -> Self {
        // Build a rotation from angle
        let (s, c) = radians.sin_cos();
        self.def.0.rotation = ffi::b2Rot { c, s };
        self
    }
    /// Initial linear velocity (m/s).
    pub fn linear_velocity<V: Into<Vec2>>(mut self, v: V) -> Self {
        self.def.0.linearVelocity = ffi::b2Vec2::from(v.into());
        self
    }
    /// Initial angular velocity (rad/s).
    pub fn angular_velocity(mut self, v: f32) -> Self {
        self.def.0.angularVelocity = v;
        self
    }
    /// Linear damping (drag-like term).
    pub fn linear_damping(mut self, v: f32) -> Self {
        self.def.0.linearDamping = v;
        self
    }
    /// Angular damping.
    pub fn angular_damping(mut self, v: f32) -> Self {
        self.def.0.angularDamping = v;
        self
    }
    /// Per-body gravity scale (1 = normal gravity).
    pub fn gravity_scale(mut self, v: f32) -> Self {
        self.def.0.gravityScale = v;
        self
    }
    /// Allow body to go to sleep.
    pub fn enable_sleep(mut self, flag: bool) -> Self {
        self.def.0.enableSleep = flag;
        self
    }
    /// Awake/asleep flag at creation.
    pub fn awake(mut self, flag: bool) -> Self {
        self.def.0.isAwake = flag;
        self
    }
    /// Treat as bullet (CCD).
    pub fn bullet(mut self, flag: bool) -> Self {
        self.def.0.isBullet = flag;
        self
    }
    /// Enable/disable simulation for this body.
    pub fn enabled(mut self, flag: bool) -> Self {
        self.def.0.isEnabled = flag;
        self
    }

    #[must_use]
    pub fn build(self) -> BodyDef {
        self.def
    }
}

impl From<BodyDef> for BodyBuilder {
    fn from(def: BodyDef) -> Self {
        Self { def }
    }
}

// serde support for BodyDef via a transparent config struct
#[cfg(feature = "serde")]
impl serde::Serialize for BodyDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            body_type: super::body::BodyType,
            position: crate::types::Vec2,
            angle: f32,
            linear_velocity: crate::types::Vec2,
            angular_velocity: f32,
            linear_damping: f32,
            angular_damping: f32,
            gravity_scale: f32,
            enable_sleep: bool,
            awake: bool,
            bullet: bool,
            enabled: bool,
        }
        let angle = self.0.rotation.s.atan2(self.0.rotation.c);
        let r = Repr {
            body_type: match self.0.type_ {
                x if x == ffi::b2BodyType_b2_staticBody => BodyType::Static,
                x if x == ffi::b2BodyType_b2_kinematicBody => BodyType::Kinematic,
                _ => BodyType::Dynamic,
            },
            position: crate::types::Vec2::from(self.0.position),
            angle,
            linear_velocity: crate::types::Vec2::from(self.0.linearVelocity),
            angular_velocity: self.0.angularVelocity,
            linear_damping: self.0.linearDamping,
            angular_damping: self.0.angularDamping,
            gravity_scale: self.0.gravityScale,
            enable_sleep: self.0.enableSleep,
            awake: self.0.isAwake,
            bullet: self.0.isBullet,
            enabled: self.0.isEnabled,
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for BodyDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            body_type: super::body::BodyType,
            position: crate::types::Vec2,
            angle: f32,
            linear_velocity: crate::types::Vec2,
            angular_velocity: f32,
            linear_damping: f32,
            angular_damping: f32,
            gravity_scale: f32,
            enable_sleep: bool,
            awake: bool,
            bullet: bool,
            enabled: bool,
        }
        let r = Repr::deserialize(deserializer)?;
        let b = BodyBuilder::new()
            .body_type(r.body_type)
            .position(r.position)
            .angle(r.angle)
            .linear_velocity(r.linear_velocity)
            .angular_velocity(r.angular_velocity)
            .linear_damping(r.linear_damping)
            .angular_damping(r.angular_damping)
            .gravity_scale(r.gravity_scale)
            .enable_sleep(r.enable_sleep)
            .awake(r.awake)
            .bullet(r.bullet)
            .enabled(r.enabled);
        Ok(b.build())
    }
}

impl Default for BodyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A body handle with lifetime tied to the owning world.
pub struct Body<'w> {
    pub(crate) id: BodyId,
    _world: PhantomData<&'w World>,
}

impl<'w> Body<'w> {
    pub(crate) fn new(id: BodyId) -> Self {
        Self {
            id,
            _world: PhantomData,
        }
    }

    pub fn id(&self) -> BodyId {
        self.id
    }

    // Queries
    pub fn position(&self) -> Vec2 {
        Vec2::from(unsafe { ffi::b2Body_GetPosition(self.id) })
    }
    pub fn linear_velocity(&self) -> Vec2 {
        Vec2::from(unsafe { ffi::b2Body_GetLinearVelocity(self.id) })
    }
    pub fn angular_velocity(&self) -> f32 {
        unsafe { ffi::b2Body_GetAngularVelocity(self.id) }
    }
    pub fn transform(&self) -> ffi::b2Transform {
        unsafe { ffi::b2Body_GetTransform(self.id) }
    }
    pub fn transform_ex(&self) -> crate::Transform {
        crate::Transform::from(self.transform())
    }

    // Mutations
    pub fn set_position_and_rotation<V: Into<Vec2>>(&mut self, p: V, angle_radians: f32) {
        let (s, c) = angle_radians.sin_cos();
        let rot = ffi::b2Rot { c, s };
        let pos: ffi::b2Vec2 = p.into().into();
        unsafe { ffi::b2Body_SetTransform(self.id, pos, rot) };
    }
    pub fn set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        let vel: ffi::b2Vec2 = v.into().into();
        unsafe { ffi::b2Body_SetLinearVelocity(self.id, vel) }
    }
    pub fn set_angular_velocity(&mut self, w: f32) {
        unsafe { ffi::b2Body_SetAngularVelocity(self.id, w) }
    }

    pub fn contact_data(&self) -> Vec<ffi::b2ContactData> {
        let cap = unsafe { ffi::b2Body_GetContactCapacity(self.id) }.max(0) as usize;
        if cap == 0 {
            return Vec::new();
        }
        let mut vec: Vec<ffi::b2ContactData> = Vec::with_capacity(cap);
        let wrote = unsafe { ffi::b2Body_GetContactData(self.id, vec.as_mut_ptr(), cap as i32) }
            .max(0) as usize;
        unsafe { vec.set_len(wrote.min(cap)) };
        vec
    }

    // Forces/impulses
    pub fn apply_force<V: Into<Vec2>>(&mut self, force: V, point: V, wake: bool) {
        let f: ffi::b2Vec2 = force.into().into();
        let p: ffi::b2Vec2 = point.into().into();
        unsafe { ffi::b2Body_ApplyForce(self.id, f, p, wake) };
    }
    pub fn apply_force_to_center<V: Into<Vec2>>(&mut self, force: V, wake: bool) {
        let f: ffi::b2Vec2 = force.into().into();
        unsafe { ffi::b2Body_ApplyForceToCenter(self.id, f, wake) };
    }
    pub fn apply_torque(&mut self, torque: f32, wake: bool) {
        unsafe { ffi::b2Body_ApplyTorque(self.id, torque, wake) }
    }
    pub fn apply_linear_impulse<V: Into<Vec2>>(&mut self, impulse: V, point: V, wake: bool) {
        let i: ffi::b2Vec2 = impulse.into().into();
        let p: ffi::b2Vec2 = point.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulse(self.id, i, p, wake) };
    }
    pub fn apply_linear_impulse_to_center<V: Into<Vec2>>(&mut self, impulse: V, wake: bool) {
        let i: ffi::b2Vec2 = impulse.into().into();
        unsafe { ffi::b2Body_ApplyLinearImpulseToCenter(self.id, i, wake) };
    }
    pub fn apply_angular_impulse(&mut self, impulse: f32, wake: bool) {
        unsafe { ffi::b2Body_ApplyAngularImpulse(self.id, impulse, wake) }
    }

    // Names and user data (raw pointer)
    pub fn set_name(&mut self, name: &str) {
        if let Ok(cs) = CString::new(name) {
            unsafe { ffi::b2Body_SetName(self.id, cs.as_ptr()) }
        }
    }
    pub fn name(&self) -> Option<String> {
        let ptr = unsafe { ffi::b2Body_GetName(self.id) };
        if ptr.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }
    /// Set an opaque user data pointer on this body.
    ///
    /// # Safety
    /// The caller must ensure that `p` is either null or points to a valid object
    /// for the entire time the body may access it, and that any lifetimes/aliasing rules
    /// are upheld. Box2D treats this as an opaque pointer and may store/use it across steps.
    pub unsafe fn set_user_data_ptr(&mut self, p: *mut c_void) {
        unsafe { ffi::b2Body_SetUserData(self.id, p) }
    }
    pub fn user_data_ptr(&self) -> *mut c_void {
        unsafe { ffi::b2Body_GetUserData(self.id) }
    }
}

impl<'w> Drop for Body<'w> {
    fn drop(&mut self) {
        // Avoid double-destroy if already invalidated via ID-style API
        if unsafe { ffi::b2Body_IsValid(self.id) } {
            unsafe { ffi::b2DestroyBody(self.id) };
        }
    }
}
