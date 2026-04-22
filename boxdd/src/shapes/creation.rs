use super::*;

impl<'w> Body<'w> {
    pub fn create_circle_shape(&mut self, def: &ShapeDef, c: &Circle) -> Shape<'w> {
        create_body_attached_shape_handle(
            &self.core,
            self.id,
            def,
            c,
            create_circle_shape_for_body_impl,
            Shape::new,
        )
    }

    pub fn try_create_circle_shape(&mut self, def: &ShapeDef, c: &Circle) -> ApiResult<Shape<'w>> {
        try_create_body_attached_shape_handle(
            &self.core,
            self.id,
            def,
            c,
            try_create_circle_shape_for_body_impl,
            Shape::new,
        )
    }

    pub fn create_segment_shape(&mut self, def: &ShapeDef, s: &Segment) -> Shape<'w> {
        create_body_attached_shape_handle(
            &self.core,
            self.id,
            def,
            s,
            create_segment_shape_for_body_impl,
            Shape::new,
        )
    }

    pub fn try_create_segment_shape(
        &mut self,
        def: &ShapeDef,
        s: &Segment,
    ) -> ApiResult<Shape<'w>> {
        try_create_body_attached_shape_handle(
            &self.core,
            self.id,
            def,
            s,
            try_create_segment_shape_for_body_impl,
            Shape::new,
        )
    }

    pub fn create_capsule_shape(&mut self, def: &ShapeDef, c: &Capsule) -> Shape<'w> {
        create_body_attached_shape_handle(
            &self.core,
            self.id,
            def,
            c,
            create_capsule_shape_for_body_impl,
            Shape::new,
        )
    }

    pub fn try_create_capsule_shape(
        &mut self,
        def: &ShapeDef,
        c: &Capsule,
    ) -> ApiResult<Shape<'w>> {
        try_create_body_attached_shape_handle(
            &self.core,
            self.id,
            def,
            c,
            try_create_capsule_shape_for_body_impl,
            Shape::new,
        )
    }

    pub fn create_polygon_shape(&mut self, def: &ShapeDef, p: &Polygon) -> Shape<'w> {
        create_body_attached_shape_handle(
            &self.core,
            self.id,
            def,
            p,
            create_polygon_shape_for_body_impl,
            Shape::new,
        )
    }

    pub fn try_create_polygon_shape(
        &mut self,
        def: &ShapeDef,
        p: &Polygon,
    ) -> ApiResult<Shape<'w>> {
        try_create_body_attached_shape_handle(
            &self.core,
            self.id,
            def,
            p,
            try_create_polygon_shape_for_body_impl,
            Shape::new,
        )
    }

    // Convenience creators
    pub fn create_box(&mut self, def: &ShapeDef, half_w: f32, half_h: f32) -> Shape<'w> {
        create_body_attached_box_shape_handle(&self.core, self.id, def, half_w, half_h, Shape::new)
    }

    pub fn try_create_box(
        &mut self,
        def: &ShapeDef,
        half_w: f32,
        half_h: f32,
    ) -> ApiResult<Shape<'w>> {
        try_create_body_attached_box_shape_handle(
            &self.core,
            self.id,
            def,
            half_w,
            half_h,
            Shape::new,
        )
    }

    pub fn create_circle_simple(&mut self, def: &ShapeDef, radius: f32) -> Shape<'w> {
        create_body_attached_circle_simple_shape_handle(
            &self.core,
            self.id,
            def,
            radius,
            Shape::new,
        )
    }

    pub fn try_create_circle_simple(
        &mut self,
        def: &ShapeDef,
        radius: f32,
    ) -> ApiResult<Shape<'w>> {
        try_create_body_attached_circle_simple_shape_handle(
            &self.core,
            self.id,
            def,
            radius,
            Shape::new,
        )
    }

    pub fn create_segment_simple<V: Into<crate::types::Vec2>>(
        &mut self,
        def: &ShapeDef,
        p1: V,
        p2: V,
    ) -> Shape<'w> {
        create_body_attached_segment_simple_shape_handle(
            &self.core,
            self.id,
            def,
            p1,
            p2,
            Shape::new,
        )
    }

    pub fn try_create_segment_simple<V: Into<crate::types::Vec2>>(
        &mut self,
        def: &ShapeDef,
        p1: V,
        p2: V,
    ) -> ApiResult<Shape<'w>> {
        try_create_body_attached_segment_simple_shape_handle(
            &self.core,
            self.id,
            def,
            p1,
            p2,
            Shape::new,
        )
    }

    pub fn create_capsule_simple<V: Into<crate::types::Vec2>>(
        &mut self,
        def: &ShapeDef,
        c1: V,
        c2: V,
        radius: f32,
    ) -> Shape<'w> {
        create_body_attached_capsule_simple_shape_handle(
            &self.core,
            self.id,
            def,
            c1,
            c2,
            radius,
            Shape::new,
        )
    }

    pub fn try_create_capsule_simple<V: Into<crate::types::Vec2>>(
        &mut self,
        def: &ShapeDef,
        c1: V,
        c2: V,
        radius: f32,
    ) -> ApiResult<Shape<'w>> {
        try_create_body_attached_capsule_simple_shape_handle(
            &self.core,
            self.id,
            def,
            c1,
            c2,
            radius,
            Shape::new,
        )
    }

    pub fn create_polygon_from_points<I, P>(
        &mut self,
        def: &ShapeDef,
        points: I,
        radius: f32,
    ) -> Option<Shape<'w>>
    where
        I: IntoIterator<Item = P>,
        P: Into<crate::types::Vec2>,
    {
        create_body_attached_polygon_from_points_shape_handle(
            &self.core,
            self.id,
            def,
            points,
            radius,
            Shape::new,
        )
    }

    pub fn try_create_polygon_from_points<I, P>(
        &mut self,
        def: &ShapeDef,
        points: I,
        radius: f32,
    ) -> ApiResult<Shape<'w>>
    where
        I: IntoIterator<Item = P>,
        P: Into<crate::types::Vec2>,
    {
        try_create_body_attached_polygon_from_points_shape_handle(
            &self.core,
            self.id,
            def,
            points,
            radius,
            Shape::new,
        )
    }
}

impl OwnedBody {
    pub fn create_circle_shape(&mut self, def: &ShapeDef, c: &Circle) -> OwnedShape {
        create_body_attached_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            c,
            create_circle_shape_for_body_impl,
            OwnedShape::new,
        )
    }

    pub fn try_create_circle_shape(&mut self, def: &ShapeDef, c: &Circle) -> ApiResult<OwnedShape> {
        try_create_body_attached_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            c,
            try_create_circle_shape_for_body_impl,
            OwnedShape::new,
        )
    }

    pub fn create_segment_shape(&mut self, def: &ShapeDef, s: &Segment) -> OwnedShape {
        create_body_attached_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            s,
            create_segment_shape_for_body_impl,
            OwnedShape::new,
        )
    }

    pub fn try_create_segment_shape(
        &mut self,
        def: &ShapeDef,
        s: &Segment,
    ) -> ApiResult<OwnedShape> {
        try_create_body_attached_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            s,
            try_create_segment_shape_for_body_impl,
            OwnedShape::new,
        )
    }

    pub fn create_capsule_shape(&mut self, def: &ShapeDef, c: &Capsule) -> OwnedShape {
        create_body_attached_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            c,
            create_capsule_shape_for_body_impl,
            OwnedShape::new,
        )
    }

    pub fn try_create_capsule_shape(
        &mut self,
        def: &ShapeDef,
        c: &Capsule,
    ) -> ApiResult<OwnedShape> {
        try_create_body_attached_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            c,
            try_create_capsule_shape_for_body_impl,
            OwnedShape::new,
        )
    }

    pub fn create_polygon_shape(&mut self, def: &ShapeDef, p: &Polygon) -> OwnedShape {
        create_body_attached_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            p,
            create_polygon_shape_for_body_impl,
            OwnedShape::new,
        )
    }

    pub fn try_create_polygon_shape(
        &mut self,
        def: &ShapeDef,
        p: &Polygon,
    ) -> ApiResult<OwnedShape> {
        try_create_body_attached_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            p,
            try_create_polygon_shape_for_body_impl,
            OwnedShape::new,
        )
    }

    // Convenience creators
    pub fn create_box(&mut self, def: &ShapeDef, half_w: f32, half_h: f32) -> OwnedShape {
        create_body_attached_box_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            half_w,
            half_h,
            OwnedShape::new,
        )
    }

    pub fn try_create_box(
        &mut self,
        def: &ShapeDef,
        half_w: f32,
        half_h: f32,
    ) -> ApiResult<OwnedShape> {
        try_create_body_attached_box_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            half_w,
            half_h,
            OwnedShape::new,
        )
    }

    pub fn create_circle_simple(&mut self, def: &ShapeDef, radius: f32) -> OwnedShape {
        create_body_attached_circle_simple_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            radius,
            OwnedShape::new,
        )
    }

    pub fn try_create_circle_simple(
        &mut self,
        def: &ShapeDef,
        radius: f32,
    ) -> ApiResult<OwnedShape> {
        try_create_body_attached_circle_simple_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            radius,
            OwnedShape::new,
        )
    }

    pub fn create_segment_simple<V: Into<crate::types::Vec2>>(
        &mut self,
        def: &ShapeDef,
        p1: V,
        p2: V,
    ) -> OwnedShape {
        create_body_attached_segment_simple_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            p1,
            p2,
            OwnedShape::new,
        )
    }

    pub fn try_create_segment_simple<V: Into<crate::types::Vec2>>(
        &mut self,
        def: &ShapeDef,
        p1: V,
        p2: V,
    ) -> ApiResult<OwnedShape> {
        try_create_body_attached_segment_simple_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            p1,
            p2,
            OwnedShape::new,
        )
    }

    pub fn create_capsule_simple<V: Into<crate::types::Vec2>>(
        &mut self,
        def: &ShapeDef,
        c1: V,
        c2: V,
        radius: f32,
    ) -> OwnedShape {
        create_body_attached_capsule_simple_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            c1,
            c2,
            radius,
            OwnedShape::new,
        )
    }

    pub fn try_create_capsule_simple<V: Into<crate::types::Vec2>>(
        &mut self,
        def: &ShapeDef,
        c1: V,
        c2: V,
        radius: f32,
    ) -> ApiResult<OwnedShape> {
        try_create_body_attached_capsule_simple_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            c1,
            c2,
            radius,
            OwnedShape::new,
        )
    }

    pub fn create_polygon_from_points<I, P>(
        &mut self,
        def: &ShapeDef,
        points: I,
        radius: f32,
    ) -> Option<OwnedShape>
    where
        I: IntoIterator<Item = P>,
        P: Into<crate::types::Vec2>,
    {
        create_body_attached_polygon_from_points_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            points,
            radius,
            OwnedShape::new,
        )
    }

    pub fn try_create_polygon_from_points<I, P>(
        &mut self,
        def: &ShapeDef,
        points: I,
        radius: f32,
    ) -> ApiResult<OwnedShape>
    where
        I: IntoIterator<Item = P>,
        P: Into<crate::types::Vec2>,
    {
        try_create_body_attached_polygon_from_points_shape_handle(
            &self.core_arc(),
            self.id(),
            def,
            points,
            radius,
            OwnedShape::new,
        )
    }
}
// Shapes: module note moved to top-level doc above.
