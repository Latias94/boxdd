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

    /// Cast a ray and return the closest hit.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let hit = world.cast_ray_closest(Vec2::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
    /// if hit.hit { /* use hit.point / hit.normal */ }
    /// ```
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

    /// Cast a ray and collect all hits along the path.
    ///
    /// Example
    /// ```no_run
    /// use boxdd::{World, WorldDef, QueryFilter, Vec2};
    /// let mut world = World::new(WorldDef::builder().gravity([0.0,-9.8]).build()).unwrap();
    /// let hits = world.cast_ray_all(Vec2::new(0.0, 5.0), Vec2::new(0.0, -10.0), QueryFilter::default());
    /// for h in hits { let _ = (h.point, h.normal, h.fraction); }
    /// ```
    pub fn cast_ray_all<VO: Into<Vec2>, VT: Into<Vec2>>(
        &self,
        origin: VO,
        translation: VT,
        filter: QueryFilter,
    ) -> Vec<RayResult> {
        cast_ray_all_checked_impl(self.raw(), origin, translation, filter)
    }

    /// Cast a ray and append all hits into `out`, reusing the caller-owned allocation.
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

    /// Cast a capsule mover and return remaining fraction (1.0 = free, < 1.0 = hit earlier).
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

    /// Collect collision planes for a capsule mover at its current position.
    pub fn collide_mover<V1: Into<Vec2>, V2: Into<Vec2>>(
        &self,
        c1: V1,
        c2: V2,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<MoverPlaneResult> {
        collide_mover_checked_impl(self.raw(), c1, c2, radius, filter)
    }

    /// Collect collision planes for a capsule mover and reuse `out`.
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
