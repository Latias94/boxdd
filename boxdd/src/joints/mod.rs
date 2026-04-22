//! Joint builders and creation helpers (modularized).
//!
//! Two creation styles are available:
//! - Scoped handles: `World::create_*_joint(&def) -> Joint` returning a scoped handle for immediate
//!   configuration/queries. Dropping the handle does **not** destroy the joint.
//! - Owned handles: `World::create_*_joint_owned(&def) -> OwnedJoint` or `World::*().build_owned() -> OwnedJoint`
//!   returning a RAII handle that destroys the joint on drop.
//! - ID style: `World::create_*_joint_id(&def) -> b2JointId` returning the raw id for storage.
//!
//! The `World` convenience builders (`revolute`, `prismatic`, `wheel`, `distance`, `weld`,
//! `motor_joint`, `filter_joint`) help compose joints in world space and build local frames
//! from world anchors/axes.

mod base;
mod distance;
mod filter;
mod motor;
mod prismatic;
mod revolute;
mod runtime;
mod runtime_typed;
mod weld;
mod wheel;

pub use base::{ConstraintTuning, Joint, JointBase, JointBaseBuilder, JointType, OwnedJoint};
pub use distance::{DistanceJointBuilder, DistanceJointDef};
pub use filter::{FilterJointBuilder, FilterJointDef};
pub use motor::{MotorJointBuilder, MotorJointDef};
pub use prismatic::{PrismaticJointBuilder, PrismaticJointDef};
pub use revolute::{RevoluteJointBuilder, RevoluteJointDef};
pub use weld::{WeldJointBuilder, WeldJointDef};
pub use wheel::{WheelJointBuilder, WheelJointDef};

use crate::error::ApiResult;
use crate::types::{BodyId, JointId, Vec2};
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;
use runtime::*;

#[inline]
pub(crate) fn raw_body_id(id: BodyId) -> ffi::b2BodyId {
    id.into_raw()
}

#[inline]
fn raw_joint_id(id: JointId) -> ffi::b2JointId {
    id.into_raw()
}

#[inline]
fn assert_joint_valid(id: JointId) {
    crate::core::debug_checks::assert_joint_valid(id);
}

#[inline]
fn check_joint_valid(id: JointId) -> ApiResult<()> {
    crate::core::debug_checks::check_joint_valid(id)
}

#[inline]
fn joint_read_checked_impl<R>(id: JointId, f: impl FnOnce(JointId) -> R) -> R {
    assert_joint_valid(id);
    f(id)
}

#[inline]
fn try_joint_read_checked_impl<R>(id: JointId, f: impl FnOnce(JointId) -> R) -> ApiResult<R> {
    check_joint_valid(id)?;
    Ok(f(id))
}

#[inline]
pub(crate) fn joint_is_valid_impl(id: JointId) -> bool {
    base::joint_is_valid_impl(id)
}

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

type JointCreateFn<D> = unsafe extern "C" fn(ffi::b2WorldId, *const D) -> ffi::b2JointId;

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
fn assert_joint_def_targets_world(world: &World, base: &ffi::b2JointDef) {
    let target_world = world.raw().index1 - 1;
    let body_a = BodyId::from_raw(base.bodyIdA);
    let body_b = BodyId::from_raw(base.bodyIdB);
    assert!(
        body_a.world0 == target_world && body_b.world0 == target_world,
        "joint bodies must belong to the target world"
    );
}

#[inline]
fn check_joint_def_targets_world(world: &World, base: &ffi::b2JointDef) -> ApiResult<()> {
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

fn assert_distance_joint_def_raw_valid(def: &ffi::b2DistanceJointDef) {
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

fn check_distance_joint_def_raw_valid(def: &ffi::b2DistanceJointDef) -> ApiResult<()> {
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

fn assert_motor_joint_def_raw_valid(def: &ffi::b2MotorJointDef) {
    assert_joint_base_raw_valid(&def.base);
    assert!(
        motor_joint_def_cookie_is_valid(def),
        "invalid MotorJointDef: not initialized from b2DefaultMotorJointDef"
    );
}

fn check_motor_joint_def_raw_valid(def: &ffi::b2MotorJointDef) -> ApiResult<()> {
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

fn assert_filter_joint_def_raw_valid(def: &ffi::b2FilterJointDef) {
    assert_joint_base_raw_valid(&def.base);
    assert!(
        filter_joint_def_cookie_is_valid(def),
        "invalid FilterJointDef: not initialized from b2DefaultFilterJointDef"
    );
}

fn check_filter_joint_def_raw_valid(def: &ffi::b2FilterJointDef) -> ApiResult<()> {
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

fn assert_prismatic_joint_def_raw_valid(def: &ffi::b2PrismaticJointDef) {
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

fn check_prismatic_joint_def_raw_valid(def: &ffi::b2PrismaticJointDef) -> ApiResult<()> {
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

fn assert_revolute_joint_def_raw_valid(def: &ffi::b2RevoluteJointDef) {
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

fn check_revolute_joint_def_raw_valid(def: &ffi::b2RevoluteJointDef) -> ApiResult<()> {
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

fn assert_weld_joint_def_raw_valid(def: &ffi::b2WeldJointDef) {
    assert_joint_base_raw_valid(&def.base);
    assert!(
        weld_joint_def_cookie_is_valid(def),
        "invalid WeldJointDef: not initialized from b2DefaultWeldJointDef"
    );
}

fn check_weld_joint_def_raw_valid(def: &ffi::b2WeldJointDef) -> ApiResult<()> {
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

fn assert_wheel_joint_def_raw_valid(def: &ffi::b2WheelJointDef) {
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

fn check_wheel_joint_def_raw_valid(def: &ffi::b2WheelJointDef) -> ApiResult<()> {
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

fn create_joint_id_checked_impl<D>(
    world: &mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    assert_def_valid: impl FnOnce(&D),
) -> JointId {
    crate::core::callback_state::assert_not_in_callback();
    assert_joint_def_targets_world(world, base);
    assert_def_valid(raw_def);
    JointId::from_raw(unsafe { create(world.raw(), raw_def) })
}

fn try_create_joint_id_checked_impl<D>(
    world: &mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    check_def_valid: impl FnOnce(&D) -> ApiResult<()>,
) -> ApiResult<JointId> {
    crate::core::callback_state::check_not_in_callback()?;
    check_joint_def_targets_world(world, base)?;
    check_def_valid(raw_def)?;
    Ok(JointId::from_raw(unsafe { create(world.raw(), raw_def) }))
}

fn create_joint_scoped_checked_impl<'w, D>(
    world: &'w mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    assert_def_valid: impl FnOnce(&D),
) -> Joint<'w> {
    let id = create_joint_id_checked_impl(world, base, raw_def, create, assert_def_valid);
    Joint::new(world.core_arc(), id)
}

fn try_create_joint_scoped_checked_impl<'w, D>(
    world: &'w mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    check_def_valid: impl FnOnce(&D) -> ApiResult<()>,
) -> ApiResult<Joint<'w>> {
    let id = try_create_joint_id_checked_impl(world, base, raw_def, create, check_def_valid)?;
    Ok(Joint::new(world.core_arc(), id))
}

fn create_joint_owned_checked_impl<D>(
    world: &mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    assert_def_valid: impl FnOnce(&D),
) -> OwnedJoint {
    let id = create_joint_id_checked_impl(world, base, raw_def, create, assert_def_valid);
    OwnedJoint::new(world.core_arc(), id)
}

fn try_create_joint_owned_checked_impl<D>(
    world: &mut World,
    base: &ffi::b2JointDef,
    raw_def: &D,
    create: JointCreateFn<D>,
    check_def_valid: impl FnOnce(&D) -> ApiResult<()>,
) -> ApiResult<OwnedJoint> {
    let id = try_create_joint_id_checked_impl(world, base, raw_def, create, check_def_valid)?;
    Ok(OwnedJoint::new(world.core_arc(), id))
}

// Convenience builder entry points on World
impl World {
    pub fn revolute<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> RevoluteJointBuilder<'w> {
        RevoluteJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_world: None,
            def: RevoluteJointDef::new(JointBase::default()),
        }
    }
    pub fn prismatic<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> PrismaticJointBuilder<'w> {
        PrismaticJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: PrismaticJointDef::new(JointBase::default()),
        }
    }
    pub fn wheel<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WheelJointBuilder<'w> {
        WheelJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: WheelJointDef::new(JointBase::default()),
        }
    }
    pub fn distance<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> DistanceJointBuilder<'w> {
        DistanceJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            def: DistanceJointDef::new(JointBase::default()),
        }
    }
    pub fn weld<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WeldJointBuilder<'w> {
        WeldJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_world: None,
            def: WeldJointDef::new(JointBase::default()),
        }
    }
    pub fn motor_joint<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> MotorJointBuilder<'w> {
        MotorJointBuilder {
            world: self,
            body_a,
            body_b,
            def: MotorJointDef::new(JointBase::default()),
        }
    }
    pub fn filter_joint<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> FilterJointBuilder<'w> {
        FilterJointBuilder {
            world: self,
            body_a,
            body_b,
            def: FilterJointDef::new(JointBase::default()),
        }
    }
}

// Creation/destroy: scoped handles and ID style
impl World {
    pub fn create_distance_joint<'w>(&'w mut self, def: &DistanceJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            assert_distance_joint_def_raw_valid,
        )
    }

    pub fn create_distance_joint_id(&mut self, def: &DistanceJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            assert_distance_joint_def_raw_valid,
        )
    }

    pub fn create_distance_joint_owned(&mut self, def: &DistanceJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            assert_distance_joint_def_raw_valid,
        )
    }

    pub fn try_create_distance_joint<'w>(
        &'w mut self,
        def: &DistanceJointDef,
    ) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            check_distance_joint_def_raw_valid,
        )
    }

    pub fn try_create_distance_joint_id(&mut self, def: &DistanceJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            check_distance_joint_def_raw_valid,
        )
    }

    pub fn try_create_distance_joint_owned(
        &mut self,
        def: &DistanceJointDef,
    ) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateDistanceJoint,
            check_distance_joint_def_raw_valid,
        )
    }

    pub fn create_revolute_joint<'w>(&'w mut self, def: &RevoluteJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            assert_revolute_joint_def_raw_valid,
        )
    }

    pub fn create_revolute_joint_id(&mut self, def: &RevoluteJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            assert_revolute_joint_def_raw_valid,
        )
    }

    pub fn create_revolute_joint_owned(&mut self, def: &RevoluteJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            assert_revolute_joint_def_raw_valid,
        )
    }

    pub fn try_create_revolute_joint<'w>(
        &'w mut self,
        def: &RevoluteJointDef,
    ) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            check_revolute_joint_def_raw_valid,
        )
    }

    pub fn try_create_revolute_joint_id(&mut self, def: &RevoluteJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            check_revolute_joint_def_raw_valid,
        )
    }

    pub fn try_create_revolute_joint_owned(
        &mut self,
        def: &RevoluteJointDef,
    ) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateRevoluteJoint,
            check_revolute_joint_def_raw_valid,
        )
    }

    pub fn create_prismatic_joint<'w>(&'w mut self, def: &PrismaticJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            assert_prismatic_joint_def_raw_valid,
        )
    }

    pub fn create_prismatic_joint_id(&mut self, def: &PrismaticJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            assert_prismatic_joint_def_raw_valid,
        )
    }

    pub fn create_prismatic_joint_owned(&mut self, def: &PrismaticJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            assert_prismatic_joint_def_raw_valid,
        )
    }

    pub fn try_create_prismatic_joint<'w>(
        &'w mut self,
        def: &PrismaticJointDef,
    ) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            check_prismatic_joint_def_raw_valid,
        )
    }

    pub fn try_create_prismatic_joint_id(&mut self, def: &PrismaticJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            check_prismatic_joint_def_raw_valid,
        )
    }

    pub fn try_create_prismatic_joint_owned(
        &mut self,
        def: &PrismaticJointDef,
    ) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreatePrismaticJoint,
            check_prismatic_joint_def_raw_valid,
        )
    }

    pub fn create_wheel_joint<'w>(&'w mut self, def: &WheelJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            assert_wheel_joint_def_raw_valid,
        )
    }

    pub fn create_wheel_joint_id(&mut self, def: &WheelJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            assert_wheel_joint_def_raw_valid,
        )
    }

    pub fn create_wheel_joint_owned(&mut self, def: &WheelJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            assert_wheel_joint_def_raw_valid,
        )
    }

    pub fn try_create_wheel_joint<'w>(&'w mut self, def: &WheelJointDef) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            check_wheel_joint_def_raw_valid,
        )
    }

    pub fn try_create_wheel_joint_id(&mut self, def: &WheelJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            check_wheel_joint_def_raw_valid,
        )
    }

    pub fn try_create_wheel_joint_owned(&mut self, def: &WheelJointDef) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWheelJoint,
            check_wheel_joint_def_raw_valid,
        )
    }

    pub fn create_weld_joint<'w>(&'w mut self, def: &WeldJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            assert_weld_joint_def_raw_valid,
        )
    }

    pub fn create_weld_joint_id(&mut self, def: &WeldJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            assert_weld_joint_def_raw_valid,
        )
    }

    pub fn create_weld_joint_owned(&mut self, def: &WeldJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            assert_weld_joint_def_raw_valid,
        )
    }

    pub fn try_create_weld_joint<'w>(&'w mut self, def: &WeldJointDef) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            check_weld_joint_def_raw_valid,
        )
    }

    pub fn try_create_weld_joint_id(&mut self, def: &WeldJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            check_weld_joint_def_raw_valid,
        )
    }

    pub fn try_create_weld_joint_owned(&mut self, def: &WeldJointDef) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateWeldJoint,
            check_weld_joint_def_raw_valid,
        )
    }

    pub fn create_motor_joint<'w>(&'w mut self, def: &MotorJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            assert_motor_joint_def_raw_valid,
        )
    }

    pub fn create_motor_joint_id(&mut self, def: &MotorJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            assert_motor_joint_def_raw_valid,
        )
    }

    pub fn create_motor_joint_owned(&mut self, def: &MotorJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            assert_motor_joint_def_raw_valid,
        )
    }

    pub fn try_create_motor_joint<'w>(&'w mut self, def: &MotorJointDef) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            check_motor_joint_def_raw_valid,
        )
    }

    pub fn try_create_motor_joint_id(&mut self, def: &MotorJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            check_motor_joint_def_raw_valid,
        )
    }

    pub fn try_create_motor_joint_owned(&mut self, def: &MotorJointDef) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateMotorJoint,
            check_motor_joint_def_raw_valid,
        )
    }

    pub fn create_filter_joint<'w>(&'w mut self, def: &FilterJointDef) -> Joint<'w> {
        create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            assert_filter_joint_def_raw_valid,
        )
    }

    pub fn create_filter_joint_id(&mut self, def: &FilterJointDef) -> JointId {
        create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            assert_filter_joint_def_raw_valid,
        )
    }

    pub fn create_filter_joint_owned(&mut self, def: &FilterJointDef) -> OwnedJoint {
        create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            assert_filter_joint_def_raw_valid,
        )
    }

    pub fn try_create_filter_joint<'w>(&'w mut self, def: &FilterJointDef) -> ApiResult<Joint<'w>> {
        try_create_joint_scoped_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            check_filter_joint_def_raw_valid,
        )
    }

    pub fn try_create_filter_joint_id(&mut self, def: &FilterJointDef) -> ApiResult<JointId> {
        try_create_joint_id_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            check_filter_joint_def_raw_valid,
        )
    }

    pub fn try_create_filter_joint_owned(&mut self, def: &FilterJointDef) -> ApiResult<OwnedJoint> {
        try_create_joint_owned_checked_impl(
            self,
            &def.0.base,
            &def.0,
            ffi::b2CreateFilterJoint,
            check_filter_joint_def_raw_valid,
        )
    }

    pub fn destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Joint_IsValid(raw_joint_id(id)) } {
            unsafe { ffi::b2DestroyJoint(raw_joint_id(id), wake_bodies) };
            let _ = self.core_arc().clear_joint_user_data(id);
        }
    }

    pub fn try_destroy_joint_id(&mut self, id: JointId, wake_bodies: bool) -> ApiResult<()> {
        check_joint_valid(id)?;
        unsafe { ffi::b2DestroyJoint(raw_joint_id(id), wake_bodies) };
        let _ = self.core_arc().clear_joint_user_data(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn try_joint_apis_return_in_callback() {
        let mut world = crate::World::new(crate::WorldDef::default()).unwrap();
        let a = world.create_body_id(crate::BodyBuilder::new().build());
        let b = world.create_body_id(crate::BodyBuilder::new().build());

        let def = crate::DistanceJointDef::new(
            crate::JointBaseBuilder::new()
                .bodies_by_id(a, b)
                .collide_connected(false)
                .build(),
        );

        let _g = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(
            world.try_create_distance_joint_id(&def).unwrap_err(),
            crate::ApiError::InCallback
        );
        assert_eq!(
            world
                .revolute(a, b)
                .anchor_world([0.0, 0.0])
                .try_build()
                .unwrap_err(),
            crate::ApiError::InCallback
        );
    }
}
