use super::*;

impl Segment {
    #[inline]
    pub fn new<P1: Into<Vec2>, P2: Into<Vec2>>(point1: P1, point2: P2) -> Result<Self> {
        let segment = Self {
            point1: point1.into(),
            point2: point2.into(),
        };
        check_segment_geometry_valid_for_operation("Segment::new", segment)?;
        Ok(segment)
    }

    #[inline]
    /// Construct from a raw Box2D geometry value after validating its invariants.
    pub fn from_raw(raw: ffi::b2Segment) -> Result<Self> {
        let segment = Self {
            point1: Vec2::from_raw(raw.point1),
            point2: Vec2::from_raw(raw.point2),
        };
        check_segment_geometry_valid_for_operation("Segment::from_raw", segment)?;
        Ok(segment)
    }

    #[inline]
    pub const fn point1(self) -> Vec2 {
        self.point1
    }

    #[inline]
    pub const fn point2(self) -> Vec2 {
        self.point2
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2Segment {
        ffi::b2Segment {
            point1: self.point1.into_raw(),
            point2: self.point2.into_raw(),
        }
    }

    #[inline]
    /// Validate this segment for safe Box2D shape and standalone collision use.
    pub fn is_valid(self) -> bool {
        segment_geometry_is_valid(self)
    }

    #[inline]
    /// Validate this segment for safe Box2D shape and standalone collision use.
    pub fn validate(self) -> Result<()> {
        check_segment_geometry_valid_for_operation("Segment::validate", self)
    }

    /// Compute an absolute world-space AABB using `transform` as the segment's
    /// local-to-world transform.
    ///
    /// The result uses `f32` coordinates in both precision modes. Double-precision world bounds
    /// are narrowed outward by Box2D so the returned AABB remains conservative.
    #[inline]
    pub fn aabb(self, transform: WorldTransform) -> Result<Aabb> {
        check_segment_helper_geometry_valid("Segment::aabb", self)?;
        check_world_transform_valid("Segment::aabb", transform)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        check_native_geometry_aabb("Segment::aabb", unsafe {
            ffi::b2ComputeSegmentAABB(&raw, transform.into_raw())
        })
    }

    #[inline]
    pub fn ray_cast<VO: Into<Vec2>, VT: Into<Vec2>>(
        self,
        origin: VO,
        translation: VT,
        one_sided: bool,
    ) -> Result<CastOutput> {
        let input = materialize_ray_input(origin, translation);
        check_segment_helper_geometry_valid("Segment::ray_cast", self)?;
        check_ray_input_valid("Segment::ray_cast", &input)?;
        let raw = self.into_raw();
        let _lease = transient_native_lease()?;
        CastOutput::from_native("Segment::ray_cast", unsafe {
            ffi::b2RayCastSegment(&raw, &input, one_sided)
        })
    }

    #[inline]
    pub fn shape_cast(self, input: ShapeCastInput) -> Result<CastOutput> {
        check_segment_helper_geometry_valid("Segment::shape_cast", self)?;
        input.validate()?;
        let raw = self.into_raw();
        let input = input.into_raw();
        let _lease = transient_native_lease()?;
        CastOutput::from_native("Segment::shape_cast", unsafe {
            ffi::b2ShapeCastSegment(&raw, &input)
        })
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Segment {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            point1: Vec2,
            point2: Vec2,
        }

        let repr = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(repr.point1, repr.point2).map_err(serde::de::Error::custom)
    }
}
