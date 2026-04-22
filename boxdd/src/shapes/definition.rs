use crate::debug_draw::HexColor;
use crate::error::ApiResult;
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
        Self(unsafe { ffi::b2DefaultSurfaceMaterial() })
    }
}

impl SurfaceMaterial {
    #[inline]
    pub const fn from_raw(raw: ffi::b2SurfaceMaterial) -> Self {
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

    pub fn with_friction(mut self, v: f32) -> Self {
        self.0.friction = v;
        self
    }
    pub fn with_restitution(mut self, v: f32) -> Self {
        self.0.restitution = v;
        self
    }
    pub fn with_rolling_resistance(mut self, v: f32) -> Self {
        self.0.rollingResistance = v;
        self
    }
    pub fn with_tangent_speed(mut self, v: f32) -> Self {
        self.0.tangentSpeed = v;
        self
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
    pub fn validate(&self) -> ApiResult<()> {
        check_surface_material_valid(self)
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

/// Shape definition with Builder pattern.
#[doc(alias = "shape_def")]
#[doc(alias = "shapedef")]
#[derive(Clone, Debug)]
pub struct ShapeDef(pub(crate) ffi::b2ShapeDef);

impl Default for ShapeDef {
    fn default() -> Self {
        Self(unsafe { ffi::b2DefaultShapeDef() })
    }
}

impl ShapeDef {
    /// Start building a new `ShapeDef` from defaults.
    pub fn builder() -> ShapeDefBuilder {
        ShapeDefBuilder {
            def: Self::default(),
        }
    }

    /// Construct from the raw Box2D shape definition value.
    #[inline]
    pub fn from_raw(raw: ffi::b2ShapeDef) -> Self {
        Self(raw)
    }

    /// Surface material parameters used by the shape.
    #[inline]
    pub const fn material(&self) -> SurfaceMaterial {
        SurfaceMaterial::from_raw(self.0.material)
    }

    /// Density in kg/m².
    #[inline]
    pub const fn density(&self) -> f32 {
        self.0.density
    }

    /// Collision filter used by the shape.
    #[inline]
    pub const fn filter(&self) -> Filter {
        Filter::from_raw(self.0.filter)
    }

    /// Whether the shape is configured as a sensor.
    #[inline]
    pub const fn is_sensor(&self) -> bool {
        self.0.isSensor
    }

    /// Whether world-level custom filtering is enabled for the shape.
    #[inline]
    pub const fn custom_filtering_enabled(&self) -> bool {
        self.0.enableCustomFiltering
    }

    /// Whether sensor begin/end events are enabled for the shape.
    #[inline]
    pub const fn sensor_events_enabled(&self) -> bool {
        self.0.enableSensorEvents
    }

    /// Whether contact begin/end events are enabled for the shape.
    #[inline]
    pub const fn contact_events_enabled(&self) -> bool {
        self.0.enableContactEvents
    }

    /// Whether hit events are enabled for the shape.
    #[inline]
    pub const fn hit_events_enabled(&self) -> bool {
        self.0.enableHitEvents
    }

    /// Whether pre-solve events are enabled for the shape.
    #[inline]
    pub const fn pre_solve_events_enabled(&self) -> bool {
        self.0.enablePreSolveEvents
    }

    /// Whether contact-creation callbacks are invoked for the shape.
    #[inline]
    pub const fn invokes_contact_creation(&self) -> bool {
        self.0.invokeContactCreation
    }

    /// Whether creating or destroying the shape updates the owning body's mass.
    #[inline]
    pub const fn updates_body_mass(&self) -> bool {
        self.0.updateBodyMass
    }

    /// Convert into the raw Box2D shape definition value.
    #[inline]
    pub fn into_raw(self) -> ffi::b2ShapeDef {
        self.0
    }

    #[inline]
    pub fn validate(&self) -> ApiResult<()> {
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
        self.def.0.material = mat.0;
        self
    }
    /// Density in kg/m². Affects mass.
    pub fn density(mut self, v: f32) -> Self {
        self.def.0.density = v;
        self
    }
    /// Collision filter (category/mask/group).
    pub fn filter(mut self, f: Filter) -> Self {
        self.def.0.filter = f.into_raw();
        self
    }
    /// Raw Box2D filter escape hatch.
    pub fn filter_raw(mut self, f: ffi::b2Filter) -> Self {
        self.def.0.filter = f;
        self
    }
    /// Enable user-provided filtering callback.
    ///
    /// Note: To receive custom filter calls you must also register a world-level
    /// callback via `World::set_custom_filter_callback` or `World::set_custom_filter_with_ctx`.
    pub fn enable_custom_filtering(mut self, flag: bool) -> Self {
        self.def.0.enableCustomFiltering = flag;
        self
    }
    /// Mark as sensor (no collision response).
    pub fn sensor(mut self, flag: bool) -> Self {
        self.def.0.isSensor = flag;
        self
    }
    /// Emit sensor begin/end touch events.
    pub fn enable_sensor_events(mut self, flag: bool) -> Self {
        self.def.0.enableSensorEvents = flag;
        self
    }
    /// Emit contact begin/end events.
    pub fn enable_contact_events(mut self, flag: bool) -> Self {
        self.def.0.enableContactEvents = flag;
        self
    }
    /// Emit impact hit events when above threshold.
    pub fn enable_hit_events(mut self, flag: bool) -> Self {
        self.def.0.enableHitEvents = flag;
        self
    }
    /// Emit pre-solve events (advanced).
    ///
    /// Note: To receive pre-solve events you must also register a world-level
    /// callback via `World::set_pre_solve_callback` or `World::set_pre_solve_with_ctx`.
    pub fn enable_pre_solve_events(mut self, flag: bool) -> Self {
        self.def.0.enablePreSolveEvents = flag;
        self
    }
    /// Invoke user callback on contact creation.
    pub fn invoke_contact_creation(mut self, flag: bool) -> Self {
        self.def.0.invokeContactCreation = flag;
        self
    }
    /// Recompute body mass when adding/removing this shape.
    pub fn update_body_mass(mut self, flag: bool) -> Self {
        self.def.0.updateBodyMass = flag;
        self
    }
    #[must_use]
    pub fn build(self) -> ShapeDef {
        self.def
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
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            #[serde(default)]
            friction: f32,
            #[serde(default)]
            restitution: f32,
            #[serde(default)]
            rolling_resistance: f32,
            #[serde(default)]
            tangent_speed: f32,
            #[serde(default)]
            user_material_id: u64,
            #[serde(default)]
            custom_color: HexColor,
        }
        let r = Repr::deserialize(deserializer)?;
        let mut sm = SurfaceMaterial::default();
        sm = sm
            .with_friction(r.friction)
            .with_restitution(r.restitution)
            .with_rolling_resistance(r.rolling_resistance)
            .with_tangent_speed(r.tangent_speed)
            .with_user_material_id(r.user_material_id)
            .with_custom_color(r.custom_color);
        Ok(sm)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ShapeDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
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
            material: SurfaceMaterial::from_raw(self.0.material),
            density: self.0.density,
            filter: Filter::from_raw(self.0.filter),
            enable_custom_filtering: self.0.enableCustomFiltering,
            is_sensor: self.0.isSensor,
            enable_sensor_events: self.0.enableSensorEvents,
            enable_contact_events: self.0.enableContactEvents,
            enable_hit_events: self.0.enableHitEvents,
            enable_pre_solve_events: self.0.enablePreSolveEvents,
            invoke_contact_creation: self.0.invokeContactCreation,
            update_body_mass: self.0.updateBodyMass,
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ShapeDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            #[serde(default)]
            material: Option<SurfaceMaterial>,
            #[serde(default)]
            density: f32,
            #[serde(default)]
            filter: Option<Filter>,
            #[serde(default)]
            enable_custom_filtering: bool,
            #[serde(default)]
            is_sensor: bool,
            #[serde(default)]
            enable_sensor_events: bool,
            #[serde(default)]
            enable_contact_events: bool,
            #[serde(default)]
            enable_hit_events: bool,
            #[serde(default)]
            enable_pre_solve_events: bool,
            #[serde(default)]
            invoke_contact_creation: bool,
            #[serde(default)]
            update_body_mass: bool,
        }
        let r = Repr::deserialize(deserializer)?;
        let mut b = ShapeDef::builder();
        if let Some(mat) = r.material {
            b = b.material(mat);
        }
        if let Some(f) = r.filter {
            b = b.filter(f);
        }
        b = b
            .density(r.density)
            .enable_custom_filtering(r.enable_custom_filtering)
            .sensor(r.is_sensor)
            .enable_sensor_events(r.enable_sensor_events)
            .enable_contact_events(r.enable_contact_events)
            .enable_hit_events(r.enable_hit_events)
            .enable_pre_solve_events(r.enable_pre_solve_events)
            .invoke_contact_creation(r.invoke_contact_creation)
            .update_body_mass(r.update_body_mass);
        Ok(b.build())
    }
}
