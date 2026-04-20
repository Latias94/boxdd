use boxdd_sys::ffi;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Filter {
    pub category_bits: u64,
    pub mask_bits: u64,
    pub group_index: i32,
}

impl Default for Filter {
    fn default() -> Self {
        Self::from_raw(unsafe { ffi::b2DefaultFilter() })
    }
}

impl Filter {
    #[inline]
    /// Construct from the raw Box2D filter value.
    pub const fn from_raw(raw: ffi::b2Filter) -> Self {
        Self {
            category_bits: raw.categoryBits,
            mask_bits: raw.maskBits,
            group_index: raw.groupIndex,
        }
    }

    #[inline]
    /// Convert into the raw Box2D filter value.
    pub const fn into_raw(self) -> ffi::b2Filter {
        ffi::b2Filter {
            categoryBits: self.category_bits,
            maskBits: self.mask_bits,
            groupIndex: self.group_index,
        }
    }
}
