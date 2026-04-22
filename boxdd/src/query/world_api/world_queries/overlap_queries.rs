use super::*;

impl World {
    /// Overlap test for all shapes in an AABB. Returns matching shape ids.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, BodyBuilder, ShapeDef, shapes, Vec2, Aabb, QueryFilter};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let b = world.create_body_id(BodyBuilder::new().position([0.0, 2.0]).build());
    /// let sdef = ShapeDef::builder().density(1.0).build();
    /// world.create_polygon_shape_for(b, &sdef, &shapes::box_polygon(0.5, 0.5));
    /// let hits = world.overlap_aabb(Aabb { lower: Vec2::new(-1.0, -1.0), upper: Vec2::new(1.0, 3.0) }, QueryFilter::default());
    /// assert!(!hits.is_empty());
    /// ```
    pub fn overlap_aabb(&self, aabb: Aabb, filter: QueryFilter) -> Vec<ShapeId> {
        overlap_aabb_checked_impl(self.raw(), aabb, filter)
    }

    /// Overlap test for all shapes in an AABB and write matching shape ids into `out`.
    ///
    /// `out` is cleared before new hits are appended so its allocation can be reused across frames.
    pub fn overlap_aabb_into(&self, aabb: Aabb, filter: QueryFilter, out: &mut Vec<ShapeId>) {
        overlap_aabb_into_checked_impl(self.raw(), aabb, filter, out);
    }

    /// Visit matching shape ids in an AABB without allocating a result container.
    ///
    /// Return `true` from the visitor to continue, or `false` to stop early.
    /// Returns `true` if all hits were visited, or `false` if the visitor stopped early.
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

    /// Overlap polygon points (creates a temporary shape proxy from given points + radius) and collect all shape ids.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let square = [Vec2::new(-0.5, -0.5), Vec2::new(0.5, -0.5), Vec2::new(0.5, 0.5), Vec2::new(-0.5, 0.5)];
    /// let hits = world.overlap_polygon_points(square, 0.0, QueryFilter::default());
    /// let _ = hits;
    /// ```
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

    /// Overlap a temporary polygon proxy and write matching shape ids into `out`.
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

    /// Visit matching shape ids for a temporary polygon proxy without allocating a result container.
    ///
    /// Return `true` from the visitor to continue, or `false` to stop early.
    /// Returns `true` if all hits were visited, or `false` if the visitor stopped early.
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

    /// Overlap polygon points with an offset transform.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let rect = [Vec2::new(-0.5, -0.25), Vec2::new(0.5, -0.25), Vec2::new(0.5, 0.25), Vec2::new(-0.5, 0.25)];
    /// let hits = world.overlap_polygon_points_with_offset(rect, 0.0, Vec2::new(0.0, 2.0), 0.0_f32, QueryFilter::default());
    /// let _ = hits;
    /// ```
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

    /// Overlap an offset polygon proxy and write matching shape ids into `out`.
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

    /// Visit matching shape ids for an offset temporary polygon proxy without allocating a result container.
    ///
    /// Return `true` from the visitor to continue, or `false` to stop early.
    /// Returns `true` if all hits were visited, or `false` if the visitor stopped early.
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
