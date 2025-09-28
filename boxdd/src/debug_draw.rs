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
//!     fn draw_polygon(&mut self, vertices: &[Vec2], color: i32) {
//!         println!("poly {} color={:#x}", vertices.len(), color);
//!     }
//! }
//! # let def = WorldDef::builder().build();
//! # let mut world = World::new(def).unwrap();
//! let mut drawer = Printer;
//! world.debug_draw(&mut drawer, DebugDrawOptions::default());
//! ```
use crate::Transform;
use crate::types::Vec2;
use crate::world::World;
use boxdd_sys::ffi;
use smallvec::SmallVec;
use std::ffi::CStr;

// Safe debug draw trait (no ffi types)
pub trait DebugDraw {
    fn draw_polygon(&mut self, _vertices: &[Vec2], _color: i32) {}
    fn draw_solid_polygon(
        &mut self,
        _transform: Transform,
        _vertices: &[Vec2],
        _radius: f32,
        _color: i32,
    ) {
    }
    fn draw_circle(&mut self, _center: Vec2, _radius: f32, _color: i32) {}
    fn draw_solid_circle(&mut self, _transform: Transform, _radius: f32, _color: i32) {}
    fn draw_solid_capsule(&mut self, _p1: Vec2, _p2: Vec2, _radius: f32, _color: i32) {}
    fn draw_segment(&mut self, _p1: Vec2, _p2: Vec2, _color: i32) {}
    fn draw_transform(&mut self, _transform: Transform) {}
    fn draw_point(&mut self, _p: Vec2, _size: f32, _color: i32) {}
    fn draw_string(&mut self, _p: Vec2, _s: &str, _color: i32) {}
}

// Raw low-level trait (kept for performance/zero-copy use-cases)
pub trait RawDebugDraw {
    fn draw_polygon(&mut self, _vertices: &[ffi::b2Vec2], _color: i32) {}
    fn draw_solid_polygon(
        &mut self,
        _transform: ffi::b2Transform,
        _vertices: &[ffi::b2Vec2],
        _radius: f32,
        _color: i32,
    ) {
    }
    fn draw_circle(&mut self, _center: ffi::b2Vec2, _radius: f32, _color: i32) {}
    fn draw_solid_circle(&mut self, _transform: ffi::b2Transform, _radius: f32, _color: i32) {}
    fn draw_solid_capsule(
        &mut self,
        _p1: ffi::b2Vec2,
        _p2: ffi::b2Vec2,
        _radius: f32,
        _color: i32,
    ) {
    }
    fn draw_segment(&mut self, _p1: ffi::b2Vec2, _p2: ffi::b2Vec2, _color: i32) {}
    fn draw_transform(&mut self, _transform: ffi::b2Transform) {}
    fn draw_point(&mut self, _p: ffi::b2Vec2, _size: f32, _color: i32) {}
    fn draw_string(&mut self, _p: ffi::b2Vec2, _s: &CStr, _color: i32) {}
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
}
struct RawDebugCtx<'a> {
    drawer: &'a mut dyn RawDebugDraw,
}

impl World {
    // Safe wrapper: converts to Vec2/Transform and &str
    pub fn debug_draw(&mut self, drawer: &mut impl DebugDraw, opts: DebugDrawOptions) {
        let mut ctx = DebugCtx { drawer };
        let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
        // Hook callbacks
        unsafe extern "C" fn draw_polygon_cb(
            vertices: *const ffi::b2Vec2,
            count: i32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            let src = unsafe { core::slice::from_raw_parts(vertices, count as usize) };
            let mut verts: SmallVec<[Vec2; 8]> = SmallVec::with_capacity(src.len().min(8));
            for v in src.iter().copied() {
                verts.push(Vec2::from(v));
            }
            ctx.drawer.draw_polygon(&verts, color);
        }
        unsafe extern "C" fn draw_solid_polygon_cb(
            transform: ffi::b2Transform,
            vertices: *const ffi::b2Vec2,
            count: i32,
            radius: f32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            let src = unsafe { core::slice::from_raw_parts(vertices, count as usize) };
            let mut verts: SmallVec<[Vec2; 8]> = SmallVec::with_capacity(src.len().min(8));
            for v in src.iter().copied() {
                verts.push(Vec2::from(v));
            }
            ctx.drawer
                .draw_solid_polygon(Transform::from(transform), &verts, radius, color);
        }
        unsafe extern "C" fn draw_circle_cb(
            center: ffi::b2Vec2,
            radius: f32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            ctx.drawer.draw_circle(Vec2::from(center), radius, color);
        }
        unsafe extern "C" fn draw_solid_circle_cb(
            transform: ffi::b2Transform,
            radius: f32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            ctx.drawer
                .draw_solid_circle(Transform::from(transform), radius, color);
        }
        unsafe extern "C" fn draw_solid_capsule_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            radius: f32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            ctx.drawer
                .draw_solid_capsule(Vec2::from(p1), Vec2::from(p2), radius, color);
        }
        unsafe extern "C" fn draw_segment_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            ctx.drawer
                .draw_segment(Vec2::from(p1), Vec2::from(p2), color);
        }
        unsafe extern "C" fn draw_transform_cb(
            transform: ffi::b2Transform,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            ctx.drawer.draw_transform(Transform::from(transform));
        }
        unsafe extern "C" fn draw_point_cb(
            p: ffi::b2Vec2,
            size: f32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            ctx.drawer.draw_point(Vec2::from(p), size, color);
        }
        unsafe extern "C" fn draw_string_cb(
            p: ffi::b2Vec2,
            s: *const core::ffi::c_char,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut DebugCtx) };
            if !s.is_null() {
                let cs = unsafe { CStr::from_ptr(s) };
                ctx.drawer
                    .draw_string(Vec2::from(p), &cs.to_string_lossy(), color);
            }
        }

        dd.DrawPolygonFcn = Some(draw_polygon_cb);
        dd.DrawSolidPolygonFcn = Some(draw_solid_polygon_cb);
        dd.DrawCircleFcn = Some(draw_circle_cb);
        dd.DrawSolidCircleFcn = Some(draw_solid_circle_cb);
        dd.DrawSolidCapsuleFcn = Some(draw_solid_capsule_cb);
        dd.DrawSegmentFcn = Some(draw_segment_cb);
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
        dd.drawContacts = opts.draw_contacts;
        dd.drawGraphColors = opts.draw_graph_colors;
        dd.drawContactFeatures = opts.draw_contact_features;
        dd.drawContactNormals = opts.draw_contact_normals;
        dd.drawContactForces = opts.draw_contact_forces;
        dd.drawFrictionForces = opts.draw_friction_forces;
        dd.drawIslands = opts.draw_islands;
        dd.context = &mut ctx as *mut _ as *mut _;

        unsafe { ffi::b2World_Draw(self.raw(), &mut dd) };
    }

    // Raw path: zero-copy FFI types to trait
    pub fn debug_draw_raw(&mut self, drawer: &mut impl RawDebugDraw, opts: DebugDrawOptions) {
        let mut ctx = RawDebugCtx { drawer };
        let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
        unsafe extern "C" fn draw_polygon_cb(
            vertices: *const ffi::b2Vec2,
            count: i32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            let slice = unsafe { core::slice::from_raw_parts(vertices, count as usize) };
            ctx.drawer.draw_polygon(slice, color);
        }
        unsafe extern "C" fn draw_solid_polygon_cb(
            transform: ffi::b2Transform,
            vertices: *const ffi::b2Vec2,
            count: i32,
            radius: f32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            let slice = unsafe { core::slice::from_raw_parts(vertices, count as usize) };
            ctx.drawer
                .draw_solid_polygon(transform, slice, radius, color);
        }
        unsafe extern "C" fn draw_circle_cb(
            center: ffi::b2Vec2,
            radius: f32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            ctx.drawer.draw_circle(center, radius, color);
        }
        unsafe extern "C" fn draw_solid_circle_cb(
            transform: ffi::b2Transform,
            radius: f32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            ctx.drawer.draw_solid_circle(transform, radius, color);
        }
        unsafe extern "C" fn draw_solid_capsule_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            radius: f32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            ctx.drawer.draw_solid_capsule(p1, p2, radius, color);
        }
        unsafe extern "C" fn draw_segment_cb(
            p1: ffi::b2Vec2,
            p2: ffi::b2Vec2,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            ctx.drawer.draw_segment(p1, p2, color);
        }
        unsafe extern "C" fn draw_transform_cb(
            transform: ffi::b2Transform,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            ctx.drawer.draw_transform(transform);
        }
        unsafe extern "C" fn draw_point_cb(
            p: ffi::b2Vec2,
            size: f32,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            ctx.drawer.draw_point(p, size, color);
        }
        unsafe extern "C" fn draw_string_cb(
            p: ffi::b2Vec2,
            s: *const core::ffi::c_char,
            color: i32,
            context: *mut core::ffi::c_void,
        ) {
            let ctx = unsafe { &mut *(context as *mut RawDebugCtx) };
            if !s.is_null() {
                let cs = unsafe { CStr::from_ptr(s) };
                ctx.drawer.draw_string(p, cs, color);
            }
        }
        dd.DrawPolygonFcn = Some(draw_polygon_cb);
        dd.DrawSolidPolygonFcn = Some(draw_solid_polygon_cb);
        dd.DrawCircleFcn = Some(draw_circle_cb);
        dd.DrawSolidCircleFcn = Some(draw_solid_circle_cb);
        dd.DrawSolidCapsuleFcn = Some(draw_solid_capsule_cb);
        dd.DrawSegmentFcn = Some(draw_segment_cb);
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
        dd.drawContacts = opts.draw_contacts;
        dd.drawGraphColors = opts.draw_graph_colors;
        dd.drawContactFeatures = opts.draw_contact_features;
        dd.drawContactNormals = opts.draw_contact_normals;
        dd.drawContactForces = opts.draw_contact_forces;
        dd.drawFrictionForces = opts.draw_friction_forces;
        dd.drawIslands = opts.draw_islands;
        dd.context = &mut ctx as *mut _ as *mut _;
        unsafe { ffi::b2World_Draw(self.raw(), &mut dd) };
    }
}
