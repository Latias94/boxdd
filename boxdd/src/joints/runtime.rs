use super::*;

// Runtime joint control APIs (by joint type)
impl World {
    pub fn joint_type(&self, id: JointId) -> JointType {
        joint_read_checked_impl(self.core(), id, base::joint_type_impl)
    }

    pub fn try_joint_type(&self, id: JointId) -> ApiResult<JointType> {
        try_joint_read_checked_impl(self.core(), id, base::joint_type_impl)
    }

    pub fn joint_type_raw(&self, id: JointId) -> ffi::b2JointType {
        joint_read_checked_impl(self.core(), id, base::joint_type_raw_impl)
    }

    pub fn try_joint_type_raw(&self, id: JointId) -> ApiResult<ffi::b2JointType> {
        try_joint_read_checked_impl(self.core(), id, base::joint_type_raw_impl)
    }

    pub fn joint_body_a_id(&self, id: JointId) -> BodyId {
        joint_read_checked_impl(self.core(), id, base::joint_body_a_id_impl)
    }

    pub fn try_joint_body_a_id(&self, id: JointId) -> ApiResult<BodyId> {
        try_joint_read_checked_impl(self.core(), id, base::joint_body_a_id_impl)
    }

    pub fn joint_body_b_id(&self, id: JointId) -> BodyId {
        joint_read_checked_impl(self.core(), id, base::joint_body_b_id_impl)
    }

    pub fn try_joint_body_b_id(&self, id: JointId) -> ApiResult<BodyId> {
        try_joint_read_checked_impl(self.core(), id, base::joint_body_b_id_impl)
    }

    pub fn joint_world_id_raw(&self, id: JointId) -> ffi::b2WorldId {
        joint_read_checked_impl(self.core(), id, base::joint_world_id_raw_impl)
    }

    pub fn try_joint_world_id_raw(&self, id: JointId) -> ApiResult<ffi::b2WorldId> {
        try_joint_read_checked_impl(self.core(), id, base::joint_world_id_raw_impl)
    }

    pub fn joint_collide_connected(&self, id: JointId) -> bool {
        joint_read_checked_impl(self.core(), id, base::joint_collide_connected_impl)
    }

    pub fn try_joint_collide_connected(&self, id: JointId) -> ApiResult<bool> {
        try_joint_read_checked_impl(self.core(), id, base::joint_collide_connected_impl)
    }

    pub fn set_joint_collide_connected(&mut self, id: JointId, flag: bool) {
        assert_joint_valid(self.core(), id);
        base::joint_set_collide_connected_impl(id, flag)
    }

    pub fn try_set_joint_collide_connected(&mut self, id: JointId, flag: bool) -> ApiResult<()> {
        check_joint_valid(self.core(), id)?;
        base::joint_set_collide_connected_impl(id, flag);
        Ok(())
    }

    pub fn joint_constraint_tuning(&self, id: JointId) -> ConstraintTuning {
        joint_read_checked_impl(self.core(), id, base::joint_constraint_tuning_impl)
    }

    pub fn try_joint_constraint_tuning(&self, id: JointId) -> ApiResult<ConstraintTuning> {
        try_joint_read_checked_impl(self.core(), id, base::joint_constraint_tuning_impl)
    }

    pub fn set_joint_constraint_tuning(&mut self, id: JointId, tuning: ConstraintTuning) {
        assert_joint_valid(self.core(), id);
        base::joint_set_constraint_tuning_impl(id, tuning)
    }

    pub fn try_set_joint_constraint_tuning(
        &mut self,
        id: JointId,
        tuning: ConstraintTuning,
    ) -> ApiResult<()> {
        check_joint_valid(self.core(), id)?;
        base::joint_set_constraint_tuning_impl(id, tuning);
        Ok(())
    }

    pub fn joint_local_frame_a(&self, id: JointId) -> crate::Transform {
        joint_read_checked_impl(self.core(), id, base::joint_local_frame_a_impl)
    }

    pub fn try_joint_local_frame_a(&self, id: JointId) -> ApiResult<crate::Transform> {
        try_joint_read_checked_impl(self.core(), id, base::joint_local_frame_a_impl)
    }

    pub fn set_joint_local_frame_a(&mut self, id: JointId, frame: crate::Transform) {
        assert_joint_valid(self.core(), id);
        base::assert_joint_local_frame_valid(frame);
        base::joint_set_local_frame_a_impl(id, frame)
    }

    pub fn try_set_joint_local_frame_a(
        &mut self,
        id: JointId,
        frame: crate::Transform,
    ) -> ApiResult<()> {
        check_joint_valid(self.core(), id)?;
        base::check_joint_local_frame_valid(frame)?;
        base::joint_set_local_frame_a_impl(id, frame);
        Ok(())
    }

    pub fn joint_local_frame_b(&self, id: JointId) -> crate::Transform {
        joint_read_checked_impl(self.core(), id, base::joint_local_frame_b_impl)
    }

    pub fn try_joint_local_frame_b(&self, id: JointId) -> ApiResult<crate::Transform> {
        try_joint_read_checked_impl(self.core(), id, base::joint_local_frame_b_impl)
    }

    pub fn set_joint_local_frame_b(&mut self, id: JointId, frame: crate::Transform) {
        assert_joint_valid(self.core(), id);
        base::assert_joint_local_frame_valid(frame);
        base::joint_set_local_frame_b_impl(id, frame)
    }

    pub fn try_set_joint_local_frame_b(
        &mut self,
        id: JointId,
        frame: crate::Transform,
    ) -> ApiResult<()> {
        check_joint_valid(self.core(), id)?;
        base::check_joint_local_frame_valid(frame)?;
        base::joint_set_local_frame_b_impl(id, frame);
        Ok(())
    }

    pub fn joint_wake_bodies(&mut self, id: JointId) {
        assert_joint_valid(self.core(), id);
        base::joint_wake_bodies_impl(id)
    }

    pub fn try_joint_wake_bodies(&mut self, id: JointId) -> ApiResult<()> {
        check_joint_valid(self.core(), id)?;
        base::joint_wake_bodies_impl(id);
        Ok(())
    }

    pub fn joint_linear_separation(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_linear_separation_impl)
    }

    pub fn try_joint_linear_separation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_linear_separation_impl)
    }

    pub fn joint_angular_separation(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_angular_separation_impl)
    }

    pub fn try_joint_angular_separation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_angular_separation_impl)
    }

    pub fn joint_constraint_force(&self, id: JointId) -> Vec2 {
        joint_read_checked_impl(self.core(), id, base::joint_constraint_force_impl)
    }

    pub fn try_joint_constraint_force(&self, id: JointId) -> ApiResult<Vec2> {
        try_joint_read_checked_impl(self.core(), id, base::joint_constraint_force_impl)
    }

    pub fn joint_constraint_torque(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_constraint_torque_impl)
    }

    pub fn try_joint_constraint_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_constraint_torque_impl)
    }

    pub fn joint_force_threshold(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_force_threshold_impl)
    }

    pub fn try_joint_force_threshold(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_force_threshold_impl)
    }

    pub fn set_joint_force_threshold(&mut self, id: JointId, threshold: f32) {
        assert_joint_valid(self.core(), id);
        base::joint_set_force_threshold_impl(id, threshold)
    }

    pub fn try_set_joint_force_threshold(&mut self, id: JointId, threshold: f32) -> ApiResult<()> {
        check_joint_valid(self.core(), id)?;
        base::joint_set_force_threshold_impl(id, threshold);
        Ok(())
    }

    pub fn joint_torque_threshold(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_torque_threshold_impl)
    }

    pub fn try_joint_torque_threshold(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_torque_threshold_impl)
    }

    pub fn set_joint_torque_threshold(&mut self, id: JointId, threshold: f32) {
        assert_joint_valid(self.core(), id);
        base::joint_set_torque_threshold_impl(id, threshold)
    }

    pub fn try_set_joint_torque_threshold(&mut self, id: JointId, threshold: f32) -> ApiResult<()> {
        check_joint_valid(self.core(), id)?;
        base::joint_set_torque_threshold_impl(id, threshold);
        Ok(())
    }
}

impl WorldHandle {
    pub fn joint_type(&self, id: JointId) -> JointType {
        joint_read_checked_impl(self.core(), id, base::joint_type_impl)
    }

    pub fn try_joint_type(&self, id: JointId) -> ApiResult<JointType> {
        try_joint_read_checked_impl(self.core(), id, base::joint_type_impl)
    }

    pub fn joint_body_a_id(&self, id: JointId) -> BodyId {
        joint_read_checked_impl(self.core(), id, base::joint_body_a_id_impl)
    }

    pub fn try_joint_body_a_id(&self, id: JointId) -> ApiResult<BodyId> {
        try_joint_read_checked_impl(self.core(), id, base::joint_body_a_id_impl)
    }

    pub fn joint_body_b_id(&self, id: JointId) -> BodyId {
        joint_read_checked_impl(self.core(), id, base::joint_body_b_id_impl)
    }

    pub fn try_joint_body_b_id(&self, id: JointId) -> ApiResult<BodyId> {
        try_joint_read_checked_impl(self.core(), id, base::joint_body_b_id_impl)
    }

    pub fn joint_world_id_raw(&self, id: JointId) -> ffi::b2WorldId {
        joint_read_checked_impl(self.core(), id, base::joint_world_id_raw_impl)
    }

    pub fn try_joint_world_id_raw(&self, id: JointId) -> ApiResult<ffi::b2WorldId> {
        try_joint_read_checked_impl(self.core(), id, base::joint_world_id_raw_impl)
    }

    pub fn joint_collide_connected(&self, id: JointId) -> bool {
        joint_read_checked_impl(self.core(), id, base::joint_collide_connected_impl)
    }

    pub fn try_joint_collide_connected(&self, id: JointId) -> ApiResult<bool> {
        try_joint_read_checked_impl(self.core(), id, base::joint_collide_connected_impl)
    }

    pub fn joint_constraint_tuning(&self, id: JointId) -> ConstraintTuning {
        joint_read_checked_impl(self.core(), id, base::joint_constraint_tuning_impl)
    }

    pub fn try_joint_constraint_tuning(&self, id: JointId) -> ApiResult<ConstraintTuning> {
        try_joint_read_checked_impl(self.core(), id, base::joint_constraint_tuning_impl)
    }

    pub fn joint_local_frame_a(&self, id: JointId) -> crate::Transform {
        joint_read_checked_impl(self.core(), id, base::joint_local_frame_a_impl)
    }

    pub fn try_joint_local_frame_a(&self, id: JointId) -> ApiResult<crate::Transform> {
        try_joint_read_checked_impl(self.core(), id, base::joint_local_frame_a_impl)
    }

    pub fn joint_local_frame_b(&self, id: JointId) -> crate::Transform {
        joint_read_checked_impl(self.core(), id, base::joint_local_frame_b_impl)
    }

    pub fn try_joint_local_frame_b(&self, id: JointId) -> ApiResult<crate::Transform> {
        try_joint_read_checked_impl(self.core(), id, base::joint_local_frame_b_impl)
    }

    pub fn joint_linear_separation(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_linear_separation_impl)
    }

    pub fn try_joint_linear_separation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_linear_separation_impl)
    }

    pub fn joint_angular_separation(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_angular_separation_impl)
    }

    pub fn try_joint_angular_separation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_angular_separation_impl)
    }

    pub fn joint_constraint_force(&self, id: JointId) -> Vec2 {
        joint_read_checked_impl(self.core(), id, base::joint_constraint_force_impl)
    }

    pub fn try_joint_constraint_force(&self, id: JointId) -> ApiResult<Vec2> {
        try_joint_read_checked_impl(self.core(), id, base::joint_constraint_force_impl)
    }

    pub fn joint_constraint_torque(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_constraint_torque_impl)
    }

    pub fn try_joint_constraint_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_constraint_torque_impl)
    }

    pub fn joint_force_threshold(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_force_threshold_impl)
    }

    pub fn try_joint_force_threshold(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_force_threshold_impl)
    }

    pub fn joint_torque_threshold(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_torque_threshold_impl)
    }

    pub fn try_joint_torque_threshold(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_torque_threshold_impl)
    }
}

#[inline]
fn assert_joint_kind_in(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
) {
    assert_joint_valid(core, id);
    assert_joint_kind_matches(id, expected);
}

#[inline]
fn check_joint_kind_in(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
) -> ApiResult<()> {
    check_joint_valid(core, id)?;
    check_joint_kind_matches(id, expected)
}

#[inline]
fn assert_joint_kind_matches(id: JointId, expected: JointType) {
    let actual = base::joint_type_impl(id);
    assert!(
        actual == expected,
        "joint type mismatch: expected {:?}, got {:?}",
        expected,
        actual
    );
}

#[inline]
fn check_joint_kind_matches(id: JointId, expected: JointType) -> ApiResult<()> {
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

pub(super) trait TypedJointRuntimeHandle {
    fn typed_joint_id(&self) -> JointId;
    fn typed_joint_world_core(&self) -> &crate::core::world_core::WorldCore;
}

impl TypedJointRuntimeHandle for OwnedJoint {
    #[inline]
    fn typed_joint_id(&self) -> JointId {
        self.id()
    }

    #[inline]
    fn typed_joint_world_core(&self) -> &crate::core::world_core::WorldCore {
        self.runtime_world_core()
    }
}

impl TypedJointRuntimeHandle for Joint<'_> {
    #[inline]
    fn typed_joint_id(&self) -> JointId {
        self.id()
    }

    #[inline]
    fn typed_joint_world_core(&self) -> &crate::core::world_core::WorldCore {
        self.runtime_world_core()
    }
}

#[inline]
pub(super) fn joint_kind_get_checked_in_impl<T>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    f: impl FnOnce(JointId) -> T,
) -> T {
    assert_joint_kind_in(core, id, expected);
    f(id)
}

#[inline]
pub(super) fn try_joint_kind_get_checked_in_impl<T>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    f: impl FnOnce(JointId) -> T,
) -> ApiResult<T> {
    check_joint_kind_in(core, id, expected)?;
    Ok(f(id))
}

#[inline]
pub(super) fn joint_kind_set_checked_in_impl<T>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    value: T,
    f: impl FnOnce(JointId, T),
) {
    assert_joint_kind_in(core, id, expected);
    f(id, value)
}

#[inline]
pub(super) fn try_joint_kind_set_checked_in_impl<T>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    value: T,
    f: impl FnOnce(JointId, T),
) -> ApiResult<()> {
    check_joint_kind_in(core, id, expected)?;
    f(id, value);
    Ok(())
}

#[inline]
pub(super) fn joint_kind_set2_checked_in_impl<A, B>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    a: A,
    b: B,
    f: impl FnOnce(JointId, A, B),
) {
    assert_joint_kind_in(core, id, expected);
    f(id, a, b)
}

#[inline]
pub(super) fn joint_kind_set2_checked_validated_in_impl<A, B>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    a: A,
    b: B,
    validate: impl FnOnce(&A, &B),
    f: impl FnOnce(JointId, A, B),
) {
    assert_joint_kind_in(core, id, expected);
    validate(&a, &b);
    f(id, a, b)
}

#[inline]
pub(super) fn try_joint_kind_set2_checked_in_impl<A, B>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    a: A,
    b: B,
    f: impl FnOnce(JointId, A, B),
) -> ApiResult<()> {
    check_joint_kind_in(core, id, expected)?;
    f(id, a, b);
    Ok(())
}

#[inline]
pub(super) fn try_joint_kind_set2_checked_validated_in_impl<A, B>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    a: A,
    b: B,
    validate: impl FnOnce(&A, &B) -> ApiResult<()>,
    f: impl FnOnce(JointId, A, B),
) -> ApiResult<()> {
    check_joint_kind_in(core, id, expected)?;
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

#[cfg(test)]
mod tests {
    use crate::core::world_core::ActivityState;

    struct ActivityReset<'a> {
        core: &'a crate::core::world_core::WorldCore,
        active: ActivityState,
    }

    impl Drop for ActivityReset<'_> {
        fn drop(&mut self) {
            let _ = self.core.set_activity(self.active, ActivityState::Idle);
        }
    }

    fn distance_joint_def(world: &mut crate::World) -> crate::DistanceJointDef {
        let a = world.create_body_id(crate::BodyBuilder::new().build());
        let b = world.create_body_id(crate::BodyBuilder::new().build());
        crate::DistanceJointDef::new(crate::JointBase::new(a, b))
    }

    #[test]
    fn owned_typed_joint_checks_recording_before_kind_and_native_calls() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let def = distance_joint_def(&mut world);
        let core = world.core_rc();
        let mut joint = world.create_distance_joint_owned(&def);

        core.set_activity(ActivityState::Idle, ActivityState::Recording)
            .unwrap();
        let _reset = ActivityReset {
            core: &core,
            active: ActivityState::Recording,
        };

        assert_eq!(joint.try_distance_length(), Err(crate::ApiError::WorldBusy));
        assert_eq!(
            joint.try_distance_set_length(2.0),
            Err(crate::ApiError::WorldBusy)
        );
        assert_eq!(joint.try_revolute_angle(), Err(crate::ApiError::WorldBusy));

        let guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(
            joint.try_distance_length(),
            Err(crate::ApiError::InCallback)
        );
        drop(guard);
    }

    #[test]
    fn scoped_typed_joint_checks_restoring_before_kind_and_native_calls() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let def = distance_joint_def(&mut world);
        let core = world.core_rc();
        let mut joint = world.create_distance_joint(&def);

        core.set_activity(ActivityState::Idle, ActivityState::Restoring)
            .unwrap();
        let _reset = ActivityReset {
            core: &core,
            active: ActivityState::Restoring,
        };

        assert_eq!(joint.try_distance_length(), Err(crate::ApiError::WorldBusy));
        assert_eq!(
            joint.try_distance_set_length(2.0),
            Err(crate::ApiError::WorldBusy)
        );
        assert_eq!(joint.try_revolute_angle(), Err(crate::ApiError::WorldBusy));

        let guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(
            joint.try_distance_set_length(2.0),
            Err(crate::ApiError::InCallback)
        );
        drop(guard);
    }

    #[test]
    fn owned_and_scoped_typed_joints_reject_poisoned_worlds() {
        {
            let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
            let def = distance_joint_def(&mut world);
            let core = world.core_rc();
            let mut joint = world.create_distance_joint_owned(&def);

            core.poison();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                assert_eq!(
                    joint.try_distance_length(),
                    Err(crate::ApiError::WorldPoisoned)
                );
                assert_eq!(
                    joint.try_distance_set_length(2.0),
                    Err(crate::ApiError::WorldPoisoned)
                );
            }));
            drop(joint);
            result.unwrap();
        }

        {
            let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
            let def = distance_joint_def(&mut world);
            let core = world.core_rc();
            let mut joint = world.create_distance_joint(&def);

            core.poison();
            assert_eq!(
                joint.try_distance_length(),
                Err(crate::ApiError::WorldPoisoned)
            );
            assert_eq!(
                joint.try_distance_set_length(2.0),
                Err(crate::ApiError::WorldPoisoned)
            );
        }
    }
}
