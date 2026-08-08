use boxdd_sys::ffi;

use crate::error::Result;
use crate::types::{BodyId, ContactData, JointId, ShapeId};
use crate::world::BodyCall;

use super::super::scoped::Body;
use super::{attachments::body_contact_data_in_impl, check_native_body_count, raw_body_id};

#[inline]
pub(crate) fn body_shape_count_impl(id: BodyId) -> i32 {
    unsafe { ffi::b2Body_GetShapeCount(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_shapes_in_impl(body: BodyCall<'_>) -> Result<Vec<ShapeId>> {
    let cap = check_native_body_count(
        "Body::shapes",
        "shape_count",
        body_shape_count_impl(body.id()),
    )?;
    let id = raw_body_id(body.id());
    body.with_output_identity_resolver(|resolver| unsafe {
        crate::core::ffi_vec::try_read_mapped_from_ffi(
            cap,
            |ptr, cap| ffi::b2Body_GetShapes(id, ptr, cap),
            |raw| resolver.shape(raw),
        )
    })
}

#[inline]
pub(crate) fn body_joint_count_impl(id: BodyId) -> i32 {
    unsafe { ffi::b2Body_GetJointCount(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_joints_in_impl(body: BodyCall<'_>) -> Result<Vec<JointId>> {
    let cap = check_native_body_count(
        "Body::joints",
        "joint_count",
        body_joint_count_impl(body.id()),
    )?;
    let id = raw_body_id(body.id());
    body.with_output_identity_resolver(|resolver| unsafe {
        crate::core::ffi_vec::try_read_mapped_from_ffi(
            cap,
            |ptr, cap| ffi::b2Body_GetJoints(id, ptr, cap),
            |raw| resolver.joint(raw),
        )
    })
}

impl Body<'_> {
    pub fn contact_data(&self) -> Result<Vec<ContactData>> {
        self.body_access().call(body_contact_data_in_impl)
    }

    pub fn shape_count(&self) -> Result<i32> {
        self.body_access().call(|_| {
            check_native_body_count(
                "Body::shape_count",
                "shape_count",
                body_shape_count_impl(self.body_id()),
            )
        })
    }

    pub fn shapes(&self) -> Result<Vec<ShapeId>> {
        self.body_access().call(body_shapes_in_impl)
    }

    pub fn joint_count(&self) -> Result<i32> {
        self.body_access().call(|_| {
            check_native_body_count(
                "Body::joint_count",
                "joint_count",
                body_joint_count_impl(self.body_id()),
            )
        })
    }

    pub fn joints(&self) -> Result<Vec<JointId>> {
        self.body_access().call(body_joints_in_impl)
    }
}
