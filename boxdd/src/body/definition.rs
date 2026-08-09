use crate::error::{Error, Result};
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
    pub(crate) fn decode_native(raw: ffi::b2BodyType) -> Result<Self> {
        Self::from_raw(raw).ok_or(Error::InvalidNativeBodyType { raw })
    }
}

impl TryFrom<ffi::b2BodyType> for BodyType {
    type Error = ffi::b2BodyType;

    #[inline]
    fn try_from(value: ffi::b2BodyType) -> std::result::Result<Self, Self::Error> {
        Self::from_raw(value).ok_or(value)
    }
}

#[inline]
pub(crate) fn check_non_negative_finite_body_scalar(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> Result<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite value greater than or equal to zero",
        ))
    }
}

#[inline]
pub(crate) fn check_mass_data_valid(mass_data: MassData) -> Result<()> {
    check_non_negative_finite_body_scalar("Body::set_mass_data", "mass", mass_data.mass)?;
    check_non_negative_finite_body_scalar(
        "Body::set_mass_data",
        "rotational_inertia",
        mass_data.rotational_inertia,
    )?;
    if mass_data.center.is_valid() {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            "Body::set_mass_data",
            "center",
            "a finite vector",
        ))
    }
}

pub(crate) fn check_body_def_valid(def: &BodyDef) -> Result<()> {
    const OPERATION: &str = "BodyDef::validate";
    if def
        .name()
        .is_some_and(|name| name.to_bytes().len() > super::MAX_BODY_NAME_BYTES)
    {
        return Err(Error::invalid_argument(
            OPERATION,
            "name",
            "at most 10 UTF-8 bytes",
        ));
    }
    if !def.position.is_valid() {
        return Err(Error::invalid_argument(
            OPERATION,
            "position",
            "a finite world position",
        ));
    }
    if !def.rotation.is_valid() {
        return Err(Error::invalid_argument(
            OPERATION,
            "rotation",
            "a normalized finite rotation",
        ));
    }
    if !def.linear_velocity.is_valid() {
        return Err(Error::invalid_argument(
            OPERATION,
            "linear_velocity",
            "a finite vector",
        ));
    }
    if !crate::is_valid_float(def.angular_velocity) {
        return Err(Error::invalid_argument(
            OPERATION,
            "angular_velocity",
            "a finite value",
        ));
    }
    check_non_negative_finite_body_scalar(OPERATION, "linear_damping", def.linear_damping)?;
    check_non_negative_finite_body_scalar(OPERATION, "angular_damping", def.angular_damping)?;
    check_non_negative_finite_body_scalar(OPERATION, "sleep_threshold", def.sleep_threshold)?;
    if !crate::is_valid_float(def.gravity_scale) {
        return Err(Error::invalid_argument(
            OPERATION,
            "gravity_scale",
            "a finite value",
        ));
    }
    Ok(())
}

/// A scale-provenanced Rust body definition.
///
/// Obtain one from [`crate::Foundation::body_def`], [`crate::World::body_def`], or
/// [`crate::RecordingSession::body_def`]. A body definition has no context-free default because
/// native body defaults depend on the selected world length scale.
#[derive(Debug)]
pub struct BodyDef {
    name: Option<CString>,
    body_type: BodyType,
    position: Position,
    rotation: crate::Rot,
    linear_velocity: Vec2,
    angular_velocity: f32,
    linear_damping: f32,
    angular_damping: f32,
    gravity_scale: f32,
    sleep_threshold: f32,
    motion_locks: MotionLocks,
    enable_sleep: bool,
    awake: bool,
    bullet: bool,
    enabled: bool,
    allow_fast_rotation: bool,
    enable_contact_recycling: bool,
    length_scale: crate::core::length_scale::LengthScale,
}

impl Clone for BodyDef {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            body_type: self.body_type,
            position: self.position,
            rotation: self.rotation,
            linear_velocity: self.linear_velocity,
            angular_velocity: self.angular_velocity,
            linear_damping: self.linear_damping,
            angular_damping: self.angular_damping,
            gravity_scale: self.gravity_scale,
            sleep_threshold: self.sleep_threshold,
            motion_locks: self.motion_locks,
            enable_sleep: self.enable_sleep,
            awake: self.awake,
            bullet: self.bullet,
            enabled: self.enabled,
            allow_fast_rotation: self.allow_fast_rotation,
            enable_contact_recycling: self.enable_contact_recycling,
            length_scale: self.length_scale,
        }
    }
}

pub(crate) struct PreparedBodyDef {
    raw: ffi::b2BodyDef,
    _name: Option<CString>,
}

impl PreparedBodyDef {
    #[inline]
    pub(crate) fn as_raw(&self) -> &ffi::b2BodyDef {
        &self.raw
    }
}

impl BodyDef {
    #[inline]
    pub(crate) fn with_length_scale(length_scale: crate::core::length_scale::LengthScale) -> Self {
        let raw: ffi::b2BodyDef =
            crate::core::native_defaults::body_def(length_scale.units_per_meter());
        Self {
            name: None,
            body_type: BodyType::Static,
            position: Position::from_raw(raw.position),
            rotation: crate::Rot::from_raw_unvalidated(raw.rotation),
            linear_velocity: Vec2::from_raw(raw.linearVelocity),
            angular_velocity: raw.angularVelocity,
            linear_damping: raw.linearDamping,
            angular_damping: raw.angularDamping,
            gravity_scale: raw.gravityScale,
            sleep_threshold: raw.sleepThreshold,
            motion_locks: MotionLocks::from_raw(raw.motionLocks),
            enable_sleep: raw.enableSleep,
            awake: raw.isAwake,
            bullet: raw.isBullet,
            enabled: raw.isEnabled,
            allow_fast_rotation: raw.allowFastRotation,
            enable_contact_recycling: raw.enableContactRecycling,
            length_scale,
        }
    }

    #[inline]
    fn set_name(&mut self, name: Option<CString>) {
        self.name = name;
    }

    /// Optional name assigned when the body is created.
    #[inline]
    pub fn name(&self) -> Option<&CStr> {
        self.name.as_deref()
    }

    /// Body type used when the body is created.
    #[inline]
    pub fn body_type(&self) -> BodyType {
        self.body_type
    }

    /// Initial world-space position.
    #[inline]
    pub fn position(&self) -> Position {
        self.position
    }

    /// Initial rotation value.
    #[inline]
    pub fn rotation(&self) -> crate::Rot {
        self.rotation
    }

    /// Initial angle in radians.
    #[inline]
    pub fn angle(&self) -> f32 {
        self.rotation().angle()
    }

    /// Initial linear velocity in m/s.
    #[inline]
    pub fn linear_velocity(&self) -> Vec2 {
        self.linear_velocity
    }

    /// Initial angular velocity in rad/s.
    #[inline]
    pub fn angular_velocity(&self) -> f32 {
        self.angular_velocity
    }

    /// Linear damping.
    #[inline]
    pub fn linear_damping(&self) -> f32 {
        self.linear_damping
    }

    /// Angular damping.
    #[inline]
    pub fn angular_damping(&self) -> f32 {
        self.angular_damping
    }

    /// Linear speed below which the body may transition to sleep.
    #[inline]
    pub fn sleep_threshold(&self) -> f32 {
        self.sleep_threshold
    }

    /// Per-body gravity scale.
    #[inline]
    pub fn gravity_scale(&self) -> f32 {
        self.gravity_scale
    }

    /// Whether sleeping is enabled at creation.
    #[inline]
    pub fn is_sleep_enabled(&self) -> bool {
        self.enable_sleep
    }

    /// Whether the body starts awake.
    #[inline]
    pub fn is_awake(&self) -> bool {
        self.awake
    }

    /// Whether the body starts as a bullet.
    #[inline]
    pub fn is_bullet(&self) -> bool {
        self.bullet
    }

    /// Whether the body allows fast rotation without Box2D's default clamp.
    #[inline]
    pub fn is_fast_rotation_allowed(&self) -> bool {
        self.allow_fast_rotation
    }

    /// Per-axis motion locks applied when the body is created.
    #[inline]
    pub fn motion_locks(&self) -> MotionLocks {
        self.motion_locks
    }

    /// Whether newly created contacts may recycle this body's previous manifolds.
    #[inline]
    pub fn is_contact_recycling_enabled(&self) -> bool {
        self.enable_contact_recycling
    }

    /// Whether the body starts enabled for simulation.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn length_scale(&self) -> crate::core::length_scale::LengthScale {
        self.length_scale
    }

    pub(crate) fn prepare(self) -> PreparedBodyDef {
        let mut raw: ffi::b2BodyDef =
            crate::core::native_defaults::body_def(self.length_scale.units_per_meter());
        raw.type_ = self.body_type.into_raw();
        raw.position = self.position.into_raw();
        raw.rotation = self.rotation.into_raw();
        raw.linearVelocity = self.linear_velocity.into_raw();
        raw.angularVelocity = self.angular_velocity;
        raw.linearDamping = self.linear_damping;
        raw.angularDamping = self.angular_damping;
        raw.gravityScale = self.gravity_scale;
        raw.sleepThreshold = self.sleep_threshold;
        raw.motionLocks = self.motion_locks.into_raw();
        raw.enableSleep = self.enable_sleep;
        raw.isAwake = self.awake;
        raw.isBullet = self.bullet;
        raw.isEnabled = self.enabled;
        raw.allowFastRotation = self.allow_fast_rotation;
        raw.enableContactRecycling = self.enable_contact_recycling;
        let mut prepared = PreparedBodyDef {
            raw,
            _name: self.name,
        };
        prepared.raw.name = prepared
            ._name
            .as_ref()
            .map_or(std::ptr::null(), |name| name.as_ptr());
        prepared
    }

    #[inline]
    pub fn validate(&self) -> Result<()> {
        check_body_def_valid(self)
    }
}

/// Fluent builder for `BodyDef`.
#[doc(alias = "body_builder")]
#[doc(alias = "bodybuilder")]
///
/// Obtain this builder from `Foundation`, `World`, or `RecordingSession`, chain methods to
/// configure a body, and finish with `build()`. This maps to the upstream `b2BodyDef` fields.
#[derive(Clone, Debug)]
pub struct BodyBuilder {
    def: BodyDef,
}

impl BodyBuilder {
    /// Set the body type (static, kinematic, dynamic).
    pub fn body_type(mut self, t: BodyType) -> Self {
        self.def.body_type = t;
        self
    }
    /// Set the optional body name, returning an error for an interior NUL or an oversized name.
    pub fn name(mut self, name: &str) -> Result<Self> {
        self.def.set_name(Some(super::check_valid_body_name(
            "BodyBuilder::name",
            name,
        )?));
        Ok(self)
    }
    /// Remove a previously configured body name.
    pub fn clear_name(mut self) -> Self {
        self.def.set_name(None);
        self
    }
    /// Initial world-space position.
    pub fn position<P: Into<Position>>(mut self, p: P) -> Self {
        self.def.position = p.into();
        self
    }
    /// Initial rotation in radians.
    pub fn angle(mut self, radians: f32) -> Self {
        // Build a rotation from angle
        self.def.rotation = crate::Rot::from_radians_unvalidated(radians);
        self
    }
    /// Initial linear velocity (m/s).
    pub fn linear_velocity<V: Into<Vec2>>(mut self, v: V) -> Self {
        self.def.linear_velocity = v.into();
        self
    }
    /// Initial angular velocity (rad/s).
    pub fn angular_velocity(mut self, v: f32) -> Self {
        self.def.angular_velocity = v;
        self
    }
    /// Linear damping (drag-like term).
    pub fn linear_damping(mut self, v: f32) -> Self {
        self.def.linear_damping = v;
        self
    }
    /// Angular damping.
    pub fn angular_damping(mut self, v: f32) -> Self {
        self.def.angular_damping = v;
        self
    }
    /// Linear speed below which the body may transition to sleep.
    pub fn sleep_threshold(mut self, v: f32) -> Self {
        self.def.sleep_threshold = v;
        self
    }
    /// Per-body gravity scale (1 = normal gravity).
    pub fn gravity_scale(mut self, v: f32) -> Self {
        self.def.gravity_scale = v;
        self
    }
    /// Allow body to go to sleep.
    pub fn enable_sleep(mut self, flag: bool) -> Self {
        self.def.enable_sleep = flag;
        self
    }
    /// Awake/asleep flag at creation.
    pub fn awake(mut self, flag: bool) -> Self {
        self.def.awake = flag;
        self
    }
    /// Treat as bullet (CCD).
    pub fn bullet(mut self, flag: bool) -> Self {
        self.def.bullet = flag;
        self
    }
    /// Allow high angular speed without Box2D's default clamp.
    pub fn allow_fast_rotation(mut self, flag: bool) -> Self {
        self.def.allow_fast_rotation = flag;
        self
    }
    /// Lock selected translation and rotation axes at creation.
    pub fn motion_locks(mut self, locks: MotionLocks) -> Self {
        self.def.motion_locks = locks;
        self
    }
    /// Enable contact-manifold recycling for contacts created after this body is created.
    pub fn enable_contact_recycling(mut self, flag: bool) -> Self {
        self.def.enable_contact_recycling = flag;
        self
    }
    /// Enable/disable simulation for this body.
    pub fn enabled(mut self, flag: bool) -> Self {
        self.def.enabled = flag;
        self
    }

    pub fn build(self) -> Result<BodyDef> {
        self.def.validate()?;
        Ok(self.def)
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
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
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
            length_units_per_meter: f32,
        }
        let name = self
            .name()
            .map(CStr::to_str)
            .transpose()
            .map_err(serde::ser::Error::custom)?;
        let r = Repr {
            name,
            body_type: self.body_type,
            position: self.position,
            angle: self.rotation.angle(),
            linear_velocity: self.linear_velocity,
            angular_velocity: self.angular_velocity,
            linear_damping: self.linear_damping,
            angular_damping: self.angular_damping,
            sleep_threshold: self.sleep_threshold,
            gravity_scale: self.gravity_scale,
            enable_sleep: self.enable_sleep,
            awake: self.awake,
            bullet: self.bullet,
            allow_fast_rotation: self.allow_fast_rotation,
            motion_locks: self.motion_locks,
            enable_contact_recycling: self.enable_contact_recycling,
            enabled: self.enabled,
            length_units_per_meter: self.length_scale.units_per_meter(),
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for BodyDef {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Repr {
            name: Option<String>,
            body_type: Option<BodyType>,
            position: Option<crate::types::Position>,
            angle: Option<f32>,
            linear_velocity: Option<crate::types::Vec2>,
            angular_velocity: Option<f32>,
            linear_damping: Option<f32>,
            angular_damping: Option<f32>,
            sleep_threshold: Option<f32>,
            gravity_scale: Option<f32>,
            enable_sleep: Option<bool>,
            awake: Option<bool>,
            bullet: Option<bool>,
            allow_fast_rotation: Option<bool>,
            motion_locks: Option<MotionLocks>,
            enable_contact_recycling: Option<bool>,
            enabled: Option<bool>,
            length_units_per_meter: f32,
        }
        let r = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        let length_scale = crate::core::length_scale::LengthScale::try_new(
            r.length_units_per_meter,
        )
        .ok_or_else(|| {
            serde::de::Error::custom(
                "length_units_per_meter must preserve safe Box2D ray and shape tolerances",
            )
        })?;
        let mut b = BodyDef::with_length_scale(length_scale);
        if let Some(name) = r.name {
            b.set_name(Some(
                super::check_valid_body_name("BodyDef::deserialize", &name)
                    .map_err(serde::de::Error::custom)?,
            ));
        }
        if let Some(value) = r.body_type {
            b.body_type = value;
        }
        if let Some(value) = r.position {
            b.position = value;
        }
        if let Some(value) = r.angle {
            b.rotation = crate::Rot::from_radians(value).map_err(serde::de::Error::custom)?;
        }
        if let Some(value) = r.linear_velocity {
            b.linear_velocity = value;
        }
        if let Some(value) = r.angular_velocity {
            b.angular_velocity = value;
        }
        if let Some(value) = r.linear_damping {
            b.linear_damping = value;
        }
        if let Some(value) = r.angular_damping {
            b.angular_damping = value;
        }
        if let Some(value) = r.sleep_threshold {
            b.sleep_threshold = value;
        }
        if let Some(value) = r.gravity_scale {
            b.gravity_scale = value;
        }
        if let Some(value) = r.enable_sleep {
            b.enable_sleep = value;
        }
        if let Some(value) = r.awake {
            b.awake = value;
        }
        if let Some(value) = r.bullet {
            b.bullet = value;
        }
        if let Some(value) = r.allow_fast_rotation {
            b.allow_fast_rotation = value;
        }
        if let Some(value) = r.motion_locks {
            b.motion_locks = value;
        }
        if let Some(value) = r.enable_contact_recycling {
            b.enable_contact_recycling = value;
        }
        if let Some(value) = r.enabled {
            b.enabled = value;
        }
        b.validate().map_err(serde::de::Error::custom)?;
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::BodyType;
    use crate::{Error, Foundation, MotionLocks};
    use boxdd_sys::ffi;

    fn foundation() -> &'static Foundation {
        Foundation::get().unwrap_or_else(|| Foundation::initialize_default().unwrap())
    }

    #[test]
    fn body_type_native_decoder_preserves_known_values_and_reports_the_raw_unknown() {
        for expected in [BodyType::Static, BodyType::Kinematic, BodyType::Dynamic] {
            assert_eq!(BodyType::decode_native(expected.into_raw()), Ok(expected));
        }

        let raw = ffi::b2BodyType_b2_bodyTypeCount;
        assert_eq!(
            BodyType::decode_native(raw),
            Err(Error::InvalidNativeBodyType { raw })
        );
    }

    #[test]
    fn body_builder_configures_fast_rotation() {
        assert!(
            !foundation()
                .body_builder()
                .build()
                .unwrap()
                .is_fast_rotation_allowed()
        );
        assert!(
            foundation()
                .body_builder()
                .allow_fast_rotation(true)
                .build()
                .unwrap()
                .is_fast_rotation_allowed()
        );
    }

    #[test]
    fn body_builder_contact_recycling_defaults_on_and_can_disable() {
        let default = foundation().body_builder().build().unwrap();
        assert!(default.is_contact_recycling_enabled());

        let disabled = foundation()
            .body_builder()
            .enable_contact_recycling(false)
            .build()
            .unwrap();
        assert!(!disabled.is_contact_recycling_enabled());
    }

    #[test]
    fn body_builder_motion_locks_round_trip() {
        let locks = MotionLocks::new(true, false, true);
        let definition = foundation()
            .body_builder()
            .motion_locks(locks)
            .build()
            .unwrap();

        assert_eq!(definition.motion_locks(), locks);
    }

    #[test]
    fn body_builder_rejects_invalid_draft_at_build() {
        assert_eq!(
            foundation()
                .body_builder()
                .linear_damping(-1.0)
                .build()
                .unwrap_err(),
            Error::invalid_argument(
                "BodyDef::validate",
                "linear_damping",
                "a finite value greater than or equal to zero",
            )
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn body_definition_serde_preserves_motion_locks() {
        let locks = MotionLocks::new(true, false, true);
        let definition = foundation()
            .body_builder()
            .motion_locks(locks)
            .build()
            .unwrap();
        let encoded = serde_json::to_string(&definition).unwrap();
        let decoded: super::BodyDef = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.motion_locks(), locks);
    }
}
