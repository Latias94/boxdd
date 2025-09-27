#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// Math interop mirroring dear-imgui-sys style: accept mint vectors directly.
impl From<mint::Vector2<f32>> for ffi::b2Vec2 {
    #[inline]
    fn from(v: mint::Vector2<f32>) -> Self {
        Self { x: v.x, y: v.y }
    }
}

impl From<ffi::b2Vec2> for mint::Vector2<f32> {
    #[inline]
    fn from(v: ffi::b2Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

// Also support mint::Point2 for ergonomics when treating points as vectors.
impl From<mint::Point2<f32>> for ffi::b2Vec2 {
    #[inline]
    fn from(p: mint::Point2<f32>) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<ffi::b2Vec2> for mint::Point2<f32> {
    #[inline]
    fn from(v: ffi::b2Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

impl From<[f32; 2]> for ffi::b2Vec2 {
    #[inline]
    fn from(a: [f32; 2]) -> Self {
        Self { x: a[0], y: a[1] }
    }
}

impl From<(f32, f32)> for ffi::b2Vec2 {
    #[inline]
    fn from(t: (f32, f32)) -> Self {
        Self { x: t.0, y: t.1 }
    }
}
