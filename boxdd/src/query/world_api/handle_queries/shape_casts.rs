#![allow(
    clippy::too_many_arguments,
    reason = "query geometry and its absolute world origin are deliberately explicit"
)]

use super::*;

impl WorldHandle {
    /// Cast a proxy whose points and translation are local to the absolute
    /// world `origin`. Returned hit points are absolute world positions.
    pub fn cast_shape_points<I, P, VT>(
        &self,
        origin: Position,
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
        cast_shape_points_checked_impl(
            self.query_target(),
            origin,
            points,
            radius,
            translation,
            filter,
        )
    }

    pub fn cast_shape_points_into<I, P, VT>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        cast_shape_points_into_checked_impl(
            self.query_target(),
            origin,
            points,
            radius,
            translation,
            filter,
            out,
        );
    }

    pub fn try_cast_shape_points<I, P, VT>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_impl(
            self.query_target(),
            origin,
            points,
            radius,
            translation,
            filter,
        )
    }

    pub fn try_cast_shape_points_into<I, P, VT>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_into_impl(
            self.query_target(),
            origin,
            points,
            radius,
            translation,
            filter,
            out,
        )
    }

    /// Cast an offset proxy using the same coordinate contract as
    /// [`Self::cast_shape_points`]; `position` is also local to `origin`.
    pub fn cast_shape_points_with_offset<I, P, V, A, VT>(
        &self,
        origin: Position,
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
        cast_shape_points_with_offset_checked_impl(
            self.query_target(),
            origin,
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn cast_shape_points_with_offset_into<I, P, V, A, VT>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        cast_shape_points_with_offset_into_checked_impl(
            self.query_target(),
            origin,
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
            out,
        );
    }

    pub fn try_cast_shape_points_with_offset<I, P, V, A, VT>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
    ) -> ApiResult<Vec<RayResult>>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_with_offset_impl(
            self.query_target(),
            origin,
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_cast_shape_points_with_offset_into<I, P, V, A, VT>(
        &self,
        origin: Position,
        points: I,
        radius: f32,
        position: V,
        angle_radians: A,
        translation: VT,
        filter: QueryFilter,
        out: &mut Vec<RayResult>,
    ) -> ApiResult<()>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        try_cast_shape_points_with_offset_into_impl(
            self.query_target(),
            origin,
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
            out,
        )
    }
}
