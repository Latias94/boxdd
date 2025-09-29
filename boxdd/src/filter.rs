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
        let f = unsafe { ffi::b2DefaultFilter() };
        Self {
            category_bits: f.categoryBits,
            mask_bits: f.maskBits,
            group_index: f.groupIndex,
        }
    }
}

impl From<Filter> for ffi::b2Filter {
    #[inline]
    fn from(f: Filter) -> Self {
        Self {
            categoryBits: f.category_bits,
            maskBits: f.mask_bits,
            groupIndex: f.group_index,
        }
    }
}

impl From<ffi::b2Filter> for Filter {
    #[inline]
    fn from(f: ffi::b2Filter) -> Self {
        Self {
            category_bits: f.categoryBits,
            mask_bits: f.maskBits,
            group_index: f.groupIndex,
        }
    }
}
