use crate::error::{Error, Result};
use crate::types::BodyId;
use crate::world::BodyProof;
use boxdd_sys::ffi;

use super::scoped::Body;

mod attachments;
mod dynamics;
mod mass;
mod metadata;
mod relations;
mod state;
mod transform;
mod user_data;

pub(crate) use transform::body_position_impl;

#[inline]
pub(crate) fn raw_body_id(id: BodyId) -> ffi::b2BodyId {
    id.into_raw()
}

#[inline]
pub(crate) fn check_native_body_finite(
    operation: &'static str,
    output: &'static str,
    value: f32,
) -> Result<f32> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite value",
        })
    }
}

#[inline]
pub(crate) fn check_native_body_non_negative(
    operation: &'static str,
    output: &'static str,
    value: f32,
) -> Result<f32> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite non-negative value",
        })
    }
}

#[inline]
pub(crate) fn check_native_body_count(
    operation: &'static str,
    output: &'static str,
    value: i32,
) -> Result<i32> {
    if value >= 0 {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a non-negative native int",
        })
    }
}

impl Body<'_> {
    #[inline]
    fn body_id(&self) -> BodyId {
        self.proof.id()
    }

    #[inline]
    fn body_access(&self) -> &BodyProof<'_> {
        &self.proof
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_body_scalar_and_count_checks_fail_closed() {
        assert_eq!(
            check_native_body_finite("Body::angular_velocity", "angular_velocity", f32::NAN),
            Err(Error::InvalidNativeOutput {
                operation: "Body::angular_velocity",
                output: "angular_velocity",
                constraint: "a finite value",
            })
        );
        assert_eq!(
            check_native_body_non_negative("Body::mass", "mass", -1.0),
            Err(Error::InvalidNativeOutput {
                operation: "Body::mass",
                output: "mass",
                constraint: "a finite non-negative value",
            })
        );
        assert_eq!(
            check_native_body_count("Body::shape_count", "shape_count", -1),
            Err(Error::InvalidNativeOutput {
                operation: "Body::shape_count",
                output: "shape_count",
                constraint: "a non-negative native int",
            })
        );

        assert_eq!(
            check_native_body_finite("Body::gravity_scale", "gravity_scale", -2.0),
            Ok(-2.0)
        );
        assert_eq!(
            check_native_body_non_negative("Body::mass", "mass", f32::INFINITY),
            Err(Error::InvalidNativeOutput {
                operation: "Body::mass",
                output: "mass",
                constraint: "a finite non-negative value",
            })
        );
    }
}
