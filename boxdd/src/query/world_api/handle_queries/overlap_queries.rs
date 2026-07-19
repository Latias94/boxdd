#![allow(
    clippy::too_many_arguments,
    reason = "query geometry and its absolute world origin are deliberately explicit"
)]

use super::*;

impl WorldHandle {
    /// Query an AABB whose bounds are local to the absolute world `origin`.
    pub fn overlap_aabb(&self, origin: Position, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
        overlap_aabb_checked_impl(self.raw(), origin, aabb, filter)
    }

    pub fn overlap_aabb_into(
        &self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) {
        overlap_aabb_into_checked_impl(self.raw(), origin, aabb, filter, out);
    }

    pub fn visit_overlap_aabb<F>(
        &self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
        mut visit: F,
    ) -> bool
    where
        F: FnMut(ShapeId) -> bool,
    {
        visit_overlap_aabb_checked_impl(self.raw(), origin, aabb, filter, &mut visit)
    }

    pub fn try_overlap_aabb(
        &self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
    ) -> ApiResult<Vec<ShapeId>> {
        try_overlap_aabb_impl(self.raw(), origin, aabb, filter)
    }

    pub fn try_overlap_aabb_into(
        &self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()> {
        try_overlap_aabb_into_impl(self.raw(), origin, aabb, filter, out)
    }

    pub fn try_visit_overlap_aabb<F>(
        &self,
        origin: Position,
        aabb: Aabb,
        filter: QueryFilter,
        mut visit: F,
    ) -> ApiResult<bool>
    where
        F: FnMut(ShapeId) -> bool,
    {
        try_visit_overlap_aabb_impl(self.raw(), origin, aabb, filter, &mut visit)
    }

    /// Query a proxy whose points are local to the absolute world `origin`.
    pub fn overlap_polygon_points<I, P>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<ShapeId>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        overlap_polygon_points_checked_impl(self.raw(), origin, points, radius, filter)
    }

    pub fn overlap_polygon_points_into<I, P>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        overlap_polygon_points_into_checked_impl(self.raw(), origin, points, radius, filter, out);
    }

    pub fn visit_overlap_polygon_points<I, P, F>(
        &self,
        origin: Position,
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
        visit_overlap_polygon_points_checked_impl(
            self.raw(),
            origin,
            points,
            radius,
            filter,
            &mut visit,
        )
    }

    pub fn try_overlap_polygon_points<I, P>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        filter: QueryFilter,
    ) -> ApiResult<Vec<ShapeId>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        try_overlap_polygon_points_impl(self.raw(), origin, points, radius, filter)
    }

    pub fn try_overlap_polygon_points_into<I, P>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        filter: QueryFilter,
        out: &mut Vec<ShapeId>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        try_overlap_polygon_points_into_impl(self.raw(), origin, points, radius, filter, out)
    }

    pub fn try_visit_overlap_polygon_points<I, P, F>(
        &self,
        origin: Position,
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
        try_visit_overlap_polygon_points_impl(
            self.raw(),
            origin,
            points,
            radius,
            filter,
            &mut visit,
        )
    }

    /// Query a proxy with points and an offset position local to the absolute
    /// world `origin`.
    pub fn overlap_polygon_points_with_offset<I, P, V, A>(
        &self,
        origin: Position,
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
            origin,
            points,
            radius,
            position,
            angle_radians,
            filter,
        )
    }

    pub fn overlap_polygon_points_with_offset_into<I, P, V, A>(
        &self,
        origin: Position,
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
            origin,
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
        origin: Position,
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
            origin,
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
        origin: Position,
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
            origin,
            points,
            radius,
            position,
            angle_radians,
            filter,
        )
    }

    pub fn try_overlap_polygon_points_with_offset_into<I, P, V, A>(
        &self,
        origin: Position,
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
            origin,
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
        origin: Position,
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
            origin,
            points,
            radius,
            position,
            angle_radians,
            filter,
            &mut visit,
        )
    }
}
