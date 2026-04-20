//! Debug Draw bridge to Box2D v3 callbacks.
//!
//! Implement the `DebugDraw` trait to receive drawing commands and call `World::debug_draw` each
//! step with `DebugDrawOptions` to render. Colors use the crate-owned [`HexColor`] type, which
//! stores Box2D's packed `0xRRGGBB` convention.
//!
//! Example
//! ```no_run
//! use boxdd::{DebugDraw, DebugDrawOptions, HexColor, Vec2, World, WorldDef};
//! struct Printer;
//! impl DebugDraw for Printer {
//!     fn draw_polygon(&mut self, vertices: &[Vec2], color: HexColor) {
//!         println!("poly {} color={:#x}", vertices.len(), color.rgb_u32());
//!     }
//! }
//! # let def = WorldDef::builder().build();
//! # let mut world = World::new(def).unwrap();
//! let mut cmds = Vec::new();
//! world.debug_draw_collect_into(&mut cmds, DebugDrawOptions::default());
//! let mut drawer = Printer;
//! for cmd in cmds {
//!     let _ = cmd;
//! }
//! ```
use crate::Transform;
use crate::types::Vec2;
use crate::world::World;
use boxdd_sys::ffi;
use smallvec::SmallVec;
use std::any::Any;
use std::ffi::CStr;

type DebugDrawPanic = Box<dyn Any + Send + 'static>;

/// Packed Box2D debug-draw RGB color (`0xRRGGBB`).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct HexColor(u32);

impl HexColor {
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
        Self(rgb & 0x00ff_ffff)
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

const _: () = {
    assert!(core::mem::size_of::<HexColor>() == core::mem::size_of::<ffi::b2HexColor>());
    assert!(core::mem::align_of::<HexColor>() == core::mem::align_of::<ffi::b2HexColor>());
};

#[derive(Clone, Debug)]
pub enum DebugDrawCmd {
    Polygon {
        vertices: Vec<Vec2>,
        color: HexColor,
    },
    SolidPolygon {
        transform: Transform,
        vertices: Vec<Vec2>,
        radius: f32,
        color: HexColor,
    },
    Circle {
        center: Vec2,
        radius: f32,
        color: HexColor,
    },
    SolidCircle {
        transform: Transform,
        radius: f32,
        color: HexColor,
    },
    SolidCapsule {
        p1: Vec2,
        p2: Vec2,
        radius: f32,
        color: HexColor,
    },
    Segment {
        p1: Vec2,
        p2: Vec2,
        color: HexColor,
    },
    Transform(Transform),
    Point {
        p: Vec2,
        size: f32,
        color: HexColor,
    },
    String {
        p: Vec2,
        s: String,
        color: HexColor,
    },
}

// Safe debug draw trait (no ffi types)
pub trait DebugDraw {
    fn draw_polygon(&mut self, _vertices: &[Vec2], _color: HexColor) {}
    fn draw_solid_polygon(
        &mut self,
        _transform: Transform,
        _vertices: &[Vec2],
        _radius: f32,
        _color: HexColor,
    ) {
    }
    fn draw_circle(&mut self, _center: Vec2, _radius: f32, _color: HexColor) {}
    fn draw_solid_circle(&mut self, _transform: Transform, _radius: f32, _color: HexColor) {}
    fn draw_solid_capsule(&mut self, _p1: Vec2, _p2: Vec2, _radius: f32, _color: HexColor) {}
    fn draw_segment(&mut self, _p1: Vec2, _p2: Vec2, _color: HexColor) {}
    fn draw_transform(&mut self, _transform: Transform) {}
    fn draw_point(&mut self, _p: Vec2, _size: f32, _color: HexColor) {}
    fn draw_string(&mut self, _p: Vec2, _s: &str, _color: HexColor) {}
}

// Raw low-level trait (kept for performance/zero-copy use-cases)
pub trait RawDebugDraw {
    fn draw_polygon(&mut self, _vertices: &[ffi::b2Vec2], _color: HexColor) {}
    fn draw_solid_polygon(
        &mut self,
        _transform: ffi::b2Transform,
        _vertices: &[ffi::b2Vec2],
        _radius: f32,
        _color: HexColor,
    ) {
    }
    fn draw_circle(&mut self, _center: ffi::b2Vec2, _radius: f32, _color: HexColor) {}
    fn draw_solid_circle(&mut self, _transform: ffi::b2Transform, _radius: f32, _color: HexColor) {}
    fn draw_solid_capsule(
        &mut self,
        _p1: ffi::b2Vec2,
        _p2: ffi::b2Vec2,
        _radius: f32,
        _color: HexColor,
    ) {
    }
    fn draw_segment(&mut self, _p1: ffi::b2Vec2, _p2: ffi::b2Vec2, _color: HexColor) {}
    fn draw_transform(&mut self, _transform: ffi::b2Transform) {}
    fn draw_point(&mut self, _p: ffi::b2Vec2, _size: f32, _color: HexColor) {}
    fn draw_string(&mut self, _p: ffi::b2Vec2, _s: &CStr, _color: HexColor) {}
}

#[derive(Copy, Clone, Debug)]
pub struct DebugDrawOptions {
    pub drawing_bounds: ffi::b2AABB,
    pub force_scale: f32,
    pub joint_scale: f32,
    pub draw_shapes: bool,
    pub draw_joints: bool,
    pub draw_joint_extras: bool,
    pub draw_bounds: bool,
    pub draw_mass: bool,
    pub draw_body_names: bool,
    /// Draw contact points (upstream name: `drawContactPoints`).
    pub draw_contacts: bool,
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
            drawing_bounds: ffi::b2AABB {
                lowerBound: ffi::b2Vec2 {
                    x: -1.0e9,
                    y: -1.0e9,
                },
                upperBound: ffi::b2Vec2 { x: 1.0e9, y: 1.0e9 },
            },
            force_scale: 1.0,
            joint_scale: 1.0,
            draw_shapes: true,
            draw_joints: true,
            draw_joint_extras: false,
            draw_bounds: false,
            draw_mass: false,
            draw_body_names: false,
            draw_contacts: false,
            draw_graph_colors: false,
            draw_contact_features: false,
            draw_contact_normals: false,
            draw_contact_forces: false,
            draw_friction_forces: false,
            draw_islands: false,
        }
    }
}

struct DebugDrawCtx<'a, T: ?Sized> {
    drawer: &'a mut T,
    panicked: &'a mut bool,
    panic: &'a mut Option<DebugDrawPanic>,
}

type SafeDebugCtx<'a> = DebugDrawCtx<'a, dyn DebugDraw + 'a>;
type RawDebugCtx<'a> = DebugDrawCtx<'a, dyn RawDebugDraw + 'a>;

#[inline]
fn run_debug_draw_callback<T: ?Sized>(ctx: &mut DebugDrawCtx<'_, T>, f: impl FnOnce(&mut T)) {
    if *ctx.panicked {
        return;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = crate::core::callback_state::CallbackGuard::enter();
        f(ctx.drawer);
    }));
    if let Err(p) = result {
        *ctx.panicked = true;
        *ctx.panic = Some(p);
    }
}

#[inline]
unsafe fn ffi_debug_draw_vertices<'a>(
    vertices: *const ffi::b2Vec2,
    count: i32,
) -> Option<&'a [ffi::b2Vec2]> {
    let n = count.max(0) as usize;
    if n == 0 || vertices.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(vertices, n) })
    }
}

unsafe fn safe_debug_draw_vertices(
    vertices: *const ffi::b2Vec2,
    count: i32,
) -> Option<SmallVec<[Vec2; 8]>> {
    let src = unsafe { ffi_debug_draw_vertices(vertices, count) }?;
    let mut verts: SmallVec<[Vec2; 8]> = SmallVec::with_capacity(src.len().min(8));
    verts.extend(src.iter().copied().map(Vec2::from));
    Some(verts)
}

fn apply_debug_draw_options(
    dd: &mut ffi::b2DebugDraw,
    opts: DebugDrawOptions,
    context: *mut core::ffi::c_void,
) {
    dd.drawingBounds = opts.drawing_bounds;
    dd.forceScale = opts.force_scale;
    dd.jointScale = opts.joint_scale;
    dd.drawShapes = opts.draw_shapes;
    dd.drawJoints = opts.draw_joints;
    dd.drawJointExtras = opts.draw_joint_extras;
    dd.drawBounds = opts.draw_bounds;
    dd.drawMass = opts.draw_mass;
    dd.drawBodyNames = opts.draw_body_names;
    dd.drawContactPoints = opts.draw_contacts;
    dd.drawGraphColors = opts.draw_graph_colors;
    dd.drawContactFeatures = opts.draw_contact_features;
    dd.drawContactNormals = opts.draw_contact_normals;
    dd.drawContactForces = opts.draw_contact_forces;
    dd.drawFrictionForces = opts.draw_friction_forces;
    dd.drawIslands = opts.draw_islands;
    dd.context = context;
}

fn finish_debug_draw(world: &World, panic: &mut Option<DebugDrawPanic>) {
    world.core_arc().process_deferred_destroys();
    if let Some(p) = panic.take() {
        std::panic::resume_unwind(p);
    }
}

struct CollectDebugDraw<'a> {
    cmds: &'a mut Vec<DebugDrawCmd>,
    len: usize,
}

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

impl DebugDraw for CollectDebugDraw<'_> {
    fn draw_polygon(&mut self, vertices: &[Vec2], color: HexColor) {
        match self.cmds.get_mut(self.len) {
            Some(DebugDrawCmd::Polygon {
                vertices: stored,
                color: stored_color,
            }) => {
                stored.clear();
                stored.extend_from_slice(vertices);
                *stored_color = color;
                self.len += 1;
            }
            _ => self.replace_or_push(DebugDrawCmd::Polygon {
                vertices: vertices.to_vec(),
                color,
            }),
        }
    }

    fn draw_solid_polygon(
        &mut self,
        transform: Transform,
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

    fn draw_circle(&mut self, center: Vec2, radius: f32, color: HexColor) {
        self.replace_or_push(DebugDrawCmd::Circle {
            center,
            radius,
            color,
        });
    }

    fn draw_solid_circle(&mut self, transform: Transform, radius: f32, color: HexColor) {
        self.replace_or_push(DebugDrawCmd::SolidCircle {
            transform,
            radius,
            color,
        });
    }

    fn draw_solid_capsule(&mut self, p1: Vec2, p2: Vec2, radius: f32, color: HexColor) {
        self.replace_or_push(DebugDrawCmd::SolidCapsule {
            p1,
            p2,
            radius,
            color,
        });
    }

    fn draw_segment(&mut self, p1: Vec2, p2: Vec2, color: HexColor) {
        self.replace_or_push(DebugDrawCmd::Segment { p1, p2, color });
    }

    fn draw_transform(&mut self, transform: Transform) {
        self.replace_or_push(DebugDrawCmd::Transform(transform));
    }

    fn draw_point(&mut self, p: Vec2, size: f32, color: HexColor) {
        self.replace_or_push(DebugDrawCmd::Point { p, size, color });
    }

    fn draw_string(&mut self, p: Vec2, s: &str, color: HexColor) {
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
}

impl World {
    /// Collect debug draw commands into a vector (fully safe).
    ///
    /// This calls into Box2D debug draw but does not invoke user code during the draw.
    pub fn debug_draw_collect(&mut self, opts: DebugDrawOptions) -> Vec<DebugDrawCmd> {
        crate::core::callback_state::assert_not_in_callback();
        let mut cmds = Vec::new();
        self.debug_draw_collect_into(&mut cmds, opts);
        cmds
    }

    /// Collect debug draw commands into a caller-owned buffer.
    ///
    /// This reuses the outer command buffer and, when the command sequence stays
    /// stable, also reuses nested polygon vertex and string storage.
    pub fn debug_draw_collect_into(&mut self, out: &mut Vec<DebugDrawCmd>, opts: DebugDrawOptions) {
        crate::core::callback_state::assert_not_in_callback();
        let mut collector = CollectDebugDraw::new(out);
        self.debug_draw(&mut collector, opts);
        collector.finish();
    }

    // Safe wrapper: converts to Vec2/Transform and &str
    ///
    /// Box2D invokes the draw callbacks while traversing internal world state. During this call,
    /// any attempt to call into the Box2D world through `boxdd` will panic, since the world is
    /// considered locked by Box2D.
    pub fn debug_draw(&mut self, drawer: &mut impl DebugDraw, opts: DebugDrawOptions) {
        crate::core::callback_state::assert_not_in_callback();
        let mut panicked = false;
        let mut panic: Option<DebugDrawPanic> = None;
        let drawer: &mut dyn DebugDraw = drawer;
        let mut ctx = SafeDebugCtx {
            drawer,
            panicked: &mut panicked,
            panic: &mut panic,
        };
        let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
        // Hook callbacks
        unsafe extern "C" fn draw_polygon_cb(
            vertices: *const ffi::b2Vec2,
            count: i32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut SafeDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            let Some(verts) = (unsafe { safe_debug_draw_vertices(vertices, count) }) else {
                return;
            };
            run_debug_draw_callback(ctx, |drawer| drawer.draw_polygon(&verts, color));
        }
        unsafe extern "C" fn draw_solid_polygon_cb(
            transform: ffi::b2Transform,
            vertices: *const ffi::b2Vec2,
            count: i32,
            radius: f32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut SafeDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            let Some(verts) = (unsafe { safe_debug_draw_vertices(vertices, count) }) else {
                return;
            };
            let transform = Transform::from(transform);
            run_debug_draw_callback(ctx, |drawer| {
                drawer.draw_solid_polygon(transform, &verts, radius, color);
            });
        }
        unsafe extern "C" fn draw_circle_cb(
            center: ffi::b2Vec2,
            radius: f32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut SafeDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            let center = Vec2::from(center);
            run_debug_draw_callback(ctx, |drawer| drawer.draw_circle(center, radius, color));
        }
        unsafe extern "C" fn draw_solid_circle_cb(
            transform: ffi::b2Transform,
            radius: f32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut SafeDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            let transform = Transform::from(transform);
            run_debug_draw_callback(ctx, |drawer| {
                drawer.draw_solid_circle(transform, radius, color);
            });
        }
        unsafe extern "C" fn draw_solid_capsule_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            radius: f32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut SafeDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            let p1 = Vec2::from(p1);
            let p2 = Vec2::from(p2);
            run_debug_draw_callback(ctx, |drawer| {
                drawer.draw_solid_capsule(p1, p2, radius, color);
            });
        }
        unsafe extern "C" fn draw_line_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut SafeDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            let p1 = Vec2::from(p1);
            let p2 = Vec2::from(p2);
            run_debug_draw_callback(ctx, |drawer| drawer.draw_segment(p1, p2, color));
        }
        unsafe extern "C" fn draw_transform_cb(
            transform: ffi::b2Transform,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut SafeDebugCtx<'_>) };
            let transform = Transform::from(transform);
            run_debug_draw_callback(ctx, |drawer| drawer.draw_transform(transform));
        }
        unsafe extern "C" fn draw_point_cb(
            p: ffi::b2Vec2,
            size: f32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut SafeDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            let p = Vec2::from(p);
            run_debug_draw_callback(ctx, |drawer| drawer.draw_point(p, size, color));
        }
        unsafe extern "C" fn draw_string_cb(
            p: ffi::b2Vec2,
            s: *const core::ffi::c_char,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut SafeDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            if !s.is_null() {
                let cs = unsafe { CStr::from_ptr(s) };
                let s = cs.to_string_lossy();
                let p = Vec2::from(p);
                run_debug_draw_callback(ctx, |drawer| drawer.draw_string(p, &s, color));
            }
        }

        dd.DrawPolygonFcn = Some(draw_polygon_cb);
        dd.DrawSolidPolygonFcn = Some(draw_solid_polygon_cb);
        dd.DrawCircleFcn = Some(draw_circle_cb);
        dd.DrawSolidCircleFcn = Some(draw_solid_circle_cb);
        dd.DrawSolidCapsuleFcn = Some(draw_solid_capsule_cb);
        dd.DrawLineFcn = Some(draw_line_cb);
        dd.DrawTransformFcn = Some(draw_transform_cb);
        dd.DrawPointFcn = Some(draw_point_cb);
        dd.DrawStringFcn = Some(draw_string_cb);
        apply_debug_draw_options(&mut dd, opts, &mut ctx as *mut _ as *mut _);

        unsafe { ffi::b2World_Draw(self.raw(), &mut dd) };
        finish_debug_draw(self, &mut panic);
    }

    // Raw path: zero-copy FFI types to trait
    ///
    /// Box2D invokes the draw callbacks while traversing internal world state. During this call,
    /// any attempt to call into the Box2D world through `boxdd` will panic, since the world is
    /// considered locked by Box2D.
    pub fn debug_draw_raw(&mut self, drawer: &mut impl RawDebugDraw, opts: DebugDrawOptions) {
        crate::core::callback_state::assert_not_in_callback();
        let mut panicked = false;
        let mut panic: Option<DebugDrawPanic> = None;
        let drawer: &mut dyn RawDebugDraw = drawer;
        let mut ctx = RawDebugCtx {
            drawer,
            panicked: &mut panicked,
            panic: &mut panic,
        };
        let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
        unsafe extern "C" fn draw_polygon_cb(
            vertices: *const ffi::b2Vec2,
            count: i32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            let Some(vertices) = (unsafe { ffi_debug_draw_vertices(vertices, count) }) else {
                return;
            };
            run_debug_draw_callback(ctx, |drawer| drawer.draw_polygon(vertices, color));
        }
        unsafe extern "C" fn draw_solid_polygon_cb(
            transform: ffi::b2Transform,
            vertices: *const ffi::b2Vec2,
            count: i32,
            radius: f32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            let Some(vertices) = (unsafe { ffi_debug_draw_vertices(vertices, count) }) else {
                return;
            };
            run_debug_draw_callback(ctx, |drawer| {
                drawer.draw_solid_polygon(transform, vertices, radius, color);
            });
        }
        unsafe extern "C" fn draw_circle_cb(
            center: ffi::b2Vec2,
            radius: f32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            run_debug_draw_callback(ctx, |drawer| drawer.draw_circle(center, radius, color));
        }
        unsafe extern "C" fn draw_solid_circle_cb(
            transform: ffi::b2Transform,
            radius: f32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            run_debug_draw_callback(ctx, |drawer| {
                drawer.draw_solid_circle(transform, radius, color);
            });
        }
        unsafe extern "C" fn draw_solid_capsule_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            radius: f32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            run_debug_draw_callback(ctx, |drawer| {
                drawer.draw_solid_capsule(p1, p2, radius, color);
            });
        }
        unsafe extern "C" fn draw_line_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            run_debug_draw_callback(ctx, |drawer| drawer.draw_segment(p1, p2, color));
        }
        unsafe extern "C" fn draw_transform_cb(
            transform: ffi::b2Transform,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx<'_>) };
            run_debug_draw_callback(ctx, |drawer| drawer.draw_transform(transform));
        }
        unsafe extern "C" fn draw_point_cb(
            p: ffi::b2Vec2,
            size: f32,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            run_debug_draw_callback(ctx, |drawer| drawer.draw_point(p, size, color));
        }
        unsafe extern "C" fn draw_string_cb(
            p: ffi::b2Vec2,
            s: *const core::ffi::c_char,
            color: ffi::b2HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx<'_>) };
            let color = HexColor::from_raw(color);
            if !s.is_null() {
                let cs = unsafe { CStr::from_ptr(s) };
                run_debug_draw_callback(ctx, |drawer| drawer.draw_string(p, cs, color));
            }
        }
        dd.DrawPolygonFcn = Some(draw_polygon_cb);
        dd.DrawSolidPolygonFcn = Some(draw_solid_polygon_cb);
        dd.DrawCircleFcn = Some(draw_circle_cb);
        dd.DrawSolidCircleFcn = Some(draw_solid_circle_cb);
        dd.DrawSolidCapsuleFcn = Some(draw_solid_capsule_cb);
        dd.DrawLineFcn = Some(draw_line_cb);
        dd.DrawTransformFcn = Some(draw_transform_cb);
        dd.DrawPointFcn = Some(draw_point_cb);
        dd.DrawStringFcn = Some(draw_string_cb);
        apply_debug_draw_options(&mut dd, opts, &mut ctx as *mut _ as *mut _);
        unsafe { ffi::b2World_Draw(self.raw(), &mut dd) };
        finish_debug_draw(self, &mut panic);
    }
}
