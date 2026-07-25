use std::marker::PhantomData;

use super::*;

// Runtime joint control APIs (by joint type)
impl World {
    /// Return the joint's constraint type.
    ///
    /// # Panics
    ///
    /// Panics if the joint or world is unavailable or Box2D returns an unknown native
    /// discriminant. An unknown discriminant poisons the world before this method panics.
    pub fn joint_type(&self, id: JointId) -> JointType {
        self.try_joint_type(id)
            .expect("joint is unavailable or Box2D returned an unknown joint type")
    }

    /// Try to return the joint's constraint type.
    ///
    /// An unknown native discriminant returns
    /// [`ApiError::InvalidNativeJointType`](crate::ApiError::InvalidNativeJointType) and poisons
    /// the world.
    pub fn try_joint_type(&self, id: JointId) -> ApiResult<JointType> {
        check_joint_valid(self.core(), id)?;
        base::try_joint_type_impl(self.core(), id)
    }

    /// Returns Box2D's raw joint-type discriminant for the selected joint.
    pub fn joint_type_raw(&self, id: JointId) -> ffi::b2JointType {
        joint_read_checked_impl(self.core(), id, base::joint_type_raw_impl)
    }

    /// Fallible variant of [`Self::joint_type_raw`].
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
        joint_set_checked_in_impl(self.core(), id, flag, base::JOINT_SET_COLLIDE_CONNECTED)
    }

    pub fn try_set_joint_collide_connected(&mut self, id: JointId, flag: bool) -> ApiResult<()> {
        try_joint_set_checked_in_impl(self.core(), id, flag, base::JOINT_SET_COLLIDE_CONNECTED)
    }

    pub fn joint_constraint_tuning(&self, id: JointId) -> ConstraintTuning {
        joint_read_checked_impl(self.core(), id, base::joint_constraint_tuning_impl)
    }

    pub fn try_joint_constraint_tuning(&self, id: JointId) -> ApiResult<ConstraintTuning> {
        try_joint_read_checked_impl(self.core(), id, base::joint_constraint_tuning_impl)
    }

    pub fn set_joint_constraint_tuning(&mut self, id: JointId, tuning: ConstraintTuning) {
        joint_set_checked_in_impl(self.core(), id, tuning, base::JOINT_SET_CONSTRAINT_TUNING)
    }

    pub fn try_set_joint_constraint_tuning(
        &mut self,
        id: JointId,
        tuning: ConstraintTuning,
    ) -> ApiResult<()> {
        try_joint_set_checked_in_impl(self.core(), id, tuning, base::JOINT_SET_CONSTRAINT_TUNING)
    }

    pub fn joint_local_frame_a(&self, id: JointId) -> crate::Transform {
        joint_read_checked_impl(self.core(), id, base::joint_local_frame_a_impl)
    }

    pub fn try_joint_local_frame_a(&self, id: JointId) -> ApiResult<crate::Transform> {
        try_joint_read_checked_impl(self.core(), id, base::joint_local_frame_a_impl)
    }

    pub fn set_joint_local_frame_a(&mut self, id: JointId, frame: crate::Transform) {
        joint_set_checked_in_impl(self.core(), id, frame, base::JOINT_SET_LOCAL_FRAME_A)
    }

    pub fn try_set_joint_local_frame_a(
        &mut self,
        id: JointId,
        frame: crate::Transform,
    ) -> ApiResult<()> {
        try_joint_set_checked_in_impl(self.core(), id, frame, base::JOINT_SET_LOCAL_FRAME_A)
    }

    pub fn joint_local_frame_b(&self, id: JointId) -> crate::Transform {
        joint_read_checked_impl(self.core(), id, base::joint_local_frame_b_impl)
    }

    pub fn try_joint_local_frame_b(&self, id: JointId) -> ApiResult<crate::Transform> {
        try_joint_read_checked_impl(self.core(), id, base::joint_local_frame_b_impl)
    }

    pub fn set_joint_local_frame_b(&mut self, id: JointId, frame: crate::Transform) {
        joint_set_checked_in_impl(self.core(), id, frame, base::JOINT_SET_LOCAL_FRAME_B)
    }

    pub fn try_set_joint_local_frame_b(
        &mut self,
        id: JointId,
        frame: crate::Transform,
    ) -> ApiResult<()> {
        try_joint_set_checked_in_impl(self.core(), id, frame, base::JOINT_SET_LOCAL_FRAME_B)
    }

    pub fn joint_wake_bodies(&mut self, id: JointId) {
        joint_set_checked_in_impl(self.core(), id, (), base::JOINT_WAKE_BODIES)
    }

    pub fn try_joint_wake_bodies(&mut self, id: JointId) -> ApiResult<()> {
        try_joint_set_checked_in_impl(self.core(), id, (), base::JOINT_WAKE_BODIES)
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
        joint_set_checked_in_impl(self.core(), id, threshold, base::JOINT_SET_FORCE_THRESHOLD)
    }

    pub fn try_set_joint_force_threshold(&mut self, id: JointId, threshold: f32) -> ApiResult<()> {
        try_joint_set_checked_in_impl(self.core(), id, threshold, base::JOINT_SET_FORCE_THRESHOLD)
    }

    pub fn joint_torque_threshold(&self, id: JointId) -> f32 {
        joint_read_checked_impl(self.core(), id, base::joint_torque_threshold_impl)
    }

    pub fn try_joint_torque_threshold(&self, id: JointId) -> ApiResult<f32> {
        try_joint_read_checked_impl(self.core(), id, base::joint_torque_threshold_impl)
    }

    pub fn set_joint_torque_threshold(&mut self, id: JointId, threshold: f32) {
        joint_set_checked_in_impl(self.core(), id, threshold, base::JOINT_SET_TORQUE_THRESHOLD)
    }

    pub fn try_set_joint_torque_threshold(&mut self, id: JointId, threshold: f32) -> ApiResult<()> {
        try_joint_set_checked_in_impl(self.core(), id, threshold, base::JOINT_SET_TORQUE_THRESHOLD)
    }
}

impl WorldHandle {
    /// Return the joint's constraint type.
    ///
    /// # Panics
    ///
    /// Panics if the joint or world is unavailable or Box2D returns an unknown native
    /// discriminant. An unknown discriminant poisons the world before this method panics.
    pub fn joint_type(&self, id: JointId) -> JointType {
        self.try_joint_type(id)
            .expect("joint is unavailable or Box2D returned an unknown joint type")
    }

    /// Try to return the joint's constraint type.
    ///
    /// An unknown native discriminant returns
    /// [`ApiError::InvalidNativeJointType`](crate::ApiError::InvalidNativeJointType) and poisons
    /// the world.
    pub fn try_joint_type(&self, id: JointId) -> ApiResult<JointType> {
        check_joint_valid(self.core(), id)?;
        base::try_joint_type_impl(self.core(), id)
    }

    /// Returns Box2D's raw joint-type discriminant for the selected joint.
    pub fn joint_type_raw(&self, id: JointId) -> ffi::b2JointType {
        joint_read_checked_impl(self.core(), id, base::joint_type_raw_impl)
    }

    /// Fallible variant of [`Self::joint_type_raw`].
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

#[derive(Copy, Clone)]
pub(super) struct JointAccess(JointId);

impl JointAccess {
    #[inline]
    fn id(self) -> JointId {
        self.0
    }
}

#[inline]
pub(super) fn check_joint_access(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: Option<JointType>,
) -> ApiResult<JointAccess> {
    let access = check_joint_access_identity_with_access(
        core,
        id,
        expected,
        crate::core::world_core::WorldAccess::Idle,
    )?;
    core.check_joint_native_after_identity(id)?;
    Ok(access)
}

#[inline]
pub(super) fn check_joint_access_identity(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: Option<JointType>,
) -> ApiResult<JointAccess> {
    check_joint_access_identity_with_access(
        core,
        id,
        expected,
        crate::core::world_core::WorldAccess::Idle,
    )
}

#[inline]
pub(crate) fn check_joint_access_identity_with_access(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: Option<JointType>,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<JointAccess> {
    crate::core::callback_state::check_not_in_callback()?;
    let actual = core.check_joint_identity_with_access(id, access)?;

    if expected.is_some_and(|kind| actual != kind) {
        return Err(crate::error::ApiError::InvalidJointType);
    }

    Ok(JointAccess(id))
}

#[inline]
#[track_caller]
pub(super) fn assert_joint_access_identity(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: Option<JointType>,
) -> JointAccess {
    check_joint_access_identity(core, id, expected)
        .unwrap_or_else(|error| panic!("joint runtime access failed: {error}"))
}

#[inline]
#[track_caller]
pub(super) fn assert_joint_access(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: Option<JointType>,
) -> JointAccess {
    check_joint_access(core, id, expected)
        .unwrap_or_else(|error| panic!("joint runtime access failed: {error}"))
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
    f(assert_joint_access(core, id, Some(expected)).id())
}

#[inline]
pub(super) fn try_joint_kind_get_checked_in_impl<T>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    f: impl FnOnce(JointId) -> T,
) -> ApiResult<T> {
    Ok(f(check_joint_access(core, id, Some(expected))?.id()))
}

#[inline]
pub(super) fn joint_kind_set_checked_in_impl<T>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    value: T,
    operation: JointSetOp<T>,
) where
    T: Copy + IntoJointWriteValue,
{
    let id = assert_joint_access_identity(core, id, Some(expected)).id();
    let value = operation
        .validated(value)
        .expect("invalid joint runtime argument");
    core.check_joint_native_after_identity(id)
        .unwrap_or_else(|error| panic!("joint runtime access failed: {error}"));
    apply_joint_write(id, operation.kind, value)
        .expect("typed joint write operation must match its value")
}

#[inline]
pub(super) fn try_joint_kind_set_checked_in_impl<T>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    value: T,
    operation: JointSetOp<T>,
) -> ApiResult<()>
where
    T: Copy + IntoJointWriteValue,
{
    let id = check_joint_access_identity(core, id, Some(expected))?.id();
    let value = operation.validated(value)?;
    core.check_joint_native_after_identity(id)?;
    apply_joint_write(id, operation.kind, value)
}

#[inline]
pub(super) fn joint_kind_set2_checked_in_impl<A, B>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    a: A,
    b: B,
    operation: JointSet2Op<A, B>,
) where
    A: Copy + IntoJointWriteValue,
    B: Copy + IntoJointWriteValue,
{
    let id = assert_joint_access_identity(core, id, Some(expected)).id();
    let value = operation
        .validated(a, b)
        .expect("invalid joint runtime arguments");
    core.check_joint_native_after_identity(id)
        .unwrap_or_else(|error| panic!("joint runtime access failed: {error}"));
    apply_joint_write(id, operation.kind, value)
        .expect("typed joint write operation must match its values")
}

#[inline]
pub(super) fn try_joint_kind_set2_checked_in_impl<A, B>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: JointType,
    a: A,
    b: B,
    operation: JointSet2Op<A, B>,
) -> ApiResult<()>
where
    A: Copy + IntoJointWriteValue,
    B: Copy + IntoJointWriteValue,
{
    let id = check_joint_access_identity(core, id, Some(expected))?.id();
    let value = operation.validated(a, b)?;
    core.check_joint_native_after_identity(id)?;
    apply_joint_write(id, operation.kind, value)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum JointWriteKind {
    JointSetCollideConnected,
    JointSetConstraintTuning,
    JointSetLocalFrameA,
    JointSetLocalFrameB,
    JointWakeBodies,
    JointSetForceThreshold,
    JointSetTorqueThreshold,
    DistanceSetLength,
    DistanceEnableSpring,
    DistanceSetSpringForceRange,
    DistanceSetSpringHertz,
    DistanceSetSpringDampingRatio,
    DistanceEnableLimit,
    DistanceSetLengthRange,
    DistanceEnableMotor,
    DistanceSetMotorSpeed,
    DistanceSetMaxMotorForce,
    MotorSetLinearVelocity,
    MotorSetAngularVelocity,
    MotorSetMaxVelocityForce,
    MotorSetMaxVelocityTorque,
    MotorSetLinearHertz,
    MotorSetLinearDampingRatio,
    MotorSetAngularHertz,
    MotorSetAngularDampingRatio,
    MotorSetMaxSpringForce,
    MotorSetMaxSpringTorque,
    PrismaticEnableSpring,
    PrismaticSetSpringHertz,
    PrismaticSetSpringDampingRatio,
    PrismaticSetTargetTranslation,
    PrismaticEnableLimit,
    PrismaticSetLimits,
    PrismaticEnableMotor,
    PrismaticSetMotorSpeed,
    PrismaticSetMaxMotorForce,
    RevoluteEnableSpring,
    RevoluteSetSpringHertz,
    RevoluteSetSpringDampingRatio,
    RevoluteSetTargetAngle,
    RevoluteEnableLimit,
    RevoluteSetLimits,
    RevoluteEnableMotor,
    RevoluteSetMotorSpeed,
    RevoluteSetMaxMotorTorque,
    WeldSetLinearHertz,
    WeldSetLinearDampingRatio,
    WeldSetAngularHertz,
    WeldSetAngularDampingRatio,
    WheelEnableSpring,
    WheelSetSpringHertz,
    WheelSetSpringDampingRatio,
    WheelEnableLimit,
    WheelSetLimits,
    WheelEnableMotor,
    WheelSetMotorSpeed,
    WheelSetMaxMotorTorque,
}

#[derive(Copy, Clone)]
pub(super) struct JointSetOp<T> {
    kind: JointWriteKind,
    value: PhantomData<fn(T)>,
}

impl<T> JointSetOp<T> {
    pub(super) const fn new(kind: JointWriteKind) -> Self {
        Self {
            kind,
            value: PhantomData,
        }
    }

    fn validated(self, value: T) -> ApiResult<JointWriteValue>
    where
        T: IntoJointWriteValue,
    {
        let value = value.into_joint_write_value();
        validate_joint_write(self.kind, value)?;
        Ok(value)
    }
}

#[derive(Copy, Clone)]
pub(super) struct JointSet2Op<A, B> {
    kind: JointWriteKind,
    values: PhantomData<fn(A, B)>,
}

impl<A, B> JointSet2Op<A, B> {
    pub(super) const fn new(kind: JointWriteKind) -> Self {
        Self {
            kind,
            values: PhantomData,
        }
    }

    fn validated(self, a: A, b: B) -> ApiResult<JointWriteValue>
    where
        A: IntoJointWriteValue,
        B: IntoJointWriteValue,
    {
        let value = match (a.into_joint_write_value(), b.into_joint_write_value()) {
            (JointWriteValue::Scalar(a), JointWriteValue::Scalar(b)) => {
                JointWriteValue::ScalarPair(a, b)
            }
            _ => return Err(crate::ApiError::InvalidArgument),
        };
        validate_joint_write(self.kind, value)?;
        Ok(value)
    }
}

#[derive(Copy, Clone)]
pub(crate) enum JointWriteValue {
    Bool(bool),
    Scalar(f32),
    ScalarPair(f32, f32),
    Vector(Vec2),
    Transform(crate::Transform),
    Tuning(ConstraintTuning),
    Unit,
}

pub(super) trait IntoJointWriteValue {
    fn into_joint_write_value(self) -> JointWriteValue;
}

impl IntoJointWriteValue for bool {
    fn into_joint_write_value(self) -> JointWriteValue {
        JointWriteValue::Bool(self)
    }
}

impl IntoJointWriteValue for f32 {
    fn into_joint_write_value(self) -> JointWriteValue {
        JointWriteValue::Scalar(self)
    }
}

impl IntoJointWriteValue for Vec2 {
    fn into_joint_write_value(self) -> JointWriteValue {
        JointWriteValue::Vector(self)
    }
}

impl IntoJointWriteValue for crate::Transform {
    fn into_joint_write_value(self) -> JointWriteValue {
        JointWriteValue::Transform(self)
    }
}

impl IntoJointWriteValue for ConstraintTuning {
    fn into_joint_write_value(self) -> JointWriteValue {
        JointWriteValue::Tuning(self)
    }
}

impl IntoJointWriteValue for () {
    fn into_joint_write_value(self) -> JointWriteValue {
        JointWriteValue::Unit
    }
}

fn validate_joint_write(kind: JointWriteKind, value: JointWriteValue) -> ApiResult<()> {
    use JointWriteKind as K;
    use JointWriteValue as V;

    match (kind, value) {
        (
            K::JointSetCollideConnected
            | K::DistanceEnableSpring
            | K::DistanceEnableLimit
            | K::DistanceEnableMotor
            | K::PrismaticEnableSpring
            | K::PrismaticEnableLimit
            | K::PrismaticEnableMotor
            | K::RevoluteEnableSpring
            | K::RevoluteEnableLimit
            | K::RevoluteEnableMotor
            | K::WheelEnableSpring
            | K::WheelEnableLimit
            | K::WheelEnableMotor,
            V::Bool(_),
        )
        | (K::JointWakeBodies, V::Unit) => Ok(()),
        (K::JointSetConstraintTuning, V::Tuning(value)) => check_joint_tuning(value),
        (K::JointSetLocalFrameA | K::JointSetLocalFrameB, V::Transform(value)) => {
            check_joint_transform(value)
        }
        (
            K::JointSetForceThreshold
            | K::JointSetTorqueThreshold
            | K::DistanceSetSpringHertz
            | K::DistanceSetSpringDampingRatio
            | K::DistanceSetMaxMotorForce
            | K::MotorSetMaxVelocityForce
            | K::MotorSetMaxVelocityTorque
            | K::MotorSetLinearHertz
            | K::MotorSetLinearDampingRatio
            | K::MotorSetAngularHertz
            | K::MotorSetAngularDampingRatio
            | K::MotorSetMaxSpringForce
            | K::MotorSetMaxSpringTorque
            | K::PrismaticSetSpringHertz
            | K::PrismaticSetSpringDampingRatio
            | K::PrismaticSetMaxMotorForce
            | K::RevoluteSetSpringHertz
            | K::RevoluteSetSpringDampingRatio
            | K::RevoluteSetMaxMotorTorque
            | K::WeldSetLinearHertz
            | K::WeldSetLinearDampingRatio
            | K::WeldSetAngularHertz
            | K::WeldSetAngularDampingRatio
            | K::WheelSetSpringHertz
            | K::WheelSetSpringDampingRatio
            | K::WheelSetMaxMotorTorque,
            V::Scalar(value),
        ) => check_joint_non_negative(value),
        (K::DistanceSetLength, V::Scalar(value)) => check_joint_positive(value),
        (
            K::DistanceSetMotorSpeed
            | K::MotorSetAngularVelocity
            | K::PrismaticSetTargetTranslation
            | K::PrismaticSetMotorSpeed
            | K::RevoluteSetTargetAngle
            | K::RevoluteSetMotorSpeed
            | K::WheelSetMotorSpeed,
            V::Scalar(value),
        ) => check_joint_finite(value),
        (K::MotorSetLinearVelocity, V::Vector(value)) => check_joint_vec2(value),
        (K::DistanceSetSpringForceRange, V::ScalarPair(lower, upper))
        | (K::PrismaticSetLimits, V::ScalarPair(lower, upper))
        | (K::WheelSetLimits, V::ScalarPair(lower, upper)) => {
            check_joint_ordered_range(lower, upper)
        }
        (K::DistanceSetLengthRange, V::ScalarPair(lower, upper)) => {
            check_joint_non_negative_range(lower, upper)
        }
        (K::RevoluteSetLimits, V::ScalarPair(lower, upper)) => {
            check_revolute_joint_range(lower, upper)
        }
        _ => Err(crate::ApiError::InvalidArgument),
    }
}

fn apply_joint_write(id: JointId, kind: JointWriteKind, value: JointWriteValue) -> ApiResult<()> {
    use JointWriteKind as K;
    use JointWriteValue as V;

    match (kind, value) {
        (K::JointSetCollideConnected, V::Bool(value)) => unsafe {
            ffi::b2Joint_SetCollideConnected(raw_joint_id(id), value)
        },
        (K::JointSetConstraintTuning, V::Tuning(value)) => unsafe {
            ffi::b2Joint_SetConstraintTuning(raw_joint_id(id), value.hertz, value.damping_ratio)
        },
        (K::JointSetLocalFrameA, V::Transform(value)) => unsafe {
            ffi::b2Joint_SetLocalFrameA(raw_joint_id(id), value.into_raw())
        },
        (K::JointSetLocalFrameB, V::Transform(value)) => unsafe {
            ffi::b2Joint_SetLocalFrameB(raw_joint_id(id), value.into_raw())
        },
        (K::JointWakeBodies, V::Unit) => unsafe { ffi::b2Joint_WakeBodies(raw_joint_id(id)) },
        (K::JointSetForceThreshold, V::Scalar(value)) => unsafe {
            ffi::b2Joint_SetForceThreshold(raw_joint_id(id), value)
        },
        (K::JointSetTorqueThreshold, V::Scalar(value)) => unsafe {
            ffi::b2Joint_SetTorqueThreshold(raw_joint_id(id), value)
        },
        (K::DistanceSetLength, V::Scalar(value)) => unsafe {
            ffi::b2DistanceJoint_SetLength(raw_joint_id(id), value)
        },
        (K::DistanceEnableSpring, V::Bool(value)) => unsafe {
            ffi::b2DistanceJoint_EnableSpring(raw_joint_id(id), value)
        },
        (K::DistanceSetSpringForceRange, V::ScalarPair(lower, upper)) => unsafe {
            ffi::b2DistanceJoint_SetSpringForceRange(raw_joint_id(id), lower, upper)
        },
        (K::DistanceSetSpringHertz, V::Scalar(value)) => unsafe {
            ffi::b2DistanceJoint_SetSpringHertz(raw_joint_id(id), value)
        },
        (K::DistanceSetSpringDampingRatio, V::Scalar(value)) => unsafe {
            ffi::b2DistanceJoint_SetSpringDampingRatio(raw_joint_id(id), value)
        },
        (K::DistanceEnableLimit, V::Bool(value)) => unsafe {
            ffi::b2DistanceJoint_EnableLimit(raw_joint_id(id), value)
        },
        (K::DistanceSetLengthRange, V::ScalarPair(lower, upper)) => unsafe {
            ffi::b2DistanceJoint_SetLengthRange(raw_joint_id(id), lower, upper)
        },
        (K::DistanceEnableMotor, V::Bool(value)) => unsafe {
            ffi::b2DistanceJoint_EnableMotor(raw_joint_id(id), value)
        },
        (K::DistanceSetMotorSpeed, V::Scalar(value)) => unsafe {
            ffi::b2DistanceJoint_SetMotorSpeed(raw_joint_id(id), value)
        },
        (K::DistanceSetMaxMotorForce, V::Scalar(value)) => unsafe {
            ffi::b2DistanceJoint_SetMaxMotorForce(raw_joint_id(id), value)
        },
        (K::MotorSetLinearVelocity, V::Vector(value)) => unsafe {
            ffi::b2MotorJoint_SetLinearVelocity(raw_joint_id(id), value.into_raw())
        },
        (K::MotorSetAngularVelocity, V::Scalar(value)) => unsafe {
            ffi::b2MotorJoint_SetAngularVelocity(raw_joint_id(id), value)
        },
        (K::MotorSetMaxVelocityForce, V::Scalar(value)) => unsafe {
            ffi::b2MotorJoint_SetMaxVelocityForce(raw_joint_id(id), value)
        },
        (K::MotorSetMaxVelocityTorque, V::Scalar(value)) => unsafe {
            ffi::b2MotorJoint_SetMaxVelocityTorque(raw_joint_id(id), value)
        },
        (K::MotorSetLinearHertz, V::Scalar(value)) => unsafe {
            ffi::b2MotorJoint_SetLinearHertz(raw_joint_id(id), value)
        },
        (K::MotorSetLinearDampingRatio, V::Scalar(value)) => unsafe {
            ffi::b2MotorJoint_SetLinearDampingRatio(raw_joint_id(id), value)
        },
        (K::MotorSetAngularHertz, V::Scalar(value)) => unsafe {
            ffi::b2MotorJoint_SetAngularHertz(raw_joint_id(id), value)
        },
        (K::MotorSetAngularDampingRatio, V::Scalar(value)) => unsafe {
            ffi::b2MotorJoint_SetAngularDampingRatio(raw_joint_id(id), value)
        },
        (K::MotorSetMaxSpringForce, V::Scalar(value)) => unsafe {
            ffi::b2MotorJoint_SetMaxSpringForce(raw_joint_id(id), value)
        },
        (K::MotorSetMaxSpringTorque, V::Scalar(value)) => unsafe {
            ffi::b2MotorJoint_SetMaxSpringTorque(raw_joint_id(id), value)
        },
        (K::PrismaticEnableSpring, V::Bool(value)) => unsafe {
            ffi::b2PrismaticJoint_EnableSpring(raw_joint_id(id), value)
        },
        (K::PrismaticSetSpringHertz, V::Scalar(value)) => unsafe {
            ffi::b2PrismaticJoint_SetSpringHertz(raw_joint_id(id), value)
        },
        (K::PrismaticSetSpringDampingRatio, V::Scalar(value)) => unsafe {
            ffi::b2PrismaticJoint_SetSpringDampingRatio(raw_joint_id(id), value)
        },
        (K::PrismaticSetTargetTranslation, V::Scalar(value)) => unsafe {
            ffi::b2PrismaticJoint_SetTargetTranslation(raw_joint_id(id), value)
        },
        (K::PrismaticEnableLimit, V::Bool(value)) => unsafe {
            ffi::b2PrismaticJoint_EnableLimit(raw_joint_id(id), value)
        },
        (K::PrismaticSetLimits, V::ScalarPair(lower, upper)) => unsafe {
            ffi::b2PrismaticJoint_SetLimits(raw_joint_id(id), lower, upper)
        },
        (K::PrismaticEnableMotor, V::Bool(value)) => unsafe {
            ffi::b2PrismaticJoint_EnableMotor(raw_joint_id(id), value)
        },
        (K::PrismaticSetMotorSpeed, V::Scalar(value)) => unsafe {
            ffi::b2PrismaticJoint_SetMotorSpeed(raw_joint_id(id), value)
        },
        (K::PrismaticSetMaxMotorForce, V::Scalar(value)) => unsafe {
            ffi::b2PrismaticJoint_SetMaxMotorForce(raw_joint_id(id), value)
        },
        (K::RevoluteEnableSpring, V::Bool(value)) => unsafe {
            ffi::b2RevoluteJoint_EnableSpring(raw_joint_id(id), value)
        },
        (K::RevoluteSetSpringHertz, V::Scalar(value)) => unsafe {
            ffi::b2RevoluteJoint_SetSpringHertz(raw_joint_id(id), value)
        },
        (K::RevoluteSetSpringDampingRatio, V::Scalar(value)) => unsafe {
            ffi::b2RevoluteJoint_SetSpringDampingRatio(raw_joint_id(id), value)
        },
        (K::RevoluteSetTargetAngle, V::Scalar(value)) => unsafe {
            ffi::b2RevoluteJoint_SetTargetAngle(raw_joint_id(id), value)
        },
        (K::RevoluteEnableLimit, V::Bool(value)) => unsafe {
            ffi::b2RevoluteJoint_EnableLimit(raw_joint_id(id), value)
        },
        (K::RevoluteSetLimits, V::ScalarPair(lower, upper)) => unsafe {
            ffi::b2RevoluteJoint_SetLimits(raw_joint_id(id), lower, upper)
        },
        (K::RevoluteEnableMotor, V::Bool(value)) => unsafe {
            ffi::b2RevoluteJoint_EnableMotor(raw_joint_id(id), value)
        },
        (K::RevoluteSetMotorSpeed, V::Scalar(value)) => unsafe {
            ffi::b2RevoluteJoint_SetMotorSpeed(raw_joint_id(id), value)
        },
        (K::RevoluteSetMaxMotorTorque, V::Scalar(value)) => unsafe {
            ffi::b2RevoluteJoint_SetMaxMotorTorque(raw_joint_id(id), value)
        },
        (K::WeldSetLinearHertz, V::Scalar(value)) => unsafe {
            ffi::b2WeldJoint_SetLinearHertz(raw_joint_id(id), value)
        },
        (K::WeldSetLinearDampingRatio, V::Scalar(value)) => unsafe {
            ffi::b2WeldJoint_SetLinearDampingRatio(raw_joint_id(id), value)
        },
        (K::WeldSetAngularHertz, V::Scalar(value)) => unsafe {
            ffi::b2WeldJoint_SetAngularHertz(raw_joint_id(id), value)
        },
        (K::WeldSetAngularDampingRatio, V::Scalar(value)) => unsafe {
            ffi::b2WeldJoint_SetAngularDampingRatio(raw_joint_id(id), value)
        },
        (K::WheelEnableSpring, V::Bool(value)) => unsafe {
            ffi::b2WheelJoint_EnableSpring(raw_joint_id(id), value)
        },
        (K::WheelSetSpringHertz, V::Scalar(value)) => unsafe {
            ffi::b2WheelJoint_SetSpringHertz(raw_joint_id(id), value)
        },
        (K::WheelSetSpringDampingRatio, V::Scalar(value)) => unsafe {
            ffi::b2WheelJoint_SetSpringDampingRatio(raw_joint_id(id), value)
        },
        (K::WheelEnableLimit, V::Bool(value)) => unsafe {
            ffi::b2WheelJoint_EnableLimit(raw_joint_id(id), value)
        },
        (K::WheelSetLimits, V::ScalarPair(lower, upper)) => unsafe {
            ffi::b2WheelJoint_SetLimits(raw_joint_id(id), lower, upper)
        },
        (K::WheelEnableMotor, V::Bool(value)) => unsafe {
            ffi::b2WheelJoint_EnableMotor(raw_joint_id(id), value)
        },
        (K::WheelSetMotorSpeed, V::Scalar(value)) => unsafe {
            ffi::b2WheelJoint_SetMotorSpeed(raw_joint_id(id), value)
        },
        (K::WheelSetMaxMotorTorque, V::Scalar(value)) => unsafe {
            ffi::b2WheelJoint_SetMaxMotorTorque(raw_joint_id(id), value)
        },
        _ => return Err(crate::ApiError::InvalidArgument),
    }
    Ok(())
}

/// Apply one validated joint recording mutation through the requested world activity gate.
pub(crate) fn try_joint_write_with_access(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    expected: Option<JointType>,
    kind: JointWriteKind,
    value: JointWriteValue,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<()> {
    let id = check_joint_access_identity_with_access(core, id, expected, access)?.id();
    validate_joint_write(kind, value)?;
    core.check_joint_native_after_identity(id)?;
    apply_joint_write(id, kind, value)
}

#[inline]
pub(super) fn joint_set_checked_in_impl<T>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    value: T,
    operation: JointSetOp<T>,
) where
    T: Copy + IntoJointWriteValue,
{
    let id = assert_joint_access_identity(core, id, None).id();
    let value = operation
        .validated(value)
        .expect("invalid joint runtime argument");
    core.check_joint_native_after_identity(id)
        .unwrap_or_else(|error| panic!("joint runtime access failed: {error}"));
    apply_joint_write(id, operation.kind, value)
        .expect("typed joint write operation must match its value")
}

#[inline]
pub(super) fn try_joint_set_checked_in_impl<T>(
    core: &crate::core::world_core::WorldCore,
    id: JointId,
    value: T,
    operation: JointSetOp<T>,
) -> ApiResult<()>
where
    T: Copy + IntoJointWriteValue,
{
    let id = check_joint_access_identity(core, id, None)?.id();
    let value = operation.validated(value)?;
    core.check_joint_native_after_identity(id)?;
    apply_joint_write(id, operation.kind, value)
}

type JointScalarReadFn<T> = unsafe extern "C" fn(ffi::b2JointId) -> T;
type JointVec2ReadFn = unsafe extern "C" fn(ffi::b2JointId) -> ffi::b2Vec2;

#[inline]
pub(super) fn joint_scalar_read_impl<T>(id: JointId, read: JointScalarReadFn<T>) -> T {
    unsafe { read(raw_joint_id(id)) }
}

#[inline]
pub(super) fn joint_vec2_read_impl(id: JointId, read: JointVec2ReadFn) -> Vec2 {
    Vec2::from_raw(unsafe { read(raw_joint_id(id)) })
}

#[cfg(test)]
mod tests {
    use crate::core::world_core::ActivityState;

    struct DropWorldOnInto(Option<crate::World>);

    impl From<DropWorldOnInto> for crate::Vec2 {
        fn from(mut value: DropWorldOnInto) -> Self {
            drop(value.0.take());
            Self::ZERO
        }
    }

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
        let handle = world.handle();

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
        assert_eq!(
            world.try_distance_length(joint.id()),
            Err(crate::ApiError::InCallback)
        );
        assert_eq!(
            handle.try_distance_length(joint.id()),
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
            let handle = world.handle();

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
                assert_eq!(
                    world.try_distance_length(joint.id()),
                    Err(crate::ApiError::WorldPoisoned)
                );
                assert_eq!(
                    handle.try_distance_length(joint.id()),
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

    #[test]
    fn invalid_joint_mutations_do_not_reach_native_object_checks() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let def = distance_joint_def(&mut world);
        let core = world.core_rc();
        let mut joint = world.create_distance_joint_owned(&def);
        let before = core.native_object_check_count_for_test();

        assert_eq!(
            joint.try_distance_set_length(f32::NAN),
            Err(crate::ApiError::InvalidArgument)
        );
        assert_eq!(
            joint.try_set_constraint_tuning(crate::ConstraintTuning::new(-1.0, 0.0)),
            Err(crate::ApiError::InvalidArgument)
        );
        assert_eq!(
            core.native_object_check_count_for_test(),
            before,
            "pure argument validation must run before every native object check"
        );
    }

    #[test]
    fn joint_identity_and_kind_errors_precede_arguments_and_native_checks() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let def = distance_joint_def(&mut world);
        let id = world.create_distance_joint_id(&def);
        let core = world.core_rc();
        let before = core.native_object_check_count_for_test();

        assert_eq!(
            world.try_revolute_set_target_angle(id, f32::NAN),
            Err(crate::ApiError::InvalidJointType)
        );
        assert_eq!(core.native_object_check_count_for_test(), before);

        let mut foreign = crate::World::new(crate::WorldDef::default()).unwrap();
        let foreign_core = foreign.core_rc();
        let foreign_before = foreign_core.native_object_check_count_for_test();
        assert_eq!(
            foreign.try_distance_set_length(id, f32::NAN),
            Err(crate::ApiError::WrongWorld)
        );
        assert_eq!(
            foreign_core.native_object_check_count_for_test(),
            foreign_before
        );

        world.try_destroy_joint_id(id, true).unwrap();
        let after_destroy = core.native_object_check_count_for_test();
        assert_eq!(
            world.try_distance_set_length(id, f32::NAN),
            Err(crate::ApiError::InvalidJointId)
        );
        assert_eq!(core.native_object_check_count_for_test(), after_destroy);
    }

    #[test]
    fn destroyed_world_precedes_invalid_joint_arguments_without_native_checks() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let def = distance_joint_def(&mut world);
        let core = world.core_rc();
        let mut joint = world.create_distance_joint_owned(&def);
        drop(world);
        let before = core.native_object_check_count_for_test();

        assert_eq!(
            joint.try_distance_set_length(f32::NAN),
            Err(crate::ApiError::WorldDestroyed)
        );
        assert_eq!(core.native_object_check_count_for_test(), before);
    }

    #[test]
    fn joint_conversion_rechecks_world_lifecycle_before_native_calls() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let def = distance_joint_def(&mut world);
        let core = world.core_rc();
        let mut joint = world.create_motor_joint_owned(&crate::MotorJointDef::new(*def.base()));
        let before = core.native_object_check_count_for_test();

        assert_eq!(
            joint.try_motor_set_linear_velocity(DropWorldOnInto(Some(world))),
            Err(crate::ApiError::WorldDestroyed)
        );
        assert_eq!(core.native_object_check_count_for_test(), before);
    }
}
