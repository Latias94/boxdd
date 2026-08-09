use super::*;

#[inline]
pub(crate) fn check_world_gravity_valid(
    operation: &'static str,
    argument: &'static str,
    gravity: Vec2,
) -> crate::error::Result<()> {
    if gravity.is_valid() {
        Ok(())
    } else {
        Err(crate::error::Error::invalid_argument(
            operation,
            argument,
            "a finite vector",
        ))
    }
}

#[inline]
pub(crate) fn check_non_negative_finite_world_scalar(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> crate::error::Result<()> {
    if crate::is_valid_float(value) && value >= 0.0 {
        Ok(())
    } else {
        Err(crate::error::Error::invalid_argument(
            operation,
            argument,
            "a finite value greater than or equal to zero",
        ))
    }
}

#[inline]
pub(crate) fn check_positive_finite_world_linear_speed(
    operation: &'static str,
    argument: &'static str,
    value: f32,
) -> crate::error::Result<()> {
    if crate::is_valid_float(value) && value > 0.0 && (value * value).is_finite() {
        Ok(())
    } else {
        Err(crate::error::Error::invalid_argument(
            operation,
            argument,
            "a positive finite value whose square is finite",
        ))
    }
}

#[inline]
pub(crate) fn check_world_def_valid(def: &WorldDef) -> crate::error::Result<()> {
    const OPERATION: &str = "WorldDef::validate";
    check_world_gravity_valid(OPERATION, "gravity", def.gravity())?;
    check_non_negative_finite_world_scalar(
        OPERATION,
        "restitution_threshold",
        def.restitution_threshold(),
    )?;
    check_non_negative_finite_world_scalar(
        OPERATION,
        "hit_event_threshold",
        def.hit_event_threshold(),
    )?;
    check_non_negative_finite_world_scalar(OPERATION, "contact_hertz", def.contact_hertz())?;
    check_non_negative_finite_world_scalar(
        OPERATION,
        "contact_damping_ratio",
        def.contact_damping_ratio(),
    )?;
    check_non_negative_finite_world_scalar(OPERATION, "contact_speed", def.contact_speed())?;
    check_positive_finite_world_linear_speed(
        OPERATION,
        "maximum_linear_speed",
        def.maximum_linear_speed(),
    )?;
    Ok(())
}

/// A scale-provenanced definition for constructing a simulation world.
///
/// Obtain one from [`crate::Foundation::world_def`] or configure one with
/// [`crate::Foundation::world_builder`]. A world definition has no context-free default because
/// its native defaults depend on the process-global length scale.
#[doc(alias = "world_def")]
#[doc(alias = "worlddef")]
#[derive(Clone, Debug)]
pub struct WorldDef {
    gravity: Vec2,
    restitution_threshold: f32,
    hit_event_threshold: f32,
    contact_hertz: f32,
    contact_damping_ratio: f32,
    contact_speed: f32,
    maximum_linear_speed: f32,
    enable_sleep: bool,
    enable_continuous: bool,
    enable_contact_softening: bool,
    worker_count: WorkerCount,
    capacity: WorldCapacity,
    length_scale: crate::core::length_scale::LengthScale,
}

impl WorldDef {
    pub(crate) fn with_length_scale(length_scale: crate::core::length_scale::LengthScale) -> Self {
        let raw: ffi::b2WorldDef = crate::core::native_defaults::world_def(
            length_scale.units_per_meter(),
            WorkerCount::default().as_i32(),
        );
        Self {
            gravity: Vec2::from_raw(raw.gravity),
            restitution_threshold: raw.restitutionThreshold,
            hit_event_threshold: raw.hitEventThreshold,
            contact_hertz: raw.contactHertz,
            contact_damping_ratio: raw.contactDampingRatio,
            contact_speed: raw.contactSpeed,
            maximum_linear_speed: raw.maximumLinearSpeed,
            enable_sleep: raw.enableSleep,
            enable_continuous: raw.enableContinuous,
            enable_contact_softening: raw.enableContactSoftening,
            worker_count: WorkerCount::default(),
            capacity: WorldCapacity::default(),
            length_scale,
        }
    }

    pub fn gravity(&self) -> crate::types::Vec2 {
        self.gravity
    }

    pub fn restitution_threshold(&self) -> f32 {
        self.restitution_threshold
    }

    pub fn hit_event_threshold(&self) -> f32 {
        self.hit_event_threshold
    }

    pub fn contact_hertz(&self) -> f32 {
        self.contact_hertz
    }

    pub fn contact_damping_ratio(&self) -> f32 {
        self.contact_damping_ratio
    }

    pub fn contact_speed(&self) -> f32 {
        self.contact_speed
    }

    pub fn maximum_linear_speed(&self) -> f32 {
        self.maximum_linear_speed
    }

    pub fn is_sleep_enabled(&self) -> bool {
        self.enable_sleep
    }

    pub fn is_continuous_enabled(&self) -> bool {
        self.enable_continuous
    }

    pub fn is_contact_softening_enabled(&self) -> bool {
        self.enable_contact_softening
    }

    pub fn worker_count(&self) -> WorkerCount {
        self.worker_count
    }

    pub fn capacity(&self) -> WorldCapacity {
        self.capacity
    }

    pub(crate) fn length_scale(&self) -> crate::core::length_scale::LengthScale {
        self.length_scale
    }

    pub(crate) fn into_raw(self) -> ffi::b2WorldDef {
        let mut raw: ffi::b2WorldDef = crate::core::native_defaults::world_def(
            self.length_scale.units_per_meter(),
            self.worker_count.as_i32(),
        );
        raw.gravity = self.gravity.into_raw();
        raw.restitutionThreshold = self.restitution_threshold;
        raw.hitEventThreshold = self.hit_event_threshold;
        raw.contactHertz = self.contact_hertz;
        raw.contactDampingRatio = self.contact_damping_ratio;
        raw.contactSpeed = self.contact_speed;
        raw.maximumLinearSpeed = self.maximum_linear_speed;
        raw.enableSleep = self.enable_sleep;
        raw.enableContinuous = self.enable_continuous;
        raw.enableContactSoftening = self.enable_contact_softening;
        raw.capacity = self.capacity.into_raw();
        raw
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        check_world_def_valid(self)
    }
}

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
            worker_count: WorkerCount,
            capacity: WorldCapacity,
            length_units_per_meter: f32,
        }
        let r = Repr {
            gravity: self.gravity,
            restitution_threshold: self.restitution_threshold,
            hit_event_threshold: self.hit_event_threshold,
            contact_hertz: self.contact_hertz,
            contact_damping_ratio: self.contact_damping_ratio,
            contact_speed: self.contact_speed,
            maximum_linear_speed: self.maximum_linear_speed,
            enable_sleep: self.enable_sleep,
            enable_continuous: self.enable_continuous,
            enable_contact_softening: self.enable_contact_softening,
            worker_count: self.worker_count,
            capacity: self.capacity,
            length_units_per_meter: self.length_scale.units_per_meter(),
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
        #[serde(deny_unknown_fields)]
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
            worker_count: Option<WorkerCount>,
            #[serde(default)]
            capacity: Option<WorldCapacity>,
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
        let mut b = WorldDef::with_length_scale(length_scale);
        if let Some(g) = r.gravity {
            b.gravity = g;
        }
        if let Some(v) = r.restitution_threshold {
            b.restitution_threshold = v;
        }
        if let Some(v) = r.hit_event_threshold {
            b.hit_event_threshold = v;
        }
        if let Some(v) = r.contact_hertz {
            b.contact_hertz = v;
        }
        if let Some(v) = r.contact_damping_ratio {
            b.contact_damping_ratio = v;
        }
        if let Some(v) = r.contact_speed {
            b.contact_speed = v;
        }
        if let Some(v) = r.maximum_linear_speed {
            b.maximum_linear_speed = v;
        }
        if let Some(v) = r.enable_sleep {
            b.enable_sleep = v;
        }
        if let Some(v) = r.enable_continuous {
            b.enable_continuous = v;
        }
        if let Some(v) = r.enable_contact_softening {
            b.enable_contact_softening = v;
        }
        if let Some(v) = r.worker_count {
            b.worker_count = v;
        }
        if let Some(v) = r.capacity {
            b.capacity = v;
        }
        b.validate().map_err(serde::de::Error::custom)?;
        Ok(b)
    }
}

/// Fluent builder for `WorldDef`.
///
/// Obtain this builder from [`crate::Foundation::world_builder`], chain configuration calls, and
/// finish with `build()`. All fields map 1:1 to the upstream `b2WorldDef`.
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
        self.def.gravity = g.into();
        self
    }

    /// Restitution threshold (m/s) under which collisions don't bounce.
    pub fn restitution_threshold(mut self, v: f32) -> Self {
        self.def.restitution_threshold = v;
        self
    }

    /// Impulse magnitude that generates hit events.
    pub fn hit_event_threshold(mut self, v: f32) -> Self {
        self.def.hit_event_threshold = v;
        self
    }

    /// Contact solver target stiffness in Hertz.
    pub fn contact_hertz(mut self, v: f32) -> Self {
        self.def.contact_hertz = v;
        self
    }

    /// Contact damping ratio (non-dimensional).
    pub fn contact_damping_ratio(mut self, v: f32) -> Self {
        self.def.contact_damping_ratio = v;
        self
    }

    /// Velocity used by continuous collision detection.
    pub fn contact_speed(mut self, v: f32) -> Self {
        self.def.contact_speed = v;
        self
    }

    /// Maximum linear speed clamp for bodies.
    pub fn maximum_linear_speed(mut self, v: f32) -> Self {
        self.def.maximum_linear_speed = v;
        self
    }

    /// Enable/disable sleeping globally.
    pub fn enable_sleep(mut self, flag: bool) -> Self {
        self.def.enable_sleep = flag;
        self
    }

    /// Enable/disable continuous collision detection globally.
    pub fn enable_continuous(mut self, flag: bool) -> Self {
        self.def.enable_continuous = flag;
        self
    }

    /// Enable/disable contact softening.
    pub fn enable_contact_softening(mut self, flag: bool) -> Self {
        self.def.enable_contact_softening = flag;
        self
    }

    /// Number of worker threads Box2D may use during stepping.
    ///
    /// Values above one select Box2D's built-in scheduler. The validated value rejects unsupported
    /// targets and counts outside Box2D's native range. This does not make `World` `Send` or `Sync`.
    pub fn worker_count(mut self, count: WorkerCount) -> Self {
        self.def.worker_count = count;
        self
    }

    /// Reserve initial world storage to avoid predictable run-time allocations.
    pub fn capacity(mut self, capacity: WorldCapacity) -> Self {
        self.def.capacity = capacity;
        self
    }

    pub fn build(self) -> crate::error::Result<WorldDef> {
        self.def.validate()?;
        Ok(self.def)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, Foundation};

    #[test]
    fn world_builder_rejects_invalid_draft_at_build() {
        assert_eq!(
            Foundation::initialize_default()
                .unwrap()
                .world_builder()
                .maximum_linear_speed(0.0)
                .build()
                .unwrap_err(),
            Error::invalid_argument(
                "WorldDef::validate",
                "maximum_linear_speed",
                "a positive finite value whose square is finite",
            )
        );
    }

    #[test]
    fn world_builder_rejects_speed_whose_square_overflows() {
        assert_eq!(
            Foundation::initialize_default()
                .unwrap()
                .world_builder()
                .maximum_linear_speed(f32::MAX)
                .build()
                .unwrap_err(),
            Error::invalid_argument(
                "WorldDef::validate",
                "maximum_linear_speed",
                "a positive finite value whose square is finite",
            )
        );
    }
}
