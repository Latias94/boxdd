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
    pub fn overlap_aabb(&self, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
        unsafe extern "C" fn cb(shape_id: ffi::b2ShapeId, ctx: *mut core::ffi::c_void) -> bool {
            let out = &mut *(ctx as *mut Vec<ShapeId>);
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
    pub fn cast_ray_closest<VO: Into<ffi::b2Vec2>, VT: Into<ffi::b2Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> RayResult {
        let raw = unsafe {
            ffi::b2World_CastRayClosest(self.raw(), origin.into(), translation.into(), filter.0)
        };
        RayResult::from(raw)
    }

    /// Cast a ray and collect all hits along the path.
    pub fn cast_ray_all<VO: Into<ffi::b2Vec2>, VT: Into<ffi::b2Vec2>>(
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
            let out = &mut *(ctx as *mut Vec<RayResult>);
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
        unsafe {
            let _ = ffi::b2World_CastRay(
                self.raw(),
                origin.into(),
                translation.into(),
                filter.0,
                Some(cb),
                &mut out as *mut _ as *mut _,
            );
        }
        out
    }

    /// Overlap polygon points (creates a temporary shape proxy from given points + radius) and collect all shape ids.
    pub fn overlap_polygon_points<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<ShapeId>
    where
        I: IntoIterator<Item = P>,
        P: Into<ffi::b2Vec2>,
    {
        let pts: Vec<ffi::b2Vec2> = points.into_iter().map(Into::into).collect();
        if pts.is_empty() {
            return Vec::new();
        }
        let proxy = unsafe { ffi::b2MakeProxy(pts.as_ptr(), pts.len() as i32, radius) };
        unsafe extern "C" fn cb(shape_id: ffi::b2ShapeId, ctx: *mut core::ffi::c_void) -> bool {
            let out = &mut *(ctx as *mut Vec<ShapeId>);
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
    pub fn cast_shape_points<I, P, VT>(
        &self,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult>
    where
        I: IntoIterator<Item = P>,
        P: Into<ffi::b2Vec2>,
        VT: Into<ffi::b2Vec2>,
    {
        #[allow(clippy::unnecessary_cast)]
        unsafe extern "C" fn cb(
            shape_id: ffi::b2ShapeId,
            point: ffi::b2Vec2,
            normal: ffi::b2Vec2,
            fraction: f32,
            ctx: *mut core::ffi::c_void,
        ) -> f32 {
            let out = &mut *(ctx as *mut Vec<RayResult>);
            out.push(RayResult {
                shape_id,
                point: point.into(),
                normal: normal.into(),
                fraction,
                hit: true,
            });
            1.0
        }
        let pts: Vec<ffi::b2Vec2> = points.into_iter().map(Into::into).collect();
        if pts.is_empty() {
            return Vec::new();
        }
        let proxy = unsafe { ffi::b2MakeProxy(pts.as_ptr(), pts.len() as i32, radius) };
        let mut out: Vec<RayResult> = Vec::new();
        unsafe {
            let _ = ffi::b2World_CastShape(
                self.raw(),
                &proxy,
                translation.into(),
                filter.0,
                Some(cb),
                &mut out as *mut _ as *mut _,
            );
        }
        out
    }

    /// Cast a capsule mover and return remaining fraction (1.0 = free, < 1.0 = hit earlier).
    pub fn cast_mover<V1: Into<ffi::b2Vec2>, V2: Into<ffi::b2Vec2>, VT: Into<ffi::b2Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> f32 {
        let cap = ffi::b2Capsule {
            center1: c1.into(),
            center2: c2.into(),
            radius,
        };
        unsafe { ffi::b2World_CastMover(self.raw(), &cap, translation.into(), filter.0) }
    }

    /// Overlap polygon points with an offset transform.
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
        P: Into<ffi::b2Vec2>,
        V: Into<ffi::b2Vec2>,
        A: Into<f32>,
    {
        let pts: Vec<ffi::b2Vec2> = points.into_iter().map(Into::into).collect();
        if pts.is_empty() {
            return Vec::new();
        }
        let (s, c) = angle_radians.into().sin_cos();
        let proxy = unsafe {
            ffi::b2MakeOffsetProxy(
                pts.as_ptr(),
                pts.len() as i32,
                radius,
                position.into(),
                ffi::b2Rot { c, s },
            )
        };
        unsafe extern "C" fn cb(shape_id: ffi::b2ShapeId, ctx: *mut core::ffi::c_void) -> bool {
            let out = &mut *(ctx as *mut Vec<ShapeId>);
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
        P: Into<ffi::b2Vec2>,
        V: Into<ffi::b2Vec2>,
        A: Into<f32>,
        VT: Into<ffi::b2Vec2>,
    {
        #[allow(clippy::unnecessary_cast)]
        unsafe extern "C" fn cb(
            shape_id: ffi::b2ShapeId,
            point: ffi::b2Vec2,
            normal: ffi::b2Vec2,
            fraction: f32,
            ctx: *mut core::ffi::c_void,
        ) -> f32 {
            let out = &mut *(ctx as *mut Vec<RayResult>);
            out.push(RayResult {
                shape_id,
                point: point.into(),
                normal: normal.into(),
                fraction,
                hit: true,
            });
            1.0
        }
        let pts: Vec<ffi::b2Vec2> = points.into_iter().map(Into::into).collect();
        if pts.is_empty() {
            return Vec::new();
        }
        let (s, c) = angle_radians.into().sin_cos();
        let proxy = unsafe {
            ffi::b2MakeOffsetProxy(
                pts.as_ptr(),
                pts.len() as i32,
                radius,
                position.into(),
                ffi::b2Rot { c, s },
            )
        };
        let mut out: Vec<RayResult> = Vec::new();
        unsafe {
            let _ = ffi::b2World_CastShape(
                self.raw(),
                &proxy,
                translation.into(),
                filter.0,
                Some(cb),
                &mut out as *mut _ as *mut _,
            );
        }
        out
    }
}
