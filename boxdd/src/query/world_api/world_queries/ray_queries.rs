use super::*;

impl World {
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
        cast_ray_closest_checked_impl(self.raw(), origin, translation, filter)
    }

    pub fn try_cast_ray_closest<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<RayResult> {
        try_cast_ray_closest_impl(self.raw(), origin, translation, filter)
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
        cast_ray_all_checked_impl(self.raw(), origin, translation, filter)
    }

    /// Cast a ray and append all hits into `out`, reusing the caller-owned allocation.
    pub fn cast_ray_all_into<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) {
        cast_ray_all_into_checked_impl(self.raw(), origin, translation, filter, out);
    }

    pub fn try_cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>> {
        try_cast_ray_all_impl(self.raw(), origin, translation, filter)
    }

    pub fn try_cast_ray_all_into<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> ApiResult<()> {
        try_cast_ray_all_into_impl(self.raw(), origin, translation, filter, out)
    }
}
