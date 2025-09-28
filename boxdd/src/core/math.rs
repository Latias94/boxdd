use crate::types::Vec2;
use boxdd_sys::ffi;

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Rot(pub(crate) ffi::b2Rot);

impl Rot {
    pub const IDENTITY: Self = Self(ffi::b2Rot { c: 1.0, s: 0.0 });
    #[inline]
    pub fn from_radians(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self(ffi::b2Rot { c, s })
    }
    #[inline]
    pub fn from_degrees(deg: f32) -> Self {
        Self::from_radians(deg.to_radians())
    }
    #[inline]
    pub fn angle(self) -> f32 {
        self.0.s.atan2(self.0.c)
    }
    #[inline]
    pub fn rotate_vec(self, v: Vec2) -> Vec2 {
        let c = self.0.c;
        let s = self.0.s;
        Vec2 {
            x: c * v.x - s * v.y,
            y: s * v.x + c * v.y,
        }
    }
    #[inline]
    pub fn inv_rotate_vec(self, v: Vec2) -> Vec2 {
        let c = self.0.c;
        let s = self.0.s;
        Vec2 {
            x: c * v.x + s * v.y,
            y: -s * v.x + c * v.y,
        }
    }
}

impl From<Rot> for ffi::b2Rot {
    #[inline]
    fn from(r: Rot) -> Self {
        r.0
    }
}
impl From<ffi::b2Rot> for Rot {
    #[inline]
    fn from(r: ffi::b2Rot) -> Self {
        Self(r)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Transform(pub(crate) ffi::b2Transform);

impl Transform {
    pub const IDENTITY: Self = Self(ffi::b2Transform {
        p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
        q: ffi::b2Rot { c: 1.0, s: 0.0 },
    });
    #[inline]
    pub fn from_pos_angle<P: Into<Vec2>>(p: P, angle_radians: f32) -> Self {
        let pv: ffi::b2Vec2 = p.into().into();
        Self(ffi::b2Transform {
            p: pv,
            q: Rot::from_radians(angle_radians).into(),
        })
    }
    #[inline]
    pub fn position(self) -> Vec2 {
        Vec2::from(self.0.p)
    }
    #[inline]
    pub fn rotation(self) -> Rot {
        Rot(self.0.q)
    }
    #[inline]
    pub fn transform_point(self, v: Vec2) -> Vec2 {
        let r = Rot(self.0.q).rotate_vec(v);
        Vec2 {
            x: r.x + self.0.p.x,
            y: r.y + self.0.p.y,
        }
    }
    #[inline]
    pub fn inv_transform_point(self, v: Vec2) -> Vec2 {
        let dx = v.x - self.0.p.x;
        let dy = v.y - self.0.p.y;
        Rot(self.0.q).inv_rotate_vec(Vec2 { x: dx, y: dy })
    }
}

impl From<Transform> for ffi::b2Transform {
    #[inline]
    fn from(t: Transform) -> Self {
        t.0
    }
}
impl From<ffi::b2Transform> for Transform {
    #[inline]
    fn from(t: ffi::b2Transform) -> Self {
        Self(t)
    }
}

/// Small helpers for common world→local conversions used across joints/builders.
///
/// These match Box2D's convention for transforming a world-space point `p` into the
/// local frame given by `t`: `R^T * (p - t.p)`, where `R` is the rotation in `t.q`.
#[inline]
pub fn world_to_local_point(t: ffi::b2Transform, p_world: ffi::b2Vec2) -> ffi::b2Vec2 {
    let dx = p_world.x - t.p.x;
    let dy = p_world.y - t.p.y;
    let c = t.q.c;
    let s = t.q.s;
    ffi::b2Vec2 {
        x: c * dx + s * dy,
        y: -s * dx + c * dy,
    }
}

/// Compute a local rotation whose X-axis aligns with the given world axis.
///
/// This is used to construct joint frames that translate along or rotate about a world axis.
#[inline]
pub fn world_axis_to_local_rot(t: ffi::b2Transform, axis_world: ffi::b2Vec2) -> ffi::b2Rot {
    let angle_w = axis_world.y.atan2(axis_world.x);
    let angle_b = t.q.s.atan2(t.q.c);
    let (s, c) = (angle_w - angle_b).sin_cos();
    ffi::b2Rot { c, s }
}
