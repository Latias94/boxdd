use std::marker::PhantomData;
pub mod chain;

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

    pub fn contact_data(&self) -> Vec<ffi::b2ContactData> {
        let cap = 64;
        let mut vec: Vec<ffi::b2ContactData> = Vec::with_capacity(cap);
        let written = unsafe { ffi::b2Shape_GetContactData(self.id, vec.as_mut_ptr(), cap as i32) };
        unsafe { vec.set_len(written.max(0) as usize) };
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
        let wrote = unsafe { ffi::b2Shape_GetSensorData(self.id, ids.as_mut_ptr(), cap) };
        unsafe { ids.set_len(wrote.max(0) as usize) };
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
        unsafe { ffi::b2DestroyShape(self.id, true) };
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

#[derive(Clone, Debug)]
pub struct ShapeDefBuilder {
    def: ShapeDef,
}

impl ShapeDefBuilder {
    pub fn material(mut self, mat: SurfaceMaterial) -> Self {
        self.def.0.material = mat.0;
        self
    }
    pub fn density(mut self, v: f32) -> Self {
        self.def.0.density = v;
        self
    }
    pub fn filter(mut self, f: ffi::b2Filter) -> Self {
        self.def.0.filter = f;
        self
    }
    pub fn filter_ex(mut self, f: Filter) -> Self {
        self.def.0.filter = f.into();
        self
    }
    pub fn enable_custom_filtering(mut self, flag: bool) -> Self {
        self.def.0.enableCustomFiltering = flag;
        self
    }
    pub fn sensor(mut self, flag: bool) -> Self {
        self.def.0.isSensor = flag;
        self
    }
    pub fn enable_sensor_events(mut self, flag: bool) -> Self {
        self.def.0.enableSensorEvents = flag;
        self
    }
    pub fn enable_contact_events(mut self, flag: bool) -> Self {
        self.def.0.enableContactEvents = flag;
        self
    }
    pub fn enable_hit_events(mut self, flag: bool) -> Self {
        self.def.0.enableHitEvents = flag;
        self
    }
    pub fn enable_pre_solve_events(mut self, flag: bool) -> Self {
        self.def.0.enablePreSolveEvents = flag;
        self
    }
    pub fn invoke_contact_creation(mut self, flag: bool) -> Self {
        self.def.0.invokeContactCreation = flag;
        self
    }
    pub fn update_body_mass(mut self, flag: bool) -> Self {
        self.def.0.updateBodyMass = flag;
        self
    }
    pub fn build(self) -> ShapeDef {
        self.def
    }
}

/// Circle primitive helper
pub fn circle<V: Into<ffi::b2Vec2>>(center: V, radius: f32) -> ffi::b2Circle {
    ffi::b2Circle {
        center: center.into(),
        radius,
    }
}

/// Segment primitive helper
pub fn segment<V: Into<ffi::b2Vec2>>(p1: V, p2: V) -> ffi::b2Segment {
    ffi::b2Segment {
        point1: p1.into(),
        point2: p2.into(),
    }
}

/// Capsule primitive helper
pub fn capsule<V: Into<ffi::b2Vec2>>(c1: V, c2: V, radius: f32) -> ffi::b2Capsule {
    ffi::b2Capsule {
        center1: c1.into(),
        center2: c2.into(),
        radius,
    }
}

/// Polygon helpers
pub fn box_polygon(half_width: f32, half_height: f32) -> ffi::b2Polygon {
    unsafe { ffi::b2MakeBox(half_width, half_height) }
}

pub fn polygon_from_points<I, P>(points: I, radius: f32) -> Option<ffi::b2Polygon>
where
    I: IntoIterator<Item = P>,
    P: Into<ffi::b2Vec2>,
{
    let pts: Vec<ffi::b2Vec2> = points.into_iter().map(Into::into).collect();
    if pts.is_empty() {
        return None;
    }
    let hull = unsafe { ffi::b2ComputeHull(pts.as_ptr(), pts.len() as i32) };
    let poly = unsafe { ffi::b2MakePolygon(&hull, radius) };
    Some(poly)
}

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
    pub fn create_segment_simple<'b, V: Into<ffi::b2Vec2>>(
        &'b mut self,
        def: &ShapeDef,
        p1: V,
        p2: V,
    ) -> Shape<'b, 'w> {
        let seg = ffi::b2Segment {
            point1: p1.into(),
            point2: p2.into(),
        };
        self.create_segment_shape(def, &seg)
    }
    pub fn create_capsule_simple<'b, V: Into<ffi::b2Vec2>>(
        &'b mut self,
        def: &ShapeDef,
        c1: V,
        c2: V,
        radius: f32,
    ) -> Shape<'b, 'w> {
        let cap = ffi::b2Capsule {
            center1: c1.into(),
            center2: c2.into(),
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
        P: Into<ffi::b2Vec2>,
    {
        let poly = crate::shapes::polygon_from_points(points, radius)?;
        Some(self.create_polygon_shape(def, &poly))
    }
}
