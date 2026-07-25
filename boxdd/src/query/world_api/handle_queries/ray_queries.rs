use super::*;

impl WorldHandle {
    /// Cast a ray from an absolute `origin` by a local `translation`.
    ///
    /// The returned hit point is an absolute world position.
    pub fn cast_ray_closest<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> Option<RayResult> {
        self.cast_ray_closest_with_stats(origin, translation, filter)
            .hit
    }

    pub fn try_cast_ray_closest<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Option<RayResult>> {
        self.try_cast_ray_closest_with_stats(origin, translation, filter)
            .map(|result| result.hit)
    }

    /// Cast a ray and return the closest hit together with traversal statistics.
    ///
    /// Statistics remain available when the ray misses every shape.
    pub fn cast_ray_closest_with_stats<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ClosestRayCastResult {
        cast_ray_closest_with_stats_checked_impl(self.query_target(), origin, translation, filter)
    }

    pub fn try_cast_ray_closest_with_stats<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<ClosestRayCastResult> {
        try_cast_ray_closest_with_stats_impl(self.query_target(), origin, translation, filter)
    }

    /// Cast a ray and collect hits using the same coordinate contract as
    /// [`Self::cast_ray_closest`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn cast_ray_all<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult> {
        cast_ray_all_checked_impl(self.query_target(), origin, translation, filter)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn cast_ray_all_into<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) {
        cast_ray_all_into_checked_impl(self.query_target(), origin, translation, filter, out);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_cast_ray_all<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>> {
        try_cast_ray_all_impl(self.query_target(), origin, translation, filter)
    }

    #[cfg(not(target_arch = "wasm32"))]
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
