//! Debug Draw bridge to Box2D v3 callbacks.
//!
//! Implement the `DebugDraw` trait to receive drawing commands and call `World::debug_draw` each
//! step with `DebugDrawOptions` to render. Colors use the crate-owned [`HexColor`] type, which
//! stores Box2D's packed `0xRRGGBB` convention.
//!
//! Example
//! ```no_run
//! use boxdd::{DebugDraw, DebugDrawOptions, HexColor, Vec2, World, WorldDef, WorldTransform};
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
//! # let def = WorldDef::builder().build();
//! # let mut world = World::new(def).unwrap();
//! let mut cmds = Vec::new();
//! world.debug_draw_collect_into(&mut cmds, DebugDrawOptions::default());
//! let mut drawer = Printer;
//! for cmd in cmds {
//!     let _ = cmd;
//! }
//! ```
use crate::Aabb;
use crate::types::{Position, Vec2, WorldTransform};
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
            drawing_bounds: Aabb::new([-1.0e9, -1.0e9], [1.0e9, 1.0e9]),
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
    pub fn validate(&self) -> crate::error::ApiResult<()> {
        if self.drawing_bounds.is_valid()
            && self.force_scale.is_finite()
            && self.joint_scale.is_finite()
        {
            Ok(())
        } else {
            Err(crate::error::ApiError::InvalidArgument)
        }
    }

    #[track_caller]
    fn assert_valid(self) -> ValidatedDebugDrawOptions {
        assert!(
            self.validate().is_ok(),
            "debug draw options require valid drawing bounds and finite scale values"
        );
        ValidatedDebugDrawOptions(self)
    }

    fn checked(self) -> crate::error::ApiResult<ValidatedDebugDrawOptions> {
        self.validate()?;
        Ok(ValidatedDebugDrawOptions(self))
    }
}

#[derive(Copy, Clone)]
struct ValidatedDebugDrawOptions(DebugDrawOptions);

struct DebugDrawCtx<'a> {
    drawer: &'a mut (dyn NativeDebugDraw + 'a),
    panicked: &'a mut bool,
    panic: &'a mut Option<DebugDrawPanic>,
}

#[inline]
unsafe fn with_ffi_debug_draw_vertices(
    vertices: *const ffi::b2Vec2,
    count: i32,
    visit: impl FnOnce(&[ffi::b2Vec2]),
) {
    let Ok(len) = usize::try_from(count) else {
        return;
    };
    if len == 0 {
        visit(&[]);
        return;
    }
    if vertices.is_null()
        || !vertices.is_aligned()
        || len > (isize::MAX as usize) / core::mem::size_of::<ffi::b2Vec2>()
    {
        return;
    }
    visit(unsafe { core::slice::from_raw_parts(vertices, len) });
}

trait NativeDebugDraw {
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

struct SafeDebugDrawAdapter<'a> {
    drawer: &'a mut dyn DebugDraw,
}

impl NativeDebugDraw for SafeDebugDrawAdapter<'_> {
    fn draw_polygon(
        &mut self,
        transform: ffi::b2WorldTransform,
        vertices: &[ffi::b2Vec2],
        color: HexColor,
    ) {
        let vertices = vertices
            .iter()
            .copied()
            .map(Vec2::from_raw)
            .collect::<SmallVec<[Vec2; 8]>>();
        self.drawer
            .draw_polygon(WorldTransform::from_raw(transform), &vertices, color);
    }

    fn draw_solid_polygon(
        &mut self,
        transform: ffi::b2WorldTransform,
        vertices: &[ffi::b2Vec2],
        radius: f32,
        color: HexColor,
    ) {
        let vertices = vertices
            .iter()
            .copied()
            .map(Vec2::from_raw)
            .collect::<SmallVec<[Vec2; 8]>>();
        self.drawer.draw_solid_polygon(
            WorldTransform::from_raw(transform),
            &vertices,
            radius,
            color,
        );
    }

    fn draw_circle(&mut self, center: ffi::b2Pos, radius: f32, color: HexColor) {
        self.drawer
            .draw_circle(Position::from_raw(center), radius, color);
    }

    fn draw_solid_circle(
        &mut self,
        transform: ffi::b2WorldTransform,
        center: ffi::b2Vec2,
        radius: f32,
        color: HexColor,
    ) {
        self.drawer.draw_solid_circle(
            WorldTransform::from_raw(transform),
            Vec2::from_raw(center),
            radius,
            color,
        );
    }

    fn draw_solid_capsule(&mut self, p1: ffi::b2Pos, p2: ffi::b2Pos, radius: f32, color: HexColor) {
        self.drawer.draw_solid_capsule(
            Position::from_raw(p1),
            Position::from_raw(p2),
            radius,
            color,
        );
    }

    fn draw_segment(&mut self, p1: ffi::b2Pos, p2: ffi::b2Pos, color: HexColor) {
        self.drawer
            .draw_segment(Position::from_raw(p1), Position::from_raw(p2), color);
    }

    fn draw_transform(&mut self, transform: ffi::b2WorldTransform) {
        self.drawer
            .draw_transform(WorldTransform::from_raw(transform));
    }

    fn draw_point(&mut self, p: ffi::b2Pos, size: f32, color: HexColor) {
        self.drawer.draw_point(Position::from_raw(p), size, color);
    }

    fn draw_string(&mut self, p: ffi::b2Pos, s: &CStr, color: HexColor) {
        self.drawer
            .draw_string(Position::from_raw(p), &s.to_string_lossy(), color);
    }

    fn draw_bounds(&mut self, bounds: ffi::b2AABB, color: HexColor) {
        self.drawer.draw_bounds(Aabb::from_raw(bounds), color);
    }
}

/// Reborrow the stack context installed immediately around `b2World_Draw`.
///
/// # Safety
///
/// `context` must be the non-null pointer installed by `draw_with_adapter`. Box2D must invoke
/// callbacks synchronously and serially, and must not retain the pointer after `b2World_Draw`.
unsafe fn native_debug_draw_context<'a>(
    context: *mut core::ffi::c_void,
) -> Option<&'a mut DebugDrawCtx<'a>> {
    unsafe { (context as *mut DebugDrawCtx<'a>).as_mut() }
}

unsafe fn run_native_debug_draw_callback<'a>(
    context: *mut core::ffi::c_void,
    callback: impl FnOnce(&mut (dyn NativeDebugDraw + 'a)),
) {
    let Some(ctx) = (unsafe { native_debug_draw_context::<'a>(context) }) else {
        return;
    };
    if *ctx.panicked {
        return;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = crate::core::callback_state::CallbackGuard::enter();
        callback(ctx.drawer);
    }));
    if let Err(panic) = result {
        *ctx.panicked = true;
        *ctx.panic = Some(panic);
    }
}

unsafe extern "C" fn draw_polygon_cb(
    transform: ffi::b2WorldTransform,
    vertices: *const ffi::b2Vec2,
    count: i32,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            with_ffi_debug_draw_vertices(vertices, count, |vertices| {
                drawer.draw_polygon(transform, vertices, HexColor::from_raw(color));
            });
        });
    }
}

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
            with_ffi_debug_draw_vertices(vertices, count, |vertices| {
                drawer.draw_solid_polygon(transform, vertices, radius, HexColor::from_raw(color));
            });
        });
    }
}

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

unsafe extern "C" fn draw_transform_cb(
    transform: ffi::b2WorldTransform,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| drawer.draw_transform(transform));
    }
}

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

unsafe extern "C" fn draw_string_cb(
    p: ffi::b2Pos,
    s: *const core::ffi::c_char,
    color: ffi::b2HexColor,
    context: *mut core::ffi::c_void,
) {
    unsafe {
        run_native_debug_draw_callback(context, |drawer| {
            if !s.is_null() {
                let s = CStr::from_ptr(s);
                drawer.draw_string(p, s, HexColor::from_raw(color));
            }
        });
    }
}

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

impl World {
    /// Collect debug draw commands into a vector (fully safe).
    ///
    /// This calls into Box2D debug draw but does not invoke user code during the draw.
    pub fn debug_draw_collect(&mut self, opts: DebugDrawOptions) -> Vec<DebugDrawCmd> {
        crate::core::callback_state::assert_not_in_callback();
        let opts = opts.assert_valid();
        let mut cmds = Vec::new();
        self.debug_draw_collect_into_validated(&mut cmds, opts);
        cmds
    }

    /// Collect debug draw commands into a vector (fully safe).
    ///
    /// Returns `ApiError::InCallback` if called while Box2D is already executing a callback.
    pub fn try_debug_draw_collect(
        &mut self,
        opts: DebugDrawOptions,
    ) -> crate::error::ApiResult<Vec<DebugDrawCmd>> {
        crate::core::callback_state::check_not_in_callback()?;
        let opts = opts.checked()?;
        let mut cmds = Vec::new();
        self.debug_draw_collect_into_validated(&mut cmds, opts);
        Ok(cmds)
    }

    /// Collect debug draw commands into a caller-owned buffer.
    ///
    /// This reuses the outer command buffer and, when the command sequence stays
    /// stable, also reuses nested polygon vertex and string storage.
    pub fn debug_draw_collect_into(&mut self, out: &mut Vec<DebugDrawCmd>, opts: DebugDrawOptions) {
        crate::core::callback_state::assert_not_in_callback();
        self.debug_draw_collect_into_validated(out, opts.assert_valid());
    }

    fn debug_draw_collect_into_validated(
        &mut self,
        out: &mut Vec<DebugDrawCmd>,
        opts: ValidatedDebugDrawOptions,
    ) {
        let mut collector = CollectDebugDraw::new(out);
        self.debug_draw_validated(&mut collector, opts);
        collector.finish();
    }

    /// Collect debug draw commands into a caller-owned buffer.
    ///
    /// Returns `ApiError::InCallback` if called while Box2D is already executing a callback.
    pub fn try_debug_draw_collect_into(
        &mut self,
        out: &mut Vec<DebugDrawCmd>,
        opts: DebugDrawOptions,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.debug_draw_collect_into_validated(out, opts.checked()?);
        Ok(())
    }

    fn draw_with_adapter(
        &mut self,
        adapter: &mut dyn NativeDebugDraw,
        opts: ValidatedDebugDrawOptions,
    ) {
        crate::core::callback_state::assert_not_in_callback();
        let mut panicked = false;
        let mut panic: Option<DebugDrawPanic> = None;
        {
            let mut ctx = DebugDrawCtx {
                drawer: adapter,
                panicked: &mut panicked,
                panic: &mut panic,
            };
            let context = core::ptr::from_mut(&mut ctx).cast::<core::ffi::c_void>();
            let mut dd = unsafe { ffi::b2DefaultDebugDraw() };
            install_debug_draw_callbacks(&mut dd);
            apply_debug_draw_options(&mut dd, opts.0, context);

            unsafe { ffi::b2World_Draw(self.raw(), &mut dd) };
        }
        finish_debug_draw(self, &mut panic);
    }

    /// Draw the world through the safe, precision-aware callback interface.
    ///
    /// Box2D invokes the draw callbacks while traversing internal world state. During this call,
    /// any attempt to call into the Box2D world through `boxdd` will panic, since the world is
    /// considered locked by Box2D.
    pub fn debug_draw(&mut self, drawer: &mut impl DebugDraw, opts: DebugDrawOptions) {
        crate::core::callback_state::assert_not_in_callback();
        self.debug_draw_validated(drawer, opts.assert_valid());
    }

    fn debug_draw_validated(
        &mut self,
        drawer: &mut impl DebugDraw,
        opts: ValidatedDebugDrawOptions,
    ) {
        let drawer: &mut dyn DebugDraw = drawer;
        let mut adapter = SafeDebugDrawAdapter { drawer };
        self.draw_with_adapter(&mut adapter, opts);
    }

    /// Safe debug draw bridge with recoverable callback-lock checking.
    ///
    /// Returns `ApiError::InCallback` if called while Box2D is already executing a callback.
    pub fn try_debug_draw(
        &mut self,
        drawer: &mut impl DebugDraw,
        opts: DebugDrawOptions,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.debug_draw_validated(drawer, opts.checked()?);
        Ok(())
    }
}

#[cfg(test)]
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
        panicked: &'a mut bool,
        panic: &'a mut Option<DebugDrawPanic>,
    ) -> DebugDrawCtx<'a> {
        DebugDrawCtx {
            drawer,
            panicked,
            panic,
        }
    }

    #[test]
    fn invalid_options_are_rejected_before_native_debug_draw() {
        struct NoopDrawer;
        impl DebugDraw for NoopDrawer {}

        let invalid_bounds = DebugDrawOptions {
            drawing_bounds: Aabb::new([f32::NAN, 0.0], [1.0, 1.0]),
            ..DebugDrawOptions::default()
        };
        let invalid_scale = DebugDrawOptions {
            force_scale: f32::INFINITY,
            ..DebugDrawOptions::default()
        };
        assert_eq!(
            invalid_bounds.validate().unwrap_err(),
            crate::ApiError::InvalidArgument
        );
        assert_eq!(
            invalid_scale.validate().unwrap_err(),
            crate::ApiError::InvalidArgument
        );

        let mut world = World::new(crate::WorldDef::default()).unwrap();
        let mut out = Vec::new();
        let mut drawer = NoopDrawer;
        assert_eq!(
            world.try_debug_draw_collect(invalid_bounds).unwrap_err(),
            crate::ApiError::InvalidArgument
        );
        assert_eq!(
            world
                .try_debug_draw_collect_into(&mut out, invalid_bounds)
                .unwrap_err(),
            crate::ApiError::InvalidArgument
        );
        assert_eq!(
            world
                .try_debug_draw(&mut drawer, invalid_scale)
                .unwrap_err(),
            crate::ApiError::InvalidArgument
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                world.debug_draw_collect(invalid_bounds)
            }))
            .is_err()
        );
    }

    #[test]
    fn box2d_32_callbacks_dispatch_every_command() {
        #[cfg(feature = "double-precision")]
        let far = 1_000_000_000_000.25;
        #[cfg(not(feature = "double-precision"))]
        let far = 1_000_000.25;

        let transform = WorldTransform::from_pos_angle(test_position(far, -far), 0.25);
        let circle_center = test_position(far + 10.5, -far - 20.25);
        let p1 = test_position(far + 1.0, -far - 2.0);
        let p2 = test_position(far + 3.0, -far - 4.0);
        let vertices = [Vec2::new(-1.0, -2.0), Vec2::new(3.0, 4.0)];
        let raw_vertices = vertices.map(Vec2::into_raw);
        let local_center = Vec2::new(0.5, -0.75);
        let bounds = Aabb::new([-5.0, -6.0], [7.0, 8.0]);
        let color = HexColor::from_rgb(0x12, 0x34, 0x56);
        let mut commands = Vec::new();
        let mut panicked = false;
        let mut panic = None;

        {
            let mut collector = CollectDebugDraw::new(&mut commands);
            {
                let drawer: &mut dyn DebugDraw = &mut collector;
                let mut adapter = SafeDebugDrawAdapter { drawer };
                let adapter: &mut dyn NativeDebugDraw = &mut adapter;
                let mut ctx = test_context(adapter, &mut panicked, &mut panic);
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

        assert!(!panicked);
        assert!(panic.is_none());
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
        let bounds = Aabb::new([-11.0, -12.0], [13.0, 14.0]);
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
        assert_eq!(Aabb::from_raw(dd.drawingBounds), bounds);
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
    fn polygon_callback_rejects_invalid_buffers_without_forging_a_slice() {
        let mut lengths = PolygonLengths::default();
        let mut panicked = false;
        let mut panic = None;
        {
            let drawer: &mut dyn DebugDraw = &mut lengths;
            let mut adapter = SafeDebugDrawAdapter { drawer };
            let adapter: &mut dyn NativeDebugDraw = &mut adapter;
            let mut ctx = test_context(adapter, &mut panicked, &mut panic);
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
                    1,
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

        assert!(!panicked);
        assert!(panic.is_none());
        assert_eq!(lengths.0, [0, 1]);
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
        let mut panicked = false;
        let mut panic = None;
        {
            let safe_drawer: &mut dyn DebugDraw = &mut drawer;
            let mut adapter = SafeDebugDrawAdapter {
                drawer: safe_drawer,
            };
            let adapter: &mut dyn NativeDebugDraw = &mut adapter;
            let mut ctx = test_context(adapter, &mut panicked, &mut panic);
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

        assert!(panicked);
        assert!(panic.is_some());
        assert_eq!(drawer.point_calls, 0);
    }
}
