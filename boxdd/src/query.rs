//! Broad-phase queries and casting helpers.
//!
//! - AABB overlap: collect matching shape ids.
//! - Ray casts: closest or all hits along a path.
//! - Shape overlap / casting: build a temporary proxy from points + radius (accepts mint vectors).
//! - Offset proxies: apply translation + rotation to the proxy for queries in local frames.
//!
//! Filters: use `QueryFilter` to restrict categories/masks.
use crate::types::{ShapeId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;

/// Axis-aligned bounding box
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb {
    pub lower: Vec2,
    pub upper: Vec2,
}

impl From<Aabb> for ffi::b2AABB {
    fn from(a: Aabb) -> Self {
        ffi::b2AABB {
            lowerBound: a.lower.into(),
            upperBound: a.upper.into(),
        }
    }
}

impl Aabb {
    /// Create an AABB from lower and upper points.
    #[inline]
    pub fn new<L: Into<Vec2>, U: Into<Vec2>>(lower: L, upper: U) -> Self {
        Self {
            lower: lower.into(),
            upper: upper.into(),
        }
    }
    /// Create an AABB from center and half-extents (both in world units).
    #[inline]
    pub fn from_center_half_extents<C: Into<Vec2>, H: Into<Vec2>>(center: C, half: H) -> Self {
        let c = center.into();
        let h = half.into();
        Self {
            lower: Vec2::new(c.x - h.x, c.y - h.y),
            upper: Vec2::new(c.x + h.x, c.y + h.y),
        }
    }
}

/// Filter for queries
#[derive(Copy, Clone, Debug)]
pub struct QueryFilter(pub(crate) ffi::b2QueryFilter);

impl Default for QueryFilter {
    fn default() -> Self {
        Self(unsafe { ffi::b2DefaultQueryFilter() })
    }
}

impl QueryFilter {
    pub fn mask(mut self, bits: u64) -> Self {
        self.0.maskBits = bits;
        self
    }
    pub fn category(mut self, bits: u64) -> Self {
        self.0.categoryBits = bits;
        self
    }
}

/// Result of a closest ray cast
#[derive(Copy, Clone, Debug)]
pub struct RayResult {
    pub shape_id: ShapeId,
    pub point: Vec2,
    pub normal: Vec2,
    pub fraction: f32,
    pub hit: bool,
}

impl From<ffi::b2RayResult> for RayResult {
    fn from(r: ffi::b2RayResult) -> Self {
        Self {
            shape_id: r.shapeId,
            point: r.point.into(),
            normal: r.normal.into(),
            fraction: r.fraction,
            hit: r.hit,
        }
    }
}

impl World {
    /// Overlap test for all shapes in an AABB. Returns matching shape ids.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2, Aabb, QueryFilter};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let b = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
    /// let sdef = ShapeDef::builder().density(1.0).build();
    /// world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));
    /// let hits = world.overlap_aabb(Aabb { lower: Vec2::new(-1.0, -1.0), upper: Vec2::new(1.0, 3.0) }, QueryFilter::default());
    /// assert!(!hits.is_empty());
    /// ```
    pub fn overlap_aabb(&self, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
        unsafe extern "C" fn cb(shape_id: ffi::b2ShapeId, ctx: *mut core::ffi::c_void) -> bool {
            let out = unsafe { &mut *(ctx as *mut Vec<ShapeId>) };
            out.push(shape_id);
            true // continue
        }
        let mut out: Vec<ShapeId> = Vec::new();
        unsafe {
            let _ = ffi::b2World_OverlapAABB(
                self.raw(),
                aabb.into(),
                filter.0,
                Some(cb),
                &mut out as *mut _ as *mut _,
            );
        }
        out
    }

    /// Cast a ray and return the closest hit.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let hit = world.cast_ray_closest(Vec2::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
    /// if hit.hit { /* use hit.point / hit.normal */ }
    /// ```
    pub fn cast_ray_closest<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> RayResult {
        let o: ffi::b2Vec2 = origin.into().into();
        let t: ffi::b2Vec2 = translation.into().into();
        let raw = unsafe { ffi::b2World_CastRayClosest(self.raw(), o, t, filter.0) };
        RayResult::from(raw)
    }

    /// Cast a ray and collect all hits along the path.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let hits = world.cast_ray_all(Vec2::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
    /// for h in hits { let _ = (h.point, h.normal, h.fraction); }
    /// ```
    pub fn cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult> {
        #[allow(clippy::unnecessary_cast)]
        unsafe extern "C" fn cb(
            shape_id: ffi::b2ShapeId,
            point: ffi::b2Vec2,
            normal: ffi::b2Vec2,
            fraction: f32,
            ctx: *mut core::ffi::c_void,
        ) -> f32 {
            let out = unsafe { &mut *(ctx as *mut Vec<RayResult>) };
            out.push(RayResult {
                shape_id,
                point: point.into(),
                normal: normal.into(),
                fraction,
                hit: true,
            });
            1.0 // continue, don't clip
        }
        let mut out: Vec<RayResult> = Vec::new();
        let o: ffi::b2Vec2 = origin.into().into();
        let t: ffi::b2Vec2 = translation.into().into();
        unsafe {
            let _ = ffi::b2World_CastRay(
                self.raw(),
                o,
                t,
                filter.0,
                Some(cb),
                &mut out as *mut _ as *mut _,
            );
        }
        out
    }

    /// Overlap polygon points (creates a temporary shape proxy from given points + radius) and collect all shape ids.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let square = [Vec2::new(-0.5, -0.5), Vec2::new(0.5, -0.5), Vec2::new(0.5, 0.5), Vec2::new(-0.5, 0.5)];
    /// let hits = world.overlap_polygon_points(square, 0.0, QueryFilter::default());
    /// let _ = hits;
    /// ```
    pub fn overlap_polygon_points<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<ShapeId>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        let pts: Vec<ffi::b2Vec2> = points
            .into_iter()
            .map(|p| ffi::b2Vec2::from(p.into()))
            .collect();
        if pts.is_empty() {
            return Vec::new();
        }
        let proxy = unsafe { ffi::b2MakeProxy(pts.as_ptr(), pts.len() as i32, radius) };
        unsafe extern "C" fn cb(shape_id: ffi::b2ShapeId, ctx: *mut core::ffi::c_void) -> bool {
            let out = unsafe { &mut *(ctx as *mut Vec<ShapeId>) };
            out.push(shape_id);
            true
        }
        let mut out = Vec::new();
        unsafe {
            let _ = ffi::b2World_OverlapShape(
                self.raw(),
                &proxy,
                filter.0,
                Some(cb),
                &mut out as *mut _ as *mut _,
            );
        }
        out
    }

    /// Cast a polygon proxy and collect hits. Returns all intersections with fraction and contact info.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let tri = [Vec2::new(0.0, 0.0), Vec2::new(0.5, 0.0), Vec2::new(0.25, 0.5)];
    /// let hits = world.cast_shape_points(tri, 0.0, Vec2::new(0.0, -1.0), QueryFilter::default());
    /// for h in hits { let _ = (h.point, h.normal, h.fraction); }
    /// ```
    pub fn cast_shape_points<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        #[allow(clippy::unnecessary_cast)]
        unsafe extern "C" fn cb(
            shape_id: ffi::b2ShapeId,
            point: ffi::b2Vec2,
            normal: ffi::b2Vec2,
            fraction: f32,
            ctx: *mut core::ffi::c_void,
        ) -> f32 {
            let out = unsafe { &mut *(ctx as *mut Vec<RayResult>) };
            out.push(RayResult {
                shape_id,
                point: point.into(),
                normal: normal.into(),
                fraction,
                hit: true,
            });
            1.0
        }
        let pts: Vec<ffi::b2Vec2> = points
            .into_iter()
            .map(|p| ffi::b2Vec2::from(p.into()))
            .collect();
        if pts.is_empty() {
            return Vec::new();
        }
        let proxy = unsafe { ffi::b2MakeProxy(pts.as_ptr(), pts.len() as i32, radius) };
        let mut out: Vec<RayResult> = Vec::new();
        let t: ffi::b2Vec2 = translation.into().into();
        unsafe {
            let _ = ffi::b2World_CastShape(
                self.raw(),
                &proxy,
                t,
                filter.0,
                Some(cb),
                &mut out as *mut _ as *mut _,
            );
        }
        out
    }

    /// Cast a capsule mover and return remaining fraction (1.0 = free, < 1.0 = hit earlier).
    pub fn cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> f32 {
        let cap = ffi::b2Capsule {
            center1: c1.into().into(),
            center2: c2.into().into(),
            radius,
        };
        let t: ffi::b2Vec2 = translation.into().into();
        unsafe { ffi::b2World_CastMover(self.raw(), &cap, t, filter.0) }
    }

    /// Overlap polygon points with an offset transform.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let rect = [Vec2::new(-0.5, -0.25), Vec2::new(0.5, -0.25), Vec2::new(0.5, 0.25), Vec2::new(-0.5, 0.25)];
    /// let hits = world.overlap_polygon_points_with_offset(rect, 0.0, Vec2::new(0.0, 2.0), 0.0_f32, QueryFilter::default());
    /// let _ = hits;
    /// ```
    pub fn overlap_polygon_points_with_offset<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
    ) -> Vec<ShapeId>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        let pts: Vec<ffi::b2Vec2> = points
            .into_iter()
            .map(|p| ffi::b2Vec2::from(p.into()))
            .collect();
        if pts.is_empty() {
            return Vec::new();
        }
        let (s, c) = angle_radians.into().sin_cos();
        let pos: ffi::b2Vec2 = position.into().into();
        let proxy = unsafe {
            ffi::b2MakeOffsetProxy(
                pts.as_ptr(),
                pts.len() as i32,
                radius,
                pos,
                ffi::b2Rot { c, s },
            )
        };
        unsafe extern "C" fn cb(shape_id: ffi::b2ShapeId, ctx: *mut core::ffi::c_void) -> bool {
            let out = unsafe { &mut *(ctx as *mut Vec<ShapeId>) };
            out.push(shape_id);
            true
        }
        let mut out = Vec::new();
        unsafe {
            let _ = ffi::b2World_OverlapShape(
                self.raw(),
                &proxy,
                filter.0,
                Some(cb),
                &mut out as *mut _ as *mut _,
            );
        }
        out
    }

    /// Cast polygon points with an offset transform (position + angle).
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let rect = [Vec2::new(-0.5, -0.25), Vec2::new(0.5, -0.25), Vec2::new(0.5, 0.25), Vec2::new(-0.5, 0.25)];
    /// let hits = world.cast_shape_points_with_offset(rect, 0.0, Vec2::new(0.0, 2.0), 0.0_f32, Vec2::new(0.0, -1.0), QueryFilter::default());
    /// for h in hits { let _ = (h.point, h.normal, h.fraction); }
    /// ```
    pub fn cast_shape_points_with_offset<I, P, V, A, VT>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        #[allow(clippy::unnecessary_cast)]
        unsafe extern "C" fn cb(
            shape_id: ffi::b2ShapeId,
            point: ffi::b2Vec2,
            normal: ffi::b2Vec2,
            fraction: f32,
            ctx: *mut core::ffi::c_void,
        ) -> f32 {
            let out = unsafe { &mut *(ctx as *mut Vec<RayResult>) };
            out.push(RayResult {
                shape_id,
                point: point.into(),
                normal: normal.into(),
                fraction,
                hit: true,
            });
            1.0
        }
        let pts: Vec<ffi::b2Vec2> = points
            .into_iter()
            .map(|p| ffi::b2Vec2::from(p.into()))
            .collect();
        if pts.is_empty() {
            return Vec::new();
        }
        let (s, c) = angle_radians.into().sin_cos();
        let pos: ffi::b2Vec2 = position.into().into();
        let proxy = unsafe {
            ffi::b2MakeOffsetProxy(
                pts.as_ptr(),
                pts.len() as i32,
                radius,
                pos,
                ffi::b2Rot { c, s },
            )
        };
        let mut out: Vec<RayResult> = Vec::new();
        let t: ffi::b2Vec2 = translation.into().into();
        unsafe {
            let _ = ffi::b2World_CastShape(
                self.raw(),
                &proxy,
                t,
                filter.0,
                Some(cb),
                &mut out as *mut _ as *mut _,
            );
        }
        out
    }
}
