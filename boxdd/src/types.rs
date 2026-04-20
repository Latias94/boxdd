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
}

impl From<ffi::b2Vec2> for Vec2 {
    #[inline]
    fn from(v: ffi::b2Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

impl From<Vec2> for ffi::b2Vec2 {
    #[inline]
    fn from(v: Vec2) -> Self {
        ffi::b2Vec2 { x: v.x, y: v.y }
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

// Public id aliases to avoid exposing `ffi::` in user-facing API/docstrings.
pub type BodyId = ffi::b2BodyId;
pub type ShapeId = ffi::b2ShapeId;
pub type JointId = ffi::b2JointId;
pub type ChainId = ffi::b2ChainId;
pub type ContactId = ffi::b2ContactId;

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
}

impl From<ffi::b2MassData> for MassData {
    #[inline]
    fn from(raw: ffi::b2MassData) -> Self {
        Self {
            mass: raw.mass,
            center: raw.center.into(),
            rotational_inertia: raw.rotationalInertia,
        }
    }
}

impl From<MassData> for ffi::b2MassData {
    #[inline]
    fn from(raw: MassData) -> Self {
        Self {
            mass: raw.mass,
            center: raw.center.into(),
            rotationalInertia: raw.rotational_inertia,
        }
    }
}

/// Maximum number of contact points supported by a Box2D manifold in 2D.
pub const MAX_MANIFOLD_POINTS: usize = 2;

/// A single contact point inside a contact manifold.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ManifoldPoint {
    pub point: Vec2,
    pub anchor_a: Vec2,
    pub anchor_b: Vec2,
    pub separation: f32,
    pub normal_impulse: f32,
    pub tangent_impulse: f32,
    pub total_normal_impulse: f32,
    pub normal_velocity: f32,
    pub id: u16,
    pub persisted: bool,
}

impl From<ffi::b2ManifoldPoint> for ManifoldPoint {
    #[inline]
    fn from(raw: ffi::b2ManifoldPoint) -> Self {
        Self {
            point: raw.point.into(),
            anchor_a: raw.anchorA.into(),
            anchor_b: raw.anchorB.into(),
            separation: raw.separation,
            normal_impulse: raw.normalImpulse,
            tangent_impulse: raw.tangentImpulse,
            total_normal_impulse: raw.totalNormalImpulse,
            normal_velocity: raw.normalVelocity,
            id: raw.id,
            persisted: raw.persisted,
        }
    }
}

impl From<ManifoldPoint> for ffi::b2ManifoldPoint {
    #[inline]
    fn from(raw: ManifoldPoint) -> Self {
        Self {
            point: raw.point.into(),
            anchorA: raw.anchor_a.into(),
            anchorB: raw.anchor_b.into(),
            separation: raw.separation,
            normalImpulse: raw.normal_impulse,
            tangentImpulse: raw.tangent_impulse,
            totalNormalImpulse: raw.total_normal_impulse,
            normalVelocity: raw.normal_velocity,
            id: raw.id,
            persisted: raw.persisted,
        }
    }
}

/// Contact manifold data between two colliding shapes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Manifold {
    pub normal: Vec2,
    pub rolling_impulse: f32,
    pub contact_points: [ManifoldPoint; MAX_MANIFOLD_POINTS],
    pub point_count: i32,
}

impl Manifold {
    #[inline]
    pub fn points(&self) -> &[ManifoldPoint] {
        let count = self.point_count.clamp(0, MAX_MANIFOLD_POINTS as i32) as usize;
        &self.contact_points[..count]
    }
}

impl From<ffi::b2Manifold> for Manifold {
    #[inline]
    fn from(raw: ffi::b2Manifold) -> Self {
        Self {
            normal: raw.normal.into(),
            rolling_impulse: raw.rollingImpulse,
            contact_points: raw.points.map(Into::into),
            point_count: raw.pointCount.clamp(0, MAX_MANIFOLD_POINTS as i32),
        }
    }
}

impl From<Manifold> for ffi::b2Manifold {
    #[inline]
    fn from(raw: Manifold) -> Self {
        Self {
            normal: raw.normal.into(),
            rollingImpulse: raw.rolling_impulse,
            points: raw.contact_points.map(Into::into),
            pointCount: raw.point_count.clamp(0, MAX_MANIFOLD_POINTS as i32),
        }
    }
}

/// Contact data for a single contact touching two shapes.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ContactData {
    pub contact_id: ContactId,
    pub shape_id_a: ShapeId,
    pub shape_id_b: ShapeId,
    pub manifold: Manifold,
}

impl From<ffi::b2ContactData> for ContactData {
    #[inline]
    fn from(raw: ffi::b2ContactData) -> Self {
        Self {
            contact_id: raw.contactId,
            shape_id_a: raw.shapeIdA,
            shape_id_b: raw.shapeIdB,
            manifold: raw.manifold.into(),
        }
    }
}

impl From<ContactData> for ffi::b2ContactData {
    #[inline]
    fn from(raw: ContactData) -> Self {
        Self {
            contactId: raw.contact_id,
            shapeIdA: raw.shape_id_a,
            shapeIdB: raw.shape_id_b,
            manifold: raw.manifold.into(),
        }
    }
}

const _: () = {
    assert!(core::mem::size_of::<MassData>() == core::mem::size_of::<ffi::b2MassData>());
    assert!(core::mem::align_of::<MassData>() == core::mem::align_of::<ffi::b2MassData>());
    assert!(core::mem::size_of::<ManifoldPoint>() == core::mem::size_of::<ffi::b2ManifoldPoint>());
    assert!(
        core::mem::align_of::<ManifoldPoint>() == core::mem::align_of::<ffi::b2ManifoldPoint>()
    );
    assert!(core::mem::size_of::<Manifold>() == core::mem::size_of::<ffi::b2Manifold>());
    assert!(core::mem::align_of::<Manifold>() == core::mem::align_of::<ffi::b2Manifold>());
    assert!(core::mem::size_of::<ContactData>() == core::mem::size_of::<ffi::b2ContactData>());
    assert!(core::mem::align_of::<ContactData>() == core::mem::align_of::<ffi::b2ContactData>());
};
