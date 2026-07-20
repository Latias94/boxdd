use super::*;

impl World {
    /// Cast a ray and return the closest hit.
    ///
    /// `origin` is an absolute world position, while `translation` is a local
    /// displacement. A hit's [`RayResult::point`] is an absolute world position.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{Position, QueryFilter, Vec2, World, WorldDef};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let hit = world.cast_ray_closest(Position::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
    /// if let Some(hit) = hit { /* use hit.point / hit.normal */ }
    /// ```
    pub fn cast_ray_closest<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> Option<RayResult> {
        cast_ray_closest_checked_impl(self.query_target(), origin, translation, filter)
    }

    pub fn try_cast_ray_closest<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Option<RayResult>> {
        try_cast_ray_closest_impl(self.query_target(), origin, translation, filter)
    }

    /// Cast a ray and collect all hits along the path.
    ///
    /// `origin` is an absolute world position, while `translation` is a local
    /// displacement. Every returned hit point is an absolute world position.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{Position, QueryFilter, Vec2, World, WorldDef};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let hits = world.cast_ray_all(Position::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
    /// for h in hits { let _ = (h.point, h.normal, h.fraction); }
    /// ```
    pub fn cast_ray_all<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult> {
        cast_ray_all_checked_impl(self.query_target(), origin, translation, filter)
    }

    /// Cast a ray and append all hits into `out`, reusing the caller-owned allocation.
    pub fn cast_ray_all_into<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) {
        cast_ray_all_into_checked_impl(self.query_target(), origin, translation, filter, out);
    }

    pub fn try_cast_ray_all<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>> {
        try_cast_ray_all_impl(self.query_target(), origin, translation, filter)
    }

    pub fn try_cast_ray_all_into<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> ApiResult<()> {
        try_cast_ray_all_into_impl(self.query_target(), origin, translation, filter, out)
    }
}
