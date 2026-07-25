/// The native Box2D world-coordinate precision selected for this crate build.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Precision {
    /// Single-precision world coordinates.
    Single,
    /// Double-precision world coordinates for large worlds.
    Double,
}

impl Precision {
    /// The precision selected by this crate's Cargo features.
    pub const ACTIVE: Self = if cfg!(feature = "double-precision") {
        Self::Double
    } else {
        Self::Single
    };

    /// Return the stable manifest spelling for this precision.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Double => "double",
        }
    }

    /// Return whether this is the double-precision ABI.
    pub const fn is_double(self) -> bool {
        matches!(self, Self::Double)
    }

    /// Return the checked-in bindings artifact selected for this ABI.
    pub const fn pregenerated_bindings_file(self) -> &'static str {
        match self {
            Self::Single => "bindings_pregenerated.rs",
            Self::Double => "bindings_double.rs",
        }
    }

    /// Return the stable WASM provider import module for this ABI.
    pub const fn wasm_import_module(self) -> &'static str {
        match self {
            Self::Single => "box2d-sys-v1-single",
            Self::Double => "box2d-sys-v1-double",
        }
    }
}

/// Marker type for the single-precision native ABI.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SinglePrecision;

/// Marker type for the double-precision native ABI.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DoublePrecision;

/// Marker type for the native ABI selected by this crate build.
#[cfg(not(feature = "double-precision"))]
pub type ActivePrecision = SinglePrecision;

/// Marker type for the native ABI selected by this crate build.
#[cfg(feature = "double-precision")]
pub type ActivePrecision = DoublePrecision;

/// Compile-time identity of the selected native ABI.
pub const ABI_PRECISION: Precision = Precision::ACTIVE;

/// Whether this crate was compiled for the double-precision native ABI.
pub const IS_DOUBLE_PRECISION: bool = ABI_PRECISION.is_double();
