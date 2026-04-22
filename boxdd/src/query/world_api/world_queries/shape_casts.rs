use super::*;

impl World {
    /// Cast a polygon proxy and collect hits. Returns all intersections with fraction and contact info.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let tri = [Vec2::new(0.0, 0.0), Vec2::new(0.5, 0.0), Vec2::new(0.25, 0.5)];
    /// let hits = world.cast_shape_points(tri, 0.0, Vec2::new(0.0, -1.0), QueryFilter::default());
    /// for h in hits { let _ = (h.point, h.normal, h.fraction); }
    /// ```
    pub fn cast_shape_points<I, P, VT>(
        &self,
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
        cast_shape_points_checked_impl(self.raw(), points, radius, translation, filter)
    }

    /// Cast a temporary polygon proxy and write all hits into `out`.
    pub fn cast_shape_points_into<I, P, VT>(
        &self,
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
        cast_shape_points_into_checked_impl(self.raw(), points, radius, translation, filter, out);
    }

    pub fn try_cast_shape_points<I, P, VT>(
        &self,
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
        try_cast_shape_points_impl(self.raw(), points, radius, translation, filter)
    }

    pub fn try_cast_shape_points_into<I, P, VT>(
        &self,
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
        try_cast_shape_points_into_impl(self.raw(), points, radius, translation, filter, out)
    }

    /// Cast polygon points with an offset transform (position + angle).
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let rect = [Vec2::new(-0.5, -0.25), Vec2::new(0.5, -0.25), Vec2::new(0.5, 0.25), Vec2::new(-0.5, 0.25)];
    /// let hits = world.cast_shape_points_with_offset(rect, 0.0, Vec2::new(0.0, 2.0), 0.0_f32, Vec2::new(0.0, -1.0), QueryFilter::default());
    /// for h in hits { let _ = (h.point, h.normal, h.fraction); }
    /// ```
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
        P: Into<Vec2>,
        V: Into<Vec2>,
        A: Into<f32>,
        VT: Into<Vec2>,
    {
        cast_shape_points_with_offset_checked_impl(
            self.raw(),
            points,
            radius,
            position,
            angle_radians,
            translation,
            filter,
        )
    }

    /// Cast an offset polygon proxy and write all hits into `out`.
    #[allow(clippy::too_many_arguments)]
    pub fn cast_shape_points_with_offset_into<I, P, V, A, VT>(
        &self,
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
            self.raw(),
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
            self.raw(),
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
            self.raw(),
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
