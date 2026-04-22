use super::*;

impl WorldHandle {
    pub fn overlap_aabb(&self, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
        overlap_aabb_checked_impl(self.raw(), aabb, filter)
    }

    pub fn overlap_aabb_into(&self, aabb: Aabb, filter: QueryFilter, out: &mut Vec<ShapeId>) {
        overlap_aabb_into_checked_impl(self.raw(), aabb, filter, out);
    }

    pub fn visit_overlap_aabb<F>(&self, aabb: Aabb, filter: QueryFilter, mut visit: F) -> bool
    where
        F: FnMut(ShapeId) -> bool,
    {
        visit_overlap_aabb_checked_impl(self.raw(), aabb, filter, &mut visit)
    }

    pub fn try_overlap_aabb(&self, aabb: Aabb, filter: QueryFilter) -> ApiResult<Vec<ShapeId>> {
        try_overlap_aabb_impl(self.raw(), aabb, filter)
    }

    pub fn try_overlap_aabb_into(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()> {
        try_overlap_aabb_into_impl(self.raw(), aabb, filter, out)
    }

    pub fn try_visit_overlap_aabb<F>(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        mut visit: F,
    ) -> ApiResult<bool>
    where
        F: FnMut(ShapeId) -> bool,
    {
        try_visit_overlap_aabb_impl(self.raw(), aabb, filter, &mut visit)
    }

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
        overlap_polygon_points_checked_impl(self.raw(), points, radius, filter)
    }

    pub fn overlap_polygon_points_into<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        overlap_polygon_points_into_checked_impl(self.raw(), points, radius, filter, out);
    }

    pub fn visit_overlap_polygon_points<I, P, F>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        mut visit: F,
    ) -> bool
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        F: FnMut(ShapeId) -> bool,
    {
        visit_overlap_polygon_points_checked_impl(self.raw(), points, radius, filter, &mut visit)
    }

    pub fn try_overlap_polygon_points<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        try_overlap_polygon_points_impl(self.raw(), points, radius, filter)
    }

    pub fn try_overlap_polygon_points_into<I, P>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        try_overlap_polygon_points_into_impl(self.raw(), points, radius, filter, out)
    }

    pub fn try_visit_overlap_polygon_points<I, P, F>(
        &self,
        points: I,
        radius: f32,
        filter: QueryFilter,
        mut visit: F,
    ) -> ApiResult<bool>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        F: FnMut(ShapeId) -> bool,
    {
        try_visit_overlap_polygon_points_impl(self.raw(), points, radius, filter, &mut visit)
    }

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
        overlap_polygon_points_with_offset_checked_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
        )
    }

    pub fn overlap_polygon_points_with_offset_into<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        overlap_polygon_points_with_offset_into_checked_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        );
    }

    pub fn visit_overlap_polygon_points_with_offset<I, P, V, A, F>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        mut visit: F,
    ) -> bool
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        F: FnMut(ShapeId) -> bool,
    {
        visit_overlap_polygon_points_with_offset_checked_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            &mut visit,
        )
    }

    pub fn try_overlap_polygon_points_with_offset<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
    ) -> ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        try_overlap_polygon_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
        )
    }

    pub fn try_overlap_polygon_points_with_offset_into<I, P, V, A>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
    {
        try_overlap_polygon_points_with_offset_into_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            out,
        )
    }

    pub fn try_visit_overlap_polygon_points_with_offset<I, P, V, A, F>(
        &self,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        filter: QueryFilter,
        mut visit: F,
    ) -> ApiResult<bool>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        F: FnMut(ShapeId) -> bool,
    {
        try_visit_overlap_polygon_points_with_offset_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            filter,
            &mut visit,
        )
    }
}
