use super::*;

impl Body<'_> {
    /// Create a circle attached to this body and return its storage id.
    pub fn create_circle(&mut self, def: &ShapeDef, circle: &Circle) -> Result<ShapeId> {
        let (creation, body) = self.proof.begin_creation()?;
        create_circle_shape_for_body(creation, body, def, circle)
    }

    /// Create a segment attached to this body and return its storage id.
    pub fn create_segment(&mut self, def: &ShapeDef, segment: &Segment) -> Result<ShapeId> {
        let (creation, body) = self.proof.begin_creation()?;
        create_segment_shape_for_body(creation, body, def, segment)
    }

    /// Create an orphan chain segment attached to this body.
    pub fn create_chain_segment(
        &mut self,
        def: &ShapeDef,
        segment: &ChainSegment,
    ) -> Result<ShapeId> {
        let (creation, body) = self.proof.begin_creation()?;
        create_chain_segment_shape_for_body(creation, body, def, segment)
    }

    /// Create a capsule attached to this body and return its storage id.
    pub fn create_capsule(&mut self, def: &ShapeDef, capsule: &Capsule) -> Result<ShapeId> {
        let (creation, body) = self.proof.begin_creation()?;
        create_capsule_shape_for_body(creation, body, def, capsule)
    }

    /// Create a polygon attached to this body and return its storage id.
    pub fn create_polygon(&mut self, def: &ShapeDef, polygon: &Polygon) -> Result<ShapeId> {
        let (creation, body) = self.proof.begin_creation()?;
        create_polygon_shape_for_body(creation, body, def, polygon)
    }

    pub fn create_box(
        &mut self,
        def: &ShapeDef,
        half_width: f32,
        half_height: f32,
    ) -> Result<ShapeId> {
        let (creation, body) = self.proof.begin_creation()?;
        let polygon = match Polygon::box_polygon(half_width, half_height) {
            Ok(polygon) => polygon,
            Err(error) => return creation.abort(error),
        };
        create_polygon_shape_for_body(creation, body, def, &polygon)
    }

    pub fn create_centered_circle(&mut self, def: &ShapeDef, radius: f32) -> Result<ShapeId> {
        let (creation, body) = self.proof.begin_creation()?;
        let circle = match Circle::new([0.0_f32, 0.0], radius) {
            Ok(circle) => circle,
            Err(error) => return creation.abort(error),
        };
        create_circle_shape_for_body(creation, body, def, &circle)
    }

    pub fn create_segment_between<P1: Into<Vec2>, P2: Into<Vec2>>(
        &mut self,
        def: &ShapeDef,
        point1: P1,
        point2: P2,
    ) -> Result<ShapeId> {
        self.proof.run_creation(move |creation, body| {
            let segment = match Segment::new(point1, point2) {
                Ok(segment) => segment,
                Err(error) => return creation.abort(error),
            };
            create_segment_shape_for_body(creation, body, def, &segment)
        })
    }

    pub fn create_capsule_between<C1: Into<Vec2>, C2: Into<Vec2>>(
        &mut self,
        def: &ShapeDef,
        center1: C1,
        center2: C2,
        radius: f32,
    ) -> Result<ShapeId> {
        self.proof.run_creation(move |creation, body| {
            let capsule = match Capsule::new(center1, center2, radius) {
                Ok(capsule) => capsule,
                Err(error) => return creation.abort(error),
            };
            create_capsule_shape_for_body(creation, body, def, &capsule)
        })
    }

    pub fn create_polygon_from_points<I, P>(
        &mut self,
        def: &ShapeDef,
        points: I,
        radius: f32,
    ) -> Result<ShapeId>
    where
        I: IntoIterator<Item = P>,
        P: Into<Vec2>,
    {
        self.proof.run_creation(move |creation, body| {
            let polygon = match Polygon::from_points(points, radius) {
                Ok(polygon) => polygon,
                Err(error) => return creation.abort(error),
            };
            create_polygon_shape_for_body(creation, body, def, &polygon)
        })
    }
}
