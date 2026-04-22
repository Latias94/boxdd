use super::*;

impl WorldHandle {
    pub fn cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> f32 {
        cast_mover_checked_impl(self.raw(), c1, c2, radius, translation, filter)
    }

    pub fn try_cast_mover<V1: Into<Vec2>, V2: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<f32> {
        try_cast_mover_impl(self.raw(), c1, c2, radius, translation, filter)
    }

    pub fn collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<MoverPlaneResult> {
        collide_mover_checked_impl(self.raw(), c1, c2, radius, filter)
    }

    pub fn collide_mover_into<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<MoverPlaneResult>,
    ) {
        collide_mover_into_checked_impl(self.raw(), c1, c2, radius, filter, out);
    }

    pub fn try_collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> ApiResult<Vec<MoverPlaneResult>> {
        try_collide_mover_impl(self.raw(), c1, c2, radius, filter)
    }

    pub fn try_collide_mover_into<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<MoverPlaneResult>,
    ) -> ApiResult<()> {
        try_collide_mover_into_impl(self.raw(), c1, c2, radius, filter, out)
    }
}
