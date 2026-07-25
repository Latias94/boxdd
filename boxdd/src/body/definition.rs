use crate::error::{ApiError, ApiResult};
use crate::types::{MassData, MotionLocks, Position, Vec2};
use boxdd_sys::ffi;
use std::ffi::{CStr, CString};

/// Body types.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BodyType {
    Static,
    Kinematic,
    Dynamic,
}

impl BodyType {
    #[inline]
    pub const fn into_raw(self) -> ffi::b2BodyType {
        match self {
            BodyType::Static => ffi::b2BodyType_b2_staticBody,
            BodyType::Kinematic => ffi::b2BodyType_b2_kinematicBody,
            BodyType::Dynamic => ffi::b2BodyType_b2_dynamicBody,
        }
    }

    /// Convert a raw Box2D body-type discriminant when it is known to this binding.
    #[inline]
    pub const fn from_raw(raw: ffi::b2BodyType) -> Option<Self> {
        match raw {
            ffi::b2BodyType_b2_staticBody => Some(Self::Static),
            ffi::b2BodyType_b2_kinematicBody => Some(Self::Kinematic),
            ffi::b2BodyType_b2_dynamicBody => Some(Self::Dynamic),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn decode_native(raw: ffi::b2BodyType) -> ApiResult<Self> {
        Self::from_raw(raw).ok_or(ApiError::InvalidNativeBodyType { raw })
    }
}

impl TryFrom<ffi::b2BodyType> for BodyType {
    type Error = ffi::b2BodyType;

    #[inline]
    fn try_from(value: ffi::b2BodyType) -> Result<Self, Self::Error> {
        Self::from_raw(value).ok_or(value)
    }
}

#[inline]
fn body_type_is_known(raw: ffi::b2BodyType) -> bool {
    BodyType::from_raw(raw).is_some()
}

#[inline]
pub(crate) fn assert_non_negative_finite_body_scalar(name: &str, value: f32) {
    assert!(
        value.is_finite() && value >= 0.0,
        "{name} must be finite and >= 0.0, got {value}"
    );
}

#[inline]
pub(crate) fn check_non_negative_finite_body_scalar(value: f32) -> ApiResult<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
pub(crate) fn assert_mass_data_valid(mass_data: MassData) {
    assert_non_negative_finite_body_scalar("mass", mass_data.mass);
    assert_non_negative_finite_body_scalar("rotational_inertia", mass_data.rotational_inertia);
    assert!(
        mass_data.center.is_valid(),
        "mass_data.center must be a valid Box2D vector, got {:?}",
        mass_data.center
    );
}

#[inline]
pub(crate) fn check_mass_data_valid(mass_data: MassData) -> ApiResult<()> {
    check_non_negative_finite_body_scalar(mass_data.mass)?;
    check_non_negative_finite_body_scalar(mass_data.rotational_inertia)?;
    if mass_data.center.is_valid() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

pub(crate) fn assert_body_def_valid(def: &BodyDef) {
    assert!(
        def.name()
            .is_none_or(|name| name.to_bytes().len() <= super::MAX_BODY_NAME_BYTES),
        "invalid BodyDef: name must contain at most {} bytes",
        super::MAX_BODY_NAME_BYTES
    );
    assert!(
        body_type_is_known(def.0.type_),
        "invalid BodyDef: unknown body type value {}",
        def.0.type_
    );
    assert!(
        Position::from_raw(def.0.position).is_valid(),
        "invalid BodyDef: position must be a valid Box2D world position"
    );
    assert!(
        crate::Rot::from_raw(def.0.rotation).is_valid(),
        "invalid BodyDef: rotation must be a valid Box2D rotation"
    );
    assert!(
        Vec2::from_raw(def.0.linearVelocity).is_valid(),
        "invalid BodyDef: linearVelocity must be a valid Box2D vector"
    );
    assert!(
        crate::is_valid_float(def.0.angularVelocity),
        "invalid BodyDef: angularVelocity must be finite"
    );
    assert_non_negative_finite_body_scalar("linearDamping", def.0.linearDamping);
    assert_non_negative_finite_body_scalar("angularDamping", def.0.angularDamping);
    assert_non_negative_finite_body_scalar("sleepThreshold", def.0.sleepThreshold);
    assert!(
        crate::is_valid_float(def.0.gravityScale),
        "invalid BodyDef: gravityScale must be finite"
    );
    let _lease = crate::core::foundation::assert_transient_native_lease();
    assert!(
        def.0.internalValue == unsafe { ffi::b2DefaultBodyDef() }.internalValue,
        "invalid BodyDef: not initialized from b2DefaultBodyDef"
    );
}

pub(crate) fn check_body_def_valid(def: &BodyDef) -> ApiResult<()> {
    if def
        .name()
        .is_some_and(|name| name.to_bytes().len() > super::MAX_BODY_NAME_BYTES)
        || !body_type_is_known(def.0.type_)
        || !Position::from_raw(def.0.position).is_valid()
        || !crate::Rot::from_raw(def.0.rotation).is_valid()
        || !Vec2::from_raw(def.0.linearVelocity).is_valid()
        || !crate::is_valid_float(def.0.angularVelocity)
        || check_non_negative_finite_body_scalar(def.0.linearDamping).is_err()
        || check_non_negative_finite_body_scalar(def.0.angularDamping).is_err()
        || check_non_negative_finite_body_scalar(def.0.sleepThreshold).is_err()
        || !crate::is_valid_float(def.0.gravityScale)
    {
        return Err(ApiError::InvalidArgument);
    }
    let _lease = crate::core::foundation::transient_native_lease()?;
    if def.0.internalValue == unsafe { ffi::b2DefaultBodyDef() }.internalValue {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

/// Body definition wrapper with builder API.
///
/// The wrapper owns the optional body name and keeps the raw `name` pointer bound to that owned
/// allocation. Use [`BodyDef::into_raw_guard`] when passing the definition to an unsafe API.
#[derive(Debug)]
pub struct BodyDef(pub(crate) ffi::b2BodyDef, Option<CString>);

impl Clone for BodyDef {
    fn clone(&self) -> Self {
        Self::from_parts(self.0, self.1.clone())
    }
}

/// An owned raw body definition whose pointer-bearing fields remain valid.
///
/// Keep this guard alive for the complete duration of any unsafe FFI call that reads the returned
/// raw definition or pointer.
#[derive(Debug)]
pub struct RawBodyDef {
    raw: ffi::b2BodyDef,
    _name: Option<CString>,
}

impl RawBodyDef {
    /// Borrow the raw Box2D body definition.
    #[inline]
    pub fn as_raw(&self) -> &ffi::b2BodyDef {
        &self.raw
    }

    /// Return a pointer to the raw definition stored inside this guard.
    ///
    /// The pointer is invalidated if the guard is moved or dropped. Callers must finish using the
    /// pointer before either operation; prefer [`Self::as_raw`] when a borrowed definition is
    /// accepted.
    #[inline]
    pub fn as_ptr(&self) -> *const ffi::b2BodyDef {
        &self.raw
    }
}

impl Default for BodyDef {
    fn default() -> Self {
        let _lease = crate::core::foundation::assert_transient_native_lease();
        let def = unsafe { ffi::b2DefaultBodyDef() };
        Self::from_parts(def, None)
    }
}

impl BodyDef {
    #[inline]
    fn from_parts(mut raw: ffi::b2BodyDef, name: Option<CString>) -> Self {
        raw.name = name.as_ref().map_or(std::ptr::null(), |name| name.as_ptr());
        Self(raw, name)
    }

    #[inline]
    fn set_name(&mut self, name: Option<CString>) {
        self.1 = name;
        self.0.name = self
            .1
            .as_ref()
            .map_or(std::ptr::null(), |name| name.as_ptr());
    }

    /// Start building a new `BodyDef` from defaults.
    pub fn builder() -> BodyBuilder {
        BodyBuilder::new()
    }

    /// Construct from the raw Box2D body definition value.
    ///
    /// # Safety
    /// If `raw.name` is non-null, it must point to a readable NUL-terminated string for the
    /// duration of this call. The string is copied before this function returns.
    #[inline]
    pub unsafe fn from_raw(raw: ffi::b2BodyDef) -> Self {
        let name = if raw.name.is_null() {
            None
        } else {
            // SAFETY: The caller guarantees that `raw.name` points to a readable C string for
            // this call. `to_owned` copies it before the borrowed view can escape.
            Some(unsafe { CStr::from_ptr(raw.name) }.to_owned())
        };
        Self::from_parts(raw, name)
    }

    /// Optional name assigned when the body is created.
    #[inline]
    pub fn name(&self) -> Option<&CStr> {
        self.1.as_deref()
    }

    /// Body type used when the body is created.
    #[inline]
    pub fn body_type(&self) -> Option<BodyType> {
        BodyType::from_raw(self.0.type_)
    }

    /// Initial world-space position.
    #[inline]
    pub fn position(&self) -> Position {
        Position::from_raw(self.0.position)
    }

    /// Initial rotation value.
    #[inline]
    pub fn rotation(&self) -> crate::Rot {
        crate::Rot::from_raw(self.0.rotation)
    }

    /// Initial angle in radians.
    #[inline]
    pub fn angle(&self) -> f32 {
        self.rotation().angle()
    }

    /// Initial linear velocity in m/s.
    #[inline]
    pub fn linear_velocity(&self) -> Vec2 {
        Vec2::from_raw(self.0.linearVelocity)
    }

    /// Initial angular velocity in rad/s.
    #[inline]
    pub fn angular_velocity(&self) -> f32 {
        self.0.angularVelocity
    }

    /// Linear damping.
    #[inline]
    pub fn linear_damping(&self) -> f32 {
        self.0.linearDamping
    }

    /// Angular damping.
    #[inline]
    pub fn angular_damping(&self) -> f32 {
        self.0.angularDamping
    }

    /// Linear speed below which the body may transition to sleep.
    #[inline]
    pub fn sleep_threshold(&self) -> f32 {
        self.0.sleepThreshold
    }

    /// Per-body gravity scale.
    #[inline]
    pub fn gravity_scale(&self) -> f32 {
        self.0.gravityScale
    }

    /// Whether sleeping is enabled at creation.
    #[inline]
    pub fn is_sleep_enabled(&self) -> bool {
        self.0.enableSleep
    }

    /// Whether the body starts awake.
    #[inline]
    pub fn is_awake(&self) -> bool {
        self.0.isAwake
    }

    /// Whether the body starts as a bullet.
    #[inline]
    pub fn is_bullet(&self) -> bool {
        self.0.isBullet
    }

    /// Whether the body allows fast rotation without Box2D's default clamp.
    #[inline]
    pub fn is_fast_rotation_allowed(&self) -> bool {
        self.0.allowFastRotation
    }

    /// Per-axis motion locks applied when the body is created.
    #[inline]
    pub fn motion_locks(&self) -> MotionLocks {
        MotionLocks {
            linear_x: self.0.motionLocks.linearX,
            linear_y: self.0.motionLocks.linearY,
            angular_z: self.0.motionLocks.angularZ,
        }
    }

    /// Whether newly created contacts may recycle this body's previous manifolds.
    #[inline]
    pub fn is_contact_recycling_enabled(&self) -> bool {
        self.0.enableContactRecycling
    }

    /// Whether the body starts enabled for simulation.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.0.isEnabled
    }

    /// Convert into an owned raw guard that preserves pointer-bearing fields.
    #[inline]
    pub fn into_raw_guard(self) -> RawBodyDef {
        let Self(raw, name) = self;
        let mut guard = RawBodyDef { raw, _name: name };
        guard.raw.name = guard
            ._name
            .as_ref()
            .map_or(std::ptr::null(), |name| name.as_ptr());
        guard
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
        check_body_def_valid(self)
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
        self.def.0.type_ = t.into_raw();
        self
    }
    /// Set the optional body name.
    #[track_caller]
    pub fn name(mut self, name: &str) -> Self {
        self.def.1 = Some(super::assert_valid_body_name(name));
        self.def.0.name = self
            .def
            .1
            .as_ref()
            .map_or(std::ptr::null(), |name| name.as_ptr());
        self
    }
    /// Set the optional body name, returning an error for an interior NUL or an oversized name.
    pub fn try_name(mut self, name: &str) -> ApiResult<Self> {
        self.def.set_name(Some(super::check_valid_body_name(name)?));
        Ok(self)
    }
    /// Remove a previously configured body name.
    pub fn clear_name(mut self) -> Self {
        self.def.set_name(None);
        self
    }
    /// Initial world-space position.
    pub fn position<P: Into<Position>>(mut self, p: P) -> Self {
        self.def.0.position = p.into().into_raw();
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
        self.def.0.linearVelocity = v.into().into_raw();
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
    /// Linear speed below which the body may transition to sleep.
    pub fn sleep_threshold(mut self, v: f32) -> Self {
        self.def.0.sleepThreshold = v;
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
    /// Allow high angular speed without Box2D's default clamp.
    pub fn allow_fast_rotation(mut self, flag: bool) -> Self {
        self.def.0.allowFastRotation = flag;
        self
    }
    /// Lock selected translation and rotation axes at creation.
    pub fn motion_locks(mut self, locks: MotionLocks) -> Self {
        self.def.0.motionLocks = locks.into_raw();
        self
    }
    /// Enable contact-manifold recycling for contacts created after this body is created.
    pub fn enable_contact_recycling(mut self, flag: bool) -> Self {
        self.def.0.enableContactRecycling = flag;
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
        struct Repr<'a> {
            name: Option<&'a str>,
            body_type: BodyType,
            position: crate::types::Position,
            angle: f32,
            linear_velocity: crate::types::Vec2,
            angular_velocity: f32,
            linear_damping: f32,
            angular_damping: f32,
            sleep_threshold: f32,
            gravity_scale: f32,
            enable_sleep: bool,
            awake: bool,
            bullet: bool,
            allow_fast_rotation: bool,
            motion_locks: MotionLocks,
            enable_contact_recycling: bool,
            enabled: bool,
        }
        let name = self
            .name()
            .map(CStr::to_str)
            .transpose()
            .map_err(serde::ser::Error::custom)?;
        let angle = self.0.rotation.s.atan2(self.0.rotation.c);
        let r = Repr {
            name,
            body_type: BodyType::from_raw(self.0.type_).ok_or_else(|| {
                serde::ser::Error::custom(format!(
                    "unknown Box2D body type discriminant {}",
                    self.0.type_
                ))
            })?,
            position: crate::types::Position::from_raw(self.0.position),
            angle,
            linear_velocity: crate::types::Vec2::from_raw(self.0.linearVelocity),
            angular_velocity: self.0.angularVelocity,
            linear_damping: self.0.linearDamping,
            angular_damping: self.0.angularDamping,
            sleep_threshold: self.0.sleepThreshold,
            gravity_scale: self.0.gravityScale,
            enable_sleep: self.0.enableSleep,
            awake: self.0.isAwake,
            bullet: self.0.isBullet,
            allow_fast_rotation: self.0.allowFastRotation,
            motion_locks: MotionLocks::from_raw(self.0.motionLocks),
            enable_contact_recycling: self.0.enableContactRecycling,
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
            #[serde(default)]
            name: Option<String>,
            body_type: BodyType,
            position: crate::types::Position,
            angle: f32,
            linear_velocity: crate::types::Vec2,
            angular_velocity: f32,
            linear_damping: f32,
            angular_damping: f32,
            #[serde(default = "default_sleep_threshold")]
            sleep_threshold: f32,
            gravity_scale: f32,
            enable_sleep: bool,
            awake: bool,
            bullet: bool,
            allow_fast_rotation: bool,
            #[serde(default)]
            motion_locks: MotionLocks,
            #[serde(default = "default_contact_recycling")]
            enable_contact_recycling: bool,
            enabled: bool,
        }
        fn default_contact_recycling() -> bool {
            true
        }
        fn default_sleep_threshold() -> f32 {
            BodyDef::default().sleep_threshold()
        }
        let r = Repr::deserialize(deserializer)?;
        let mut b = BodyBuilder::new()
            .body_type(r.body_type)
            .position(r.position)
            .angle(r.angle)
            .linear_velocity(r.linear_velocity)
            .angular_velocity(r.angular_velocity)
            .linear_damping(r.linear_damping)
            .angular_damping(r.angular_damping)
            .sleep_threshold(r.sleep_threshold)
            .gravity_scale(r.gravity_scale)
            .enable_sleep(r.enable_sleep)
            .awake(r.awake)
            .bullet(r.bullet)
            .allow_fast_rotation(r.allow_fast_rotation)
            .motion_locks(r.motion_locks)
            .enable_contact_recycling(r.enable_contact_recycling)
            .enabled(r.enabled);
        if let Some(name) = r.name {
            b = b.try_name(&name).map_err(serde::de::Error::custom)?;
        }
        Ok(b.build())
    }
}

impl Default for BodyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{BodyBuilder, BodyDef, BodyType};
    use crate::{ApiError, MotionLocks};
    use boxdd_sys::ffi;

    #[test]
    fn body_type_rejects_unknown_ffi_discriminants() {
        let unknown = ffi::b2BodyType_b2_bodyTypeCount;

        assert_eq!(BodyType::from_raw(unknown), None);
        assert_eq!(BodyType::try_from(unknown), Err(unknown));

        let mut raw = unsafe { ffi::b2DefaultBodyDef() };
        raw.type_ = unknown;
        // SAFETY: the default raw definition has no name pointer, and this test only changes the
        // body-type discriminant to exercise the checked conversion.
        let definition = unsafe { BodyDef::from_raw(raw) };
        assert_eq!(definition.body_type(), None);
    }

    #[test]
    fn body_type_native_decoder_preserves_known_values_and_reports_the_raw_unknown() {
        for expected in [BodyType::Static, BodyType::Kinematic, BodyType::Dynamic] {
            assert_eq!(BodyType::decode_native(expected.into_raw()), Ok(expected));
        }

        let raw = ffi::b2BodyType_b2_bodyTypeCount;
        assert_eq!(
            BodyType::decode_native(raw),
            Err(ApiError::InvalidNativeBodyType { raw })
        );
    }

    #[test]
    fn body_builder_allow_fast_rotation_sets_raw_field() {
        assert!(!BodyBuilder::new().build().0.allowFastRotation);
        assert!(
            BodyBuilder::new()
                .allow_fast_rotation(true)
                .build()
                .0
                .allowFastRotation
        );
    }

    #[test]
    fn body_builder_contact_recycling_defaults_on_and_can_disable() {
        let default = BodyBuilder::new().build();
        assert!(default.is_contact_recycling_enabled());

        let disabled = BodyBuilder::new().enable_contact_recycling(false).build();
        assert!(!disabled.is_contact_recycling_enabled());
        assert!(!disabled.0.enableContactRecycling);
    }

    #[test]
    fn body_builder_motion_locks_round_trip() {
        let locks = MotionLocks::new(true, false, true);
        let definition = BodyBuilder::new().motion_locks(locks).build();

        assert_eq!(definition.motion_locks(), locks);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn body_definition_serde_preserves_motion_locks() {
        let locks = MotionLocks::new(true, false, true);
        let definition = BodyBuilder::new().motion_locks(locks).build();
        let encoded = serde_json::to_string(&definition).unwrap();
        let decoded: super::BodyDef = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.motion_locks(), locks);
    }
}
