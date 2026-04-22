use crate::error::{ApiError, ApiResult};
use crate::types::{MassData, Vec2};
use boxdd_sys::ffi;

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

    #[inline]
    pub const fn from_raw(raw: ffi::b2BodyType) -> Self {
        match raw {
            x if x == ffi::b2BodyType_b2_staticBody => BodyType::Static,
            x if x == ffi::b2BodyType_b2_kinematicBody => BodyType::Kinematic,
            _ => BodyType::Dynamic,
        }
    }
}

#[inline]
fn body_type_is_known(raw: ffi::b2BodyType) -> bool {
    raw == ffi::b2BodyType_b2_staticBody
        || raw == ffi::b2BodyType_b2_kinematicBody
        || raw == ffi::b2BodyType_b2_dynamicBody
}

#[inline]
fn body_def_cookie_is_valid(def: &BodyDef) -> bool {
    def.0.internalValue == unsafe { ffi::b2DefaultBodyDef() }.internalValue
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
        body_def_cookie_is_valid(def),
        "invalid BodyDef: not initialized from b2DefaultBodyDef"
    );
    assert!(
        body_type_is_known(def.0.type_),
        "invalid BodyDef: unknown body type value {}",
        def.0.type_
    );
    assert!(
        Vec2::from_raw(def.0.position).is_valid(),
        "invalid BodyDef: position must be a valid Box2D vector"
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
}

pub(crate) fn check_body_def_valid(def: &BodyDef) -> ApiResult<()> {
    if !body_def_cookie_is_valid(def)
        || !body_type_is_known(def.0.type_)
        || !Vec2::from_raw(def.0.position).is_valid()
        || !crate::Rot::from_raw(def.0.rotation).is_valid()
        || !Vec2::from_raw(def.0.linearVelocity).is_valid()
        || !crate::is_valid_float(def.0.angularVelocity)
        || check_non_negative_finite_body_scalar(def.0.linearDamping).is_err()
        || check_non_negative_finite_body_scalar(def.0.angularDamping).is_err()
        || check_non_negative_finite_body_scalar(def.0.sleepThreshold).is_err()
        || !crate::is_valid_float(def.0.gravityScale)
    {
        Err(ApiError::InvalidArgument)
    } else {
        Ok(())
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

impl BodyDef {
    /// Start building a new `BodyDef` from defaults.
    pub fn builder() -> BodyBuilder {
        BodyBuilder::new()
    }

    /// Construct from the raw Box2D body definition value.
    ///
    /// # Safety
    /// If `raw.name` is non-null, it must point to a readable NUL-terminated string for any
    /// later safe body-creation call. This constructor does not copy or validate the raw name
    /// pointer.
    #[inline]
    pub unsafe fn from_raw(raw: ffi::b2BodyDef) -> Self {
        Self(raw)
    }

    /// Body type used when the body is created.
    #[inline]
    pub fn body_type(&self) -> BodyType {
        BodyType::from_raw(self.0.type_)
    }

    /// Initial world-space position.
    #[inline]
    pub fn position(&self) -> Vec2 {
        Vec2::from_raw(self.0.position)
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

    /// Whether the body starts enabled for simulation.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.0.isEnabled
    }

    /// Convert into the raw Box2D body definition value.
    #[inline]
    pub fn into_raw(self) -> ffi::b2BodyDef {
        self.0
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
    /// Initial world-space position.
    pub fn position<V: Into<Vec2>>(mut self, p: V) -> Self {
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
            body_type: BodyType,
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
            allow_fast_rotation: bool,
            enabled: bool,
        }
        let angle = self.0.rotation.s.atan2(self.0.rotation.c);
        let r = Repr {
            body_type: match self.0.type_ {
                x if x == ffi::b2BodyType_b2_staticBody => BodyType::Static,
                x if x == ffi::b2BodyType_b2_kinematicBody => BodyType::Kinematic,
                _ => BodyType::Dynamic,
            },
            position: crate::types::Vec2::from_raw(self.0.position),
            angle,
            linear_velocity: crate::types::Vec2::from_raw(self.0.linearVelocity),
            angular_velocity: self.0.angularVelocity,
            linear_damping: self.0.linearDamping,
            angular_damping: self.0.angularDamping,
            gravity_scale: self.0.gravityScale,
            enable_sleep: self.0.enableSleep,
            awake: self.0.isAwake,
            bullet: self.0.isBullet,
            allow_fast_rotation: self.0.allowFastRotation,
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
            body_type: BodyType,
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
            allow_fast_rotation: bool,
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
            .allow_fast_rotation(r.allow_fast_rotation)
            .enabled(r.enabled);
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
    use super::BodyBuilder;

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
}
