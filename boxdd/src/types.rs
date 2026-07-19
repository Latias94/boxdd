use crate::core::math::Rot;
use boxdd_sys::ffi;

/// A simple 2D vector in meters.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Zeroable for Vec2 {}
#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Pod for Vec2 {}

#[cfg(feature = "bytemuck")]
const _: () = {
    assert!(core::mem::size_of::<Vec2>() == 8);
    assert!(core::mem::align_of::<Vec2>() == 4);
};

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn from_raw(raw: ffi::b2Vec2) -> Self {
        Self { x: raw.x, y: raw.y }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2Vec2 {
        ffi::b2Vec2 {
            x: self.x,
            y: self.y,
        }
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        unsafe { ffi::b2IsValidVec2(self.into_raw()) }
    }
}

// Conversions from common 2D types to Vec2 for ergonomic APIs
impl From<[f32; 2]> for Vec2 {
    #[inline]
    fn from(a: [f32; 2]) -> Self {
        Self { x: a[0], y: a[1] }
    }
}
impl From<(f32, f32)> for Vec2 {
    #[inline]
    fn from(t: (f32, f32)) -> Self {
        Self { x: t.0, y: t.1 }
    }
}

#[cfg(feature = "mint")]
impl From<mint::Vector2<f32>> for Vec2 {
    #[inline]
    fn from(v: mint::Vector2<f32>) -> Self {
        Self { x: v.x, y: v.y }
    }
}
#[cfg(feature = "mint")]
impl From<mint::Point2<f32>> for Vec2 {
    #[inline]
    fn from(p: mint::Point2<f32>) -> Self {
        Self { x: p.x, y: p.y }
    }
}

#[cfg(feature = "mint")]
impl From<Vec2> for mint::Vector2<f32> {
    #[inline]
    fn from(v: Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

#[cfg(feature = "mint")]
impl From<Vec2> for mint::Point2<f32> {
    #[inline]
    fn from(v: Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

// Optional conversions with common math libraries
#[cfg(feature = "cgmath")]
impl From<cgmath::Vector2<f32>> for Vec2 {
    #[inline]
    fn from(v: cgmath::Vector2<f32>) -> Self {
        Self { x: v.x, y: v.y }
    }
}
#[cfg(feature = "cgmath")]
impl From<Vec2> for cgmath::Vector2<f32> {
    #[inline]
    fn from(v: Vec2) -> Self {
        cgmath::Vector2 { x: v.x, y: v.y }
    }
}
#[cfg(feature = "cgmath")]
impl From<cgmath::Point2<f32>> for Vec2 {
    #[inline]
    fn from(p: cgmath::Point2<f32>) -> Self {
        Self { x: p.x, y: p.y }
    }
}
#[cfg(feature = "cgmath")]
impl From<Vec2> for cgmath::Point2<f32> {
    #[inline]
    fn from(v: Vec2) -> Self {
        cgmath::Point2 { x: v.x, y: v.y }
    }
}

#[cfg(feature = "nalgebra")]
impl From<nalgebra::Vector2<f32>> for Vec2 {
    #[inline]
    fn from(v: nalgebra::Vector2<f32>) -> Self {
        Self { x: v.x, y: v.y }
    }
}
#[cfg(feature = "nalgebra")]
impl From<Vec2> for nalgebra::Vector2<f32> {
    #[inline]
    fn from(v: Vec2) -> Self {
        nalgebra::Vector2::new(v.x, v.y)
    }
}
#[cfg(feature = "nalgebra")]
impl From<nalgebra::Point2<f32>> for Vec2 {
    #[inline]
    fn from(p: nalgebra::Point2<f32>) -> Self {
        Self { x: p.x, y: p.y }
    }
}
#[cfg(feature = "nalgebra")]
impl From<Vec2> for nalgebra::Point2<f32> {
    #[inline]
    fn from(v: Vec2) -> Self {
        nalgebra::Point2::new(v.x, v.y)
    }
}

#[cfg(feature = "glam")]
impl From<glam::Vec2> for Vec2 {
    #[inline]
    fn from(v: glam::Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}
#[cfg(feature = "glam")]
impl From<Vec2> for glam::Vec2 {
    #[inline]
    fn from(v: Vec2) -> Self {
        glam::Vec2::new(v.x, v.y)
    }
}

/// Scalar used for absolute world coordinates in the active Box2D precision mode.
#[cfg(not(feature = "double-precision"))]
pub type WorldScalar = f32;

/// Scalar used for absolute world coordinates in the active Box2D precision mode.
#[cfg(feature = "double-precision")]
pub type WorldScalar = f64;

#[inline]
fn world_scalar_to_f32_lossy(value: WorldScalar) -> f32 {
    #[cfg(not(feature = "double-precision"))]
    {
        value
    }

    #[cfg(feature = "double-precision")]
    {
        value as f32
    }
}

/// An absolute position in a Box2D world.
///
/// World positions use `f64` when the `double-precision` feature is enabled. Local offsets,
/// directions, extents, and relative geometry continue to use [`Vec2`] and `f32`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Position {
    pub x: WorldScalar,
    pub y: WorldScalar,
}

/// Failure while converting the difference between two world positions to a local [`Vec2`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PositionToLocalError {
    /// At least one coordinate or the computed difference was NaN or infinite.
    #[error("world-position difference is not finite")]
    NonFinite,
    /// The finite difference cannot be represented by an `f32` local coordinate.
    #[error("world-position difference exceeds the local f32 range")]
    OutOfRange,
}

impl Position {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    #[inline]
    pub const fn new(x: WorldScalar, y: WorldScalar) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn from_raw(raw: ffi::b2Pos) -> Self {
        Self { x: raw.x, y: raw.y }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2Pos {
        #[cfg(not(feature = "double-precision"))]
        {
            ffi::b2Vec2 {
                x: self.x,
                y: self.y,
            }
        }

        #[cfg(feature = "double-precision")]
        {
            ffi::b2Pos {
                x: self.x,
                y: self.y,
            }
        }
    }

    /// Returns whether both coordinates are finite and valid for Box2D.
    #[inline]
    pub fn is_valid(self) -> bool {
        unsafe { ffi::b2IsValidPosition(self.into_raw()) }
    }

    /// Offsets this world position by a local `f32` vector without narrowing coordinates.
    #[inline]
    pub fn offset(self, offset: Vec2) -> Self {
        Self {
            x: self.x + WorldScalar::from(offset.x),
            y: self.y + WorldScalar::from(offset.y),
        }
    }

    /// Computes this position relative to `origin`, rejecting invalid or out-of-range results.
    ///
    /// The returned local vector uses `f32`, so a finite in-range double-precision difference
    /// is rounded to the nearest representable `f32` value.
    #[inline]
    pub fn checked_relative_to(self, origin: Self) -> Result<Vec2, PositionToLocalError> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !origin.x.is_finite()
            || !origin.y.is_finite()
        {
            return Err(PositionToLocalError::NonFinite);
        }

        let x = self.x - origin.x;
        let y = self.y - origin.y;
        if !x.is_finite() || !y.is_finite() {
            return Err(PositionToLocalError::NonFinite);
        }

        let local_max = WorldScalar::from(f32::MAX);
        if x < -local_max || x > local_max || y < -local_max || y > local_max {
            return Err(PositionToLocalError::OutOfRange);
        }

        Ok(Vec2::new(
            world_scalar_to_f32_lossy(x),
            world_scalar_to_f32_lossy(y),
        ))
    }

    /// Computes this position relative to `origin` with unchecked, potentially lossy narrowing.
    ///
    /// NaN, infinity, overflow, and precision loss are preserved according to Rust's float-cast
    /// rules. Prefer [`Self::checked_relative_to`] at Safe Rust world/local boundaries.
    #[inline]
    pub fn relative_to_lossy(self, origin: Self) -> Vec2 {
        Vec2::new(
            world_scalar_to_f32_lossy(self.x - origin.x),
            world_scalar_to_f32_lossy(self.y - origin.y),
        )
    }
}

impl From<Vec2> for Position {
    #[inline]
    fn from(value: Vec2) -> Self {
        Self::new(WorldScalar::from(value.x), WorldScalar::from(value.y))
    }
}

impl From<[WorldScalar; 2]> for Position {
    #[inline]
    fn from(value: [WorldScalar; 2]) -> Self {
        Self::new(value[0], value[1])
    }
}

#[cfg(feature = "double-precision")]
impl From<[f32; 2]> for Position {
    #[inline]
    fn from(value: [f32; 2]) -> Self {
        Self::new(f64::from(value[0]), f64::from(value[1]))
    }
}

impl From<(WorldScalar, WorldScalar)> for Position {
    #[inline]
    fn from(value: (WorldScalar, WorldScalar)) -> Self {
        Self::new(value.0, value.1)
    }
}

#[cfg(feature = "mint")]
impl From<mint::Point2<WorldScalar>> for Position {
    #[inline]
    fn from(value: mint::Point2<WorldScalar>) -> Self {
        Self::new(value.x, value.y)
    }
}

#[cfg(feature = "mint")]
impl From<Position> for mint::Point2<WorldScalar> {
    #[inline]
    fn from(value: Position) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[cfg(feature = "cgmath")]
impl From<cgmath::Point2<WorldScalar>> for Position {
    #[inline]
    fn from(value: cgmath::Point2<WorldScalar>) -> Self {
        Self::new(value.x, value.y)
    }
}

#[cfg(feature = "cgmath")]
impl From<Position> for cgmath::Point2<WorldScalar> {
    #[inline]
    fn from(value: Position) -> Self {
        Self::new(value.x, value.y)
    }
}

#[cfg(feature = "nalgebra")]
impl From<nalgebra::Point2<WorldScalar>> for Position {
    #[inline]
    fn from(value: nalgebra::Point2<WorldScalar>) -> Self {
        Self::new(value.x, value.y)
    }
}

#[cfg(feature = "nalgebra")]
impl From<Position> for nalgebra::Point2<WorldScalar> {
    #[inline]
    fn from(value: Position) -> Self {
        Self::new(value.x, value.y)
    }
}

#[cfg(all(feature = "glam", not(feature = "double-precision")))]
impl From<glam::Vec2> for Position {
    #[inline]
    fn from(value: glam::Vec2) -> Self {
        Self::new(value.x, value.y)
    }
}

#[cfg(all(feature = "glam", not(feature = "double-precision")))]
impl From<Position> for glam::Vec2 {
    #[inline]
    fn from(value: Position) -> Self {
        Self::new(value.x, value.y)
    }
}

#[cfg(all(feature = "glam", feature = "double-precision"))]
impl From<glam::DVec2> for Position {
    #[inline]
    fn from(value: glam::DVec2) -> Self {
        Self::new(value.x, value.y)
    }
}

#[cfg(all(feature = "glam", feature = "double-precision"))]
impl From<Position> for glam::DVec2 {
    #[inline]
    fn from(value: Position) -> Self {
        Self::new(value.x, value.y)
    }
}

#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Zeroable for Position {}
#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Pod for Position {}

/// A rigid transform whose translation is an absolute world [`Position`].
///
/// Rotation remains an `f32` [`Rot`] in both precision modes, matching Box2D's ABI.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct WorldTransform {
    p: Position,
    q: Rot,
}

impl WorldTransform {
    pub const IDENTITY: Self = Self {
        p: Position::ZERO,
        q: Rot::IDENTITY,
    };

    #[inline]
    pub const fn new(position: Position, rotation: Rot) -> Self {
        Self {
            p: position,
            q: rotation,
        }
    }

    #[inline]
    pub const fn from_raw(raw: ffi::b2WorldTransform) -> Self {
        Self::new(Position::from_raw(raw.p), Rot::from_raw(raw.q))
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2WorldTransform {
        #[cfg(not(feature = "double-precision"))]
        {
            ffi::b2Transform {
                p: self.p.into_raw(),
                q: self.q.into_raw(),
            }
        }

        #[cfg(feature = "double-precision")]
        {
            ffi::b2WorldTransform {
                p: self.p.into_raw(),
                q: self.q.into_raw(),
            }
        }
    }

    #[inline]
    pub fn from_pos_angle<P: Into<Position>>(position: P, angle_radians: f32) -> Self {
        Self::new(position.into(), Rot::from_radians(angle_radians))
    }

    #[inline]
    pub const fn position(self) -> Position {
        self.p
    }

    #[inline]
    pub const fn rotation(self) -> Rot {
        self.q
    }

    /// Returns whether the position and rotation are valid for Box2D.
    #[inline]
    pub fn is_valid(self) -> bool {
        unsafe { ffi::b2IsValidWorldTransform(self.into_raw()) }
    }

    /// Transforms a local point into an absolute world position.
    #[inline]
    pub fn transform_point(self, point: Vec2) -> Position {
        self.p.offset(self.q.rotate_vec(point))
    }
}

impl Default for WorldTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for WorldTransform {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            position: Position,
            angle: f32,
        }

        Repr {
            position: self.position(),
            angle: self.rotation().angle(),
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for WorldTransform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            position: Position,
            angle: f32,
        }

        let repr = Repr::deserialize(deserializer)?;
        Ok(Self::from_pos_angle(repr.position, repr.angle))
    }
}

#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Zeroable for WorldTransform {}
#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Pod for WorldTransform {}

#[cfg(feature = "bytemuck")]
const _: () = {
    assert!(core::mem::size_of::<Position>() == 2 * core::mem::size_of::<WorldScalar>());

    #[cfg(not(feature = "double-precision"))]
    assert!(core::mem::size_of::<WorldTransform>() == 16);

    #[cfg(feature = "double-precision")]
    assert!(core::mem::size_of::<WorldTransform>() == 24);
};

/// Result of a cast whose hit point is an absolute world position.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct WorldCastOutput {
    /// Unit surface normal in world orientation.
    pub normal: Vec2,
    /// Absolute world-space hit position.
    pub point: Position,
    /// Fraction of the cast translation at which the hit occurred.
    pub fraction: f32,
    pub iterations: i32,
    pub hit: bool,
}

impl WorldCastOutput {
    pub const MISS: Self = Self {
        normal: Vec2::ZERO,
        point: Position::ZERO,
        fraction: 0.0,
        iterations: 0,
        hit: false,
    };

    #[inline]
    pub const fn from_raw(raw: ffi::b2WorldCastOutput) -> Self {
        Self {
            normal: Vec2::from_raw(raw.normal),
            point: Position::from_raw(raw.point),
            fraction: raw.fraction,
            iterations: raw.iterations,
            hit: raw.hit,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2WorldCastOutput {
        #[cfg(not(feature = "double-precision"))]
        {
            ffi::b2CastOutput {
                normal: self.normal.into_raw(),
                point: self.point.into_raw(),
                fraction: self.fraction,
                iterations: self.iterations,
                hit: self.hit,
            }
        }

        #[cfg(feature = "double-precision")]
        {
            ffi::b2WorldCastOutput {
                normal: self.normal.into_raw(),
                point: self.point.into_raw(),
                fraction: self.fraction,
                iterations: self.iterations,
                hit: self.hit,
            }
        }
    }
}

/// Opaque Box2D body identifier.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BodyId {
    pub index1: i32,
    pub world0: u16,
    pub generation: u16,
}

impl BodyId {
    #[inline]
    pub const fn from_raw(raw: ffi::b2BodyId) -> Self {
        Self {
            index1: raw.index1,
            world0: raw.world0,
            generation: raw.generation,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2BodyId {
        ffi::b2BodyId {
            index1: self.index1,
            world0: self.world0,
            generation: self.generation,
        }
    }
}

const _: () = {
    assert!(core::mem::size_of::<BodyId>() == core::mem::size_of::<ffi::b2BodyId>());
    assert!(core::mem::align_of::<BodyId>() == core::mem::align_of::<ffi::b2BodyId>());
};

/// Opaque Box2D shape identifier.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShapeId {
    pub index1: i32,
    pub world0: u16,
    pub generation: u16,
}

impl ShapeId {
    #[inline]
    pub const fn from_raw(raw: ffi::b2ShapeId) -> Self {
        Self {
            index1: raw.index1,
            world0: raw.world0,
            generation: raw.generation,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2ShapeId {
        ffi::b2ShapeId {
            index1: self.index1,
            world0: self.world0,
            generation: self.generation,
        }
    }
}

const _: () = {
    assert!(core::mem::size_of::<ShapeId>() == core::mem::size_of::<ffi::b2ShapeId>());
    assert!(core::mem::align_of::<ShapeId>() == core::mem::align_of::<ffi::b2ShapeId>());
};

/// Opaque Box2D joint identifier.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct JointId {
    pub index1: i32,
    pub world0: u16,
    pub generation: u16,
}

impl JointId {
    #[inline]
    pub const fn from_raw(raw: ffi::b2JointId) -> Self {
        Self {
            index1: raw.index1,
            world0: raw.world0,
            generation: raw.generation,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2JointId {
        ffi::b2JointId {
            index1: self.index1,
            world0: self.world0,
            generation: self.generation,
        }
    }
}

const _: () = {
    assert!(core::mem::size_of::<JointId>() == core::mem::size_of::<ffi::b2JointId>());
    assert!(core::mem::align_of::<JointId>() == core::mem::align_of::<ffi::b2JointId>());
};

/// Opaque Box2D chain identifier.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChainId {
    pub index1: i32,
    pub world0: u16,
    pub generation: u16,
}

impl ChainId {
    #[inline]
    pub const fn from_raw(raw: ffi::b2ChainId) -> Self {
        Self {
            index1: raw.index1,
            world0: raw.world0,
            generation: raw.generation,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2ChainId {
        ffi::b2ChainId {
            index1: self.index1,
            world0: self.world0,
            generation: self.generation,
        }
    }
}

const _: () = {
    assert!(core::mem::size_of::<ChainId>() == core::mem::size_of::<ffi::b2ChainId>());
    assert!(core::mem::align_of::<ChainId>() == core::mem::align_of::<ffi::b2ChainId>());
};

/// Opaque Box2D contact identifier.
///
/// `ContactId` values commonly come from contact events or contact-data snapshots and expose
/// direct validity checks plus crate-owned/raw contact-data reads as inherent methods.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContactId {
    pub index1: i32,
    pub world0: u16,
    pub padding: i16,
    pub generation: u32,
}

impl ContactId {
    #[inline]
    pub const fn from_raw(raw: ffi::b2ContactId) -> Self {
        Self {
            index1: raw.index1,
            world0: raw.world0,
            padding: raw.padding,
            generation: raw.generation,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2ContactId {
        ffi::b2ContactId {
            index1: self.index1,
            world0: self.world0,
            padding: self.padding,
            generation: self.generation,
        }
    }
}

const _: () = {
    assert!(core::mem::size_of::<ContactId>() == core::mem::size_of::<ffi::b2ContactId>());
    assert!(core::mem::align_of::<ContactId>() == core::mem::align_of::<ffi::b2ContactId>());
};

/// Mass properties (mass, center, inertia) used by Box2D.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct MassData {
    pub mass: f32,
    pub center: Vec2,
    pub rotational_inertia: f32,
}

impl MassData {
    #[inline]
    pub const fn new(mass: f32, center: Vec2, rotational_inertia: f32) -> Self {
        Self {
            mass,
            center,
            rotational_inertia,
        }
    }

    #[inline]
    /// Construct from the raw Box2D value.
    pub fn from_raw(raw: ffi::b2MassData) -> Self {
        Self {
            mass: raw.mass,
            center: Vec2::from_raw(raw.center),
            rotational_inertia: raw.rotationalInertia,
        }
    }

    #[inline]
    /// Convert into the raw Box2D value.
    pub fn into_raw(self) -> ffi::b2MassData {
        ffi::b2MassData {
            mass: self.mass,
            center: self.center.into_raw(),
            rotationalInertia: self.rotational_inertia,
        }
    }
}

/// Per-body motion lock flags.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct MotionLocks {
    pub linear_x: bool,
    pub linear_y: bool,
    pub angular_z: bool,
}

impl MotionLocks {
    #[inline]
    pub const fn new(linear_x: bool, linear_y: bool, angular_z: bool) -> Self {
        Self {
            linear_x,
            linear_y,
            angular_z,
        }
    }

    #[inline]
    /// Construct from the raw Box2D value.
    pub fn from_raw(raw: ffi::b2MotionLocks) -> Self {
        Self {
            linear_x: raw.linearX,
            linear_y: raw.linearY,
            angular_z: raw.angularZ,
        }
    }

    #[inline]
    /// Convert into the raw Box2D value.
    pub fn into_raw(self) -> ffi::b2MotionLocks {
        ffi::b2MotionLocks {
            linearX: self.linear_x,
            linearY: self.linear_y,
            angularZ: self.angular_z,
        }
    }
}

/// Maximum number of contact points supported by a Box2D manifold in 2D.
pub const MAX_MANIFOLD_POINTS: usize = 2;

/// A solver contact point inside a runtime world manifold.
///
/// The anchors are `f32` offsets from each body's center of mass, not absolute world
/// positions. Reconstruct an absolute contact position by offsetting the corresponding
/// body's world center [`Position`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ManifoldPoint {
    /// Contact point relative to body A's center of mass in world axes.
    pub anchor_a: Vec2,
    /// Contact point relative to body B's center of mass in world axes.
    pub anchor_b: Vec2,
    /// Signed separation; negative values indicate penetration.
    pub separation: f32,
    /// Cached separation used by Box2D when recycling contacts.
    pub base_separation: f32,
    /// Impulse along the manifold normal.
    pub normal_impulse: f32,
    /// Friction impulse along the tangent.
    pub tangent_impulse: f32,
    /// Total normal impulse accumulated across substeps and restitution.
    pub total_normal_impulse: f32,
    /// Relative normal velocity before solving; negative values are approaching.
    pub normal_velocity: f32,
    /// Stable feature-pair identifier supplied by Box2D.
    pub id: u16,
    /// Whether this point existed during the previous step.
    pub persisted: bool,
}

impl ManifoldPoint {
    /// Reconstructs this contact point from body A's absolute world center.
    #[inline]
    pub fn world_point_a(self, body_a_world_center: Position) -> Position {
        body_a_world_center.offset(self.anchor_a)
    }

    /// Reconstructs this contact point from body B's absolute world center.
    #[inline]
    pub fn world_point_b(self, body_b_world_center: Position) -> Position {
        body_b_world_center.offset(self.anchor_b)
    }

    #[inline]
    pub fn from_raw(raw: ffi::b2ManifoldPoint) -> Self {
        Self {
            anchor_a: Vec2::from_raw(raw.anchorA),
            anchor_b: Vec2::from_raw(raw.anchorB),
            separation: raw.separation,
            base_separation: raw.baseSeparation,
            normal_impulse: raw.normalImpulse,
            tangent_impulse: raw.tangentImpulse,
            total_normal_impulse: raw.totalNormalImpulse,
            normal_velocity: raw.normalVelocity,
            id: raw.id,
            persisted: raw.persisted,
        }
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2ManifoldPoint {
        ffi::b2ManifoldPoint {
            anchorA: self.anchor_a.into_raw(),
            anchorB: self.anchor_b.into_raw(),
            separation: self.separation,
            baseSeparation: self.base_separation,
            normalImpulse: self.normal_impulse,
            tangentImpulse: self.tangent_impulse,
            totalNormalImpulse: self.total_normal_impulse,
            normalVelocity: self.normal_velocity,
            id: self.id,
            persisted: self.persisted,
        }
    }
}

/// Runtime solver manifold between two shapes in a Box2D world.
///
/// The normal is a world-space direction. Contact positions remain relative `f32` anchors in
/// [`ManifoldPoint`] so double-precision builds do not silently narrow absolute positions.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Manifold {
    pub normal: Vec2,
    pub rolling_impulse: f32,
    pub contact_points: [ManifoldPoint; MAX_MANIFOLD_POINTS],
    pub point_count: i32,
}

impl Manifold {
    /// The initialized contact points in this manifold.
    #[inline]
    pub fn points(&self) -> &[ManifoldPoint] {
        &self.contact_points[..self.point_count()]
    }

    /// The number of initialized contact points.
    #[inline]
    pub fn point_count(&self) -> usize {
        self.point_count.clamp(0, MAX_MANIFOLD_POINTS as i32) as usize
    }

    /// Whether this manifold contains no initialized contact points.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.point_count() == 0
    }

    #[inline]
    pub fn from_raw(raw: ffi::b2Manifold) -> Self {
        Self {
            normal: Vec2::from_raw(raw.normal),
            rolling_impulse: raw.rollingImpulse,
            contact_points: raw.points.map(ManifoldPoint::from_raw),
            point_count: raw.pointCount.clamp(0, MAX_MANIFOLD_POINTS as i32),
        }
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2Manifold {
        ffi::b2Manifold {
            normal: self.normal.into_raw(),
            rollingImpulse: self.rolling_impulse,
            points: self.contact_points.map(ManifoldPoint::into_raw),
            pointCount: self.point_count.clamp(0, MAX_MANIFOLD_POINTS as i32),
        }
    }
}

/// Contact data for a single contact touching two shapes.
#[derive(Copy, Clone, Debug)]
pub struct ContactData {
    pub contact_id: ContactId,
    pub shape_id_a: ShapeId,
    pub shape_id_b: ShapeId,
    pub manifold: Manifold,
}

impl ContactData {
    #[inline]
    pub fn from_raw(raw: ffi::b2ContactData) -> Self {
        Self {
            contact_id: ContactId::from_raw(raw.contactId),
            shape_id_a: ShapeId::from_raw(raw.shapeIdA),
            shape_id_b: ShapeId::from_raw(raw.shapeIdB),
            manifold: Manifold::from_raw(raw.manifold),
        }
    }

    #[inline]
    pub fn into_raw(self) -> ffi::b2ContactData {
        ffi::b2ContactData {
            contactId: self.contact_id.into_raw(),
            shapeIdA: self.shape_id_a.into_raw(),
            shapeIdB: self.shape_id_b.into_raw(),
            manifold: self.manifold.into_raw(),
        }
    }
}

const _: () = {
    assert!(core::mem::size_of::<MassData>() == core::mem::size_of::<ffi::b2MassData>());
    assert!(core::mem::align_of::<MassData>() == core::mem::align_of::<ffi::b2MassData>());
    assert!(core::mem::size_of::<MotionLocks>() == core::mem::size_of::<ffi::b2MotionLocks>());
    assert!(core::mem::align_of::<MotionLocks>() == core::mem::align_of::<ffi::b2MotionLocks>());
};

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "double-precision"))]
    const TEST_WORLD_X: WorldScalar = 16_384.25;
    #[cfg(feature = "double-precision")]
    const TEST_WORLD_X: WorldScalar = 10_000_000_000.25;

    #[test]
    fn world_value_layout_matches_the_active_precision_abi() {
        assert_eq!(
            core::mem::size_of::<Position>(),
            core::mem::size_of::<ffi::b2Pos>()
        );
        assert_eq!(
            core::mem::align_of::<Position>(),
            core::mem::align_of::<ffi::b2Pos>()
        );
        assert_eq!(
            core::mem::size_of::<WorldTransform>(),
            core::mem::size_of::<ffi::b2WorldTransform>()
        );
        assert_eq!(
            core::mem::align_of::<WorldTransform>(),
            core::mem::align_of::<ffi::b2WorldTransform>()
        );
    }

    #[test]
    fn world_position_and_transform_round_trip_fieldwise() {
        assert_eq!(
            Position::from(Vec2::new(1.25, -2.5)),
            Position::new(1.25, -2.5)
        );
        assert_eq!(
            Position::from([3.5_f32, -4.75_f32]),
            Position::new(3.5, -4.75)
        );

        let position = Position::new(TEST_WORLD_X, -TEST_WORLD_X);
        let position_round_trip = Position::from_raw(position.into_raw());
        assert_eq!(position_round_trip, position);

        let transform = WorldTransform::new(position, Rot::from_radians(0.375));
        let transform_round_trip = WorldTransform::from_raw(transform.into_raw());
        assert_eq!(transform_round_trip.position(), position);
        assert_eq!(
            transform_round_trip.rotation().cosine(),
            transform.rotation().cosine()
        );
        assert_eq!(
            transform_round_trip.rotation().sine(),
            transform.rotation().sine()
        );

        let transformed = transform.transform_point(Vec2::new(0.5, -0.25));
        assert!(transformed.is_valid());
    }

    #[test]
    fn checked_world_to_local_conversion_rejects_invalid_values() {
        let origin = Position::ZERO;
        assert_eq!(
            Position::new(WorldScalar::NAN, 0.0).checked_relative_to(origin),
            Err(PositionToLocalError::NonFinite)
        );

        #[cfg(feature = "double-precision")]
        assert_eq!(
            Position::new(f64::from(f32::MAX) * 2.0, 0.0).checked_relative_to(origin),
            Err(PositionToLocalError::OutOfRange)
        );
    }

    #[cfg(feature = "double-precision")]
    #[test]
    fn double_precision_preserves_millimeters_at_ten_million_meters() {
        let origin = Position::new(10_000_000.0, -10_000_000.0);
        let point = Position::new(10_000_000.001, -9_999_999.999);

        assert_eq!(Position::from_raw(point.into_raw()), point);
        let local = point
            .checked_relative_to(origin)
            .expect("millimeter delta should fit in local coordinates");
        assert!((local.x - 0.001).abs() < 1.0e-8);
        assert!((local.y - 0.001).abs() < 1.0e-8);
        assert_eq!(point.relative_to_lossy(origin), local);
    }

    #[test]
    fn world_cast_output_preserves_absolute_hit_point() {
        let output = WorldCastOutput {
            normal: Vec2::new(0.0, 1.0),
            point: Position::new(TEST_WORLD_X, TEST_WORLD_X + 0.5),
            fraction: 0.625,
            iterations: 7,
            hit: true,
        };

        assert_eq!(WorldCastOutput::from_raw(output.into_raw()), output);
    }

    #[test]
    fn runtime_manifold_point_maps_anchors_and_base_separation() {
        let raw = ffi::b2ManifoldPoint {
            anchorA: ffi::b2Vec2 { x: 1.0, y: 2.0 },
            anchorB: ffi::b2Vec2 { x: 3.0, y: 4.0 },
            separation: -0.25,
            baseSeparation: -0.125,
            normalImpulse: 5.0,
            tangentImpulse: 6.0,
            totalNormalImpulse: 7.0,
            normalVelocity: -8.0,
            id: 9,
            persisted: true,
        };

        let point = ManifoldPoint::from_raw(raw);
        assert_eq!(point.anchor_a, Vec2::new(1.0, 2.0));
        assert_eq!(point.anchor_b, Vec2::new(3.0, 4.0));
        assert_eq!(point.separation, -0.25);
        assert_eq!(point.base_separation, -0.125);
        assert_eq!(point.normal_impulse, 5.0);
        assert_eq!(point.tangent_impulse, 6.0);
        assert_eq!(point.total_normal_impulse, 7.0);
        assert_eq!(point.normal_velocity, -8.0);
        assert_eq!(point.id, 9);
        assert!(point.persisted);
        assert_eq!(
            point.world_point_a(Position::new(TEST_WORLD_X, TEST_WORLD_X)),
            Position::new(TEST_WORLD_X + 1.0, TEST_WORLD_X + 2.0)
        );
        assert_eq!(
            point.world_point_b(Position::new(TEST_WORLD_X, TEST_WORLD_X)),
            Position::new(TEST_WORLD_X + 3.0, TEST_WORLD_X + 4.0)
        );

        let round_trip = ManifoldPoint::from_raw(point.into_raw());
        assert_eq!(round_trip, point);
    }

    #[test]
    fn runtime_manifold_round_trip_uses_only_initialized_points() {
        let point = ManifoldPoint {
            anchor_a: Vec2::new(1.0, 2.0),
            anchor_b: Vec2::new(3.0, 4.0),
            separation: -0.25,
            base_separation: -0.125,
            normal_impulse: 5.0,
            tangent_impulse: 6.0,
            total_normal_impulse: 7.0,
            normal_velocity: -8.0,
            id: 9,
            persisted: true,
        };
        let manifold = Manifold {
            normal: Vec2::new(0.0, 1.0),
            rolling_impulse: 0.75,
            contact_points: [point, ManifoldPoint::default()],
            point_count: 1,
        };

        assert_eq!(manifold.point_count(), 1);
        assert_eq!(manifold.points(), &[point]);
        assert!(!manifold.is_empty());
        assert_eq!(Manifold::from_raw(manifold.into_raw()), manifold);
    }

    #[cfg(feature = "bytemuck")]
    #[test]
    fn world_value_layouts_have_no_padding() {
        assert_eq!(
            bytemuck::bytes_of(&Position::ZERO).len(),
            2 * core::mem::size_of::<WorldScalar>()
        );
        assert_eq!(
            bytemuck::bytes_of(&WorldTransform::IDENTITY).len(),
            core::mem::size_of::<WorldTransform>()
        );
    }
}
