use crate::{core::foundation::transient_native_lease, error::Error, types::Vec2};
use boxdd_sys::ffi;

type Result<T, E = Error> = core::result::Result<T, E>;

/// Box2D runtime version.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Version {
    pub major: i32,
    pub minor: i32,
    pub revision: i32,
}

impl Version {
    #[inline]
    pub const fn from_raw(raw: ffi::b2Version) -> Self {
        Self {
            major: raw.major,
            minor: raw.minor,
            revision: raw.revision,
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2Version {
        ffi::b2Version {
            major: self.major,
            minor: self.minor,
            revision: self.revision,
        }
    }
}

/// Get the linked Box2D version.
#[inline]
pub fn version() -> Result<Version> {
    let _lease = transient_native_lease()?;
    Ok(Version::from_raw(unsafe { ffi::b2GetVersion() }))
}

/// Initial seed used by Box2D's deterministic djb2 hash helper.
pub const HASH_INIT: u32 = ffi::B2_HASH_INIT;

/// Check whether a scalar is valid for Box2D APIs.
#[inline]
pub fn is_valid_float(value: f32) -> bool {
    value.is_finite()
}

/// Get the total number of bytes currently allocated by Box2D.
///
/// The return type is fixed-width to match upstream `int64_t`, independent of
/// the host pointer width. Box2D reports a non-negative allocation count.
#[inline]
pub fn allocated_byte_count() -> Result<i64> {
    let _lease = transient_native_lease()?;
    let count = unsafe { ffi::b2GetByteCount() };
    if count < 0 {
        Err(Error::NegativeAllocatedByteCount { count })
    } else {
        Ok(count)
    }
}

/// Get the absolute number of platform-specific system ticks.
#[inline]
pub fn ticks() -> Result<u64> {
    let _lease = transient_native_lease()?;
    Ok(unsafe { ffi::b2GetTicks() })
}

/// Get the elapsed milliseconds since `start_ticks`.
#[inline]
pub fn milliseconds_since(start_ticks: u64) -> Result<f32> {
    let _lease = transient_native_lease()?;
    let milliseconds = unsafe { ffi::b2GetMilliseconds(start_ticks) };
    if milliseconds.is_finite() && milliseconds >= 0.0 {
        Ok(milliseconds)
    } else {
        Err(Error::InvalidNativeElapsedMilliseconds)
    }
}

/// Get the elapsed milliseconds since `start_ticks` and reset it to the current tick value.
#[inline]
pub fn milliseconds_and_reset(start_ticks: &mut u64) -> Result<f32> {
    let _lease = transient_native_lease()?;
    let mut staged_ticks = *start_ticks;
    let milliseconds = unsafe { ffi::b2GetMillisecondsAndReset(&mut staged_ticks) };
    if milliseconds.is_finite() && milliseconds >= 0.0 {
        *start_ticks = staged_ticks;
        Ok(milliseconds)
    } else {
        Err(Error::InvalidNativeElapsedMilliseconds)
    }
}

/// Yield the current thread, matching Box2D's busy-loop helper.
#[inline]
pub fn yield_now() -> Result<()> {
    let _lease = transient_native_lease()?;
    unsafe { ffi::b2Yield() };
    Ok(())
}

/// Hash bytes with Box2D's deterministic djb2 helper.
#[inline]
pub fn hash_bytes(hash: u32, data: &[u8]) -> Result<u32> {
    let _lease = transient_native_lease()?;
    let count = i32::try_from(data.len()).map_err(|_| {
        Error::invalid_argument(
            "hash_bytes",
            "data",
            "a byte slice whose length is representable by a native int",
        )
    })?;
    Ok(unsafe { ffi::b2Hash(hash, data.as_ptr(), count) })
}

/// Cross-platform deterministic `atan2` in the range `[-pi, pi]`.
#[inline]
pub fn atan2(y: f32, x: f32) -> Result<f32> {
    let _lease = transient_native_lease()?;
    if !y.is_finite() {
        return Err(Error::invalid_argument("atan2", "y", "a finite value"));
    }
    if !x.is_finite() {
        return Err(Error::invalid_argument("atan2", "x", "a finite value"));
    }
    let angle = unsafe { ffi::b2Atan2(y, x) };
    if angle.is_finite() {
        Ok(angle)
    } else {
        Err(Error::InvalidNativeAngle)
    }
}

/// Cross-platform deterministic cosine/sine pair as a rotation value.
#[inline]
pub fn compute_cos_sin(radians: f32) -> Result<Rot> {
    let _lease = transient_native_lease()?;
    if !radians.is_finite() {
        return Err(Error::invalid_argument(
            "compute_cos_sin",
            "radians",
            "a finite value",
        ));
    }
    let raw: ffi::b2CosSin = unsafe { ffi::b2ComputeCosSin(radians) };
    let rotation = Rot {
        c: raw.cosine,
        s: raw.sine,
    };
    if rotation.is_valid() {
        Ok(rotation)
    } else {
        Err(Error::InvalidNativeRotation)
    }
}

const UNIT_VECTOR_LENGTH_TOLERANCE: f32 = 100.0 * f32::EPSILON;

#[inline]
fn check_unit_vector(argument: &'static str, vector: Vec2) -> Result<()> {
    let length = (vector.x * vector.x + vector.y * vector.y).sqrt();
    if vector.is_valid()
        && length.is_finite()
        && (1.0 - length).abs() < UNIT_VECTOR_LENGTH_TOLERANCE
    {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            "rotation_between_unit_vectors",
            argument,
            "a finite unit vector within Box2D's length tolerance",
        ))
    }
}

/// Compute the rotation between two finite unit vectors.
///
/// The unit-length tolerance exactly matches the pinned Box2D precondition, so invalid input is
/// rejected in Rust instead of reaching a native assertion.
#[inline]
pub fn rotation_between_unit_vectors<V1: Into<Vec2>, V2: Into<Vec2>>(
    v1: V1,
    v2: V2,
) -> Result<Rot> {
    let v1 = v1.into();
    let v2 = v2.into();
    check_unit_vector("v1", v1)?;
    check_unit_vector("v2", v2)?;
    let _lease = transient_native_lease()?;
    let rotation = Rot::from_raw_unvalidated(unsafe {
        ffi::b2ComputeRotationBetweenUnitVectors(v1.into_raw(), v2.into_raw())
    });
    if rotation.is_valid() {
        Ok(rotation)
    } else {
        Err(Error::InvalidNativeRotation)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Rot {
    pub(crate) c: f32,
    pub(crate) s: f32,
}

impl Rot {
    pub const IDENTITY: Self = Self { c: 1.0, s: 0.0 };

    #[inline]
    pub fn from_raw(raw: ffi::b2Rot) -> Result<Self> {
        let rotation = Self::from_raw_unvalidated(raw);
        if rotation.is_valid() {
            Ok(rotation)
        } else {
            Err(Error::invalid_argument(
                "Rot::from_raw",
                "raw",
                "a normalized finite rotation",
            ))
        }
    }

    #[inline]
    pub(crate) const fn from_raw_unvalidated(raw: ffi::b2Rot) -> Self {
        Self { c: raw.c, s: raw.s }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2Rot {
        ffi::b2Rot {
            c: self.c,
            s: self.s,
        }
    }

    #[inline]
    pub fn from_radians(rad: f32) -> Result<Self> {
        if !rad.is_finite() {
            return Err(Error::invalid_argument(
                "Rot::from_radians",
                "rad",
                "a finite angle",
            ));
        }
        Ok(Self::from_radians_unvalidated(rad))
    }

    #[inline]
    pub(crate) fn from_radians_unvalidated(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self { c, s }
    }
    #[inline]
    pub fn cosine(self) -> f32 {
        self.c
    }
    #[inline]
    pub fn sine(self) -> f32 {
        self.s
    }
    #[inline]
    pub fn from_degrees(deg: f32) -> Result<Self> {
        if !deg.is_finite() {
            return Err(Error::invalid_argument(
                "Rot::from_degrees",
                "deg",
                "a finite angle",
            ));
        }
        Ok(Self::from_radians_unvalidated(deg.to_radians()))
    }
    #[inline]
    pub fn angle(self) -> f32 {
        self.s.atan2(self.c)
    }
    #[inline]
    pub fn is_valid(self) -> bool {
        if !self.c.is_finite() || !self.s.is_finite() {
            return false;
        }

        let magnitude_squared = self.s * self.s + self.c * self.c;
        1.0 - 0.0006 < magnitude_squared && magnitude_squared < 1.0 + 0.0006
    }
    #[inline]
    pub fn from_unit_vectors<V1: Into<Vec2>, V2: Into<Vec2>>(v1: V1, v2: V2) -> Result<Self> {
        rotation_between_unit_vectors(v1, v2)
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

// serde support for Rot as angle (radians)
#[cfg(feature = "serde")]
impl serde::Serialize for Rot {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f32(self.angle())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Rot {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let angle = <f32 as serde::Deserialize>::deserialize(deserializer)?;
        Rot::from_radians(angle).map_err(serde::de::Error::custom)
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

#[cfg(feature = "nalgebra")]
impl From<Rot> for nalgebra::UnitComplex<f32> {
    #[inline]
    fn from(r: Rot) -> Self {
        nalgebra::UnitComplex::new(r.angle())
    }
}

#[cfg(feature = "nalgebra")]
impl<'a> TryFrom<&'a nalgebra::UnitComplex<f32>> for Rot {
    type Error = Error;

    #[inline]
    fn try_from(r: &'a nalgebra::UnitComplex<f32>) -> Result<Self> {
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
    pub fn from_raw(raw: ffi::b2Transform) -> Result<Self> {
        let transform = Self::from_raw_unvalidated(raw);
        if transform.is_valid() {
            Ok(transform)
        } else {
            Err(Error::invalid_argument(
                "Transform::from_raw",
                "raw",
                "a finite rigid transform",
            ))
        }
    }

    #[inline]
    pub(crate) const fn from_raw_unvalidated(raw: ffi::b2Transform) -> Self {
        Self {
            p: Vec2::from_raw(raw.p),
            q: Rot::from_raw_unvalidated(raw.q),
        }
    }

    #[inline]
    pub const fn into_raw(self) -> ffi::b2Transform {
        ffi::b2Transform {
            p: self.p.into_raw(),
            q: self.q.into_raw(),
        }
    }

    #[inline]
    pub fn from_pos_angle<P: Into<Vec2>>(p: P, angle_radians: f32) -> Result<Self> {
        let position = p.into();
        if !position.is_valid() {
            return Err(Error::invalid_argument(
                "Transform::from_pos_angle",
                "position",
                "a finite vector",
            ));
        }
        Ok(Self {
            p: position,
            q: Rot::from_radians(angle_radians)?,
        })
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
    pub fn is_valid(self) -> bool {
        self.p.is_valid() && self.q.is_valid()
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
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
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
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Repr {
            pos: super::super::types::Vec2,
            angle: f32,
        }
        let r = <Repr as serde::Deserialize>::deserialize(deserializer)?;
        Transform::from_pos_angle(r.pos, r.angle).map_err(serde::de::Error::custom)
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
impl<'a> TryFrom<&'a nalgebra::Isometry2<f32>> for Transform {
    type Error = Error;

    #[inline]
    fn try_from(i: &'a nalgebra::Isometry2<f32>) -> Result<Self> {
        let v = i.translation.vector;
        let angle = i.rotation.angle();
        Transform::from_pos_angle(Vec2 { x: v.x, y: v.y }, angle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_rotation_and_transform_validation_are_pure_rust() {
        let _callback_guard = crate::core::callback_state::CallbackGuard::enter();

        assert!(is_valid_float(0.0));
        assert!(!is_valid_float(f32::INFINITY));
        assert!(Rot::IDENTITY.is_valid());
        assert!(!Rot { c: 2.0, s: 0.0 }.is_valid());
        assert!(
            !Rot {
                c: f32::NAN,
                s: 0.0
            }
            .is_valid()
        );
        assert!(Transform::IDENTITY.is_valid());
        assert!(
            !Transform {
                p: Vec2::new(f32::NAN, 0.0),
                q: Rot::IDENTITY,
            }
            .is_valid()
        );
    }

    #[test]
    fn native_math_helpers_return_callback_reentry_errors() {
        let _callback_guard = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(version(), Err(Error::InCallback));
        assert_eq!(atan2(1.0, 1.0), Err(Error::InCallback));
        assert_eq!(hash_bytes(HASH_INIT, b"boxdd"), Err(Error::InCallback));
        assert!(matches!(
            rotation_between_unit_vectors(Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)),
            Err(Error::InCallback)
        ));
    }
}
