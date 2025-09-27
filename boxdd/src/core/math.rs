use crate::types::Vec2;
use boxdd_sys::ffi;

#[derive(Copy, Clone, Debug)]
pub struct Rot(pub(crate) ffi::b2Rot);

impl Rot {
    pub const IDENTITY: Self = Self(ffi::b2Rot { c: 1.0, s: 0.0 });
    pub fn from_radians(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self(ffi::b2Rot { c, s })
    }
    pub fn from_degrees(deg: f32) -> Self {
        Self::from_radians(deg.to_radians())
    }
    pub fn angle(self) -> f32 {
        self.0.s.atan2(self.0.c)
    }
    pub fn rotate_vec(self, v: Vec2) -> Vec2 {
        let c = self.0.c;
        let s = self.0.s;
        Vec2 {
            x: c * v.x - s * v.y,
            y: s * v.x + c * v.y,
        }
    }
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
    fn from(r: Rot) -> Self {
        r.0
    }
}
impl From<ffi::b2Rot> for Rot {
    fn from(r: ffi::b2Rot) -> Self {
        Self(r)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Transform(pub(crate) ffi::b2Transform);

impl Transform {
    pub const IDENTITY: Self = Self(ffi::b2Transform {
        p: ffi::b2Vec2 { x: 0.0, y: 0.0 },
        q: ffi::b2Rot { c: 1.0, s: 0.0 },
    });
    pub fn from_pos_angle<P: Into<ffi::b2Vec2>>(p: P, angle_radians: f32) -> Self {
        Self(ffi::b2Transform {
            p: p.into(),
            q: Rot::from_radians(angle_radians).into(),
        })
    }
    pub fn position(self) -> Vec2 {
        Vec2::from(self.0.p)
    }
    pub fn rotation(self) -> Rot {
        Rot(self.0.q)
    }
    pub fn transform_point(self, v: Vec2) -> Vec2 {
        let r = Rot(self.0.q).rotate_vec(v);
        Vec2 {
            x: r.x + self.0.p.x,
            y: r.y + self.0.p.y,
        }
    }
    pub fn inv_transform_point(self, v: Vec2) -> Vec2 {
        let dx = v.x - self.0.p.x;
        let dy = v.y - self.0.p.y;
        Rot(self.0.q).inv_rotate_vec(Vec2 { x: dx, y: dy })
    }
}

impl From<Transform> for ffi::b2Transform {
    fn from(t: Transform) -> Self {
        t.0
    }
}
impl From<ffi::b2Transform> for Transform {
    fn from(t: ffi::b2Transform) -> Self {
        Self(t)
    }
}
