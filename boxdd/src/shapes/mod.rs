//! Shapes API
//!
//! Safe wrappers around Box2D shapes. Shapes are attached to bodies and can be
//! modified at runtime. Use `ShapeDef` and `Body::create_*_shape` helpers to
//! create shapes.
use std::marker::PhantomData;
pub mod chain;
pub mod helpers;

use crate::body::Body;
use crate::filter::Filter;
use crate::types::ShapeId;
use boxdd_sys::ffi;

/// A shape owned by a body within a world.
pub struct Shape<'b, 'w> {
    pub(crate) id: ShapeId,
    _owner: PhantomData<&'b Body<'w>>, // ensure Body outlives Shape
}

impl<'b, 'w> Shape<'b, 'w> {
    pub fn id(&self) -> ShapeId {
        self.id
    }

    // Getters
    pub fn circle(&self) -> ffi::b2Circle {
        unsafe { ffi::b2Shape_GetCircle(self.id) }
    }
    pub fn segment(&self) -> ffi::b2Segment {
        unsafe { ffi::b2Shape_GetSegment(self.id) }
    }
    pub fn capsule(&self) -> ffi::b2Capsule {
        unsafe { ffi::b2Shape_GetCapsule(self.id) }
    }
    pub fn polygon(&self) -> ffi::b2Polygon {
        unsafe { ffi::b2Shape_GetPolygon(self.id) }
    }

    // Setters
    pub fn set_circle(&mut self, c: &ffi::b2Circle) {
        unsafe { ffi::b2Shape_SetCircle(self.id, c) }
    }
    pub fn set_segment(&mut self, s: &ffi::b2Segment) {
        unsafe { ffi::b2Shape_SetSegment(self.id, s) }
    }
    pub fn set_capsule(&mut self, c: &ffi::b2Capsule) {
        unsafe { ffi::b2Shape_SetCapsule(self.id, c) }
    }
    pub fn set_polygon(&mut self, p: &ffi::b2Polygon) {
        unsafe { ffi::b2Shape_SetPolygon(self.id, p) }
    }

    pub fn filter(&self) -> Filter {
        Filter::from(unsafe { ffi::b2Shape_GetFilter(self.id) })
    }
    pub fn set_filter(&mut self, f: Filter) {
        unsafe { ffi::b2Shape_SetFilter(self.id, f.into()) }
    }

    // Material and physical properties
    pub fn is_sensor(&self) -> bool {
        unsafe { ffi::b2Shape_IsSensor(self.id) }
    }
    pub fn set_density(&mut self, density: f32, update_body_mass: bool) {
        unsafe { ffi::b2Shape_SetDensity(self.id, density, update_body_mass) }
    }
    pub fn density(&self) -> f32 {
        unsafe { ffi::b2Shape_GetDensity(self.id) }
    }
    pub fn set_friction(&mut self, friction: f32) {
        unsafe { ffi::b2Shape_SetFriction(self.id, friction) }
    }
    pub fn friction(&self) -> f32 {
        unsafe { ffi::b2Shape_GetFriction(self.id) }
    }
    pub fn set_restitution(&mut self, restitution: f32) {
        unsafe { ffi::b2Shape_SetRestitution(self.id, restitution) }
    }
    pub fn restitution(&self) -> f32 {
        unsafe { ffi::b2Shape_GetRestitution(self.id) }
    }
    pub fn set_user_material(&mut self, material: u64) {
        unsafe { ffi::b2Shape_SetUserMaterial(self.id, material) }
    }
    pub fn user_material(&self) -> u64 {
        unsafe { ffi::b2Shape_GetUserMaterial(self.id) }
    }
    pub fn set_surface_material(&mut self, material: &SurfaceMaterial) {
        unsafe { ffi::b2Shape_SetSurfaceMaterial(self.id, &material.0) }
    }
    pub fn surface_material(&self) -> SurfaceMaterial {
        SurfaceMaterial(unsafe { ffi::b2Shape_GetSurfaceMaterial(self.id) })
    }

    // Opaque user pointer (engine-owned)
    /// Set an opaque user data pointer on this shape.
    ///
    /// # Safety
    /// The caller must ensure that `p` is valid for as long as the engine may
    /// read it and that any aliasing/lifetime constraints are upheld. Box2D stores this
    /// pointer and may access it during simulation callbacks.
    pub unsafe fn set_user_data_ptr(&mut self, p: *mut core::ffi::c_void) {
        unsafe { ffi::b2Shape_SetUserData(self.id, p) }
    }
    pub fn user_data_ptr(&self) -> *mut core::ffi::c_void {
        unsafe { ffi::b2Shape_GetUserData(self.id) }
    }

    pub fn contact_data(&self) -> Vec<ffi::b2ContactData> {
        let cap = unsafe { ffi::b2Shape_GetContactCapacity(self.id) }.max(0) as usize;
        if cap == 0 {
            return Vec::new();
        }
        let mut vec: Vec<ffi::b2ContactData> = Vec::with_capacity(cap);
        let wrote = unsafe { ffi::b2Shape_GetContactData(self.id, vec.as_mut_ptr(), cap as i32) }
            .max(0) as usize;
        unsafe { vec.set_len(wrote.min(cap)) };
        vec
    }

    /// Get the maximum capacity required for retrieving all the overlapped shapes on this sensor shape.
    /// Returns 0 if this shape is not a sensor.
    pub fn sensor_capacity(&self) -> i32 {
        unsafe { ffi::b2Shape_GetSensorCapacity(self.id) }
    }

    /// Get overlapped shapes for this sensor shape. If this is not a sensor, returns empty.
    /// Note: overlaps may contain destroyed shapes; use `sensor_overlaps_valid` to filter.
    pub fn sensor_overlaps(&self) -> Vec<ShapeId> {
        let cap = self.sensor_capacity();
        if cap <= 0 {
            return Vec::new();
        }
        let mut ids: Vec<ShapeId> = Vec::with_capacity(cap as usize);
        let wrote =
            unsafe { ffi::b2Shape_GetSensorData(self.id, ids.as_mut_ptr(), cap) }.max(0) as usize;
        unsafe { ids.set_len(wrote.min(cap as usize)) };
        ids
    }

    /// Get overlapped shapes and filter out invalid (destroyed) shape ids.
    pub fn sensor_overlaps_valid(&self) -> Vec<ShapeId> {
        self.sensor_overlaps()
            .into_iter()
            .filter(|&sid| unsafe { ffi::b2Shape_IsValid(sid) })
            .collect()
    }
}

impl<'b, 'w> Drop for Shape<'b, 'w> {
    fn drop(&mut self) {
        // Update body mass on shape destroy by default
        if unsafe { ffi::b2Shape_IsValid(self.id) } {
            unsafe { ffi::b2DestroyShape(self.id, true) };
        }
    }
}

/// Shape surface material parameters.
#[derive(Clone, Debug)]
pub struct SurfaceMaterial(pub(crate) ffi::b2SurfaceMaterial);

impl Default for SurfaceMaterial {
    fn default() -> Self {
        Self(unsafe { ffi::b2DefaultSurfaceMaterial() })
    }
}

impl SurfaceMaterial {
    pub fn friction(mut self, v: f32) -> Self {
        self.0.friction = v;
        self
    }
    pub fn restitution(mut self, v: f32) -> Self {
        self.0.restitution = v;
        self
    }
    pub fn rolling_resistance(mut self, v: f32) -> Self {
        self.0.rollingResistance = v;
        self
    }
    pub fn tangent_speed(mut self, v: f32) -> Self {
        self.0.tangentSpeed = v;
        self
    }
    pub fn user_material_id(mut self, v: u64) -> Self {
        self.0.userMaterialId = v;
        self
    }
    pub fn custom_color(mut self, rgba: u32) -> Self {
        self.0.customColor = rgba;
        self
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
    pub fn builder() -> ShapeDefBuilder {
        ShapeDefBuilder {
            def: Self::default(),
        }
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
    /// Low-level filter (category/mask/group).
    pub fn filter(mut self, f: ffi::b2Filter) -> Self {
        self.def.0.filter = f;
        self
    }
    /// High-level filter wrapper.
    pub fn filter_ex(mut self, f: Filter) -> Self {
        self.def.0.filter = f.into();
        self
    }
    /// Enable user-provided filtering callback.
    ///
    /// Note: To receive custom filter calls you must also register a world-level
    /// callback via `World::set_custom_filter_callback`.
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
    /// callback via `World::set_pre_solve_callback`.
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
            custom_color: u32,
        }
        let r = Repr {
            friction: self.0.friction,
            restitution: self.0.restitution,
            rolling_resistance: self.0.rollingResistance,
            tangent_speed: self.0.tangentSpeed,
            user_material_id: self.0.userMaterialId,
            custom_color: self.0.customColor,
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
            custom_color: u32,
        }
        let r = Repr::deserialize(deserializer)?;
        let mut sm = SurfaceMaterial::default();
        sm = sm
            .friction(r.friction)
            .restitution(r.restitution)
            .rolling_resistance(r.rolling_resistance)
            .tangent_speed(r.tangent_speed)
            .user_material_id(r.user_material_id)
            .custom_color(r.custom_color);
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
            material: SurfaceMaterial(self.0.material),
            density: self.0.density,
            filter: Filter::from(self.0.filter),
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
            b = b.filter_ex(f);
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

/// Circle primitive helper
#[inline]
pub fn circle<V: Into<crate::types::Vec2>>(center: V, radius: f32) -> ffi::b2Circle {
    ffi::b2Circle {
        center: ffi::b2Vec2::from(center.into()),
        radius,
    }
}

/// Segment primitive helper
#[inline]
pub fn segment<V: Into<crate::types::Vec2>>(p1: V, p2: V) -> ffi::b2Segment {
    ffi::b2Segment {
        point1: ffi::b2Vec2::from(p1.into()),
        point2: ffi::b2Vec2::from(p2.into()),
    }
}

/// Helper constructors (re-exported): `capsule`, `box_polygon`, `polygon_from_points`.
pub use helpers::{box_polygon, capsule, polygon_from_points};

// With sys-level mint conversions, polygon_from_points accepts mint::Vector2<f32> directly.

impl<'w> Body<'w> {
    pub fn create_circle_shape<'b>(
        &'b mut self,
        def: &ShapeDef,
        c: &ffi::b2Circle,
    ) -> Shape<'b, 'w> {
        let id = unsafe { ffi::b2CreateCircleShape(self.id, &def.0, c) };
        Shape {
            id,
            _owner: PhantomData,
        }
    }
    pub fn create_segment_shape<'b>(
        &'b mut self,
        def: &ShapeDef,
        s: &ffi::b2Segment,
    ) -> Shape<'b, 'w> {
        let id = unsafe { ffi::b2CreateSegmentShape(self.id, &def.0, s) };
        Shape {
            id,
            _owner: PhantomData,
        }
    }
    pub fn create_capsule_shape<'b>(
        &'b mut self,
        def: &ShapeDef,
        c: &ffi::b2Capsule,
    ) -> Shape<'b, 'w> {
        let id = unsafe { ffi::b2CreateCapsuleShape(self.id, &def.0, c) };
        Shape {
            id,
            _owner: PhantomData,
        }
    }
    pub fn create_polygon_shape<'b>(
        &'b mut self,
        def: &ShapeDef,
        p: &ffi::b2Polygon,
    ) -> Shape<'b, 'w> {
        let id = unsafe { ffi::b2CreatePolygonShape(self.id, &def.0, p) };
        Shape {
            id,
            _owner: PhantomData,
        }
    }

    // Convenience creators
    pub fn create_box<'b>(&'b mut self, def: &ShapeDef, half_w: f32, half_h: f32) -> Shape<'b, 'w> {
        let poly = unsafe { ffi::b2MakeBox(half_w, half_h) };
        self.create_polygon_shape(def, &poly)
    }
    pub fn create_circle_simple<'b>(&'b mut self, def: &ShapeDef, radius: f32) -> Shape<'b, 'w> {
        let c = ffi::b2Circle {
            center: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            radius,
        };
        self.create_circle_shape(def, &c)
    }
    pub fn create_segment_simple<'b, V: Into<crate::types::Vec2>>(
        &'b mut self,
        def: &ShapeDef,
        p1: V,
        p2: V,
    ) -> Shape<'b, 'w> {
        let seg = ffi::b2Segment {
            point1: ffi::b2Vec2::from(p1.into()),
            point2: ffi::b2Vec2::from(p2.into()),
        };
        self.create_segment_shape(def, &seg)
    }
    pub fn create_capsule_simple<'b, V: Into<crate::types::Vec2>>(
        &'b mut self,
        def: &ShapeDef,
        c1: V,
        c2: V,
        radius: f32,
    ) -> Shape<'b, 'w> {
        let cap = ffi::b2Capsule {
            center1: ffi::b2Vec2::from(c1.into()),
            center2: ffi::b2Vec2::from(c2.into()),
            radius,
        };
        self.create_capsule_shape(def, &cap)
    }
    pub fn create_polygon_from_points<'b, I, P>(
        &'b mut self,
        def: &ShapeDef,
        points: I,
        radius: f32,
    ) -> Option<Shape<'b, 'w>>
    where
        I: IntoIterator<Item = P>,
        P: Into<crate::types::Vec2>,
    {
        let poly = crate::shapes::polygon_from_points(points, radius)?;
        Some(self.create_polygon_shape(def, &poly))
    }
}
// Shapes: module note moved to top-level doc above.
