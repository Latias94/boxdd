use boxdd_sys::ffi;

#[inline]
#[track_caller]
pub(crate) fn assert_body_valid(id: crate::types::BodyId) {
    crate::core::callback_state::assert_not_in_callback();
    assert!(
        unsafe { ffi::b2Body_IsValid(id.into_raw()) },
        "invalid BodyId"
    );
}

#[inline]
pub(crate) fn check_body_valid(id: crate::types::BodyId) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    if unsafe { ffi::b2Body_IsValid(id.into_raw()) } {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidBodyId)
    }
}

#[inline]
#[track_caller]
pub(crate) fn assert_shape_valid(id: crate::types::ShapeId) {
    crate::core::callback_state::assert_not_in_callback();
    assert!(
        unsafe { ffi::b2Shape_IsValid(id.into_raw()) },
        "invalid ShapeId"
    );
}

#[inline]
pub(crate) fn check_shape_valid(id: crate::types::ShapeId) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    if unsafe { ffi::b2Shape_IsValid(id.into_raw()) } {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidShapeId)
    }
}

#[inline]
#[track_caller]
pub(crate) fn assert_joint_valid(id: crate::types::JointId) {
    crate::core::callback_state::assert_not_in_callback();
    assert!(
        unsafe { ffi::b2Joint_IsValid(id.into_raw()) },
        "invalid JointId"
    );
}

#[inline]
pub(crate) fn check_joint_valid(id: crate::types::JointId) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    if unsafe { ffi::b2Joint_IsValid(id.into_raw()) } {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidJointId)
    }
}

#[inline]
#[track_caller]
pub(crate) fn assert_chain_valid(id: crate::types::ChainId) {
    crate::core::callback_state::assert_not_in_callback();
    assert!(
        unsafe { ffi::b2Chain_IsValid(id.into_raw()) },
        "invalid ChainId"
    );
}

#[inline]
pub(crate) fn check_chain_valid(id: crate::types::ChainId) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    if unsafe { ffi::b2Chain_IsValid(id.into_raw()) } {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidChainId)
    }
}

#[inline]
#[track_caller]
pub(crate) fn assert_contact_valid(id: crate::types::ContactId) {
    crate::core::callback_state::assert_not_in_callback();
    assert!(
        unsafe { ffi::b2Contact_IsValid(id.into_raw()) },
        "invalid ContactId"
    );
}

#[inline]
pub(crate) fn check_contact_valid(id: crate::types::ContactId) -> crate::error::ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    if unsafe { ffi::b2Contact_IsValid(id.into_raw()) } {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidContactId)
    }
}
