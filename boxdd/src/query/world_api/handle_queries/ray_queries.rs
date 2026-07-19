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
    ) -> RayResult {
        cast_ray_closest_checked_impl(self.raw(), origin, translation, filter)
    }

    pub fn try_cast_ray_closest<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<RayResult> {
        try_cast_ray_closest_impl(self.raw(), origin, translation, filter)
    }

    /// Cast a ray and collect hits using the same coordinate contract as
    /// [`Self::cast_ray_closest`].
    pub fn cast_ray_all<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult> {
        cast_ray_all_checked_impl(self.raw(), origin, translation, filter)
    }

    pub fn cast_ray_all_into<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) {
        cast_ray_all_into_checked_impl(self.raw(), origin, translation, filter, out);
    }

    pub fn try_cast_ray_all<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>> {
        try_cast_ray_all_impl(self.raw(), origin, translation, filter)
    }

    pub fn try_cast_ray_all_into<VT: Into<Vec2>>(
        &self,
        origin: Position,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> ApiResult<()> {
        try_cast_ray_all_into_impl(self.raw(), origin, translation, filter, out)
    }
}
