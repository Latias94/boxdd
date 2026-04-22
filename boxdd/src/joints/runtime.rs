use super::*;

// Runtime joint control APIs (by joint type)
impl World {
    pub fn joint_type(&self, id: JointId) -> JointType {
        joint_read_checked_impl(id, base::joint_type_impl)
    }

    pub fn try_joint_type(&self, id: JointId) -> ApiResult<JointType> {
        try_joint_read_checked_impl(id, base::joint_type_impl)
    }

    pub fn joint_type_raw(&self, id: JointId) -> ffi::b2JointType {
        joint_read_checked_impl(id, base::joint_type_raw_impl)
    }

    pub fn try_joint_type_raw(&self, id: JointId) -> ApiResult<ffi::b2JointType> {
        try_joint_read_checked_impl(id, base::joint_type_raw_impl)
    }

    pub fn joint_body_a_id(&self, id: JointId) -> BodyId {
        joint_read_checked_impl(id, base::joint_body_a_id_impl)
    }

    pub fn try_joint_body_a_id(&self, id: JointId) -> ApiResult<BodyId> {
        try_joint_read_checked_impl(id, base::joint_body_a_id_impl)
    }

    pub fn joint_body_b_id(&self, id: JointId) -> BodyId {
        joint_read_checked_impl(id, base::joint_body_b_id_impl)
    }

    pub fn try_joint_body_b_id(&self, id: JointId) -> ApiResult<BodyId> {
        try_joint_read_checked_impl(id, base::joint_body_b_id_impl)
    }

    pub fn joint_collide_connected(&self, id: JointId) -> bool {
        joint_read_checked_impl(id, base::joint_collide_connected_impl)
    }

    pub fn try_joint_collide_connected(&self, id: JointId) -> ApiResult<bool> {
        try_joint_read_checked_impl(id, base::joint_collide_connected_impl)
    }

    pub fn set_joint_collide_connected(&mut self, id: JointId, flag: bool) {
        assert_joint_valid(id);
        base::joint_set_collide_connected_impl(id, flag)
    }

    pub fn try_set_joint_collide_connected(&mut self, id: JointId, flag: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        base::joint_set_collide_connected_impl(id, flag);
        Ok(())
    }

    pub fn joint_constraint_tuning(&self, id: JointId) -> ConstraintTuning {
        joint_read_checked_impl(id, base::joint_constraint_tuning_impl)
    }

    pub fn try_joint_constraint_tuning(&self, id: JointId) -> ApiResult<ConstraintTuning> {
        try_joint_read_checked_impl(id, base::joint_constraint_tuning_impl)
    }

    pub fn set_joint_constraint_tuning(&mut self, id: JointId, tuning: ConstraintTuning) {
        assert_joint_valid(id);
        base::joint_set_constraint_tuning_impl(id, tuning)
    }

    pub fn try_set_joint_constraint_tuning(
        &mut self,
        id: JointId,
        tuning: ConstraintTuning,
    ) -> ApiResult<()> {
        check_joint_valid(id)?;
        base::joint_set_constraint_tuning_impl(id, tuning);
        Ok(())
    }

    pub fn joint_local_frame_a(&self, id: JointId) -> crate::Transform {
        joint_read_checked_impl(id, base::joint_local_frame_a_impl)
    }

    pub fn try_joint_local_frame_a(&self, id: JointId) -> ApiResult<crate::Transform> {
        try_joint_read_checked_impl(id, base::joint_local_frame_a_impl)
    }

    pub fn joint_local_frame_b(&self, id: JointId) -> crate::Transform {
        joint_read_checked_impl(id, base::joint_local_frame_b_impl)
    }

    pub fn try_joint_local_frame_b(&self, id: JointId) -> ApiResult<crate::Transform> {
        try_joint_read_checked_impl(id, base::joint_local_frame_b_impl)
    }

    pub fn joint_wake_bodies(&mut self, id: JointId) {
        assert_joint_valid(id);
        base::joint_wake_bodies_impl(id)
    }

    pub fn try_joint_wake_bodies(&mut self, id: JointId) -> ApiResult<()> {
        check_joint_valid(id)?;
        base::joint_wake_bodies_impl(id);
        Ok(())
    }

    pub fn joint_linear_separation(&self, id: JointId) -> f32 {
        joint_read_checked_impl(id, base::joint_linear_separation_impl)
    }

    pub fn try_joint_linear_separation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(id, base::joint_linear_separation_impl)
    }

    pub fn joint_angular_separation(&self, id: JointId) -> f32 {
        joint_read_checked_impl(id, base::joint_angular_separation_impl)
    }

    pub fn try_joint_angular_separation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(id, base::joint_angular_separation_impl)
    }

    pub fn joint_constraint_force(&self, id: JointId) -> Vec2 {
        joint_read_checked_impl(id, base::joint_constraint_force_impl)
    }

    pub fn try_joint_constraint_force(&self, id: JointId) -> ApiResult<Vec2> {
        try_joint_read_checked_impl(id, base::joint_constraint_force_impl)
    }

    pub fn joint_constraint_torque(&self, id: JointId) -> f32 {
        joint_read_checked_impl(id, base::joint_constraint_torque_impl)
    }

    pub fn try_joint_constraint_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(id, base::joint_constraint_torque_impl)
    }

    pub fn joint_force_threshold(&self, id: JointId) -> f32 {
        joint_read_checked_impl(id, base::joint_force_threshold_impl)
    }

    pub fn try_joint_force_threshold(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(id, base::joint_force_threshold_impl)
    }

    pub fn set_joint_force_threshold(&mut self, id: JointId, threshold: f32) {
        assert_joint_valid(id);
        base::joint_set_force_threshold_impl(id, threshold)
    }

    pub fn try_set_joint_force_threshold(&mut self, id: JointId, threshold: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        base::joint_set_force_threshold_impl(id, threshold);
        Ok(())
    }

    pub fn joint_torque_threshold(&self, id: JointId) -> f32 {
        joint_read_checked_impl(id, base::joint_torque_threshold_impl)
    }

    pub fn try_joint_torque_threshold(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(id, base::joint_torque_threshold_impl)
    }

    pub fn set_joint_torque_threshold(&mut self, id: JointId, threshold: f32) {
        assert_joint_valid(id);
        base::joint_set_torque_threshold_impl(id, threshold)
    }

    pub fn try_set_joint_torque_threshold(&mut self, id: JointId, threshold: f32) -> ApiResult<()> {
        check_joint_valid(id)?;
        base::joint_set_torque_threshold_impl(id, threshold);
        Ok(())
    }
}

impl WorldHandle {
    pub fn joint_type(&self, id: JointId) -> JointType {
        joint_read_checked_impl(id, base::joint_type_impl)
    }

    pub fn try_joint_type(&self, id: JointId) -> ApiResult<JointType> {
        try_joint_read_checked_impl(id, base::joint_type_impl)
    }

    pub fn joint_body_a_id(&self, id: JointId) -> BodyId {
        joint_read_checked_impl(id, base::joint_body_a_id_impl)
    }

    pub fn try_joint_body_a_id(&self, id: JointId) -> ApiResult<BodyId> {
        try_joint_read_checked_impl(id, base::joint_body_a_id_impl)
    }

    pub fn joint_body_b_id(&self, id: JointId) -> BodyId {
        joint_read_checked_impl(id, base::joint_body_b_id_impl)
    }

    pub fn try_joint_body_b_id(&self, id: JointId) -> ApiResult<BodyId> {
        try_joint_read_checked_impl(id, base::joint_body_b_id_impl)
    }

    pub fn joint_collide_connected(&self, id: JointId) -> bool {
        joint_read_checked_impl(id, base::joint_collide_connected_impl)
    }

    pub fn try_joint_collide_connected(&self, id: JointId) -> ApiResult<bool> {
        try_joint_read_checked_impl(id, base::joint_collide_connected_impl)
    }

    pub fn joint_constraint_tuning(&self, id: JointId) -> ConstraintTuning {
        joint_read_checked_impl(id, base::joint_constraint_tuning_impl)
    }

    pub fn try_joint_constraint_tuning(&self, id: JointId) -> ApiResult<ConstraintTuning> {
        try_joint_read_checked_impl(id, base::joint_constraint_tuning_impl)
    }

    pub fn joint_local_frame_a(&self, id: JointId) -> crate::Transform {
        joint_read_checked_impl(id, base::joint_local_frame_a_impl)
    }

    pub fn try_joint_local_frame_a(&self, id: JointId) -> ApiResult<crate::Transform> {
        try_joint_read_checked_impl(id, base::joint_local_frame_a_impl)
    }

    pub fn joint_local_frame_b(&self, id: JointId) -> crate::Transform {
        joint_read_checked_impl(id, base::joint_local_frame_b_impl)
    }

    pub fn try_joint_local_frame_b(&self, id: JointId) -> ApiResult<crate::Transform> {
        try_joint_read_checked_impl(id, base::joint_local_frame_b_impl)
    }

    pub fn joint_linear_separation(&self, id: JointId) -> f32 {
        joint_read_checked_impl(id, base::joint_linear_separation_impl)
    }

    pub fn try_joint_linear_separation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(id, base::joint_linear_separation_impl)
    }

    pub fn joint_angular_separation(&self, id: JointId) -> f32 {
        joint_read_checked_impl(id, base::joint_angular_separation_impl)
    }

    pub fn try_joint_angular_separation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(id, base::joint_angular_separation_impl)
    }

    pub fn joint_constraint_force(&self, id: JointId) -> Vec2 {
        joint_read_checked_impl(id, base::joint_constraint_force_impl)
    }

    pub fn try_joint_constraint_force(&self, id: JointId) -> ApiResult<Vec2> {
        try_joint_read_checked_impl(id, base::joint_constraint_force_impl)
    }

    pub fn joint_constraint_torque(&self, id: JointId) -> f32 {
        joint_read_checked_impl(id, base::joint_constraint_torque_impl)
    }

    pub fn try_joint_constraint_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(id, base::joint_constraint_torque_impl)
    }

    pub fn joint_force_threshold(&self, id: JointId) -> f32 {
        joint_read_checked_impl(id, base::joint_force_threshold_impl)
    }

    pub fn try_joint_force_threshold(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(id, base::joint_force_threshold_impl)
    }

    pub fn joint_torque_threshold(&self, id: JointId) -> f32 {
        joint_read_checked_impl(id, base::joint_torque_threshold_impl)
    }

    pub fn try_joint_torque_threshold(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(id, base::joint_torque_threshold_impl)
    }
}

#[inline]
pub(super) fn assert_joint_kind(id: JointId, expected: JointType) {
    assert_joint_valid(id);
    let actual = base::joint_type_impl(id);
    assert!(
        actual == expected,
        "joint type mismatch: expected {:?}, got {:?}",
        expected,
        actual
    );
}

#[inline]
pub(super) fn check_joint_kind(id: JointId, expected: JointType) -> ApiResult<()> {
    check_joint_valid(id)?;
    if base::joint_type_impl(id) != expected {
        return Err(crate::error::ApiError::InvalidJointType);
    }
    Ok(())
}

const REVOLUTE_LIMIT_ABS_MAX: f32 = 0.99 * core::f32::consts::PI;

#[track_caller]
fn assert_ordered_joint_range(name: &str, lower: f32, upper: f32) {
    assert!(
        lower <= upper,
        "{name} requires lower <= upper, got lower={lower}, upper={upper}"
    );
}

fn check_ordered_joint_range(lower: f32, upper: f32) -> ApiResult<()> {
    if lower <= upper {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(super) fn assert_distance_spring_force_range_valid(lower: &f32, upper: &f32) {
    assert_ordered_joint_range("distance spring force range", *lower, *upper);
}

pub(super) fn check_distance_spring_force_range_valid(lower: &f32, upper: &f32) -> ApiResult<()> {
    check_ordered_joint_range(*lower, *upper)
}

pub(super) fn assert_prismatic_limits_valid(lower: &f32, upper: &f32) {
    assert_ordered_joint_range("prismatic limits", *lower, *upper);
}

pub(super) fn check_prismatic_limits_valid(lower: &f32, upper: &f32) -> ApiResult<()> {
    check_ordered_joint_range(*lower, *upper)
}

#[track_caller]
pub(super) fn assert_revolute_limits_valid(lower: &f32, upper: &f32) {
    assert_ordered_joint_range("revolute limits", *lower, *upper);
    assert!(
        *lower >= -REVOLUTE_LIMIT_ABS_MAX,
        "revolute lower limit must be >= {}, got {}",
        -REVOLUTE_LIMIT_ABS_MAX,
        *lower
    );
    assert!(
        *upper <= REVOLUTE_LIMIT_ABS_MAX,
        "revolute upper limit must be <= {}, got {}",
        REVOLUTE_LIMIT_ABS_MAX,
        *upper
    );
}

pub(super) fn check_revolute_limits_valid(lower: &f32, upper: &f32) -> ApiResult<()> {
    if *lower <= *upper && *lower >= -REVOLUTE_LIMIT_ABS_MAX && *upper <= REVOLUTE_LIMIT_ABS_MAX {
        Ok(())
    } else {
        Err(crate::error::ApiError::InvalidArgument)
    }
}

pub(super) fn assert_wheel_limits_valid(lower: &f32, upper: &f32) {
    assert_ordered_joint_range("wheel limits", *lower, *upper);
}

pub(super) fn check_wheel_limits_valid(lower: &f32, upper: &f32) -> ApiResult<()> {
    check_ordered_joint_range(*lower, *upper)
}

#[inline]
pub(super) fn joint_kind_get_checked_impl<T>(
    id: JointId,
    expected: JointType,
    f: impl FnOnce(JointId) -> T,
) -> T {
    assert_joint_kind(id, expected);
    f(id)
}

#[inline]
pub(super) fn try_joint_kind_get_checked_impl<T>(
    id: JointId,
    expected: JointType,
    f: impl FnOnce(JointId) -> T,
) -> ApiResult<T> {
    check_joint_kind(id, expected)?;
    Ok(f(id))
}

#[inline]
pub(super) fn joint_kind_set_checked_impl<T>(
    id: JointId,
    expected: JointType,
    value: T,
    f: impl FnOnce(JointId, T),
) {
    assert_joint_kind(id, expected);
    f(id, value)
}

#[inline]
pub(super) fn try_joint_kind_set_checked_impl<T>(
    id: JointId,
    expected: JointType,
    value: T,
    f: impl FnOnce(JointId, T),
) -> ApiResult<()> {
    check_joint_kind(id, expected)?;
    f(id, value);
    Ok(())
}

#[inline]
pub(super) fn joint_kind_set2_checked_impl<A, B>(
    id: JointId,
    expected: JointType,
    a: A,
    b: B,
    f: impl FnOnce(JointId, A, B),
) {
    assert_joint_kind(id, expected);
    f(id, a, b)
}

#[inline]
pub(super) fn joint_kind_set2_checked_validated_impl<A, B>(
    id: JointId,
    expected: JointType,
    a: A,
    b: B,
    validate: impl FnOnce(&A, &B),
    f: impl FnOnce(JointId, A, B),
) {
    assert_joint_kind(id, expected);
    validate(&a, &b);
    f(id, a, b)
}

#[inline]
pub(super) fn try_joint_kind_set2_checked_impl<A, B>(
    id: JointId,
    expected: JointType,
    a: A,
    b: B,
    f: impl FnOnce(JointId, A, B),
) -> ApiResult<()> {
    check_joint_kind(id, expected)?;
    f(id, a, b);
    Ok(())
}

#[inline]
pub(super) fn try_joint_kind_set2_checked_validated_impl<A, B>(
    id: JointId,
    expected: JointType,
    a: A,
    b: B,
    validate: impl FnOnce(&A, &B) -> ApiResult<()>,
    f: impl FnOnce(JointId, A, B),
) -> ApiResult<()> {
    check_joint_kind(id, expected)?;
    validate(&a, &b)?;
    f(id, a, b);
    Ok(())
}

type JointScalarReadFn<T> = unsafe extern "C" fn(ffi::b2JointId) -> T;
type JointScalarWriteFn<T> = unsafe extern "C" fn(ffi::b2JointId, T);
type JointVec2ReadFn = unsafe extern "C" fn(ffi::b2JointId) -> ffi::b2Vec2;

#[inline]
pub(super) fn joint_scalar_read_impl<T>(id: JointId, read: JointScalarReadFn<T>) -> T {
    unsafe { read(raw_joint_id(id)) }
}

#[inline]
pub(super) fn joint_scalar_write_impl<T>(id: JointId, value: T, write: JointScalarWriteFn<T>) {
    unsafe { write(raw_joint_id(id), value) }
}

#[inline]
pub(super) fn joint_vec2_read_impl(id: JointId, read: JointVec2ReadFn) -> Vec2 {
    Vec2::from_raw(unsafe { read(raw_joint_id(id)) })
}
