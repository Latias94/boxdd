use crate::debug_draw::HexColor;
use crate::error::Result;
use crate::filter::Filter;
use boxdd_sys::ffi;

use super::{check_shape_def_valid, check_surface_material_valid};

/// Shape surface material parameters.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct SurfaceMaterial(pub(crate) ffi::b2SurfaceMaterial);

const _: () = {
    assert!(
        core::mem::size_of::<SurfaceMaterial>() == core::mem::size_of::<ffi::b2SurfaceMaterial>()
    );
    assert!(
        core::mem::align_of::<SurfaceMaterial>() == core::mem::align_of::<ffi::b2SurfaceMaterial>()
    );
};

impl Default for SurfaceMaterial {
    fn default() -> Self {
        Self(crate::core::native_defaults::surface_material())
    }
}

impl SurfaceMaterial {
    /// Create a surface material using Box2D's defaults.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn from_raw(raw: ffi::b2SurfaceMaterial) -> Result<Self> {
        let material = Self::from_raw_unvalidated(raw);
        check_surface_material_valid("SurfaceMaterial::from_raw", &material)?;
        Ok(material)
    }

    #[inline]
    pub(crate) const fn from_raw_unvalidated(raw: ffi::b2SurfaceMaterial) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2SurfaceMaterial {
        self.0
    }

    #[inline]
    pub const fn friction(&self) -> f32 {
        self.0.friction
    }

    #[inline]
    pub const fn restitution(&self) -> f32 {
        self.0.restitution
    }

    #[inline]
    pub const fn rolling_resistance(&self) -> f32 {
        self.0.rollingResistance
    }

    #[inline]
    pub const fn tangent_speed(&self) -> f32 {
        self.0.tangentSpeed
    }

    #[inline]
    pub const fn user_material_id(&self) -> u64 {
        self.0.userMaterialId
    }

    #[inline]
    pub const fn custom_color(&self) -> HexColor {
        HexColor::from_rgb_u32(self.0.customColor)
    }

    #[inline]
    pub(crate) const fn custom_color_is_valid(&self) -> bool {
        self.0.customColor <= HexColor::MAX_RGB_U32
    }

    pub fn with_friction(mut self, v: f32) -> Result<Self> {
        self.0.friction = v;
        check_surface_material_valid("SurfaceMaterial::with_friction", &self)?;
        Ok(self)
    }
    pub fn with_restitution(mut self, v: f32) -> Result<Self> {
        self.0.restitution = v;
        check_surface_material_valid("SurfaceMaterial::with_restitution", &self)?;
        Ok(self)
    }
    pub fn with_rolling_resistance(mut self, v: f32) -> Result<Self> {
        self.0.rollingResistance = v;
        check_surface_material_valid("SurfaceMaterial::with_rolling_resistance", &self)?;
        Ok(self)
    }
    pub fn with_tangent_speed(mut self, v: f32) -> Result<Self> {
        self.0.tangentSpeed = v;
        check_surface_material_valid("SurfaceMaterial::with_tangent_speed", &self)?;
        Ok(self)
    }
    pub fn with_user_material_id(mut self, v: u64) -> Self {
        self.0.userMaterialId = v;
        self
    }
    pub fn with_custom_color(mut self, color: HexColor) -> Self {
        self.0.customColor = color.rgb_u32();
        self
    }

    #[inline]
    pub fn validate(&self) -> Result<()> {
        check_surface_material_valid("SurfaceMaterial::validate", self)
    }
}

impl PartialEq for SurfaceMaterial {
    fn eq(&self, other: &Self) -> bool {
        self.friction() == other.friction()
            && self.restitution() == other.restitution()
            && self.rolling_resistance() == other.rolling_resistance()
            && self.tangent_speed() == other.tangent_speed()
            && self.user_material_id() == other.user_material_id()
            && self.custom_color() == other.custom_color()
    }
}

/// Pure Rust shape definition with builder API.
#[doc(alias = "shape_def")]
#[doc(alias = "shapedef")]
#[derive(Clone, Debug)]
pub struct ShapeDef {
    material: SurfaceMaterial,
    density: f32,
    filter: Filter,
    enable_custom_filtering: bool,
    sensor: bool,
    enable_sensor_events: bool,
    enable_contact_events: bool,
    enable_hit_events: bool,
    enable_pre_solve_events: bool,
    invoke_contact_creation: bool,
    update_body_mass: bool,
}

impl Default for ShapeDef {
    fn default() -> Self {
        let raw: ffi::b2ShapeDef = crate::core::native_defaults::shape_def();
        Self {
            material: SurfaceMaterial::from_raw_unvalidated(raw.material),
            density: raw.density,
            filter: Filter::from_raw(raw.filter),
            enable_custom_filtering: raw.enableCustomFiltering,
            sensor: raw.isSensor,
            enable_sensor_events: raw.enableSensorEvents,
            enable_contact_events: raw.enableContactEvents,
            enable_hit_events: raw.enableHitEvents,
            enable_pre_solve_events: raw.enablePreSolveEvents,
            invoke_contact_creation: raw.invokeContactCreation,
            update_body_mass: raw.updateBodyMass,
        }
    }
}

impl ShapeDef {
    /// Start building a new `ShapeDef` from defaults.
    pub fn builder() -> ShapeDefBuilder {
        ShapeDefBuilder {
            def: Self::default(),
        }
    }

    /// Surface material parameters used by the shape.
    #[inline]
    pub const fn material(&self) -> SurfaceMaterial {
        self.material
    }

    /// Density in kg/m².
    #[inline]
    pub const fn density(&self) -> f32 {
        self.density
    }

    /// Collision filter used by the shape.
    #[inline]
    pub const fn filter(&self) -> Filter {
        self.filter
    }

    /// Whether the shape is configured as a sensor.
    #[inline]
    pub const fn is_sensor(&self) -> bool {
        self.sensor
    }

    /// Whether world-level custom filtering is enabled for the shape.
    #[inline]
    pub const fn custom_filtering_enabled(&self) -> bool {
        self.enable_custom_filtering
    }

    /// Whether sensor begin/end events are enabled for the shape.
    #[inline]
    pub const fn sensor_events_enabled(&self) -> bool {
        self.enable_sensor_events
    }

    /// Whether contact begin/end events are enabled for the shape.
    #[inline]
    pub const fn contact_events_enabled(&self) -> bool {
        self.enable_contact_events
    }

    /// Whether hit events are enabled for the shape.
    #[inline]
    pub const fn hit_events_enabled(&self) -> bool {
        self.enable_hit_events
    }

    /// Whether pre-solve events are enabled for the shape.
    #[inline]
    pub const fn pre_solve_events_enabled(&self) -> bool {
        self.enable_pre_solve_events
    }

    /// Whether contact-creation callbacks are invoked for the shape.
    #[inline]
    pub const fn invokes_contact_creation(&self) -> bool {
        self.invoke_contact_creation
    }

    /// Whether creating or destroying the shape updates the owning body's mass.
    #[inline]
    pub const fn updates_body_mass(&self) -> bool {
        self.update_body_mass
    }

    pub(crate) fn prepare(&self) -> ffi::b2ShapeDef {
        let mut raw: ffi::b2ShapeDef = crate::core::native_defaults::shape_def();
        raw.material = self.material.into_raw();
        raw.density = self.density;
        raw.filter = self.filter.into_raw();
        raw.enableCustomFiltering = self.enable_custom_filtering;
        raw.isSensor = self.sensor;
        raw.enableSensorEvents = self.enable_sensor_events;
        raw.enableContactEvents = self.enable_contact_events;
        raw.enableHitEvents = self.enable_hit_events;
        raw.enablePreSolveEvents = self.enable_pre_solve_events;
        raw.invokeContactCreation = self.invoke_contact_creation;
        raw.updateBodyMass = self.update_body_mass;
        raw
    }

    #[inline]
    pub fn validate(&self) -> Result<()> {
        check_shape_def_valid(self)
    }
}

#[doc(alias = "shape_builder")]
#[doc(alias = "shapebuilder")]
#[derive(Clone, Debug)]
pub struct ShapeDefBuilder {
    def: ShapeDef,
}

impl ShapeDefBuilder {
    /// Set the surface material (friction, restitution, etc.).
    pub fn material(mut self, mat: SurfaceMaterial) -> Self {
        self.def.material = mat;
        self
    }
    /// Density in kg/m². Affects mass.
    pub fn density(mut self, v: f32) -> Self {
        self.def.density = v;
        self
    }
    /// Collision filter (category/mask/group).
    pub fn filter(mut self, f: Filter) -> Self {
        self.def.filter = f;
        self
    }
    /// Enable user-provided filtering callback.
    ///
    /// Note: To receive custom filter calls you must also register a world-level callback via
    /// [`crate::World::set_custom_filter`].
    pub fn enable_custom_filtering(mut self, flag: bool) -> Self {
        self.def.enable_custom_filtering = flag;
        self
    }
    /// Mark as sensor (no collision response).
    pub fn sensor(mut self, flag: bool) -> Self {
        self.def.sensor = flag;
        self
    }
    /// Emit sensor begin/end touch events.
    pub fn enable_sensor_events(mut self, flag: bool) -> Self {
        self.def.enable_sensor_events = flag;
        self
    }
    /// Emit contact begin/end events.
    pub fn enable_contact_events(mut self, flag: bool) -> Self {
        self.def.enable_contact_events = flag;
        self
    }
    /// Emit impact hit events when above threshold.
    pub fn enable_hit_events(mut self, flag: bool) -> Self {
        self.def.enable_hit_events = flag;
        self
    }
    /// Emit pre-solve events (advanced).
    ///
    /// Note: To receive pre-solve events you must also register a world-level callback via
    /// [`crate::World::set_pre_solve`].
    pub fn enable_pre_solve_events(mut self, flag: bool) -> Self {
        self.def.enable_pre_solve_events = flag;
        self
    }
    /// Invoke user callback on contact creation.
    pub fn invoke_contact_creation(mut self, flag: bool) -> Self {
        self.def.invoke_contact_creation = flag;
        self
    }
    /// Recompute body mass when adding/removing this shape.
    pub fn update_body_mass(mut self, flag: bool) -> Self {
        self.def.update_body_mass = flag;
        self
    }
    pub fn build(self) -> Result<ShapeDef> {
        self.def.validate()?;
        Ok(self.def)
    }
}

impl From<ShapeDef> for ShapeDefBuilder {
    fn from(def: ShapeDef) -> Self {
        Self { def }
    }
}

// serde for SurfaceMaterial and ShapeDef via lightweight representations
#[cfg(feature = "serde")]
impl serde::Serialize for SurfaceMaterial {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            friction: f32,
            restitution: f32,
            rolling_resistance: f32,
            tangent_speed: f32,
            user_material_id: u64,
            custom_color: HexColor,
        }
        let r = Repr {
            friction: self.friction(),
            restitution: self.restitution(),
            rolling_resistance: self.rolling_resistance(),
            tangent_speed: self.tangent_speed(),
            user_material_id: self.user_material_id(),
            custom_color: self.custom_color(),
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for SurfaceMaterial {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Repr {
            friction: Option<f32>,
            restitution: Option<f32>,
            rolling_resistance: Option<f32>,
            tangent_speed: Option<f32>,
            user_material_id: Option<u64>,
            custom_color: Option<HexColor>,
        }
        let r = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        let mut sm = SurfaceMaterial::default();
        if let Some(value) = r.friction {
            sm = sm.with_friction(value).map_err(serde::de::Error::custom)?;
        }
        if let Some(value) = r.restitution {
            sm = sm
                .with_restitution(value)
                .map_err(serde::de::Error::custom)?;
        }
        if let Some(value) = r.rolling_resistance {
            sm = sm
                .with_rolling_resistance(value)
                .map_err(serde::de::Error::custom)?;
        }
        if let Some(value) = r.tangent_speed {
            sm = sm
                .with_tangent_speed(value)
                .map_err(serde::de::Error::custom)?;
        }
        if let Some(value) = r.user_material_id {
            sm = sm.with_user_material_id(value);
        }
        if let Some(value) = r.custom_color {
            sm = sm.with_custom_color(value);
        }
        sm.validate().map_err(serde::de::Error::custom)?;
        Ok(sm)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ShapeDef {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            material: SurfaceMaterial,
            density: f32,
            filter: Filter,
            enable_custom_filtering: bool,
            is_sensor: bool,
            enable_sensor_events: bool,
            enable_contact_events: bool,
            enable_hit_events: bool,
            enable_pre_solve_events: bool,
            invoke_contact_creation: bool,
            update_body_mass: bool,
        }
        let r = Repr {
            material: self.material,
            density: self.density,
            filter: self.filter,
            enable_custom_filtering: self.enable_custom_filtering,
            is_sensor: self.sensor,
            enable_sensor_events: self.enable_sensor_events,
            enable_contact_events: self.enable_contact_events,
            enable_hit_events: self.enable_hit_events,
            enable_pre_solve_events: self.enable_pre_solve_events,
            invoke_contact_creation: self.invoke_contact_creation,
            update_body_mass: self.update_body_mass,
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ShapeDef {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Repr {
            material: Option<SurfaceMaterial>,
            density: Option<f32>,
            filter: Option<Filter>,
            enable_custom_filtering: Option<bool>,
            is_sensor: Option<bool>,
            enable_sensor_events: Option<bool>,
            enable_contact_events: Option<bool>,
            enable_hit_events: Option<bool>,
            enable_pre_solve_events: Option<bool>,
            invoke_contact_creation: Option<bool>,
            update_body_mass: Option<bool>,
        }
        let r = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        let mut def = ShapeDef::default();
        if let Some(value) = r.material {
            def.material = value;
        }
        if let Some(value) = r.density {
            def.density = value;
        }
        if let Some(value) = r.filter {
            def.filter = value;
        }
        if let Some(value) = r.enable_custom_filtering {
            def.enable_custom_filtering = value;
        }
        if let Some(value) = r.is_sensor {
            def.sensor = value;
        }
        if let Some(value) = r.enable_sensor_events {
            def.enable_sensor_events = value;
        }
        if let Some(value) = r.enable_contact_events {
            def.enable_contact_events = value;
        }
        if let Some(value) = r.enable_hit_events {
            def.enable_hit_events = value;
        }
        if let Some(value) = r.enable_pre_solve_events {
            def.enable_pre_solve_events = value;
        }
        if let Some(value) = r.invoke_contact_creation {
            def.invoke_contact_creation = value;
        }
        if let Some(value) = r.update_body_mass {
            def.update_body_mass = value;
        }
        def.validate().map_err(serde::de::Error::custom)?;
        Ok(def)
    }
}

#[cfg(test)]
mod tests {
    use super::ShapeDef;
    use crate::Error;

    #[test]
    fn shape_definition_builder_rejects_invalid_draft_at_build() {
        assert_eq!(
            ShapeDef::builder().density(-1.0).build().unwrap_err(),
            Error::invalid_argument(
                "ShapeDef::validate",
                "density",
                "a finite value greater than or equal to zero",
            )
        );
    }
}
