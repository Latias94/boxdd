use crate::types::Vec2;
use boxdd_sys::ffi;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Rot {
    pub(crate) c: f32,
    pub(crate) s: f32,
}

impl Rot {
    pub const IDENTITY: Self = Self { c: 1.0, s: 0.0 };
    #[inline]
    pub fn from_radians(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self { c, s }
    }
    #[inline]
    pub fn from_degrees(deg: f32) -> Self {
        Self::from_radians(deg.to_radians())
    }
    #[inline]
    pub fn angle(self) -> f32 {
        self.s.atan2(self.c)
    }
    #[inline]
    pub fn rotate_vec(self, v: Vec2) -> Vec2 {
        let c = self.c;
        let s = self.s;
        Vec2 {
            x: c * v.x - s * v.y,
            y: s * v.x + c * v.y,
        }
    }
    #[inline]
    pub fn inv_rotate_vec(self, v: Vec2) -> Vec2 {
        let c = self.c;
        let s = self.s;
        Vec2 {
            x: c * v.x + s * v.y,
            y: -s * v.x + c * v.y,
        }
    }
}

impl From<Rot> for ffi::b2Rot {
    #[inline]
    fn from(r: Rot) -> Self {
        ffi::b2Rot { c: r.c, s: r.s }
    }
}
impl From<ffi::b2Rot> for Rot {
    #[inline]
    fn from(r: ffi::b2Rot) -> Self {
        Self { c: r.c, s: r.s }
    }
}

// serde support for Rot as angle (radians)
#[cfg(feature = "serde")]
impl serde::Serialize for Rot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f32(self.angle())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Rot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let angle = f32::deserialize(deserializer)?;
        Ok(Rot::from_radians(angle))
    }
}

#[cfg(feature = "mint")]
impl From<Rot> for mint::RowMatrix2<f32> {
    #[inline]
    fn from(r: Rot) -> Self {
        Self {
            x: mint::Vector2 { x: r.c, y: -r.s },
            y: mint::Vector2 { x: r.s, y: r.c },
        }
    }
}

#[cfg(feature = "mint")]
impl From<Rot> for mint::ColumnMatrix2<f32> {
    #[inline]
    fn from(r: Rot) -> Self {
        mint::RowMatrix2::from(r).into()
    }
}

#[cfg(feature = "mint")]
#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum RotFromMintError {
    #[error("non-finite value in mint rotation matrix")]
    NonFinite,
    #[error("mint matrix is not a pure rotation")]
    NotPureRotation,
}

#[cfg(feature = "mint")]
impl TryFrom<mint::RowMatrix2<f32>> for Rot {
    type Error = RotFromMintError;

    #[inline]
    fn try_from(m: mint::RowMatrix2<f32>) -> Result<Self, Self::Error> {
        let a = m.x.x;
        let b = m.x.y;
        let c = m.y.x;
        let d = m.y.y;

        if !(a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite()) {
            return Err(RotFromMintError::NonFinite);
        }

        let eps = 1.0e-4;
        let row0_len2 = a * a + b * b;
        let row1_len2 = c * c + d * d;
        if (row0_len2 - 1.0).abs() > eps || (row1_len2 - 1.0).abs() > eps {
            return Err(RotFromMintError::NotPureRotation);
        }
        if (a * c + b * d).abs() > eps {
            return Err(RotFromMintError::NotPureRotation);
        }
        let det = a * d - b * c;
        if (det - 1.0).abs() > 5.0e-4 {
            return Err(RotFromMintError::NotPureRotation);
        }

        // Expected form: [[c, -s], [s, c]].
        if (b + c).abs() > 1.0e-4 || (d - a).abs() > 1.0e-4 {
            return Err(RotFromMintError::NotPureRotation);
        }

        Ok(Rot { c: a, s: c })
    }
}

#[cfg(feature = "mint")]
impl TryFrom<&mint::RowMatrix2<f32>> for Rot {
    type Error = RotFromMintError;

    #[inline]
    fn try_from(m: &mint::RowMatrix2<f32>) -> Result<Self, Self::Error> {
        Self::try_from(*m)
    }
}

#[cfg(feature = "mint")]
impl TryFrom<mint::ColumnMatrix2<f32>> for Rot {
    type Error = RotFromMintError;

    #[inline]
    fn try_from(m: mint::ColumnMatrix2<f32>) -> Result<Self, Self::Error> {
        Self::try_from(mint::RowMatrix2::from(m))
    }
}

#[cfg(feature = "mint")]
impl TryFrom<&mint::ColumnMatrix2<f32>> for Rot {
    type Error = RotFromMintError;

    #[inline]
    fn try_from(m: &mint::ColumnMatrix2<f32>) -> Result<Self, Self::Error> {
        Self::try_from(*m)
    }
}

// Interop with common math libraries for rotations
#[cfg(feature = "cgmath")]
impl From<Rot> for cgmath::Basis2<f32> {
    #[inline]
    fn from(r: Rot) -> Self {
        use cgmath::Rotation2;
        cgmath::Basis2::from_angle(cgmath::Rad(r.angle()))
    }
}

#[cfg(feature = "cgmath")]
impl<'a> From<&'a cgmath::Basis2<f32>> for Rot {
    #[inline]
    fn from(b: &'a cgmath::Basis2<f32>) -> Self {
        let col_x = b.as_ref().x; // rotation's X axis
        Rot {
            c: col_x.x,
            s: col_x.y,
        }
    }
}

#[cfg(feature = "cgmath")]
impl From<Transform> for cgmath::Matrix3<f32> {
    #[inline]
    fn from(t: Transform) -> Self {
        use cgmath::Vector3;
        let c = t.q.c;
        let s = t.q.s;
        cgmath::Matrix3 {
            // Column-major affine 2D transform:
            // [ c -s tx ]
            // [ s  c ty ]
            // [ 0  0  1 ]
            x: Vector3::new(c, s, 0.0),
            y: Vector3::new(-s, c, 0.0),
            z: Vector3::new(t.p.x, t.p.y, 1.0),
        }
    }
}

#[cfg(feature = "cgmath")]
impl From<&Transform> for cgmath::Matrix3<f32> {
    #[inline]
    fn from(t: &Transform) -> Self {
        (*t).into()
    }
}

#[cfg(feature = "cgmath")]
#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum TransformFromCgmathError {
    #[error("non-finite value in cgmath::Matrix3")]
    NonFinite,
    #[error("cgmath::Matrix3 is not a pure rotation + translation")]
    NotPureRotation,
}

#[cfg(feature = "cgmath")]
impl TryFrom<cgmath::Matrix3<f32>> for Transform {
    type Error = TransformFromCgmathError;

    #[inline]
    fn try_from(m: cgmath::Matrix3<f32>) -> Result<Self, Self::Error> {
        let x = m.x;
        let y = m.y;
        let z = m.z;

        if !(x.x.is_finite()
            && x.y.is_finite()
            && x.z.is_finite()
            && y.x.is_finite()
            && y.y.is_finite()
            && y.z.is_finite()
            && z.x.is_finite()
            && z.y.is_finite()
            && z.z.is_finite())
        {
            return Err(TransformFromCgmathError::NonFinite);
        }

        // Reject non-affine 2D transforms.
        let eps = 1.0e-4;
        if x.z.abs() > eps || y.z.abs() > eps || (z.z - 1.0).abs() > eps {
            return Err(TransformFromCgmathError::NotPureRotation);
        }

        // We only accept pure rotations (orthonormal basis with determinant +1).
        let x_len2 = x.x * x.x + x.y * x.y;
        let y_len2 = y.x * y.x + y.y * y.y;
        if (x_len2 - 1.0).abs() > eps || (y_len2 - 1.0).abs() > eps {
            return Err(TransformFromCgmathError::NotPureRotation);
        }
        if (x.x * y.x + x.y * y.y).abs() > eps {
            return Err(TransformFromCgmathError::NotPureRotation);
        }
        let det = x.x * y.y - x.y * y.x;
        if (det - 1.0).abs() > 5.0e-4 {
            return Err(TransformFromCgmathError::NotPureRotation);
        }

        // Our convention: columns are [c, s] and [-s, c]
        let expected_y_x = -x.y;
        let expected_y_y = x.x;
        let dy_x = y.x - expected_y_x;
        let dy_y = y.y - expected_y_y;
        if dy_x * dy_x + dy_y * dy_y > 1.0e-6 {
            return Err(TransformFromCgmathError::NotPureRotation);
        }

        Ok(Transform {
            p: Vec2 { x: z.x, y: z.y },
            q: Rot { c: x.x, s: x.y },
        })
    }
}

#[cfg(feature = "cgmath")]
impl TryFrom<&cgmath::Matrix3<f32>> for Transform {
    type Error = TransformFromCgmathError;

    #[inline]
    fn try_from(m: &cgmath::Matrix3<f32>) -> Result<Self, Self::Error> {
        Self::try_from(*m)
    }
}

#[cfg(feature = "nalgebra")]
impl From<Rot> for nalgebra::UnitComplex<f32> {
    #[inline]
    fn from(r: Rot) -> Self {
        nalgebra::UnitComplex::new(r.angle())
    }
}

#[cfg(feature = "nalgebra")]
impl<'a> From<&'a nalgebra::UnitComplex<f32>> for Rot {
    #[inline]
    fn from(r: &'a nalgebra::UnitComplex<f32>) -> Self {
        Rot::from_radians(r.angle())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Transform {
    pub(crate) p: Vec2,
    pub(crate) q: Rot,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        p: Vec2 { x: 0.0, y: 0.0 },
        q: Rot::IDENTITY,
    };
    #[inline]
    pub fn from_pos_angle<P: Into<Vec2>>(p: P, angle_radians: f32) -> Self {
        Self {
            p: p.into(),
            q: Rot::from_radians(angle_radians),
        }
    }
    #[inline]
    pub fn position(self) -> Vec2 {
        self.p
    }
    #[inline]
    pub fn rotation(self) -> Rot {
        self.q
    }
    #[inline]
    pub fn transform_point(self, v: Vec2) -> Vec2 {
        let r = self.q.rotate_vec(v);
        Vec2 {
            x: r.x + self.p.x,
            y: r.y + self.p.y,
        }
    }
    #[inline]
    pub fn inv_transform_point(self, v: Vec2) -> Vec2 {
        let dx = v.x - self.p.x;
        let dy = v.y - self.p.y;
        self.q.inv_rotate_vec(Vec2 { x: dx, y: dy })
    }
}

impl From<Transform> for ffi::b2Transform {
    #[inline]
    fn from(t: Transform) -> Self {
        ffi::b2Transform {
            p: t.p.into(),
            q: t.q.into(),
        }
    }
}
impl From<ffi::b2Transform> for Transform {
    #[inline]
    fn from(t: ffi::b2Transform) -> Self {
        Self {
            p: Vec2::from(t.p),
            q: Rot::from(t.q),
        }
    }
}

#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Zeroable for Rot {}
#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Pod for Rot {}
#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Zeroable for Transform {}
#[cfg(feature = "bytemuck")]
unsafe impl bytemuck::Pod for Transform {}

#[cfg(feature = "bytemuck")]
const _: () = {
    assert!(core::mem::size_of::<Rot>() == 8);
    assert!(core::mem::align_of::<Rot>() == 4);
    assert!(core::mem::size_of::<Transform>() == 16);
    assert!(core::mem::align_of::<Transform>() == 4);
};

#[cfg(feature = "glam")]
impl From<Rot> for glam::Mat2 {
    #[inline]
    fn from(r: Rot) -> Self {
        let x = glam::Vec2::new(r.c, r.s);
        let y = glam::Vec2::new(-r.s, r.c);
        glam::Mat2::from_cols(x, y)
    }
}

#[cfg(feature = "glam")]
#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum RotFromGlamError {
    #[error("non-finite value in glam::Mat2")]
    NonFinite,
    #[error("glam::Mat2 is not a pure rotation")]
    NotPureRotation,
}

#[cfg(feature = "glam")]
impl TryFrom<glam::Mat2> for Rot {
    type Error = RotFromGlamError;

    #[inline]
    fn try_from(m: glam::Mat2) -> Result<Self, Self::Error> {
        let x = m.x_axis;
        let y = m.y_axis;

        if !(x.is_finite() && y.is_finite()) {
            return Err(RotFromGlamError::NonFinite);
        }

        let eps = 1.0e-4;
        let x_len2 = x.length_squared();
        let y_len2 = y.length_squared();
        if (x_len2 - 1.0).abs() > eps || (y_len2 - 1.0).abs() > eps {
            return Err(RotFromGlamError::NotPureRotation);
        }
        if x.dot(y).abs() > eps {
            return Err(RotFromGlamError::NotPureRotation);
        }
        let det = x.x * y.y - x.y * y.x;
        if (det - 1.0).abs() > 5.0e-4 {
            return Err(RotFromGlamError::NotPureRotation);
        }

        let expected_y = glam::Vec2::new(-x.y, x.x);
        if (y - expected_y).length_squared() > 1.0e-6 {
            return Err(RotFromGlamError::NotPureRotation);
        }

        Ok(Rot { c: x.x, s: x.y })
    }
}

#[cfg(feature = "glam")]
impl TryFrom<&glam::Mat2> for Rot {
    type Error = RotFromGlamError;

    #[inline]
    fn try_from(m: &glam::Mat2) -> Result<Self, Self::Error> {
        Self::try_from(*m)
    }
}

#[cfg(feature = "glam")]
impl From<Transform> for glam::Affine2 {
    #[inline]
    fn from(t: Transform) -> Self {
        glam::Affine2::from_mat2_translation(t.q.into(), t.p.into())
    }
}

#[cfg(feature = "glam")]
#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum TransformFromGlamError {
    #[error("non-finite value in glam::Affine2")]
    NonFinite,
    #[error("glam::Affine2 is not a pure rotation + translation")]
    NotPureRotation,
}

#[cfg(feature = "glam")]
impl TryFrom<glam::Affine2> for Transform {
    type Error = TransformFromGlamError;

    #[inline]
    fn try_from(a: glam::Affine2) -> Result<Self, Self::Error> {
        let t = a.translation;
        let x = a.matrix2.x_axis;
        let y = a.matrix2.y_axis;

        if !(t.is_finite() && x.is_finite() && y.is_finite()) {
            return Err(TransformFromGlamError::NonFinite);
        }

        // We only accept pure rotations (orthonormal basis with determinant +1).
        // This rejects scale/shear/mirror transforms.
        let eps = 1.0e-4;
        let x_len2 = x.length_squared();
        let y_len2 = y.length_squared();
        if (x_len2 - 1.0).abs() > eps || (y_len2 - 1.0).abs() > eps {
            return Err(TransformFromGlamError::NotPureRotation);
        }
        if x.dot(y).abs() > eps {
            return Err(TransformFromGlamError::NotPureRotation);
        }
        let det = x.x * y.y - x.y * y.x;
        if (det - 1.0).abs() > 5.0e-4 {
            return Err(TransformFromGlamError::NotPureRotation);
        }

        // Our convention: columns are [c, s] and [-s, c]
        let expected_y = glam::Vec2::new(-x.y, x.x);
        if (y - expected_y).length_squared() > 1.0e-6 {
            return Err(TransformFromGlamError::NotPureRotation);
        }

        Ok(Transform {
            p: t.into(),
            q: Rot { c: x.x, s: x.y },
        })
    }
}

#[cfg(feature = "glam")]
impl TryFrom<&glam::Affine2> for Transform {
    type Error = TransformFromGlamError;

    #[inline]
    fn try_from(a: &glam::Affine2) -> Result<Self, Self::Error> {
        Self::try_from(*a)
    }
}

#[cfg(feature = "mint")]
#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum TransformFromMintError {
    #[error("non-finite value in mint transform matrix")]
    NonFinite,
    #[error("mint matrix is not a pure rotation + translation")]
    NotPureRotation,
}

#[cfg(feature = "mint")]
impl TryFrom<mint::RowMatrix3x2<f32>> for Transform {
    type Error = TransformFromMintError;

    #[inline]
    fn try_from(m: mint::RowMatrix3x2<f32>) -> Result<Self, Self::Error> {
        let a = m.x.x;
        let b = m.x.y;
        let c = m.y.x;
        let d = m.y.y;
        let tx = m.z.x;
        let ty = m.z.y;

        if !(a.is_finite()
            && b.is_finite()
            && c.is_finite()
            && d.is_finite()
            && tx.is_finite()
            && ty.is_finite())
        {
            return Err(TransformFromMintError::NonFinite);
        }

        // We only accept pure rotations (orthonormal basis with determinant +1).
        let eps = 1.0e-4;
        let row0_len2 = a * a + b * b;
        let row1_len2 = c * c + d * d;
        if (row0_len2 - 1.0).abs() > eps || (row1_len2 - 1.0).abs() > eps {
            return Err(TransformFromMintError::NotPureRotation);
        }
        if (a * c + b * d).abs() > eps {
            return Err(TransformFromMintError::NotPureRotation);
        }
        let det = a * d - b * c;
        if (det - 1.0).abs() > 5.0e-4 {
            return Err(TransformFromMintError::NotPureRotation);
        }

        // Expected form: [[c, -s], [s, c]].
        if (b + c).abs() > 1.0e-4 || (d - a).abs() > 1.0e-4 {
            return Err(TransformFromMintError::NotPureRotation);
        }

        Ok(Transform {
            p: Vec2 { x: tx, y: ty },
            q: Rot { c: a, s: c },
        })
    }
}

#[cfg(feature = "mint")]
impl TryFrom<&mint::RowMatrix3x2<f32>> for Transform {
    type Error = TransformFromMintError;

    #[inline]
    fn try_from(m: &mint::RowMatrix3x2<f32>) -> Result<Self, Self::Error> {
        Self::try_from(*m)
    }
}

#[cfg(feature = "mint")]
impl From<Transform> for mint::RowMatrix3x2<f32> {
    #[inline]
    fn from(t: Transform) -> Self {
        let c = t.q.c;
        let s = t.q.s;
        Self {
            x: mint::Vector2 { x: c, y: -s },
            y: mint::Vector2 { x: s, y: c },
            z: mint::Vector2 { x: t.p.x, y: t.p.y },
        }
    }
}

#[cfg(feature = "mint")]
impl TryFrom<mint::ColumnMatrix3x2<f32>> for Transform {
    type Error = TransformFromMintError;

    #[inline]
    fn try_from(m: mint::ColumnMatrix3x2<f32>) -> Result<Self, Self::Error> {
        Self::try_from(mint::RowMatrix3x2::from(m))
    }
}

#[cfg(feature = "mint")]
impl TryFrom<&mint::ColumnMatrix3x2<f32>> for Transform {
    type Error = TransformFromMintError;

    #[inline]
    fn try_from(m: &mint::ColumnMatrix3x2<f32>) -> Result<Self, Self::Error> {
        Self::try_from(*m)
    }
}

#[cfg(feature = "mint")]
impl From<Transform> for mint::ColumnMatrix3x2<f32> {
    #[inline]
    fn from(t: Transform) -> Self {
        mint::RowMatrix3x2::from(t).into()
    }
}

#[cfg(feature = "mint")]
impl TryFrom<mint::RowMatrix2x3<f32>> for Transform {
    type Error = TransformFromMintError;

    #[inline]
    fn try_from(m: mint::RowMatrix2x3<f32>) -> Result<Self, Self::Error> {
        let a = m.x.x;
        let b = m.x.y;
        let c = m.y.x;
        let d = m.y.y;
        let tx = m.x.z;
        let ty = m.y.z;

        if !(a.is_finite()
            && b.is_finite()
            && c.is_finite()
            && d.is_finite()
            && tx.is_finite()
            && ty.is_finite())
        {
            return Err(TransformFromMintError::NonFinite);
        }

        let eps = 1.0e-4;
        let row0_len2 = a * a + b * b;
        let row1_len2 = c * c + d * d;
        if (row0_len2 - 1.0).abs() > eps || (row1_len2 - 1.0).abs() > eps {
            return Err(TransformFromMintError::NotPureRotation);
        }
        if (a * c + b * d).abs() > eps {
            return Err(TransformFromMintError::NotPureRotation);
        }
        let det = a * d - b * c;
        if (det - 1.0).abs() > 5.0e-4 {
            return Err(TransformFromMintError::NotPureRotation);
        }
        if (b + c).abs() > 1.0e-4 || (d - a).abs() > 1.0e-4 {
            return Err(TransformFromMintError::NotPureRotation);
        }

        Ok(Transform {
            p: Vec2 { x: tx, y: ty },
            q: Rot { c: a, s: c },
        })
    }
}

#[cfg(feature = "mint")]
impl TryFrom<&mint::RowMatrix2x3<f32>> for Transform {
    type Error = TransformFromMintError;

    #[inline]
    fn try_from(m: &mint::RowMatrix2x3<f32>) -> Result<Self, Self::Error> {
        Self::try_from(*m)
    }
}

#[cfg(feature = "mint")]
impl From<Transform> for mint::RowMatrix2x3<f32> {
    #[inline]
    fn from(t: Transform) -> Self {
        let c = t.q.c;
        let s = t.q.s;
        Self {
            x: mint::Vector3 {
                x: c,
                y: -s,
                z: t.p.x,
            },
            y: mint::Vector3 {
                x: s,
                y: c,
                z: t.p.y,
            },
        }
    }
}

#[cfg(feature = "mint")]
impl TryFrom<mint::ColumnMatrix2x3<f32>> for Transform {
    type Error = TransformFromMintError;

    #[inline]
    fn try_from(m: mint::ColumnMatrix2x3<f32>) -> Result<Self, Self::Error> {
        Self::try_from(mint::RowMatrix2x3::from(m))
    }
}

#[cfg(feature = "mint")]
impl TryFrom<&mint::ColumnMatrix2x3<f32>> for Transform {
    type Error = TransformFromMintError;

    #[inline]
    fn try_from(m: &mint::ColumnMatrix2x3<f32>) -> Result<Self, Self::Error> {
        Self::try_from(*m)
    }
}

#[cfg(feature = "mint")]
impl From<Transform> for mint::ColumnMatrix2x3<f32> {
    #[inline]
    fn from(t: Transform) -> Self {
        mint::RowMatrix2x3::from(t).into()
    }
}

// serde support for Transform as { pos, angle } (radians)
#[cfg(feature = "serde")]
impl serde::Serialize for Transform {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(serde::Serialize)]
        struct Repr {
            pos: super::super::types::Vec2,
            angle: f32,
        }
        let r = Repr {
            pos: self.position(),
            angle: self.rotation().angle(),
        };
        r.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Transform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            pos: super::super::types::Vec2,
            angle: f32,
        }
        let r = Repr::deserialize(deserializer)?;
        Ok(Transform::from_pos_angle(r.pos, r.angle))
    }
}

// Interop with nalgebra isometry
#[cfg(feature = "nalgebra")]
impl<'a> From<&'a Transform> for nalgebra::Isometry2<f32> {
    #[inline]
    fn from(t: &'a Transform) -> Self {
        let p = t.position();
        let rot = nalgebra::UnitComplex::new(t.rotation().angle());
        nalgebra::Isometry2::from_parts(nalgebra::Translation2::new(p.x, p.y), rot)
    }
}

#[cfg(feature = "nalgebra")]
impl<'a> From<&'a nalgebra::Isometry2<f32>> for Transform {
    #[inline]
    fn from(i: &'a nalgebra::Isometry2<f32>) -> Self {
        let v = i.translation.vector;
        let angle = i.rotation.angle();
        Transform::from_pos_angle(Vec2 { x: v.x, y: v.y }, angle)
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
