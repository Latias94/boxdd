use super::*;

impl ChainSegment {
    #[inline]
    pub fn new<G1, P1, P2, G2>(ghost1: G1, point1: P1, point2: P2, ghost2: G2) -> Result<Self>
    where
        G1: Into<Vec2>,
        P1: Into<Vec2>,
        P2: Into<Vec2>,
        G2: Into<Vec2>,
    {
        let segment = Self {
            ghost1: ghost1.into(),
            segment: Segment {
                point1: point1.into(),
                point2: point2.into(),
            },
            ghost2: ghost2.into(),
        };
        check_chain_segment_geometry_valid_for_operation("ChainSegment::new", segment)?;
        Ok(segment)
    }

    #[inline]
    pub fn from_segment<G1: Into<Vec2>, G2: Into<Vec2>>(
        ghost1: G1,
        segment: Segment,
        ghost2: G2,
    ) -> Result<Self> {
        let segment = Self {
            ghost1: ghost1.into(),
            segment,
            ghost2: ghost2.into(),
        };
        check_chain_segment_geometry_valid_for_operation("ChainSegment::from_segment", segment)?;
        Ok(segment)
    }

    #[inline]
    /// Construct from a raw Box2D geometry value after validating its invariants.
    pub fn from_raw(raw: ffi::b2ChainSegment) -> Result<Self> {
        let segment = Self {
            ghost1: Vec2::from_raw(raw.ghost1),
            segment: Segment {
                point1: Vec2::from_raw(raw.segment.point1),
                point2: Vec2::from_raw(raw.segment.point2),
            },
            ghost2: Vec2::from_raw(raw.ghost2),
        };
        check_chain_segment_geometry_valid_for_operation("ChainSegment::from_raw", segment)?;
        Ok(segment)
    }

    #[inline]
    pub const fn ghost1(self) -> Vec2 {
        self.ghost1
    }

    #[inline]
    pub const fn segment(self) -> Segment {
        self.segment
    }

    #[inline]
    pub const fn ghost2(self) -> Vec2 {
        self.ghost2
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2ChainSegment {
        ffi::b2ChainSegment {
            ghost1: self.ghost1.into_raw(),
            segment: self.segment.into_raw(),
            ghost2: self.ghost2.into_raw(),
            chainId: ffi::B2_NULL_INDEX,
        }
    }

    #[inline]
    /// Validate this chain segment for standalone collision use.
    pub fn is_valid(self) -> bool {
        chain_segment_geometry_is_valid(self)
    }

    #[inline]
    /// Validate this chain segment for standalone collision use.
    pub fn validate(self) -> Result<()> {
        check_chain_segment_geometry_valid_for_operation("ChainSegment::validate", self)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ChainSegment {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            ghost1: Vec2,
            segment: Segment,
            ghost2: Vec2,
        }

        let repr = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        Self::from_segment(repr.ghost1, repr.segment, repr.ghost2).map_err(serde::de::Error::custom)
    }
}

impl fmt::Debug for ChainSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChainSegment")
            .field("ghost1", &self.ghost1)
            .field("segment", &self.segment)
            .field("ghost2", &self.ghost2)
            .finish()
    }
}

impl PartialEq for ChainSegment {
    fn eq(&self, other: &Self) -> bool {
        self.ghost1 == other.ghost1 && self.segment == other.segment && self.ghost2 == other.ghost2
    }
}
