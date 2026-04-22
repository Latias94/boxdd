use super::*;

impl WorldHandle {
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

    pub fn cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult> {
        cast_ray_all_checked_impl(self.raw(), origin, translation, filter)
    }

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
