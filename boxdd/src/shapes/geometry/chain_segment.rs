use super::*;

impl ChainSegment {
    #[inline]
    pub fn new<G1, P1, P2, G2>(ghost1: G1, point1: P1, point2: P2, ghost2: G2) -> Self
    where
        G1: Into<Vec2>,
        P1: Into<Vec2>,
        P2: Into<Vec2>,
        G2: Into<Vec2>,
    {
        Self {
            ghost1: ghost1.into(),
            segment: Segment::new(point1, point2),
            ghost2: ghost2.into(),
            chain_id: 0,
        }
    }

    #[inline]
    pub fn from_segment<G1: Into<Vec2>, G2: Into<Vec2>>(
        ghost1: G1,
        segment: Segment,
        ghost2: G2,
    ) -> Self {
        Self {
            ghost1: ghost1.into(),
            segment,
            ghost2: ghost2.into(),
            chain_id: 0,
        }
    }

    #[inline]
    pub fn chain_id_raw(self) -> i32 {
        self.chain_id
    }

    #[inline]
    /// Construct from the raw Box2D geometry value.
    pub fn from_raw(segment: ffi::b2ChainSegment) -> Self {
        Self {
            ghost1: Vec2::from_raw(segment.ghost1),
            segment: Segment::from_raw(segment.segment),
            ghost2: Vec2::from_raw(segment.ghost2),
            chain_id: segment.chainId,
        }
    }

    #[inline]
    /// Convert into the raw Box2D geometry value.
    pub fn into_raw(self) -> ffi::b2ChainSegment {
        ffi::b2ChainSegment {
            ghost1: self.ghost1.into_raw(),
            segment: self.segment.into_raw(),
            ghost2: self.ghost2.into_raw(),
            chainId: self.chain_id,
        }
    }

    #[inline]
    /// Validate this chain segment for standalone collision use.
    pub fn is_valid(self) -> bool {
        self.ghost1.is_valid() && self.segment.is_valid() && self.ghost2.is_valid()
    }

    #[inline]
    /// Validate this chain segment for standalone collision use.
    pub fn validate(self) -> ApiResult<()> {
        geometry_is_valid_or_err(self.is_valid())
    }
}

impl fmt::Debug for ChainSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChainSegment")
            .field("ghost1", &self.ghost1)
            .field("segment", &self.segment)
            .field("ghost2", &self.ghost2)
            .field("chain_id_raw", &self.chain_id)
            .finish()
    }
}

impl PartialEq for ChainSegment {
    fn eq(&self, other: &Self) -> bool {
        self.ghost1 == other.ghost1 && self.segment == other.segment && self.ghost2 == other.ghost2
    }
}
