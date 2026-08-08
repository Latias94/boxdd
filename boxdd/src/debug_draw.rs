//! Debug Draw bridge to Box2D v3 callbacks.
//!
//! Implement the `DebugDraw` trait to receive drawing commands and call `World::debug_draw` each
//! step with `DebugDrawOptions` to render. Colors use the crate-owned [`HexColor`] type, which
//! stores Box2D's packed `0xRRGGBB` convention.
//!
//! Example
//! ```no_run
//! use boxdd::{DebugDraw, DebugDrawOptions, Foundation, HexColor, Vec2, WorldTransform};
//! struct Printer;
//! impl DebugDraw for Printer {
//!     fn draw_polygon(
//!         &mut self,
//!         _transform: WorldTransform,
//!         vertices: &[Vec2],
//!         color: HexColor,
//!     ) {
//!         println!("poly {} color={:#x}", vertices.len(), color.rgb_u32());
//!     }
//! }
//! # let foundation = Foundation::initialize_default().unwrap();
//! # let def = foundation.world_builder().build().unwrap();
//! # let mut world = foundation.create_world(def).unwrap();
//! let mut cmds = Vec::new();
//! world
//!     .debug_draw_collect_into(&mut cmds, DebugDrawOptions::default())
//!     .unwrap();
//! let mut drawer = Printer;
//! for cmd in cmds {
//!     let _ = cmd;
//! }
//! ```
use crate::Aabb;
use crate::types::{Position, Vec2, WorldTransform};
#[cfg(not(target_arch = "wasm32"))]
use crate::world::{World, check_world_available};
use boxdd_sys::ffi;
#[cfg(not(target_arch = "wasm32"))]
use smallvec::SmallVec;
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::CStr;

/// Packed Box2D debug-draw RGB color (`0xRRGGBB`).
///
/// Serde rejects integers outside the 24-bit RGB range rather than truncating them.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct HexColor(u32);

impl HexColor {
    pub(crate) const MAX_RGB_U32: u32 = 0x00ff_ffff;

    pub const BLACK: Self = Self::from_rgb_u32(0x000000);
    pub const WHITE: Self = Self::from_rgb_u32(0xFFFFFF);
    pub const RED: Self = Self::from_rgb_u32(0xFF0000);
    pub const GREEN: Self = Self::from_rgb_u32(0x00FF00);
    pub const BLUE: Self = Self::from_rgb_u32(0x0000FF);
    pub const BOX2D_RED: Self = Self::from_rgb_u32(0xDC3132);
    pub const BOX2D_BLUE: Self = Self::from_rgb_u32(0x30AEBF);
    pub const BOX2D_GREEN: Self = Self::from_rgb_u32(0x8CC924);
    pub const BOX2D_YELLOW: Self = Self::from_rgb_u32(0xFFEE8C);

    #[inline]
    pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self(((red as u32) << 16) | ((green as u32) << 8) | blue as u32)
    }

    #[inline]
    pub const fn from_rgb_u32(rgb: u32) -> Self {
        Self(rgb & Self::MAX_RGB_U32)
    }

    #[inline]
    pub const fn from_raw(raw: ffi::b2HexColor) -> Self {
        Self::from_rgb_u32(raw)
    }

    #[inline]
    pub const fn rgb_u32(self) -> u32 {
        self.0
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2HexColor {
        self.0
    }

    #[inline]
    pub const fn with_alpha(self, alpha: u8) -> u32 {
        ((alpha as u32) << 24) | self.0
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let rgb = <u32 as serde::Deserialize>::deserialize(deserializer)?;
        if rgb > Self::MAX_RGB_U32 {
            return Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Unsigned(u64::from(rgb)),
                &"an RGB color in the inclusive range 0x000000..=0xFFFFFF",
            ));
        }
        Ok(Self::from_rgb_u32(rgb))
    }
}

const _: () = {
    assert!(core::mem::size_of::<HexColor>() == core::mem::size_of::<ffi::b2HexColor>());
    assert!(core::mem::align_of::<HexColor>() == core::mem::align_of::<ffi::b2HexColor>());
};

#[derive(Clone, Debug)]
pub enum DebugDrawCmd {
    Polygon {
        transform: WorldTransform,
        vertices: Vec<Vec2>,
        color: HexColor,
    },
    SolidPolygon {
        transform: WorldTransform,
        vertices: Vec<Vec2>,
        radius: f32,
        color: HexColor,
    },
    Circle {
        center: Position,
        radius: f32,
        color: HexColor,
    },
    SolidCircle {
        transform: WorldTransform,
        center: Vec2,
        radius: f32,
        color: HexColor,
    },
    SolidCapsule {
        p1: Position,
        p2: Position,
        radius: f32,
        color: HexColor,
    },
    Segment {
        p1: Position,
        p2: Position,
        color: HexColor,
    },
    Transform(WorldTransform),
    Point {
        p: Position,
        size: f32,
        color: HexColor,
    },
    String {
        p: Position,
        s: String,
        color: HexColor,
    },
    Bounds {
        bounds: Aabb,
        color: HexColor,
    },
}

/// Safe, precision-aware debug draw callbacks.
#[cfg(not(target_arch = "wasm32"))]
pub trait DebugDraw {
    fn draw_polygon(&mut self, _transform: WorldTransform, _vertices: &[Vec2], _color: HexColor) {}
    fn draw_solid_polygon(
        &mut self,
        _transform: WorldTransform,
        _vertices: &[Vec2],
        _radius: f32,
        _color: HexColor,
    ) {
    }
    fn draw_circle(&mut self, _center: Position, _radius: f32, _color: HexColor) {}
    fn draw_solid_circle(
        &mut self,
        _transform: WorldTransform,
        _center: Vec2,
        _radius: f32,
        _color: HexColor,
    ) {
    }
    fn draw_solid_capsule(&mut self, _p1: Position, _p2: Position, _radius: f32, _color: HexColor) {
    }
    fn draw_segment(&mut self, _p1: Position, _p2: Position, _color: HexColor) {}
    fn draw_transform(&mut self, _transform: WorldTransform) {}
    fn draw_point(&mut self, _p: Position, _size: f32, _color: HexColor) {}
    fn draw_string(&mut self, _p: Position, _s: &str, _color: HexColor) {}
    fn draw_bounds(&mut self, _bounds: Aabb, _color: HexColor) {}
}

#[derive(Copy, Clone, Debug)]
pub struct DebugDrawOptions {
    pub drawing_bounds: Aabb,
    pub force_scale: f32,
    pub joint_scale: f32,
    pub draw_contacts: bool,
    pub draw_anchor_a: bool,
    pub draw_shapes: bool,
    pub draw_chain_normals: bool,
    pub draw_joints: bool,
    pub draw_joint_extras: bool,
    pub draw_bounds: bool,
    pub draw_mass: bool,
    pub draw_body_names: bool,
    pub draw_graph_colors: bool,
    pub draw_contact_features: bool,
    pub draw_contact_normals: bool,
    pub draw_contact_forces: bool,
    pub draw_friction_forces: bool,
    pub draw_islands: bool,
}

impl Default for DebugDrawOptions {
    fn default() -> Self {
        Self {
            drawing_bounds: Aabb::new([-1.0e9, -1.0e9], [1.0e9, 1.0e9])
                .expect("hard-coded debug drawing bounds are valid"),
            force_scale: 1.0,
            joint_scale: 1.0,
            draw_contacts: false,
            draw_anchor_a: false,
            draw_shapes: true,
            draw_chain_normals: false,
            draw_joints: true,
            draw_joint_extras: false,
            draw_bounds: false,
            draw_mass: false,
            draw_body_names: false,
            draw_graph_colors: false,
            draw_contact_features: false,
            draw_contact_normals: false,
            draw_contact_forces: false,
            draw_friction_forces: false,
            draw_islands: false,
        }
    }
}

impl DebugDrawOptions {
    /// Validate every numeric option before it crosses the Box2D ABI boundary.
    pub fn validate(&self) -> crate::error::Result<()> {
        if !self.drawing_bounds.is_valid() {
            return Err(crate::error::Error::invalid_argument(
                "DebugDrawOptions::validate",
                "drawing_bounds",
                "finite ordered lower and upper bounds",
            ));
        }
        if !self.force_scale.is_finite() {
            return Err(crate::error::Error::invalid_argument(
                "DebugDrawOptions::validate",
                "force_scale",
                "a finite value",
            ));
        }
        if !self.joint_scale.is_finite() {
            return Err(crate::error::Error::invalid_argument(
                "DebugDrawOptions::validate",
                "joint_scale",
                "a finite value",
            ));
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn checked(self) -> crate::error::Result<ValidatedDebugDrawOptions> {
        self.validate()?;
        Ok(ValidatedDebugDrawOptions(self))
    }
}

#[derive(Copy, Clone)]
#[cfg(not(target_arch = "wasm32"))]
struct ValidatedDebugDrawOptions(DebugDrawOptions);

#[cfg(not(target_arch = "wasm32"))]
struct DebugDrawCtx<'a> {
    drawer: &'a mut (dyn NativeDebugDraw + 'a),
    panic: &'a mut crate::core::callback_state::PanicSlot,
}

#[inline]
#[cfg(not(target_arch = "wasm32"))]
unsafe fn with_ffi_debug_draw_vertices(
    vertices: *const ffi::b2Vec2,
    count: i32,
    visit: impl FnOnce(&[ffi::b2Vec2]),
) -> bool {
    let Ok(len) = usize::try_from(count) else {
        return false;
    };
    if len == 0 {
        visit(&[]);
        return true;
    }
    if vertices.is_null()
        || !vertices.is_aligned()
        || len > ffi::B2_MAX_POLYGON_VERTICES as usize
        || len > (isize::MAX as usize) / core::mem::size_of::<ffi::b2Vec2>()
        || vertices
            .addr()
            .checked_add(len * core::mem::size_of::<ffi::b2Vec2>())
            .is_none()
    {
        return false;
    }
    visit(unsafe { core::slice::from_raw_parts(vertices, len) });
    true
}

#[cfg(not(target_arch = "wasm32"))]
trait NativeDebugDraw {
    fn invalid_output(&mut self, output: &'static str, constraint: &'static str);

    fn draw_polygon(
        &mut self,
        transform: ffi::b2WorldTransform,
        vertices: &[ffi::b2Vec2],
        color: HexColor,
    );
    fn draw_solid_polygon(
        &mut self,
        transform: ffi::b2WorldTransform,
        vertices: &[ffi::b2Vec2],
        radius: f32,
        color: HexColor,
    );
    fn draw_circle(&mut self, center: ffi::b2Pos, radius: f32, color: HexColor);
    fn draw_solid_circle(
        &mut self,
        transform: ffi::b2WorldTransform,
        center: ffi::b2Vec2,
        radius: f32,
        color: HexColor,
    );
    fn draw_solid_capsule(&mut self, p1: ffi::b2Pos, p2: ffi::b2Pos, radius: f32, color: HexColor);
    fn draw_segment(&mut self, p1: ffi::b2Pos, p2: ffi::b2Pos, color: HexColor);
    fn draw_transform(&mut self, transform: ffi::b2WorldTransform);
    fn draw_point(&mut self, p: ffi::b2Pos, size: f32, color: HexColor);
    fn draw_string(&mut self, p: ffi::b2Pos, s: &CStr, color: HexColor);
    fn draw_bounds(&mut self, bounds: ffi::b2AABB, color: HexColor);
}

#[cfg(not(target_arch = "wasm32"))]
struct SafeDebugDrawAdapter<'a> {
    drawer: &'a mut dyn DebugDraw,
    operation: &'static str,
    error: Option<crate::Error>,
}

#[cfg(not(target_arch = "wasm32"))]
impl SafeDebugDrawAdapter<'_> {
    fn reject(&mut self, output: &'static str, constraint: &'static str) {
        self.error.get_or_insert(crate::Error::InvalidNativeOutput {
            operation: self.operation,
            output,
            constraint,
        });
    }

    fn transform(
        &mut self,
        raw: ffi::b2WorldTransform,
        output: &'static str,
    ) -> Option<WorldTransform> {
        if self.error.is_some() {
            return None;
        }
        match WorldTransform::from_raw(raw) {
            Ok(value) => Some(value),
            Err(_) => {
                self.reject(output, "a finite rigid world transform");
                None
            }
        }
    }

    fn position(&mut self, raw: ffi::b2Pos, output: &'static str) -> Option<Position> {
        if self.error.is_some() {
            return None;
        }
        let value = Position::from_raw(raw);
        if value.is_valid() {
            Some(value)
        } else {
            self.reject(output, "a finite world position");
            None
        }
    }

    fn vector(&mut self, raw: ffi::b2Vec2, output: &'static str) -> Option<Vec2> {
        if self.error.is_some() {
            return None;
        }
        let value = Vec2::from_raw(raw);
        if value.is_valid() {
            Some(value)
        } else {
            self.reject(output, "a finite vector");
            None
        }
    }

    fn non_negative(&mut self, value: f32, output: &'static str) -> Option<f32> {
        if self.error.is_some() {
            return None;
        }
        if value.is_finite() && value >= 0.0 {
            Some(value)
        } else {
            self.reject(output, "a finite non-negative value");
            None
        }
    }

    fn vertices(&mut self, raw: &[ffi::b2Vec2]) -> Option<SmallVec<[Vec2; 8]>> {
        if self.error.is_some() {
            return None;
        }
        let vertices = raw
            .iter()
            .copied()
            .map(Vec2::from_raw)
            .collect::<SmallVec<[Vec2; 8]>>();
        if vertices.iter().copied().all(Vec2::is_valid) {
            Some(vertices)
        } else {
            self.reject("vertices", "finite vertex coordinates");
            None
        }
    }

    fn bounds(&mut self, raw: ffi::b2AABB) -> Option<Aabb> {
        if self.error.is_some() {
            return None;
        }
        match Aabb::from_raw(raw) {
            Ok(value) => Some(value),
            Err(_) => {
                self.reject("bounds", "finite ordered lower and upper bounds");
                None
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeDebugDraw for SafeDebugDrawAdapter<'_> {
    fn invalid_output(&mut self, output: &'static str, constraint: &'static str) {
        self.reject(output, constraint);
    }

    fn draw_polygon(
        &mut self,
        transform: ffi::b2WorldTransform,
        vertices: &[ffi::b2Vec2],
        color: HexColor,
    ) {
        let Some(transform) = self.transform(transform, "transform") else {
            return;
        };
        let Some(vertices) = self.vertices(vertices) else {
            return;
        };
        self.drawer.draw_polygon(transform, &vertices, color);
    }

    fn draw_solid_polygon(
        &mut self,
        transform: ffi::b2WorldTransform,
        vertices: &[ffi::b2Vec2],
        radius: f32,
        color: HexColor,
    ) {
        let Some(transform) = self.transform(transform, "transform") else {
            return;
        };
        let Some(vertices) = self.vertices(vertices) else {
            return;
        };
        let Some(radius) = self.non_negative(radius, "radius") else {
            return;
        };
        self.drawer
            .draw_solid_polygon(transform, &vertices, radius, color);
    }

    fn draw_circle(&mut self, center: ffi::b2Pos, radius: f32, color: HexColor) {
        let Some(center) = self.position(center, "center") else {
            return;
        };
        let Some(radius) = self.non_negative(radius, "radius") else {
            return;
        };
        self.drawer.draw_circle(center, radius, color);
    }

    fn draw_solid_circle(
        &mut self,
        transform: ffi::b2WorldTransform,
        center: ffi::b2Vec2,
        radius: f32,
        color: HexColor,
    ) {
        let Some(transform) = self.transform(transform, "transform") else {
            return;
        };
        let Some(center) = self.vector(center, "center") else {
            return;
        };
        let Some(radius) = self.non_negative(radius, "radius") else {
            return;
        };
        self.drawer
            .draw_solid_circle(transform, center, radius, color);
    }

    fn draw_solid_capsule(&mut self, p1: ffi::b2Pos, p2: ffi::b2Pos, radius: f32, color: HexColor) {
        let Some(p1) = self.position(p1, "p1") else {
            return;
        };
        let Some(p2) = self.position(p2, "p2") else {
            return;
        };
        let Some(radius) = self.non_negative(radius, "radius") else {
            return;
        };
        self.drawer.draw_solid_capsule(p1, p2, radius, color);
    }

    fn draw_segment(&mut self, p1: ffi::b2Pos, p2: ffi::b2Pos, color: HexColor) {
        let Some(p1) = self.position(p1, "p1") else {
            return;
        };
        let Some(p2) = self.position(p2, "p2") else {
            return;
        };
        self.drawer.draw_segment(p1, p2, color);
    }

    fn draw_transform(&mut self, transform: ffi::b2WorldTransform) {
        let Some(transform) = self.transform(transform, "transform") else {
            return;
        };
        self.drawer.draw_transform(transform);
    }

    fn draw_point(&mut self, p: ffi::b2Pos, size: f32, color: HexColor) {
        let Some(p) = self.position(p, "point") else {
            return;
        };
        let Some(size) = self.non_negative(size, "size") else {
            return;
        };
        self.drawer.draw_point(p, size, color);
    }

    fn draw_string(&mut self, p: ffi::b2Pos, s: &CStr, color: HexColor) {
        let Some(p) = self.position(p, "point") else {
            return;
        };
        self.drawer.draw_string(p, &s.to_string_lossy(), color);
    }

    fn draw_bounds(&mut self, bounds: ffi::b2AABB, color: HexColor) {
        let Some(bounds) = self.bounds(bounds) else {
            return;
        };
        self.drawer.draw_bounds(bounds, color);
    }
}

/// Reborrow the stack context installed immediately around `b2World_Draw`.
///
/// # Safety
///
/// `context` must be the non-null pointer installed by `draw_with_adapter`. Box2D must invoke
/// callbacks synchronously and serially, and must not retain the pointer after `b2World_Draw`.
#[cfg(not(target_arch = "wasm32"))]
unsafe fn native_debug_draw_context<'a>(
    context: *mut core::ffi::c_void,
) -> Option<&'a mut DebugDrawCtx<'a>> {
    let context = context.cast::<DebugDrawCtx<'a>>();
    if context.is_null() || !context.is_aligned() {
        None
    } else {
        Some(unsafe { &mut *context })
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn run_native_debug_draw_callback<'a>(
    context: *mut core::ffi::c_void,
    callback: impl FnOnce(&mut (dyn NativeDebugDraw + 'a)),
) {
    let Some(ctx) = (unsafe { native_debug_draw_context::<'a>(context) }) else {
        return;
    };
    crate::core::callback_state::invoke_owner_callback(ctx.panic, (), || {
        callback(ctx.drawer);
    });
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn draw_polygon_cb(
    transform: ffi::b2WorldTransform,
    vertices: *const ffi::b2Vec2,
    count: i32,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            let valid = with_ffi_debug_draw_vertices(vertices, count, |vertices| {
                drawer.draw_polygon(transform, vertices, HexColor::from_raw(color));
            });
            if !valid {
                drawer.invalid_output(
                    "vertices",
                    "a non-negative bounded count and a non-null aligned pointer when non-empty",
                );
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn draw_solid_polygon_cb(
    transform: ffi::b2WorldTransform,
    vertices: *const ffi::b2Vec2,
    count: i32,
    radius: f32,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            let valid = with_ffi_debug_draw_vertices(vertices, count, |vertices| {
                drawer.draw_solid_polygon(transform, vertices, radius, HexColor::from_raw(color));
            });
            if !valid {
                drawer.invalid_output(
                    "vertices",
                    "a non-negative bounded count and a non-null aligned pointer when non-empty",
                );
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn draw_circle_cb(
    center: ffi::b2Pos,
    radius: f32,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            drawer.draw_circle(center, radius, HexColor::from_raw(color));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn draw_solid_circle_cb(
    transform: ffi::b2WorldTransform,
    center: ffi::b2Vec2,
    radius: f32,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            drawer.draw_solid_circle(transform, center, radius, HexColor::from_raw(color));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn draw_solid_capsule_cb(
    p1: ffi::b2Pos,
    p2: ffi::b2Pos,
    radius: f32,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            drawer.draw_solid_capsule(p1, p2, radius, HexColor::from_raw(color));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn draw_line_cb(
    p1: ffi::b2Pos,
    p2: ffi::b2Pos,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            drawer.draw_segment(p1, p2, HexColor::from_raw(color));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn draw_transform_cb(
    transform: ffi::b2WorldTransform,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| drawer.draw_transform(transform));
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn draw_point_cb(
    p: ffi::b2Pos,
    size: f32,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            drawer.draw_point(p, size, HexColor::from_raw(color));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn draw_string_cb(
    p: ffi::b2Pos,
    s: *const core::ffi::c_char,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            if s.is_null() {
                drawer.invalid_output("string", "a non-null NUL-terminated string pointer");
                return;
            }
            let s = CStr::from_ptr(s);
            drawer.draw_string(p, s, HexColor::from_raw(color));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn draw_bounds_cb(
    bounds: ffi::b2AABB,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            drawer.draw_bounds(bounds, HexColor::from_raw(color));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn install_debug_draw_callbacks(dd: &mut ffi::b2DebugDraw) {
    dd.DrawPolygonFcn = Some(draw_polygon_cb);
    dd.DrawSolidPolygonFcn = Some(draw_solid_polygon_cb);
    dd.DrawCircleFcn = Some(draw_circle_cb);
    dd.DrawSolidCircleFcn = Some(draw_solid_circle_cb);
    dd.DrawSolidCapsuleFcn = Some(draw_solid_capsule_cb);
    dd.DrawLineFcn = Some(draw_line_cb);
    dd.DrawTransformFcn = Some(draw_transform_cb);
    dd.DrawPointFcn = Some(draw_point_cb);
    dd.DrawStringFcn = Some(draw_string_cb);
    dd.DrawBoundsFcn = Some(draw_bounds_cb);
}

#[cfg(not(target_arch = "wasm32"))]
fn apply_debug_draw_options(
    dd: &mut ffi::b2DebugDraw,
    opts: DebugDrawOptions,
    context: *mut core::ffi::c_void,
) {
    dd.drawingBounds = opts.drawing_bounds.into_raw();
    dd.forceScale = opts.force_scale;
    dd.jointScale = opts.joint_scale;
    dd.drawContacts = opts.draw_contacts;
    dd.drawAnchorA = opts.draw_anchor_a;
    dd.drawShapes = opts.draw_shapes;
    dd.drawChainNormals = opts.draw_chain_normals;
    dd.drawJoints = opts.draw_joints;
    dd.drawJointExtras = opts.draw_joint_extras;
    dd.drawBounds = opts.draw_bounds;
    dd.drawMass = opts.draw_mass;
    dd.drawBodyNames = opts.draw_body_names;
    dd.drawGraphColors = opts.draw_graph_colors;
    dd.drawContactFeatures = opts.draw_contact_features;
    dd.drawContactNormals = opts.draw_contact_normals;
    dd.drawContactForces = opts.draw_contact_forces;
    dd.drawFrictionForces = opts.draw_friction_forces;
    dd.drawIslands = opts.draw_islands;
    dd.context = context;
}

/// Run both replay-world and recorded-query drawing under one replay-owned panic boundary.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn draw_replay_player(
    player: *mut ffi::b2RecPlayer,
    world: ffi::b2WorldId,
    drawer: &mut impl DebugDraw,
    options: DebugDrawOptions,
    query_index: i32,
) -> crate::error::Result<crate::core::callback_state::PanicSlot> {
    crate::core::callback_state::check_not_in_callback()?;
    let options = options.checked()?;
    let drawer: &mut dyn DebugDraw = drawer;
    let mut adapter = SafeDebugDrawAdapter {
        drawer,
        operation: "ReplayPlayer::debug_draw",
        error: None,
    };
    let mut panic = crate::core::callback_state::PanicSlot::default();
    {
        let adapter: &mut dyn NativeDebugDraw = &mut adapter;
        let mut context = DebugDrawCtx {
            drawer: adapter,
            panic: &mut panic,
        };
        let context_pointer = core::ptr::from_mut(&mut context).cast::<core::ffi::c_void>();
        let mut draw = unsafe { ffi::b2DefaultDebugDraw() };
        install_debug_draw_callbacks(&mut draw);
        apply_debug_draw_options(&mut draw, options.0, context_pointer);

        unsafe { ffi::b2World_Draw(world, &mut draw) };
        if !panic.has_panicked() {
            unsafe { ffi::b2RecPlayer_DrawFrameQueries(player, &mut draw, query_index) };
        }
    }
    if let Some(error) = adapter.error {
        return Err(error);
    }
    Ok(panic)
}

#[cfg(not(target_arch = "wasm32"))]
struct CollectDebugDraw<'a> {
    cmds: &'a mut Vec<DebugDrawCmd>,
    len: usize,
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> CollectDebugDraw<'a> {
    fn new(cmds: &'a mut Vec<DebugDrawCmd>) -> Self {
        Self { cmds, len: 0 }
    }

    fn finish(self) {
        self.cmds.truncate(self.len);
    }

    fn replace_or_push(&mut self, cmd: DebugDrawCmd) {
        if let Some(slot) = self.cmds.get_mut(self.len) {
            *slot = cmd;
        } else {
            self.cmds.push(cmd);
        }
        self.len += 1;
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl DebugDraw for CollectDebugDraw<'_> {
    fn draw_polygon(&mut self, transform: WorldTransform, vertices: &[Vec2], color: HexColor) {
        match self.cmds.get_mut(self.len) {
            Some(DebugDrawCmd::Polygon {
                transform: stored_transform,
                vertices: stored,
                color: stored_color,
            }) => {
                *stored_transform = transform;
                stored.clear();
                stored.extend_from_slice(vertices);
                *stored_color = color;
                self.len += 1;
            }
            _ => self.replace_or_push(DebugDrawCmd::Polygon {
                transform,
                vertices: vertices.to_vec(),
                color,
            }),
        }
    }

    fn draw_solid_polygon(
        &mut self,
        transform: WorldTransform,
        vertices: &[Vec2],
        radius: f32,
        color: HexColor,
    ) {
        match self.cmds.get_mut(self.len) {
            Some(DebugDrawCmd::SolidPolygon {
                transform: stored_transform,
                vertices: stored_vertices,
                radius: stored_radius,
                color: stored_color,
            }) => {
                *stored_transform = transform;
                stored_vertices.clear();
                stored_vertices.extend_from_slice(vertices);
                *stored_radius = radius;
                *stored_color = color;
                self.len += 1;
            }
            _ => self.replace_or_push(DebugDrawCmd::SolidPolygon {
                transform,
                vertices: vertices.to_vec(),
                radius,
                color,
            }),
        }
    }

    fn draw_circle(&mut self, center: Position, radius: f32, color: HexColor) {
        self.replace_or_push(DebugDrawCmd::Circle {
            center,
            radius,
            color,
        });
    }

    fn draw_solid_circle(
        &mut self,
        transform: WorldTransform,
        center: Vec2,
        radius: f32,
        color: HexColor,
    ) {
        self.replace_or_push(DebugDrawCmd::SolidCircle {
            transform,
            center,
            radius,
            color,
        });
    }

    fn draw_solid_capsule(&mut self, p1: Position, p2: Position, radius: f32, color: HexColor) {
        self.replace_or_push(DebugDrawCmd::SolidCapsule {
            p1,
            p2,
            radius,
            color,
        });
    }

    fn draw_segment(&mut self, p1: Position, p2: Position, color: HexColor) {
        self.replace_or_push(DebugDrawCmd::Segment { p1, p2, color });
    }

    fn draw_transform(&mut self, transform: WorldTransform) {
        self.replace_or_push(DebugDrawCmd::Transform(transform));
    }

    fn draw_point(&mut self, p: Position, size: f32, color: HexColor) {
        self.replace_or_push(DebugDrawCmd::Point { p, size, color });
    }

    fn draw_string(&mut self, p: Position, s: &str, color: HexColor) {
        match self.cmds.get_mut(self.len) {
            Some(DebugDrawCmd::String {
                p: stored_p,
                s: stored_s,
                color: stored_color,
            }) => {
                *stored_p = p;
                stored_s.clear();
                stored_s.push_str(s);
                *stored_color = color;
                self.len += 1;
            }
            _ => self.replace_or_push(DebugDrawCmd::String {
                p,
                s: s.to_owned(),
                color,
            }),
        }
    }

    fn draw_bounds(&mut self, bounds: Aabb, color: HexColor) {
        self.replace_or_push(DebugDrawCmd::Bounds { bounds, color });
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl World {
    /// Collect debug draw commands into a vector (fully safe).
    ///
    /// This calls into Box2D debug draw but does not invoke user code during the draw.
    pub fn debug_draw_collect(
        &mut self,
        opts: DebugDrawOptions,
    ) -> crate::error::Result<Vec<DebugDrawCmd>> {
        check_world_available(self)?;
        let opts = opts.checked()?;
        let mut cmds = Vec::new();
        self.debug_draw_collect_into_validated(&mut cmds, opts)?;
        Ok(cmds)
    }

    fn debug_draw_collect_into_validated(
        &mut self,
        out: &mut Vec<DebugDrawCmd>,
        opts: ValidatedDebugDrawOptions,
    ) -> crate::error::Result<()> {
        let mut collector = CollectDebugDraw::new(out);
        let result =
            self.debug_draw_validated(&mut collector, opts, "World::debug_draw_collect_into");
        collector.finish();
        if result.is_err() {
            out.clear();
        }
        result
    }

    /// Collect debug draw commands into a caller-owned buffer.
    ///
    /// This reuses the outer command buffer and, when the command sequence stays stable, also
    /// reuses nested polygon vertex and string storage. Returns `Error::InCallback` if called
    /// while Box2D is already executing a callback.
    pub fn debug_draw_collect_into(
        &mut self,
        out: &mut Vec<DebugDrawCmd>,
        opts: DebugDrawOptions,
    ) -> crate::error::Result<()> {
        check_world_available(self)?;
        self.debug_draw_collect_into_validated(out, opts.checked()?)
    }

    fn draw_with_adapter(
        &mut self,
        adapter: &mut dyn NativeDebugDraw,
        opts: ValidatedDebugDrawOptions,
    ) {
        crate::core::callback_state::run_debug_draw_boundary(
            crate::core::callback_state::CallbackOwnerToken::world(self.core().brand.token()),
            || {
                let mut callback_panic = crate::core::callback_state::PanicSlot::default();
                {
                    let mut ctx = DebugDrawCtx {
                        drawer: adapter,
                        panic: &mut callback_panic,
                    };
                    let context = core::ptr::from_mut(&mut ctx).cast::<core::ffi::c_void>();
                    let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
                    install_debug_draw_callbacks(&mut dd);
                    apply_debug_draw_options(&mut dd, opts.0, context);

                    unsafe { ffi::b2World_Draw(self.raw(), &mut dd) };
                }
                callback_panic
            },
            |callback_panic, panic| {
                callback_panic.map(|callback_panic| {
                    panic.absorb(callback_panic);
                })
            },
        );
    }

    fn debug_draw_validated(
        &mut self,
        drawer: &mut impl DebugDraw,
        opts: ValidatedDebugDrawOptions,
        operation: &'static str,
    ) -> crate::error::Result<()> {
        let drawer: &mut dyn DebugDraw = drawer;
        let mut adapter = SafeDebugDrawAdapter {
            drawer,
            operation,
            error: None,
        };
        self.draw_with_adapter(&mut adapter, opts);
        adapter.error.map_or(Ok(()), Err)
    }

    /// Draw the world through the safe, precision-aware callback interface.
    ///
    /// Box2D invokes the draw callbacks while traversing internal world state. During this call,
    /// any attempt to call into the Box2D world through `boxdd` returns `Error::InCallback`.
    pub fn debug_draw(
        &mut self,
        drawer: &mut impl DebugDraw,
        opts: DebugDrawOptions,
    ) -> crate::error::Result<()> {
        check_world_available(self)?;
        self.debug_draw_validated(drawer, opts.checked()?, "World::debug_draw")
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn test_position(x: f64, y: f64) -> Position {
        #[cfg(feature = "double-precision")]
        {
            Position::new(x, y)
        }

        #[cfg(not(feature = "double-precision"))]
        {
            Position::new(x as f32, y as f32)
        }
    }

    fn assert_position_eq(actual: Position, expected: Position) {
        assert_eq!(actual.x, expected.x);
        assert_eq!(actual.y, expected.y);
    }

    fn test_context<'a>(
        drawer: &'a mut dyn NativeDebugDraw,
        panic: &'a mut crate::core::callback_state::PanicSlot,
    ) -> DebugDrawCtx<'a> {
        DebugDrawCtx { drawer, panic }
    }

    #[test]
    fn invalid_options_are_rejected_before_native_debug_draw() {
        struct NoopDrawer;
        impl DebugDraw for NoopDrawer {}

        let invalid_bounds = DebugDrawOptions {
            drawing_bounds: Aabb {
                lower: [f32::NAN, 0.0].into(),
                upper: [1.0, 1.0].into(),
            },
            ..DebugDrawOptions::default()
        };
        let invalid_scale = DebugDrawOptions {
            force_scale: f32::INFINITY,
            ..DebugDrawOptions::default()
        };
        assert_eq!(
            invalid_bounds.validate().unwrap_err(),
            crate::Error::invalid_argument(
                "DebugDrawOptions::validate",
                "drawing_bounds",
                "finite ordered lower and upper bounds",
            )
        );
        assert_eq!(
            invalid_scale.validate().unwrap_err(),
            crate::Error::invalid_argument(
                "DebugDrawOptions::validate",
                "force_scale",
                "a finite value",
            )
        );

        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let mut out = Vec::new();
        let mut drawer = NoopDrawer;
        assert_eq!(
            world.debug_draw_collect(invalid_bounds).unwrap_err(),
            crate::Error::invalid_argument(
                "DebugDrawOptions::validate",
                "drawing_bounds",
                "finite ordered lower and upper bounds",
            )
        );
        assert_eq!(
            world
                .debug_draw_collect_into(&mut out, invalid_bounds)
                .unwrap_err(),
            crate::Error::invalid_argument(
                "DebugDrawOptions::validate",
                "drawing_bounds",
                "finite ordered lower and upper bounds",
            )
        );
        assert_eq!(
            world.debug_draw(&mut drawer, invalid_scale).unwrap_err(),
            crate::Error::invalid_argument(
                "DebugDrawOptions::validate",
                "force_scale",
                "a finite value",
            )
        );
    }

    #[test]
    fn safe_adapter_rejects_invalid_native_values_before_user_dispatch() {
        #[derive(Default)]
        struct CountDraws(usize);

        impl DebugDraw for CountDraws {
            fn draw_transform(&mut self, _transform: WorldTransform) {
                self.0 += 1;
            }

            fn draw_point(&mut self, _p: Position, _size: f32, _color: HexColor) {
                self.0 += 1;
            }
        }

        let mut drawer = CountDraws::default();
        let error = {
            let mut adapter = SafeDebugDrawAdapter {
                drawer: &mut drawer,
                operation: "test_debug_draw",
                error: None,
            };
            let mut invalid = WorldTransform::IDENTITY.into_raw();
            invalid.q.c = f32::NAN;

            adapter.draw_transform(invalid);
            adapter.draw_point(Position::ZERO.into_raw(), 1.0, HexColor::WHITE);
            adapter.error
        };
        assert_eq!(drawer.0, 0);
        assert_eq!(
            error,
            Some(crate::Error::InvalidNativeOutput {
                operation: "test_debug_draw",
                output: "transform",
                constraint: "a finite rigid world transform",
            })
        );
    }

    #[test]
    fn box2d_32_callbacks_dispatch_every_command() {
        #[cfg(feature = "double-precision")]
        let far = 1_000_000_000_000.25;
        #[cfg(not(feature = "double-precision"))]
        let far = 1_000_000.25;

        let transform = WorldTransform::from_pos_angle(test_position(far, -far), 0.25).unwrap();
        let circle_center = test_position(far + 10.5, -far - 20.25);
        let p1 = test_position(far + 1.0, -far - 2.0);
        let p2 = test_position(far + 3.0, -far - 4.0);
        let vertices = [Vec2::new(-1.0, -2.0), Vec2::new(3.0, 4.0)];
        let raw_vertices = vertices.map(Vec2::into_raw);
        let local_center = Vec2::new(0.5, -0.75);
        let bounds = Aabb::new([-5.0, -6.0], [7.0, 8.0]).unwrap();
        let color = HexColor::from_rgb(0x12, 0x34, 0x56);
        let mut commands = Vec::new();
        let mut panic = crate::core::callback_state::PanicSlot::default();

        {
            let mut collector = CollectDebugDraw::new(&mut commands);
            {
                let drawer: &mut dyn DebugDraw = &mut collector;
                let mut adapter = SafeDebugDrawAdapter {
                    drawer,
                    operation: "test",
                    error: None,
                };
                let adapter: &mut dyn NativeDebugDraw = &mut adapter;
                let mut ctx = test_context(adapter, &mut panic);
                let context = core::ptr::from_mut(&mut ctx).cast::<core::ffi::c_void>();
                let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
                install_debug_draw_callbacks(&mut dd);

                unsafe {
                    dd.DrawPolygonFcn.unwrap()(
                        transform.into_raw(),
                        raw_vertices.as_ptr(),
                        raw_vertices.len() as i32,
                        color.into_raw(),
                        context,
                    );
                    dd.DrawSolidPolygonFcn.unwrap()(
                        transform.into_raw(),
                        raw_vertices.as_ptr(),
                        raw_vertices.len() as i32,
                        0.125,
                        color.into_raw(),
                        context,
                    );
                    dd.DrawCircleFcn.unwrap()(
                        circle_center.into_raw(),
                        2.5,
                        color.into_raw(),
                        context,
                    );
                    dd.DrawSolidCircleFcn.unwrap()(
                        transform.into_raw(),
                        local_center.into_raw(),
                        3.5,
                        color.into_raw(),
                        context,
                    );
                    dd.DrawSolidCapsuleFcn.unwrap()(
                        p1.into_raw(),
                        p2.into_raw(),
                        4.5,
                        color.into_raw(),
                        context,
                    );
                    dd.DrawLineFcn.unwrap()(
                        p1.into_raw(),
                        p2.into_raw(),
                        color.into_raw(),
                        context,
                    );
                    dd.DrawTransformFcn.unwrap()(transform.into_raw(), context);
                    dd.DrawPointFcn.unwrap()(p1.into_raw(), 5.5, color.into_raw(), context);
                    dd.DrawStringFcn.unwrap()(
                        p2.into_raw(),
                        c"debug".as_ptr(),
                        color.into_raw(),
                        context,
                    );
                    dd.DrawBoundsFcn.unwrap()(bounds.into_raw(), color.into_raw(), context);
                }
            }
            collector.finish();
        }

        assert!(!panic.has_panicked());
        assert_eq!(commands.len(), 10);

        let DebugDrawCmd::Polygon {
            transform: actual_transform,
            vertices: actual_vertices,
            color: actual_color,
        } = &commands[0]
        else {
            panic!("expected polygon command");
        };
        assert_position_eq(actual_transform.position(), transform.position());
        assert_eq!(actual_vertices, &vertices);
        assert_eq!(*actual_color, color);

        let DebugDrawCmd::SolidPolygon {
            transform: actual_transform,
            vertices: actual_vertices,
            radius,
            color: actual_color,
        } = &commands[1]
        else {
            panic!("expected solid polygon command");
        };
        assert_position_eq(actual_transform.position(), transform.position());
        assert_eq!(actual_vertices, &vertices);
        assert_eq!(*radius, 0.125);
        assert_eq!(*actual_color, color);

        let DebugDrawCmd::Circle {
            center,
            radius,
            color: actual_color,
        } = commands[2]
        else {
            panic!("expected circle command");
        };
        assert_position_eq(center, circle_center);
        assert_eq!(radius, 2.5);
        assert_eq!(actual_color, color);

        let DebugDrawCmd::SolidCircle {
            transform: actual_transform,
            center,
            radius,
            color: actual_color,
        } = commands[3]
        else {
            panic!("expected solid circle command");
        };
        assert_position_eq(actual_transform.position(), transform.position());
        assert_eq!(center, local_center);
        assert_eq!(radius, 3.5);
        assert_eq!(actual_color, color);

        let DebugDrawCmd::SolidCapsule {
            p1: actual_p1,
            p2: actual_p2,
            radius,
            color: actual_color,
        } = commands[4]
        else {
            panic!("expected solid capsule command");
        };
        assert_position_eq(actual_p1, p1);
        assert_position_eq(actual_p2, p2);
        assert_eq!(radius, 4.5);
        assert_eq!(actual_color, color);

        let DebugDrawCmd::Segment {
            p1: actual_p1,
            p2: actual_p2,
            color: actual_color,
        } = commands[5]
        else {
            panic!("expected segment command");
        };
        assert_position_eq(actual_p1, p1);
        assert_position_eq(actual_p2, p2);
        assert_eq!(actual_color, color);

        let DebugDrawCmd::Transform(actual_transform) = commands[6] else {
            panic!("expected transform command");
        };
        assert_position_eq(actual_transform.position(), transform.position());

        let DebugDrawCmd::Point {
            p,
            size,
            color: actual_color,
        } = commands[7]
        else {
            panic!("expected point command");
        };
        assert_position_eq(p, p1);
        assert_eq!(size, 5.5);
        assert_eq!(actual_color, color);

        let DebugDrawCmd::String {
            p,
            ref s,
            color: actual_color,
        } = commands[8]
        else {
            panic!("expected string command");
        };
        assert_position_eq(p, p2);
        assert_eq!(s, "debug");
        assert_eq!(actual_color, color);

        let DebugDrawCmd::Bounds {
            bounds: actual_bounds,
            color: actual_color,
        } = commands[9]
        else {
            panic!("expected bounds command");
        };
        assert_eq!(actual_bounds, bounds);
        assert_eq!(actual_color, color);
    }

    #[test]
    fn options_and_callback_slots_match_box2d_32() {
        let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
        install_debug_draw_callbacks(&mut dd);
        let context = core::ptr::NonNull::<u8>::dangling().as_ptr().cast();
        let bounds = Aabb::new([-11.0, -12.0], [13.0, 14.0]).unwrap();
        let opts = DebugDrawOptions {
            drawing_bounds: bounds,
            force_scale: 2.25,
            joint_scale: 3.5,
            draw_contacts: true,
            draw_anchor_a: false,
            draw_shapes: true,
            draw_chain_normals: false,
            draw_joints: true,
            draw_joint_extras: false,
            draw_bounds: true,
            draw_mass: false,
            draw_body_names: true,
            draw_graph_colors: false,
            draw_contact_features: true,
            draw_contact_normals: false,
            draw_contact_forces: true,
            draw_friction_forces: false,
            draw_islands: true,
        };
        apply_debug_draw_options(&mut dd, opts, context);

        assert!(dd.DrawPolygonFcn.is_some());
        assert!(dd.DrawSolidPolygonFcn.is_some());
        assert!(dd.DrawCircleFcn.is_some());
        assert!(dd.DrawSolidCircleFcn.is_some());
        assert!(dd.DrawSolidCapsuleFcn.is_some());
        assert!(dd.DrawLineFcn.is_some());
        assert!(dd.DrawTransformFcn.is_some());
        assert!(dd.DrawPointFcn.is_some());
        assert!(dd.DrawStringFcn.is_some());
        assert!(dd.DrawBoundsFcn.is_some());
        assert_eq!(Aabb::from_raw(dd.drawingBounds).unwrap(), bounds);
        assert_eq!(dd.forceScale, 2.25);
        assert_eq!(dd.jointScale, 3.5);
        assert!(dd.drawContacts);
        assert!(!dd.drawAnchorA);
        assert!(dd.drawShapes);
        assert!(!dd.drawChainNormals);
        assert!(dd.drawJoints);
        assert!(!dd.drawJointExtras);
        assert!(dd.drawBounds);
        assert!(!dd.drawMass);
        assert!(dd.drawBodyNames);
        assert!(!dd.drawGraphColors);
        assert!(dd.drawContactFeatures);
        assert!(!dd.drawContactNormals);
        assert!(dd.drawContactForces);
        assert!(!dd.drawFrictionForces);
        assert!(dd.drawIslands);
        assert_eq!(dd.context, context);
    }

    #[derive(Default)]
    struct PolygonLengths(Vec<usize>);

    impl DebugDraw for PolygonLengths {
        fn draw_polygon(
            &mut self,
            _transform: WorldTransform,
            vertices: &[Vec2],
            _color: HexColor,
        ) {
            self.0.push(vertices.len());
        }
    }

    #[test]
    fn polygon_callback_fails_closed_on_an_invalid_native_buffer() {
        let mut lengths = PolygonLengths::default();
        let mut panic = crate::core::callback_state::PanicSlot::default();
        let error = {
            let drawer: &mut dyn DebugDraw = &mut lengths;
            let mut adapter = SafeDebugDrawAdapter {
                drawer,
                operation: "test",
                error: None,
            };
            {
                let adapter: &mut dyn NativeDebugDraw = &mut adapter;
                let mut ctx = test_context(adapter, &mut panic);
                let context = core::ptr::from_mut(&mut ctx).cast::<core::ffi::c_void>();
                let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
                install_debug_draw_callbacks(&mut dd);
                let vertex = Vec2::new(1.0, 2.0).into_raw();

                unsafe {
                    dd.DrawPolygonFcn.unwrap()(
                        WorldTransform::IDENTITY.into_raw(),
                        core::ptr::null(),
                        -1,
                        HexColor::WHITE.into_raw(),
                        context,
                    );
                    dd.DrawPolygonFcn.unwrap()(
                        WorldTransform::IDENTITY.into_raw(),
                        core::ptr::null(),
                        0,
                        HexColor::WHITE.into_raw(),
                        context,
                    );
                    dd.DrawPolygonFcn.unwrap()(
                        WorldTransform::IDENTITY.into_raw(),
                        &vertex,
                        1,
                        HexColor::WHITE.into_raw(),
                        context,
                    );
                }
            }
            adapter.error
        };

        assert!(!panic.has_panicked());
        assert!(lengths.0.is_empty());
        assert_eq!(
            error,
            Some(crate::Error::InvalidNativeOutput {
                operation: "test",
                output: "vertices",
                constraint: "a non-negative bounded count and a non-null aligned pointer when non-empty",
            })
        );
    }

    #[test]
    fn polygon_vertex_buffer_rejects_counts_above_the_box2d_limit() {
        let mut visited = false;
        let valid = unsafe {
            with_ffi_debug_draw_vertices(
                core::ptr::NonNull::<ffi::b2Vec2>::dangling().as_ptr(),
                ffi::B2_MAX_POLYGON_VERTICES as i32 + 1,
                |_| visited = true,
            )
        };

        assert!(!valid);
        assert!(!visited);
    }

    #[test]
    fn debug_draw_context_rejects_null_and_misaligned_native_pointers() {
        assert!(unsafe { native_debug_draw_context(core::ptr::null_mut()) }.is_none());
        let misaligned =
            core::ptr::without_provenance_mut::<core::ffi::c_void>(1).cast::<core::ffi::c_void>();
        assert!(unsafe { native_debug_draw_context(misaligned) }.is_none());
    }

    #[test]
    fn polygon_vertex_buffer_rejects_an_overflowing_address_range() {
        let align_mask = core::mem::align_of::<ffi::b2Vec2>() - 1;
        let pointer = core::ptr::without_provenance::<ffi::b2Vec2>(usize::MAX & !align_mask);
        let mut visited = false;
        let valid = unsafe {
            with_ffi_debug_draw_vertices(pointer, 1, |_| {
                visited = true;
            })
        };

        assert!(!valid);
        assert!(!visited);
    }

    #[test]
    fn string_callback_fails_closed_on_a_null_native_pointer() {
        #[derive(Default)]
        struct CountDraws(usize);

        impl DebugDraw for CountDraws {
            fn draw_string(&mut self, _p: Position, _s: &str, _color: HexColor) {
                self.0 += 1;
            }

            fn draw_point(&mut self, _p: Position, _size: f32, _color: HexColor) {
                self.0 += 1;
            }
        }

        let mut draws = CountDraws::default();
        let mut panic = crate::core::callback_state::PanicSlot::default();
        let error = {
            let drawer: &mut dyn DebugDraw = &mut draws;
            let mut adapter = SafeDebugDrawAdapter {
                drawer,
                operation: "test",
                error: None,
            };
            {
                let adapter: &mut dyn NativeDebugDraw = &mut adapter;
                let mut ctx = test_context(adapter, &mut panic);
                let context = core::ptr::from_mut(&mut ctx).cast::<core::ffi::c_void>();
                let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
                install_debug_draw_callbacks(&mut dd);

                unsafe {
                    dd.DrawStringFcn.unwrap()(
                        Position::ZERO.into_raw(),
                        core::ptr::null(),
                        HexColor::WHITE.into_raw(),
                        context,
                    );
                    dd.DrawPointFcn.unwrap()(
                        Position::ZERO.into_raw(),
                        1.0,
                        HexColor::WHITE.into_raw(),
                        context,
                    );
                }
            }
            adapter.error
        };

        assert!(!panic.has_panicked());
        assert_eq!(draws.0, 0);
        assert_eq!(
            error,
            Some(crate::Error::InvalidNativeOutput {
                operation: "test",
                output: "string",
                constraint: "a non-null NUL-terminated string pointer",
            })
        );
    }

    #[derive(Default)]
    struct PanicDraw {
        point_calls: usize,
    }

    impl DebugDraw for PanicDraw {
        fn draw_circle(&mut self, _center: Position, _radius: f32, _color: HexColor) {
            panic!("debug draw panic");
        }

        fn draw_point(&mut self, _p: Position, _size: f32, _color: HexColor) {
            self.point_calls += 1;
        }
    }

    #[test]
    fn callback_panic_is_captured_before_returning_to_box2d() {
        let mut drawer = PanicDraw::default();
        let mut panic = crate::core::callback_state::PanicSlot::default();
        {
            let safe_drawer: &mut dyn DebugDraw = &mut drawer;
            let mut adapter = SafeDebugDrawAdapter {
                drawer: safe_drawer,
                operation: "test",
                error: None,
            };
            let adapter: &mut dyn NativeDebugDraw = &mut adapter;
            let mut ctx = test_context(adapter, &mut panic);
            let context = core::ptr::from_mut(&mut ctx).cast::<core::ffi::c_void>();
            let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
            install_debug_draw_callbacks(&mut dd);

            unsafe {
                dd.DrawCircleFcn.unwrap()(
                    Position::ZERO.into_raw(),
                    1.0,
                    HexColor::RED.into_raw(),
                    context,
                );
                dd.DrawPointFcn.unwrap()(
                    Position::ZERO.into_raw(),
                    1.0,
                    HexColor::RED.into_raw(),
                    context,
                );
            }
        }

        assert!(panic.has_panicked());
        assert_eq!(drawer.point_calls, 0);
    }
}
