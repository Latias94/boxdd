use super::*;

#[inline]
fn assert_joint_def_bodies_valid(base: &ffi::b2JointDef) {
    crate::core::debug_checks::assert_body_valid(BodyId::from_raw(base.bodyIdA));
    crate::core::debug_checks::assert_body_valid(BodyId::from_raw(base.bodyIdB));
}

#[inline]
fn check_joint_def_bodies_valid(base: &ffi::b2JointDef) -> ApiResult<()> {
    crate::core::debug_checks::check_body_valid(BodyId::from_raw(base.bodyIdA))?;
    crate::core::debug_checks::check_body_valid(BodyId::from_raw(base.bodyIdB))?;
    Ok(())
}

#[inline]
fn assert_joint_def_body_pair_valid(base: &ffi::b2JointDef) {
    let body_a = BodyId::from_raw(base.bodyIdA);
    let body_b = BodyId::from_raw(base.bodyIdB);
    assert!(
        body_a.world0 == body_b.world0,
        "joint bodies must belong to the same world"
    );
    assert!(body_a != body_b, "joint bodies must be distinct");
}

#[inline]
fn check_joint_def_body_pair_valid(base: &ffi::b2JointDef) -> ApiResult<()> {
    let body_a = BodyId::from_raw(base.bodyIdA);
    let body_b = BodyId::from_raw(base.bodyIdB);
    if body_a.world0 == body_b.world0 && body_a != body_b {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
fn assert_joint_def_local_frames_valid(base: &ffi::b2JointDef) {
    assert!(
        crate::Transform::from_raw(base.localFrameA).is_valid(),
        "joint localFrameA must be a valid Box2D transform"
    );
    assert!(
        crate::Transform::from_raw(base.localFrameB).is_valid(),
        "joint localFrameB must be a valid Box2D transform"
    );
}

#[inline]
fn check_joint_def_local_frames_valid(base: &ffi::b2JointDef) -> ApiResult<()> {
    if crate::Transform::from_raw(base.localFrameA).is_valid()
        && crate::Transform::from_raw(base.localFrameB).is_valid()
    {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
fn assert_joint_def_event_thresholds_valid(base: &ffi::b2JointDef) {
    assert!(
        crate::is_valid_float(base.forceThreshold) && base.forceThreshold >= 0.0,
        "joint forceThreshold must be finite and >= 0.0, got {}",
        base.forceThreshold
    );
    assert!(
        crate::is_valid_float(base.torqueThreshold) && base.torqueThreshold >= 0.0,
        "joint torqueThreshold must be finite and >= 0.0, got {}",
        base.torqueThreshold
    );
}

#[inline]
fn check_joint_def_event_thresholds_valid(base: &ffi::b2JointDef) -> ApiResult<()> {
    if crate::is_valid_float(base.forceThreshold)
        && base.forceThreshold >= 0.0
        && crate::is_valid_float(base.torqueThreshold)
        && base.torqueThreshold >= 0.0
    {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
pub(super) fn assert_joint_def_targets_world(world: &World, base: &ffi::b2JointDef) {
    let target_world = world.raw().index1 - 1;
    let body_a = BodyId::from_raw(base.bodyIdA);
    let body_b = BodyId::from_raw(base.bodyIdB);
    assert!(
        body_a.world0 == target_world && body_b.world0 == target_world,
        "joint bodies must belong to the target world"
    );
}

#[inline]
pub(super) fn check_joint_def_targets_world(
    world: &World,
    base: &ffi::b2JointDef,
) -> ApiResult<()> {
    let target_world = world.raw().index1 - 1;
    let body_a = BodyId::from_raw(base.bodyIdA);
    let body_b = BodyId::from_raw(base.bodyIdB);
    if body_a.world0 == target_world && body_b.world0 == target_world {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

#[inline]
fn assert_joint_base_raw_valid(base: &ffi::b2JointDef) {
    assert_joint_def_bodies_valid(base);
    assert_joint_def_body_pair_valid(base);
    assert_joint_def_local_frames_valid(base);
    assert_joint_def_event_thresholds_valid(base);
}

#[inline]
fn check_joint_base_raw_valid(base: &ffi::b2JointDef) -> ApiResult<()> {
    check_joint_def_bodies_valid(base)?;
    check_joint_def_body_pair_valid(base)?;
    check_joint_def_local_frames_valid(base)?;
    check_joint_def_event_thresholds_valid(base)?;
    Ok(())
}

pub(crate) fn check_joint_base_valid(base: &JointBase) -> ApiResult<()> {
    check_joint_base_raw_valid(&base.0)
}

#[inline]
fn distance_joint_def_cookie_is_valid(def: &ffi::b2DistanceJointDef) -> bool {
    def.internalValue == unsafe { ffi::b2DefaultDistanceJointDef() }.internalValue
}

#[inline]
fn motor_joint_def_cookie_is_valid(def: &ffi::b2MotorJointDef) -> bool {
    def.internalValue == unsafe { ffi::b2DefaultMotorJointDef() }.internalValue
}

#[inline]
fn filter_joint_def_cookie_is_valid(def: &ffi::b2FilterJointDef) -> bool {
    def.internalValue == unsafe { ffi::b2DefaultFilterJointDef() }.internalValue
}

#[inline]
fn prismatic_joint_def_cookie_is_valid(def: &ffi::b2PrismaticJointDef) -> bool {
    def.internalValue == unsafe { ffi::b2DefaultPrismaticJointDef() }.internalValue
}

#[inline]
fn revolute_joint_def_cookie_is_valid(def: &ffi::b2RevoluteJointDef) -> bool {
    def.internalValue == unsafe { ffi::b2DefaultRevoluteJointDef() }.internalValue
}

#[inline]
fn weld_joint_def_cookie_is_valid(def: &ffi::b2WeldJointDef) -> bool {
    def.internalValue == unsafe { ffi::b2DefaultWeldJointDef() }.internalValue
}

#[inline]
fn wheel_joint_def_cookie_is_valid(def: &ffi::b2WheelJointDef) -> bool {
    def.internalValue == unsafe { ffi::b2DefaultWheelJointDef() }.internalValue
}

pub(super) fn assert_distance_joint_def_raw_valid(def: &ffi::b2DistanceJointDef) {
    assert_joint_base_raw_valid(&def.base);
    assert!(
        distance_joint_def_cookie_is_valid(def),
        "invalid DistanceJointDef: not initialized from b2DefaultDistanceJointDef"
    );
    assert!(
        crate::is_valid_float(def.length) && def.length > 0.0,
        "invalid DistanceJointDef: length must be finite and > 0.0, got {}",
        def.length
    );
    assert!(
        def.lowerSpringForce <= def.upperSpringForce,
        "invalid DistanceJointDef: lowerSpringForce must be <= upperSpringForce"
    );
}

pub(super) fn check_distance_joint_def_raw_valid(def: &ffi::b2DistanceJointDef) -> ApiResult<()> {
    check_joint_base_raw_valid(&def.base)?;
    if distance_joint_def_cookie_is_valid(def)
        && crate::is_valid_float(def.length)
        && def.length > 0.0
        && def.lowerSpringForce <= def.upperSpringForce
    {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(crate) fn check_distance_joint_def_valid(def: &DistanceJointDef) -> ApiResult<()> {
    check_distance_joint_def_raw_valid(&def.0)
}

pub(super) fn assert_motor_joint_def_raw_valid(def: &ffi::b2MotorJointDef) {
    assert_joint_base_raw_valid(&def.base);
    assert!(
        motor_joint_def_cookie_is_valid(def),
        "invalid MotorJointDef: not initialized from b2DefaultMotorJointDef"
    );
}

pub(super) fn check_motor_joint_def_raw_valid(def: &ffi::b2MotorJointDef) -> ApiResult<()> {
    check_joint_base_raw_valid(&def.base)?;
    if motor_joint_def_cookie_is_valid(def) {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(crate) fn check_motor_joint_def_valid(def: &MotorJointDef) -> ApiResult<()> {
    check_motor_joint_def_raw_valid(&def.0)
}

pub(super) fn assert_filter_joint_def_raw_valid(def: &ffi::b2FilterJointDef) {
    assert_joint_base_raw_valid(&def.base);
    assert!(
        filter_joint_def_cookie_is_valid(def),
        "invalid FilterJointDef: not initialized from b2DefaultFilterJointDef"
    );
}

pub(super) fn check_filter_joint_def_raw_valid(def: &ffi::b2FilterJointDef) -> ApiResult<()> {
    check_joint_base_raw_valid(&def.base)?;
    if filter_joint_def_cookie_is_valid(def) {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(crate) fn check_filter_joint_def_valid(def: &FilterJointDef) -> ApiResult<()> {
    check_filter_joint_def_raw_valid(&def.0)
}

pub(super) fn assert_prismatic_joint_def_raw_valid(def: &ffi::b2PrismaticJointDef) {
    assert_joint_base_raw_valid(&def.base);
    assert!(
        prismatic_joint_def_cookie_is_valid(def),
        "invalid PrismaticJointDef: not initialized from b2DefaultPrismaticJointDef"
    );
    assert!(
        def.lowerTranslation <= def.upperTranslation,
        "invalid PrismaticJointDef: lowerTranslation must be <= upperTranslation"
    );
}

pub(super) fn check_prismatic_joint_def_raw_valid(def: &ffi::b2PrismaticJointDef) -> ApiResult<()> {
    check_joint_base_raw_valid(&def.base)?;
    if prismatic_joint_def_cookie_is_valid(def) && def.lowerTranslation <= def.upperTranslation {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(crate) fn check_prismatic_joint_def_valid(def: &PrismaticJointDef) -> ApiResult<()> {
    check_prismatic_joint_def_raw_valid(&def.0)
}

pub(super) fn assert_revolute_joint_def_raw_valid(def: &ffi::b2RevoluteJointDef) {
    assert_joint_base_raw_valid(&def.base);
    assert!(
        revolute_joint_def_cookie_is_valid(def),
        "invalid RevoluteJointDef: not initialized from b2DefaultRevoluteJointDef"
    );
    assert!(
        def.lowerAngle <= def.upperAngle,
        "invalid RevoluteJointDef: lowerAngle must be <= upperAngle"
    );
    assert!(
        def.lowerAngle >= -0.99 * ffi::B2_PI as f32,
        "invalid RevoluteJointDef: lowerAngle must be >= -0.99 * PI"
    );
    assert!(
        def.upperAngle <= 0.99 * ffi::B2_PI as f32,
        "invalid RevoluteJointDef: upperAngle must be <= 0.99 * PI"
    );
}

pub(super) fn check_revolute_joint_def_raw_valid(def: &ffi::b2RevoluteJointDef) -> ApiResult<()> {
    check_joint_base_raw_valid(&def.base)?;
    if revolute_joint_def_cookie_is_valid(def)
        && def.lowerAngle <= def.upperAngle
        && def.lowerAngle >= -0.99 * ffi::B2_PI as f32
        && def.upperAngle <= 0.99 * ffi::B2_PI as f32
    {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(crate) fn check_revolute_joint_def_valid(def: &RevoluteJointDef) -> ApiResult<()> {
    check_revolute_joint_def_raw_valid(&def.0)
}

pub(super) fn assert_weld_joint_def_raw_valid(def: &ffi::b2WeldJointDef) {
    assert_joint_base_raw_valid(&def.base);
    assert!(
        weld_joint_def_cookie_is_valid(def),
        "invalid WeldJointDef: not initialized from b2DefaultWeldJointDef"
    );
}

pub(super) fn check_weld_joint_def_raw_valid(def: &ffi::b2WeldJointDef) -> ApiResult<()> {
    check_joint_base_raw_valid(&def.base)?;
    if weld_joint_def_cookie_is_valid(def) {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(crate) fn check_weld_joint_def_valid(def: &WeldJointDef) -> ApiResult<()> {
    check_weld_joint_def_raw_valid(&def.0)
}

pub(super) fn assert_wheel_joint_def_raw_valid(def: &ffi::b2WheelJointDef) {
    assert_joint_base_raw_valid(&def.base);
    assert!(
        wheel_joint_def_cookie_is_valid(def),
        "invalid WheelJointDef: not initialized from b2DefaultWheelJointDef"
    );
    assert!(
        def.lowerTranslation <= def.upperTranslation,
        "invalid WheelJointDef: lowerTranslation must be <= upperTranslation"
    );
}

pub(super) fn check_wheel_joint_def_raw_valid(def: &ffi::b2WheelJointDef) -> ApiResult<()> {
    check_joint_base_raw_valid(&def.base)?;
    if wheel_joint_def_cookie_is_valid(def) && def.lowerTranslation <= def.upperTranslation {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(crate) fn check_wheel_joint_def_valid(def: &WheelJointDef) -> ApiResult<()> {
    check_wheel_joint_def_raw_valid(&def.0)
}
