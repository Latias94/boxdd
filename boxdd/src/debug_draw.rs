//! Debug Draw bridge to Box2D v3 callbacks.
//!
//! Implement the `DebugDraw` trait to receive drawing commands and call `World::debug_draw` each
//! step with `DebugDrawOptions` to render. Color is a packed integer (`b2HexColor`), compatible with
//! Box2D's debug draw convention.
//!
//! Example
//! ```no_run
//! use boxdd::{World, WorldDef, DebugDraw, DebugDrawOptions, Vec2};
//! use boxdd_sys::ffi;
//! use std::ffi::CStr;
//! struct Printer;
//! impl DebugDraw for Printer {
//!     fn draw_polygon(&mut self, vertices: &[Vec2], color: u32) {
//!         println!("poly {} color={:#x}", vertices.len(), color);
//!     }
//! }
//! # let def = WorldDef::builder().build();
//! # let mut world = World::new(def).unwrap();
//! let mut drawer = Printer;
//! for cmd in world.debug_draw_collect(DebugDrawOptions::default()) {
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

pub type HexColor = ffi::b2HexColor;

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

struct DebugCtx<'a> {
    drawer: &'a mut dyn DebugDraw,
    panicked: &'a mut bool,
    panic: &'a mut Option<Box<dyn Any + Send + 'static>>,
}
struct RawDebugCtx<'a> {
    drawer: &'a mut dyn RawDebugDraw,
    panicked: &'a mut bool,
    panic: &'a mut Option<Box<dyn Any + Send + 'static>>,
}

impl World {
    /// Collect debug draw commands into a vector (fully safe).
    ///
    /// This calls into Box2D debug draw but does not invoke user code during the draw.
    pub fn debug_draw_collect(&mut self, opts: DebugDrawOptions) -> Vec<DebugDrawCmd> {
        crate::core::callback_state::assert_not_in_callback();
        struct Collector {
            cmds: Vec<DebugDrawCmd>,
        }
        impl DebugDraw for Collector {
            fn draw_polygon(&mut self, vertices: &[Vec2], color: HexColor) {
                self.cmds.push(DebugDrawCmd::Polygon {
                    vertices: vertices.to_vec(),
                    color,
                });
            }
            fn draw_solid_polygon(
                &mut self,
                transform: Transform,
                vertices: &[Vec2],
                radius: f32,
                color: HexColor,
            ) {
                self.cmds.push(DebugDrawCmd::SolidPolygon {
                    transform,
                    vertices: vertices.to_vec(),
                    radius,
                    color,
                });
            }
            fn draw_circle(&mut self, center: Vec2, radius: f32, color: HexColor) {
                self.cmds.push(DebugDrawCmd::Circle {
                    center,
                    radius,
                    color,
                });
            }
            fn draw_solid_circle(&mut self, transform: Transform, radius: f32, color: HexColor) {
                self.cmds.push(DebugDrawCmd::SolidCircle {
                    transform,
                    radius,
                    color,
                });
            }
            fn draw_solid_capsule(&mut self, p1: Vec2, p2: Vec2, radius: f32, color: HexColor) {
                self.cmds.push(DebugDrawCmd::SolidCapsule {
                    p1,
                    p2,
                    radius,
                    color,
                });
            }
            fn draw_segment(&mut self, p1: Vec2, p2: Vec2, color: HexColor) {
                self.cmds.push(DebugDrawCmd::Segment { p1, p2, color });
            }
            fn draw_transform(&mut self, transform: Transform) {
                self.cmds.push(DebugDrawCmd::Transform(transform));
            }
            fn draw_point(&mut self, p: Vec2, size: f32, color: HexColor) {
                self.cmds.push(DebugDrawCmd::Point { p, size, color });
            }
            fn draw_string(&mut self, p: Vec2, s: &str, color: HexColor) {
                self.cmds.push(DebugDrawCmd::String {
                    p,
                    s: s.to_owned(),
                    color,
                });
            }
        }

        let mut c = Collector { cmds: Vec::new() };
        // SAFETY: `Collector` is internal and does not mutate the world during callbacks.
        unsafe { self.debug_draw(&mut c, opts) };
        c.cmds
    }

    // Safe wrapper: converts to Vec2/Transform and &str
    ///
    /// # Safety
    /// Box2D invokes the draw callbacks while traversing internal world state. During this call,
    /// the `drawer` must not mutate the world (including indirectly via dropping `Owned*` handles).
    pub unsafe fn debug_draw(&mut self, drawer: &mut impl DebugDraw, opts: DebugDrawOptions) {
        crate::core::callback_state::assert_not_in_callback();
        let mut panicked = false;
        let mut panic: Option<Box<dyn Any + Send + 'static>> = None;
        let mut ctx = DebugCtx {
            drawer,
            panicked: &mut panicked,
            panic: &mut panic,
        };
        let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
        // Hook callbacks
        unsafe extern "C" fn draw_polygon_cb(
            vertices: *const ffi::b2Vec2,
            count: i32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            if *ctx.panicked {
                return;
            }
            let n = count.max(0) as usize;
            if n == 0 || vertices.is_null() {
                return;
            }
            let src = unsafe { core::slice::from_raw_parts(vertices, n) };
            let mut verts: SmallVec<[Vec2; 8]> = SmallVec::with_capacity(src.len().min(8));
            for v in src.iter().copied() {
                verts.push(Vec2::from(v));
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_polygon(&verts, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_solid_polygon_cb(
            transform: ffi::b2Transform,
            vertices: *const ffi::b2Vec2,
            count: i32,
            radius: f32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            if *ctx.panicked {
                return;
            }
            let n = count.max(0) as usize;
            if n == 0 || vertices.is_null() {
                return;
            }
            let src = unsafe { core::slice::from_raw_parts(vertices, n) };
            let mut verts: SmallVec<[Vec2; 8]> = SmallVec::with_capacity(src.len().min(8));
            for v in src.iter().copied() {
                verts.push(Vec2::from(v));
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer
                    .draw_solid_polygon(Transform::from(transform), &verts, radius, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_circle_cb(
            center: ffi::b2Vec2,
            radius: f32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_circle(Vec2::from(center), radius, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_solid_circle_cb(
            transform: ffi::b2Transform,
            radius: f32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer
                    .draw_solid_circle(Transform::from(transform), radius, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_solid_capsule_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            radius: f32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer
                    .draw_solid_capsule(Vec2::from(p1), Vec2::from(p2), radius, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_line_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer
                    .draw_segment(Vec2::from(p1), Vec2::from(p2), color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_transform_cb(
            transform: ffi::b2Transform,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_transform(Transform::from(transform));
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_point_cb(
            p: ffi::b2Vec2,
            size: f32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_point(Vec2::from(p), size, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_string_cb(
            p: ffi::b2Vec2,
            s: *const core::ffi::c_char,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            if *ctx.panicked {
                return;
            }
            if !s.is_null() {
                let cs = unsafe { CStr::from_ptr(s) };
                let s = cs.to_string_lossy();
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _g = crate::core::callback_state::CallbackGuard::enter();
                    ctx.drawer.draw_string(Vec2::from(p), &s, color);
                }));
                if let Err(p) = r {
                    *ctx.panicked = true;
                    *ctx.panic = Some(p);
                }
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

        // Options
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
        dd.context = &mut ctx as *mut _ as *mut _;

        unsafe { ffi::b2World_Draw(self.raw(), &mut dd) };

        // Flush deferred destroys scheduled from draw callbacks.
        self.core_arc().process_deferred_destroys();

        if let Some(p) = panic.take() {
            std::panic::resume_unwind(p);
        }
    }

    // Raw path: zero-copy FFI types to trait
    ///
    /// # Safety
    /// Box2D invokes the draw callbacks while traversing internal world state. During this call,
    /// the `drawer` must not mutate the world (including indirectly via dropping `Owned*` handles).
    pub unsafe fn debug_draw_raw(
        &mut self,
        drawer: &mut impl RawDebugDraw,
        opts: DebugDrawOptions,
    ) {
        crate::core::callback_state::assert_not_in_callback();
        let mut panicked = false;
        let mut panic: Option<Box<dyn Any + Send + 'static>> = None;
        let mut ctx = RawDebugCtx {
            drawer,
            panicked: &mut panicked,
            panic: &mut panic,
        };
        let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
        unsafe extern "C" fn draw_polygon_cb(
            vertices: *const ffi::b2Vec2,
            count: i32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            if *ctx.panicked {
                return;
            }
            let n = count.max(0) as usize;
            if n == 0 || vertices.is_null() {
                return;
            }
            let slice = unsafe { core::slice::from_raw_parts(vertices, n) };
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_polygon(slice, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_solid_polygon_cb(
            transform: ffi::b2Transform,
            vertices: *const ffi::b2Vec2,
            count: i32,
            radius: f32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            if *ctx.panicked {
                return;
            }
            let n = count.max(0) as usize;
            if n == 0 || vertices.is_null() {
                return;
            }
            let slice = unsafe { core::slice::from_raw_parts(vertices, n) };
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer
                    .draw_solid_polygon(transform, slice, radius, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_circle_cb(
            center: ffi::b2Vec2,
            radius: f32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_circle(center, radius, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_solid_circle_cb(
            transform: ffi::b2Transform,
            radius: f32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_solid_circle(transform, radius, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_solid_capsule_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            radius: f32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_solid_capsule(p1, p2, radius, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_line_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_segment(p1, p2, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_transform_cb(
            transform: ffi::b2Transform,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_transform(transform);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_point_cb(
            p: ffi::b2Vec2,
            size: f32,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            if *ctx.panicked {
                return;
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = crate::core::callback_state::CallbackGuard::enter();
                ctx.drawer.draw_point(p, size, color);
            }));
            if let Err(p) = r {
                *ctx.panicked = true;
                *ctx.panic = Some(p);
            }
        }
        unsafe extern "C" fn draw_string_cb(
            p: ffi::b2Vec2,
            s: *const core::ffi::c_char,
            color: HexColor,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            if *ctx.panicked {
                return;
            }
            if !s.is_null() {
                let cs = unsafe { CStr::from_ptr(s) };
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _g = crate::core::callback_state::CallbackGuard::enter();
                    ctx.drawer.draw_string(p, cs, color);
                }));
                if let Err(p) = r {
                    *ctx.panicked = true;
                    *ctx.panic = Some(p);
                }
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
        dd.context = &mut ctx as *mut _ as *mut _;
        unsafe { ffi::b2World_Draw(self.raw(), &mut dd) };

        // Flush deferred destroys scheduled from draw callbacks.
        self.core_arc().process_deferred_destroys();

        if let Some(p) = panic.take() {
            std::panic::resume_unwind(p);
        }
    }
}
