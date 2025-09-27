use boxdd_sys::ffi;

/// A simple 2D vector in meters.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<ffi::b2Vec2> for Vec2 {
    fn from(v: ffi::b2Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

impl From<Vec2> for ffi::b2Vec2 {
    fn from(v: Vec2) -> Self {
        ffi::b2Vec2 { x: v.x, y: v.y }
    }
}

// Mint interop is provided at the sys layer (impl From<mint::Vector2<f32>> for ffi::b2Vec2)

// Public id aliases to avoid exposing `ffi::` in user-facing API/docstrings.
pub type BodyId = ffi::b2BodyId;
pub type ShapeId = ffi::b2ShapeId;
pub type JointId = ffi::b2JointId;
