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

#[inline]
fn distance_length_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetLength)
}

#[inline]
fn distance_set_length_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_SetLength)
}

#[inline]
fn distance_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_IsSpringEnabled)
}

#[inline]
fn distance_enable_spring_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_EnableSpring)
}

#[inline]
fn distance_spring_force_range_impl(id: JointId) -> (f32, f32) {
    let mut lower_force = 0.0f32;
    let mut upper_force = 0.0f32;
    unsafe {
        ffi::b2DistanceJoint_GetSpringForceRange(
            raw_joint_id(id),
            &mut lower_force,
            &mut upper_force,
        )
    };
    (lower_force, upper_force)
}
#[inline]
fn distance_lower_spring_force_impl(id: JointId) -> f32 {
    distance_spring_force_range_impl(id).0
}
#[inline]
fn distance_upper_spring_force_impl(id: JointId) -> f32 {
    distance_spring_force_range_impl(id).1
}
#[inline]
fn distance_set_spring_force_range_impl(id: JointId, lower_force: f32, upper_force: f32) {
    unsafe { ffi::b2DistanceJoint_SetSpringForceRange(raw_joint_id(id), lower_force, upper_force) }
}

#[inline]
fn distance_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetSpringHertz)
}

#[inline]
fn distance_set_spring_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_SetSpringHertz)
}

#[inline]
fn distance_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetSpringDampingRatio)
}

#[inline]
fn distance_set_spring_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_SetSpringDampingRatio)
}

#[inline]
fn distance_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_IsLimitEnabled)
}

#[inline]
fn distance_enable_limit_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_EnableLimit)
}

#[inline]
fn distance_min_length_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMinLength)
}

#[inline]
fn distance_max_length_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMaxLength)
}

#[inline]
fn distance_current_length_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetCurrentLength)
}

#[inline]
fn distance_set_length_range_impl(id: JointId, min_length: f32, max_length: f32) {
    unsafe { ffi::b2DistanceJoint_SetLengthRange(raw_joint_id(id), min_length, max_length) }
}

#[inline]
fn distance_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_IsMotorEnabled)
}

#[inline]
fn distance_enable_motor_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_EnableMotor)
}

#[inline]
fn distance_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMotorSpeed)
}

#[inline]
fn distance_set_motor_speed_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_SetMotorSpeed)
}

#[inline]
fn distance_max_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMaxMotorForce)
}

#[inline]
fn distance_set_max_motor_force_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2DistanceJoint_SetMaxMotorForce)
}

#[inline]
fn distance_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2DistanceJoint_GetMotorForce)
}

#[inline]
fn prismatic_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_IsSpringEnabled)
}

#[inline]
fn prismatic_enable_spring_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_EnableSpring)
}

#[inline]
fn prismatic_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetSpringHertz)
}

#[inline]
fn prismatic_set_spring_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_SetSpringHertz)
}

#[inline]
fn prismatic_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetSpringDampingRatio)
}

#[inline]
fn prismatic_set_spring_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_SetSpringDampingRatio)
}

#[inline]
fn prismatic_target_translation_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetTargetTranslation)
}

#[inline]
fn prismatic_set_target_translation_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_SetTargetTranslation)
}

#[inline]
fn prismatic_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_IsLimitEnabled)
}

#[inline]
fn prismatic_enable_limit_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_EnableLimit)
}

#[inline]
fn prismatic_lower_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetLowerLimit)
}

#[inline]
fn prismatic_upper_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetUpperLimit)
}

#[inline]
fn prismatic_set_limits_impl(id: JointId, lower: f32, upper: f32) {
    unsafe { ffi::b2PrismaticJoint_SetLimits(raw_joint_id(id), lower, upper) }
}

#[inline]
fn prismatic_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_IsMotorEnabled)
}

#[inline]
fn prismatic_enable_motor_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_EnableMotor)
}

#[inline]
fn prismatic_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetMotorSpeed)
}

#[inline]
fn prismatic_set_motor_speed_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_SetMotorSpeed)
}

#[inline]
fn prismatic_max_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetMaxMotorForce)
}

#[inline]
fn prismatic_set_max_motor_force_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2PrismaticJoint_SetMaxMotorForce)
}

#[inline]
fn prismatic_motor_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetMotorForce)
}

#[inline]
fn prismatic_translation_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetTranslation)
}

#[inline]
fn prismatic_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2PrismaticJoint_GetSpeed)
}

#[inline]
fn revolute_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_IsSpringEnabled)
}

#[inline]
fn revolute_enable_spring_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_EnableSpring)
}

#[inline]
fn revolute_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetSpringHertz)
}

#[inline]
fn revolute_set_spring_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_SetSpringHertz)
}

#[inline]
fn revolute_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetSpringDampingRatio)
}

#[inline]
fn revolute_set_spring_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_SetSpringDampingRatio)
}

#[inline]
fn revolute_target_angle_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetTargetAngle)
}

#[inline]
fn revolute_set_target_angle_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_SetTargetAngle)
}

#[inline]
fn revolute_angle_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetAngle)
}

#[inline]
fn revolute_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_IsLimitEnabled)
}

#[inline]
fn revolute_enable_limit_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_EnableLimit)
}

#[inline]
fn revolute_lower_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetLowerLimit)
}

#[inline]
fn revolute_upper_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetUpperLimit)
}

#[inline]
fn revolute_set_limits_impl(id: JointId, lower: f32, upper: f32) {
    unsafe { ffi::b2RevoluteJoint_SetLimits(raw_joint_id(id), lower, upper) }
}

#[inline]
fn revolute_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_IsMotorEnabled)
}

#[inline]
fn revolute_enable_motor_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_EnableMotor)
}

#[inline]
fn revolute_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetMotorSpeed)
}

#[inline]
fn revolute_set_motor_speed_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_SetMotorSpeed)
}

#[inline]
fn revolute_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetMotorTorque)
}

#[inline]
fn revolute_max_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2RevoluteJoint_GetMaxMotorTorque)
}

#[inline]
fn revolute_set_max_motor_torque_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2RevoluteJoint_SetMaxMotorTorque)
}

#[inline]
fn weld_linear_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetLinearHertz)
}

#[inline]
fn weld_set_linear_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetLinearHertz)
}

#[inline]
fn weld_linear_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetLinearDampingRatio)
}

#[inline]
fn weld_set_linear_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetLinearDampingRatio)
}

#[inline]
fn weld_angular_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetAngularHertz)
}

#[inline]
fn weld_set_angular_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetAngularHertz)
}

#[inline]
fn weld_angular_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WeldJoint_GetAngularDampingRatio)
}

#[inline]
fn weld_set_angular_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WeldJoint_SetAngularDampingRatio)
}

#[inline]
fn wheel_spring_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_IsSpringEnabled)
}

#[inline]
fn wheel_enable_spring_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_EnableSpring)
}

#[inline]
fn wheel_spring_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetSpringHertz)
}

#[inline]
fn wheel_set_spring_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_SetSpringHertz)
}

#[inline]
fn wheel_spring_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetSpringDampingRatio)
}

#[inline]
fn wheel_set_spring_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_SetSpringDampingRatio)
}

#[inline]
fn wheel_limit_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_IsLimitEnabled)
}

#[inline]
fn wheel_enable_limit_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_EnableLimit)
}

#[inline]
fn wheel_lower_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetLowerLimit)
}

#[inline]
fn wheel_upper_limit_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetUpperLimit)
}

#[inline]
fn wheel_set_limits_impl(id: JointId, lower: f32, upper: f32) {
    unsafe { ffi::b2WheelJoint_SetLimits(raw_joint_id(id), lower, upper) }
}

#[inline]
fn wheel_motor_enabled_impl(id: JointId) -> bool {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_IsMotorEnabled)
}

#[inline]
fn wheel_enable_motor_impl(id: JointId, value: bool) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_EnableMotor)
}

#[inline]
fn wheel_motor_speed_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetMotorSpeed)
}

#[inline]
fn wheel_set_motor_speed_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_SetMotorSpeed)
}

#[inline]
fn wheel_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetMotorTorque)
}

#[inline]
fn wheel_max_motor_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2WheelJoint_GetMaxMotorTorque)
}

#[inline]
fn wheel_set_max_motor_torque_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2WheelJoint_SetMaxMotorTorque)
}

#[inline]
fn motor_linear_velocity_impl(id: JointId) -> Vec2 {
    joint_vec2_read_impl(id, ffi::b2MotorJoint_GetLinearVelocity)
}

#[inline]
fn motor_set_linear_velocity_impl(id: JointId, value: Vec2) {
    let raw: ffi::b2Vec2 = value.into_raw();
    unsafe { ffi::b2MotorJoint_SetLinearVelocity(raw_joint_id(id), raw) }
}

#[inline]
fn motor_angular_velocity_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetAngularVelocity)
}

#[inline]
fn motor_set_angular_velocity_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetAngularVelocity)
}

#[inline]
fn motor_max_velocity_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxVelocityForce)
}

#[inline]
fn motor_set_max_velocity_force_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetMaxVelocityForce)
}

#[inline]
fn motor_max_velocity_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxVelocityTorque)
}

#[inline]
fn motor_set_max_velocity_torque_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetMaxVelocityTorque)
}

#[inline]
fn motor_linear_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetLinearHertz)
}

#[inline]
fn motor_set_linear_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetLinearHertz)
}

#[inline]
fn motor_linear_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetLinearDampingRatio)
}

#[inline]
fn motor_set_linear_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetLinearDampingRatio)
}

#[inline]
fn motor_angular_hertz_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetAngularHertz)
}

#[inline]
fn motor_set_angular_hertz_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetAngularHertz)
}

#[inline]
fn motor_angular_damping_ratio_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetAngularDampingRatio)
}

#[inline]
fn motor_set_angular_damping_ratio_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetAngularDampingRatio)
}

#[inline]
fn motor_max_spring_force_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxSpringForce)
}

#[inline]
fn motor_set_max_spring_force_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetMaxSpringForce)
}

#[inline]
fn motor_max_spring_torque_impl(id: JointId) -> f32 {
    joint_scalar_read_impl(id, ffi::b2MotorJoint_GetMaxSpringTorque)
}

#[inline]
fn motor_set_max_spring_torque_impl(id: JointId, value: f32) {
    joint_scalar_write_impl(id, value, ffi::b2MotorJoint_SetMaxSpringTorque)
}

impl World {
    pub fn distance_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_length_impl)
    }

    pub fn try_distance_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_length_impl)
    }

    pub fn distance_set_length(&mut self, id: JointId, length: f32) {
        joint_kind_set_checked_impl(id, JointType::Distance, length, distance_set_length_impl)
    }

    pub fn try_distance_set_length(&mut self, id: JointId, length: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Distance, length, distance_set_length_impl)
    }

    pub fn distance_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_enabled_impl)
    }

    pub fn try_distance_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_enabled_impl)
    }

    pub fn distance_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Distance, enable, distance_enable_spring_impl)
    }

    pub fn try_distance_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            enable,
            distance_enable_spring_impl,
        )
    }

    pub fn distance_lower_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_lower_spring_force_impl)
    }

    pub fn try_distance_lower_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_lower_spring_force_impl)
    }

    pub fn distance_upper_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_upper_spring_force_impl)
    }

    pub fn try_distance_upper_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_upper_spring_force_impl)
    }

    pub fn distance_set_spring_force_range(
        &mut self,
        id: JointId,
        lower_force: f32,
        upper_force: f32,
    ) {
        joint_kind_set2_checked_validated_impl(
            id,
            JointType::Distance,
            lower_force,
            upper_force,
            assert_distance_spring_force_range_valid,
            distance_set_spring_force_range_impl,
        )
    }

    pub fn try_distance_set_spring_force_range(
        &mut self,
        id: JointId,
        lower_force: f32,
        upper_force: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            id,
            JointType::Distance,
            lower_force,
            upper_force,
            check_distance_spring_force_range_valid,
            distance_set_spring_force_range_impl,
        )
    }

    pub fn distance_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_hertz_impl)
    }

    pub fn try_distance_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_hertz_impl)
    }

    pub fn distance_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            hertz,
            distance_set_spring_hertz_impl,
        )
    }

    pub fn try_distance_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            hertz,
            distance_set_spring_hertz_impl,
        )
    }

    pub fn distance_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_damping_ratio_impl)
    }

    pub fn try_distance_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_damping_ratio_impl)
    }

    pub fn distance_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            damping_ratio,
            distance_set_spring_damping_ratio_impl,
        )
    }

    pub fn try_distance_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            damping_ratio,
            distance_set_spring_damping_ratio_impl,
        )
    }

    pub fn distance_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_limit_enabled_impl)
    }

    pub fn try_distance_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_limit_enabled_impl)
    }

    pub fn distance_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Distance, enable, distance_enable_limit_impl)
    }

    pub fn try_distance_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Distance, enable, distance_enable_limit_impl)
    }

    pub fn distance_min_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_min_length_impl)
    }

    pub fn try_distance_min_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_min_length_impl)
    }

    pub fn distance_max_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_max_length_impl)
    }

    pub fn try_distance_max_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_max_length_impl)
    }

    pub fn distance_current_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_current_length_impl)
    }

    pub fn try_distance_current_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_current_length_impl)
    }

    pub fn distance_set_length_range(&mut self, id: JointId, min_length: f32, max_length: f32) {
        joint_kind_set2_checked_impl(
            id,
            JointType::Distance,
            min_length,
            max_length,
            distance_set_length_range_impl,
        )
    }

    pub fn try_distance_set_length_range(
        &mut self,
        id: JointId,
        min_length: f32,
        max_length: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_impl(
            id,
            JointType::Distance,
            min_length,
            max_length,
            distance_set_length_range_impl,
        )
    }

    pub fn distance_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_enabled_impl)
    }

    pub fn try_distance_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_enabled_impl)
    }

    pub fn distance_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Distance, enable, distance_enable_motor_impl)
    }

    pub fn try_distance_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Distance, enable, distance_enable_motor_impl)
    }

    pub fn distance_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_speed_impl)
    }

    pub fn try_distance_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_speed_impl)
    }

    pub fn distance_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            speed,
            distance_set_motor_speed_impl,
        )
    }

    pub fn try_distance_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            speed,
            distance_set_motor_speed_impl,
        )
    }

    pub fn distance_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_max_motor_force_impl)
    }

    pub fn try_distance_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_max_motor_force_impl)
    }

    pub fn distance_set_max_motor_force(&mut self, id: JointId, force: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            force,
            distance_set_max_motor_force_impl,
        )
    }

    pub fn try_distance_set_max_motor_force(&mut self, id: JointId, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Distance,
            force,
            distance_set_max_motor_force_impl,
        )
    }

    pub fn distance_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_force_impl)
    }

    pub fn try_distance_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_force_impl)
    }
}

impl WorldHandle {
    pub fn distance_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_length_impl)
    }

    pub fn try_distance_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_length_impl)
    }

    pub fn distance_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_enabled_impl)
    }

    pub fn try_distance_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_enabled_impl)
    }

    pub fn distance_lower_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_lower_spring_force_impl)
    }

    pub fn try_distance_lower_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_lower_spring_force_impl)
    }

    pub fn distance_upper_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_upper_spring_force_impl)
    }

    pub fn try_distance_upper_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_upper_spring_force_impl)
    }

    pub fn distance_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_hertz_impl)
    }

    pub fn try_distance_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_hertz_impl)
    }

    pub fn distance_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_damping_ratio_impl)
    }

    pub fn try_distance_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_spring_damping_ratio_impl)
    }

    pub fn distance_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_limit_enabled_impl)
    }

    pub fn try_distance_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_limit_enabled_impl)
    }

    pub fn distance_min_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_min_length_impl)
    }

    pub fn try_distance_min_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_min_length_impl)
    }

    pub fn distance_max_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_max_length_impl)
    }

    pub fn try_distance_max_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_max_length_impl)
    }

    pub fn distance_current_length(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_current_length_impl)
    }

    pub fn try_distance_current_length(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_current_length_impl)
    }

    pub fn distance_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_enabled_impl)
    }

    pub fn try_distance_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_enabled_impl)
    }

    pub fn distance_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_speed_impl)
    }

    pub fn try_distance_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_speed_impl)
    }

    pub fn distance_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_max_motor_force_impl)
    }

    pub fn try_distance_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_max_motor_force_impl)
    }

    pub fn distance_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_force_impl)
    }

    pub fn try_distance_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Distance, distance_motor_force_impl)
    }
}

trait DistanceJointRuntimeHandle {
    fn distance_joint_id(&self) -> JointId;

    fn distance_length(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_length_impl,
        )
    }

    fn try_distance_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_length_impl,
        )
    }

    fn distance_set_length(&mut self, length: f32) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            length,
            distance_set_length_impl,
        );
    }

    fn try_distance_set_length(&mut self, length: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            length,
            distance_set_length_impl,
        )
    }

    fn distance_spring_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_enabled_impl,
        )
    }

    fn try_distance_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_enabled_impl,
        )
    }

    fn distance_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_spring_impl,
        );
    }

    fn try_distance_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_spring_impl,
        )
    }

    fn distance_lower_spring_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_lower_spring_force_impl,
        )
    }

    fn try_distance_lower_spring_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_lower_spring_force_impl,
        )
    }

    fn distance_upper_spring_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_upper_spring_force_impl,
        )
    }

    fn try_distance_upper_spring_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_upper_spring_force_impl,
        )
    }

    fn distance_set_spring_force_range(&mut self, lower_force: f32, upper_force: f32) {
        joint_kind_set2_checked_validated_impl(
            self.distance_joint_id(),
            JointType::Distance,
            lower_force,
            upper_force,
            assert_distance_spring_force_range_valid,
            distance_set_spring_force_range_impl,
        );
    }

    fn try_distance_set_spring_force_range(
        &mut self,
        lower_force: f32,
        upper_force: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            self.distance_joint_id(),
            JointType::Distance,
            lower_force,
            upper_force,
            check_distance_spring_force_range_valid,
            distance_set_spring_force_range_impl,
        )
    }

    fn distance_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_hertz_impl,
        )
    }

    fn try_distance_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_hertz_impl,
        )
    }

    fn distance_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            hertz,
            distance_set_spring_hertz_impl,
        );
    }

    fn try_distance_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            hertz,
            distance_set_spring_hertz_impl,
        )
    }

    fn distance_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_damping_ratio_impl,
        )
    }

    fn try_distance_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_spring_damping_ratio_impl,
        )
    }

    fn distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            damping_ratio,
            distance_set_spring_damping_ratio_impl,
        );
    }

    fn try_distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            damping_ratio,
            distance_set_spring_damping_ratio_impl,
        )
    }

    fn distance_limit_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_limit_enabled_impl,
        )
    }

    fn try_distance_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_limit_enabled_impl,
        )
    }

    fn distance_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_limit_impl,
        );
    }

    fn try_distance_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_limit_impl,
        )
    }

    fn distance_min_length(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_min_length_impl,
        )
    }

    fn try_distance_min_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_min_length_impl,
        )
    }

    fn distance_max_length(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_max_length_impl,
        )
    }

    fn try_distance_max_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_max_length_impl,
        )
    }

    fn distance_current_length(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_current_length_impl,
        )
    }

    fn try_distance_current_length(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_current_length_impl,
        )
    }

    fn distance_set_length_range(&mut self, min_length: f32, max_length: f32) {
        joint_kind_set2_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            min_length,
            max_length,
            distance_set_length_range_impl,
        );
    }

    fn try_distance_set_length_range(&mut self, min_length: f32, max_length: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            min_length,
            max_length,
            distance_set_length_range_impl,
        )
    }

    fn distance_motor_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_enabled_impl,
        )
    }

    fn try_distance_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_enabled_impl,
        )
    }

    fn distance_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_motor_impl,
        );
    }

    fn try_distance_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            enable,
            distance_enable_motor_impl,
        )
    }

    fn distance_motor_speed(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_speed_impl,
        )
    }

    fn try_distance_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_speed_impl,
        )
    }

    fn distance_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            speed,
            distance_set_motor_speed_impl,
        );
    }

    fn try_distance_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            speed,
            distance_set_motor_speed_impl,
        )
    }

    fn distance_max_motor_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_max_motor_force_impl,
        )
    }

    fn try_distance_max_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_max_motor_force_impl,
        )
    }

    fn distance_set_max_motor_force(&mut self, force: f32) {
        joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            force,
            distance_set_max_motor_force_impl,
        );
    }

    fn try_distance_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            force,
            distance_set_max_motor_force_impl,
        )
    }

    fn distance_motor_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_force_impl,
        )
    }

    fn try_distance_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.distance_joint_id(),
            JointType::Distance,
            distance_motor_force_impl,
        )
    }
}

impl DistanceJointRuntimeHandle for OwnedJoint {
    fn distance_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> DistanceJointRuntimeHandle for Joint<'w> {
    fn distance_joint_id(&self) -> JointId {
        self.id()
    }
}

impl OwnedJoint {
    pub fn distance_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_length(self)
    }
    pub fn try_distance_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_length(self)
    }
    pub fn distance_set_length(&mut self, length: f32) {
        DistanceJointRuntimeHandle::distance_set_length(self, length)
    }
    pub fn try_distance_set_length(&mut self, length: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length(self, length)
    }
    pub fn distance_spring_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_spring_enabled(self)
    }
    pub fn try_distance_spring_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_spring_enabled(self)
    }
    pub fn distance_enable_spring(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_spring(self, enable)
    }
    pub fn try_distance_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_spring(self, enable)
    }
    pub fn distance_lower_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_lower_spring_force(self)
    }
    pub fn try_distance_lower_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_lower_spring_force(self)
    }
    pub fn distance_upper_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_upper_spring_force(self)
    }
    pub fn try_distance_upper_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_upper_spring_force(self)
    }
    pub fn distance_set_spring_force_range(&mut self, lower_force: f32, upper_force: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_force_range(self, lower_force, upper_force)
    }
    pub fn try_distance_set_spring_force_range(
        &mut self,
        lower_force: f32,
        upper_force: f32,
    ) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_force_range(
            self,
            lower_force,
            upper_force,
        )
    }
    pub fn distance_spring_hertz(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_hertz(self)
    }
    pub fn try_distance_spring_hertz(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_hertz(self)
    }
    pub fn distance_set_spring_hertz(&mut self, hertz: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_hertz(self, hertz)
    }
    pub fn try_distance_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_hertz(self, hertz)
    }
    pub fn distance_spring_damping_ratio(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_damping_ratio(self)
    }
    pub fn try_distance_spring_damping_ratio(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_damping_ratio(self)
    }
    pub fn distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn distance_limit_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_limit_enabled(self)
    }
    pub fn try_distance_limit_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_limit_enabled(self)
    }
    pub fn distance_enable_limit(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_limit(self, enable)
    }
    pub fn try_distance_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_limit(self, enable)
    }
    pub fn distance_min_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_min_length(self)
    }
    pub fn try_distance_min_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_min_length(self)
    }
    pub fn distance_max_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_length(self)
    }
    pub fn try_distance_max_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_length(self)
    }
    pub fn distance_current_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_current_length(self)
    }
    pub fn try_distance_current_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_current_length(self)
    }
    pub fn distance_set_length_range(&mut self, min_length: f32, max_length: f32) {
        DistanceJointRuntimeHandle::distance_set_length_range(self, min_length, max_length)
    }
    pub fn try_distance_set_length_range(
        &mut self,
        min_length: f32,
        max_length: f32,
    ) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length_range(self, min_length, max_length)
    }
    pub fn distance_motor_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_motor_enabled(self)
    }
    pub fn try_distance_motor_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_motor_enabled(self)
    }
    pub fn distance_enable_motor(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_motor(self, enable)
    }
    pub fn try_distance_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_motor(self, enable)
    }
    pub fn distance_motor_speed(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_speed(self)
    }
    pub fn try_distance_motor_speed(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_speed(self)
    }
    pub fn distance_set_motor_speed(&mut self, speed: f32) {
        DistanceJointRuntimeHandle::distance_set_motor_speed(self, speed)
    }
    pub fn try_distance_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_motor_speed(self, speed)
    }
    pub fn distance_max_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_motor_force(self)
    }
    pub fn try_distance_max_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_motor_force(self)
    }
    pub fn distance_set_max_motor_force(&mut self, force: f32) {
        DistanceJointRuntimeHandle::distance_set_max_motor_force(self, force)
    }
    pub fn try_distance_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_max_motor_force(self, force)
    }
    pub fn distance_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_force(self)
    }
    pub fn try_distance_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_force(self)
    }
}

impl<'w> Joint<'w> {
    pub fn distance_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_length(self)
    }
    pub fn try_distance_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_length(self)
    }
    pub fn distance_set_length(&mut self, length: f32) {
        DistanceJointRuntimeHandle::distance_set_length(self, length)
    }
    pub fn try_distance_set_length(&mut self, length: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length(self, length)
    }
    pub fn distance_spring_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_spring_enabled(self)
    }
    pub fn try_distance_spring_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_spring_enabled(self)
    }
    pub fn distance_enable_spring(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_spring(self, enable)
    }
    pub fn try_distance_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_spring(self, enable)
    }
    pub fn distance_lower_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_lower_spring_force(self)
    }
    pub fn try_distance_lower_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_lower_spring_force(self)
    }
    pub fn distance_upper_spring_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_upper_spring_force(self)
    }
    pub fn try_distance_upper_spring_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_upper_spring_force(self)
    }
    pub fn distance_set_spring_force_range(&mut self, lower_force: f32, upper_force: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_force_range(self, lower_force, upper_force)
    }
    pub fn try_distance_set_spring_force_range(
        &mut self,
        lower_force: f32,
        upper_force: f32,
    ) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_force_range(
            self,
            lower_force,
            upper_force,
        )
    }
    pub fn distance_spring_hertz(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_hertz(self)
    }
    pub fn try_distance_spring_hertz(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_hertz(self)
    }
    pub fn distance_set_spring_hertz(&mut self, hertz: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_hertz(self, hertz)
    }
    pub fn try_distance_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_hertz(self, hertz)
    }
    pub fn distance_spring_damping_ratio(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_spring_damping_ratio(self)
    }
    pub fn try_distance_spring_damping_ratio(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_spring_damping_ratio(self)
    }
    pub fn distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        DistanceJointRuntimeHandle::distance_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_distance_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn distance_limit_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_limit_enabled(self)
    }
    pub fn try_distance_limit_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_limit_enabled(self)
    }
    pub fn distance_enable_limit(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_limit(self, enable)
    }
    pub fn try_distance_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_limit(self, enable)
    }
    pub fn distance_min_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_min_length(self)
    }
    pub fn try_distance_min_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_min_length(self)
    }
    pub fn distance_max_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_length(self)
    }
    pub fn try_distance_max_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_length(self)
    }
    pub fn distance_current_length(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_current_length(self)
    }
    pub fn try_distance_current_length(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_current_length(self)
    }
    pub fn distance_set_length_range(&mut self, min_length: f32, max_length: f32) {
        DistanceJointRuntimeHandle::distance_set_length_range(self, min_length, max_length)
    }
    pub fn try_distance_set_length_range(
        &mut self,
        min_length: f32,
        max_length: f32,
    ) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_length_range(self, min_length, max_length)
    }
    pub fn distance_motor_enabled(&self) -> bool {
        DistanceJointRuntimeHandle::distance_motor_enabled(self)
    }
    pub fn try_distance_motor_enabled(&self) -> ApiResult<bool> {
        DistanceJointRuntimeHandle::try_distance_motor_enabled(self)
    }
    pub fn distance_enable_motor(&mut self, enable: bool) {
        DistanceJointRuntimeHandle::distance_enable_motor(self, enable)
    }
    pub fn try_distance_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_enable_motor(self, enable)
    }
    pub fn distance_motor_speed(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_speed(self)
    }
    pub fn try_distance_motor_speed(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_speed(self)
    }
    pub fn distance_set_motor_speed(&mut self, speed: f32) {
        DistanceJointRuntimeHandle::distance_set_motor_speed(self, speed)
    }
    pub fn try_distance_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_motor_speed(self, speed)
    }
    pub fn distance_max_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_max_motor_force(self)
    }
    pub fn try_distance_max_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_max_motor_force(self)
    }
    pub fn distance_set_max_motor_force(&mut self, force: f32) {
        DistanceJointRuntimeHandle::distance_set_max_motor_force(self, force)
    }
    pub fn try_distance_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        DistanceJointRuntimeHandle::try_distance_set_max_motor_force(self, force)
    }
    pub fn distance_motor_force(&self) -> f32 {
        DistanceJointRuntimeHandle::distance_motor_force(self)
    }
    pub fn try_distance_motor_force(&self) -> ApiResult<f32> {
        DistanceJointRuntimeHandle::try_distance_motor_force(self)
    }
}

impl World {
    pub fn prismatic_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_enabled_impl)
    }

    pub fn try_prismatic_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_enabled_impl)
    }

    pub fn prismatic_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_spring_impl,
        )
    }

    pub fn try_prismatic_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_spring_impl,
        )
    }

    pub fn prismatic_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_hertz_impl)
    }

    pub fn try_prismatic_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_hertz_impl)
    }

    pub fn prismatic_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            hertz,
            prismatic_set_spring_hertz_impl,
        )
    }

    pub fn try_prismatic_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            hertz,
            prismatic_set_spring_hertz_impl,
        )
    }

    pub fn prismatic_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    pub fn try_prismatic_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    pub fn prismatic_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            damping_ratio,
            prismatic_set_spring_damping_ratio_impl,
        )
    }

    pub fn try_prismatic_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            damping_ratio,
            prismatic_set_spring_damping_ratio_impl,
        )
    }

    pub fn prismatic_target_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_target_translation_impl)
    }

    pub fn try_prismatic_target_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_target_translation_impl)
    }

    pub fn prismatic_set_target_translation(&mut self, id: JointId, translation: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            translation,
            prismatic_set_target_translation_impl,
        )
    }

    pub fn try_prismatic_set_target_translation(
        &mut self,
        id: JointId,
        translation: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            translation,
            prismatic_set_target_translation_impl,
        )
    }

    pub fn prismatic_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_limit_enabled_impl)
    }

    pub fn try_prismatic_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_limit_enabled_impl)
    }

    pub fn prismatic_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_limit_impl,
        )
    }

    pub fn try_prismatic_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_limit_impl,
        )
    }

    pub fn prismatic_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_lower_limit_impl)
    }

    pub fn try_prismatic_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_lower_limit_impl)
    }

    pub fn prismatic_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_upper_limit_impl)
    }

    pub fn try_prismatic_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_upper_limit_impl)
    }

    pub fn prismatic_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            id,
            JointType::Prismatic,
            lower,
            upper,
            assert_prismatic_limits_valid,
            prismatic_set_limits_impl,
        )
    }

    pub fn try_prismatic_set_limits(
        &mut self,
        id: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            id,
            JointType::Prismatic,
            lower,
            upper,
            check_prismatic_limits_valid,
            prismatic_set_limits_impl,
        )
    }

    pub fn prismatic_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_enabled_impl)
    }

    pub fn try_prismatic_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_enabled_impl)
    }

    pub fn prismatic_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_motor_impl,
        )
    }

    pub fn try_prismatic_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            enable,
            prismatic_enable_motor_impl,
        )
    }

    pub fn prismatic_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_speed_impl)
    }

    pub fn try_prismatic_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_speed_impl)
    }

    pub fn prismatic_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            speed,
            prismatic_set_motor_speed_impl,
        )
    }

    pub fn try_prismatic_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            speed,
            prismatic_set_motor_speed_impl,
        )
    }

    pub fn prismatic_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_max_motor_force_impl)
    }

    pub fn try_prismatic_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_max_motor_force_impl)
    }

    pub fn prismatic_set_max_motor_force(&mut self, id: JointId, force: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            force,
            prismatic_set_max_motor_force_impl,
        )
    }

    pub fn try_prismatic_set_max_motor_force(&mut self, id: JointId, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Prismatic,
            force,
            prismatic_set_max_motor_force_impl,
        )
    }

    pub fn prismatic_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_force_impl)
    }

    pub fn try_prismatic_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_force_impl)
    }

    pub fn prismatic_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_translation_impl)
    }

    pub fn try_prismatic_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_translation_impl)
    }

    pub fn prismatic_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_speed_impl)
    }

    pub fn try_prismatic_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_speed_impl)
    }

    pub fn revolute_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_enabled_impl)
    }

    pub fn try_revolute_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_enabled_impl)
    }

    pub fn revolute_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Revolute, enable, revolute_enable_spring_impl)
    }

    pub fn try_revolute_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            enable,
            revolute_enable_spring_impl,
        )
    }

    pub fn revolute_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_hertz_impl)
    }

    pub fn try_revolute_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_hertz_impl)
    }

    pub fn revolute_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            hertz,
            revolute_set_spring_hertz_impl,
        )
    }

    pub fn try_revolute_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            hertz,
            revolute_set_spring_hertz_impl,
        )
    }

    pub fn revolute_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_damping_ratio_impl)
    }

    pub fn try_revolute_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_damping_ratio_impl)
    }

    pub fn revolute_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            damping_ratio,
            revolute_set_spring_damping_ratio_impl,
        )
    }

    pub fn try_revolute_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            damping_ratio,
            revolute_set_spring_damping_ratio_impl,
        )
    }

    pub fn revolute_target_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_target_angle_impl)
    }

    pub fn try_revolute_target_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_target_angle_impl)
    }

    pub fn revolute_set_target_angle(&mut self, id: JointId, angle: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            angle,
            revolute_set_target_angle_impl,
        )
    }

    pub fn try_revolute_set_target_angle(&mut self, id: JointId, angle: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            angle,
            revolute_set_target_angle_impl,
        )
    }

    pub fn revolute_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_angle_impl)
    }

    pub fn try_revolute_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_angle_impl)
    }

    pub fn revolute_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_limit_enabled_impl)
    }

    pub fn try_revolute_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_limit_enabled_impl)
    }

    pub fn revolute_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Revolute, enable, revolute_enable_limit_impl)
    }

    pub fn try_revolute_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Revolute, enable, revolute_enable_limit_impl)
    }

    pub fn revolute_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_lower_limit_impl)
    }

    pub fn try_revolute_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_lower_limit_impl)
    }

    pub fn revolute_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_upper_limit_impl)
    }

    pub fn try_revolute_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_upper_limit_impl)
    }

    pub fn revolute_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            id,
            JointType::Revolute,
            lower,
            upper,
            assert_revolute_limits_valid,
            revolute_set_limits_impl,
        )
    }

    pub fn try_revolute_set_limits(
        &mut self,
        id: JointId,
        lower: f32,
        upper: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            id,
            JointType::Revolute,
            lower,
            upper,
            check_revolute_limits_valid,
            revolute_set_limits_impl,
        )
    }

    pub fn revolute_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_enabled_impl)
    }

    pub fn try_revolute_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_enabled_impl)
    }

    pub fn revolute_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Revolute, enable, revolute_enable_motor_impl)
    }

    pub fn try_revolute_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Revolute, enable, revolute_enable_motor_impl)
    }

    pub fn revolute_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_speed_impl)
    }

    pub fn try_revolute_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_speed_impl)
    }

    pub fn revolute_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            speed,
            revolute_set_motor_speed_impl,
        )
    }

    pub fn try_revolute_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            speed,
            revolute_set_motor_speed_impl,
        )
    }

    pub fn revolute_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_torque_impl)
    }

    pub fn try_revolute_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_torque_impl)
    }

    pub fn revolute_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_max_motor_torque_impl)
    }

    pub fn try_revolute_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_max_motor_torque_impl)
    }

    pub fn revolute_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            torque,
            revolute_set_max_motor_torque_impl,
        )
    }

    pub fn try_revolute_set_max_motor_torque(&mut self, id: JointId, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Revolute,
            torque,
            revolute_set_max_motor_torque_impl,
        )
    }
}

impl WorldHandle {
    pub fn prismatic_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_enabled_impl)
    }

    pub fn try_prismatic_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_enabled_impl)
    }

    pub fn prismatic_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_hertz_impl)
    }

    pub fn try_prismatic_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_spring_hertz_impl)
    }

    pub fn prismatic_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    pub fn try_prismatic_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            id,
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    pub fn prismatic_target_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_target_translation_impl)
    }

    pub fn try_prismatic_target_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_target_translation_impl)
    }

    pub fn prismatic_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_limit_enabled_impl)
    }

    pub fn try_prismatic_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_limit_enabled_impl)
    }

    pub fn prismatic_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_lower_limit_impl)
    }

    pub fn try_prismatic_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_lower_limit_impl)
    }

    pub fn prismatic_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_upper_limit_impl)
    }

    pub fn try_prismatic_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_upper_limit_impl)
    }

    pub fn prismatic_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_enabled_impl)
    }

    pub fn try_prismatic_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_enabled_impl)
    }

    pub fn prismatic_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_speed_impl)
    }

    pub fn try_prismatic_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_speed_impl)
    }

    pub fn prismatic_max_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_max_motor_force_impl)
    }

    pub fn try_prismatic_max_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_max_motor_force_impl)
    }

    pub fn prismatic_motor_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_force_impl)
    }

    pub fn try_prismatic_motor_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_motor_force_impl)
    }

    pub fn prismatic_translation(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_translation_impl)
    }

    pub fn try_prismatic_translation(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_translation_impl)
    }

    pub fn prismatic_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_speed_impl)
    }

    pub fn try_prismatic_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Prismatic, prismatic_speed_impl)
    }

    pub fn revolute_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_enabled_impl)
    }

    pub fn try_revolute_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_enabled_impl)
    }

    pub fn revolute_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_hertz_impl)
    }

    pub fn try_revolute_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_hertz_impl)
    }

    pub fn revolute_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_damping_ratio_impl)
    }

    pub fn try_revolute_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_spring_damping_ratio_impl)
    }

    pub fn revolute_target_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_target_angle_impl)
    }

    pub fn try_revolute_target_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_target_angle_impl)
    }

    pub fn revolute_angle(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_angle_impl)
    }

    pub fn try_revolute_angle(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_angle_impl)
    }

    pub fn revolute_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_limit_enabled_impl)
    }

    pub fn try_revolute_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_limit_enabled_impl)
    }

    pub fn revolute_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_lower_limit_impl)
    }

    pub fn try_revolute_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_lower_limit_impl)
    }

    pub fn revolute_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_upper_limit_impl)
    }

    pub fn try_revolute_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_upper_limit_impl)
    }

    pub fn revolute_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_enabled_impl)
    }

    pub fn try_revolute_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_enabled_impl)
    }

    pub fn revolute_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_speed_impl)
    }

    pub fn try_revolute_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_speed_impl)
    }

    pub fn revolute_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_torque_impl)
    }

    pub fn try_revolute_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_motor_torque_impl)
    }

    pub fn revolute_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Revolute, revolute_max_motor_torque_impl)
    }

    pub fn try_revolute_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Revolute, revolute_max_motor_torque_impl)
    }
}

trait PrismaticJointRuntimeHandle {
    fn prismatic_joint_id(&self) -> JointId;

    fn prismatic_spring_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_enabled_impl,
        )
    }

    fn try_prismatic_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_enabled_impl,
        )
    }

    fn prismatic_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_spring_impl,
        );
    }

    fn try_prismatic_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_spring_impl,
        )
    }

    fn prismatic_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_hertz_impl,
        )
    }

    fn try_prismatic_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_hertz_impl,
        )
    }

    fn prismatic_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            hertz,
            prismatic_set_spring_hertz_impl,
        );
    }

    fn try_prismatic_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            hertz,
            prismatic_set_spring_hertz_impl,
        )
    }

    fn prismatic_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    fn try_prismatic_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_spring_damping_ratio_impl,
        )
    }

    fn prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            damping_ratio,
            prismatic_set_spring_damping_ratio_impl,
        );
    }

    fn try_prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            damping_ratio,
            prismatic_set_spring_damping_ratio_impl,
        )
    }

    fn prismatic_target_translation(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_target_translation_impl,
        )
    }

    fn try_prismatic_target_translation(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_target_translation_impl,
        )
    }

    fn prismatic_set_target_translation(&mut self, translation: f32) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            translation,
            prismatic_set_target_translation_impl,
        );
    }

    fn try_prismatic_set_target_translation(&mut self, translation: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            translation,
            prismatic_set_target_translation_impl,
        )
    }

    fn prismatic_limit_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_limit_enabled_impl,
        )
    }

    fn try_prismatic_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_limit_enabled_impl,
        )
    }

    fn prismatic_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_limit_impl,
        );
    }

    fn try_prismatic_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_limit_impl,
        )
    }

    fn prismatic_lower_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_lower_limit_impl,
        )
    }

    fn try_prismatic_lower_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_lower_limit_impl,
        )
    }

    fn prismatic_upper_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_upper_limit_impl,
        )
    }

    fn try_prismatic_upper_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_upper_limit_impl,
        )
    }

    fn prismatic_set_limits(&mut self, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            lower,
            upper,
            assert_prismatic_limits_valid,
            prismatic_set_limits_impl,
        );
    }

    fn try_prismatic_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            lower,
            upper,
            check_prismatic_limits_valid,
            prismatic_set_limits_impl,
        )
    }

    fn prismatic_motor_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_enabled_impl,
        )
    }

    fn try_prismatic_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_enabled_impl,
        )
    }

    fn prismatic_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_motor_impl,
        );
    }

    fn try_prismatic_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            enable,
            prismatic_enable_motor_impl,
        )
    }

    fn prismatic_motor_speed(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_speed_impl,
        )
    }

    fn try_prismatic_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_speed_impl,
        )
    }

    fn prismatic_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            speed,
            prismatic_set_motor_speed_impl,
        );
    }

    fn try_prismatic_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            speed,
            prismatic_set_motor_speed_impl,
        )
    }

    fn prismatic_max_motor_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_max_motor_force_impl,
        )
    }

    fn try_prismatic_max_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_max_motor_force_impl,
        )
    }

    fn prismatic_set_max_motor_force(&mut self, force: f32) {
        joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            force,
            prismatic_set_max_motor_force_impl,
        );
    }

    fn try_prismatic_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            force,
            prismatic_set_max_motor_force_impl,
        )
    }

    fn prismatic_motor_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_force_impl,
        )
    }

    fn try_prismatic_motor_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_motor_force_impl,
        )
    }

    fn prismatic_translation(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_translation_impl,
        )
    }

    fn try_prismatic_translation(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_translation_impl,
        )
    }

    fn prismatic_speed(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_speed_impl,
        )
    }

    fn try_prismatic_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.prismatic_joint_id(),
            JointType::Prismatic,
            prismatic_speed_impl,
        )
    }
}

impl PrismaticJointRuntimeHandle for OwnedJoint {
    fn prismatic_joint_id(&self) -> JointId {
        self.id()
    }
}

impl OwnedJoint {
    pub fn prismatic_spring_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_spring_enabled(self)
    }
    pub fn try_prismatic_spring_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_enabled(self)
    }
    pub fn prismatic_enable_spring(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_spring(self, enable)
    }
    pub fn try_prismatic_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_spring(self, enable)
    }
    pub fn prismatic_spring_hertz(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_hertz(self)
    }
    pub fn try_prismatic_spring_hertz(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_hertz(self)
    }
    pub fn prismatic_set_spring_hertz(&mut self, hertz: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_hertz(self, hertz)
    }
    pub fn try_prismatic_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_hertz(self, hertz)
    }
    pub fn prismatic_spring_damping_ratio(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_damping_ratio(self)
    }
    pub fn try_prismatic_spring_damping_ratio(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_damping_ratio(self)
    }
    pub fn prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn prismatic_target_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_target_translation(self)
    }
    pub fn try_prismatic_target_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_target_translation(self)
    }
    pub fn prismatic_set_target_translation(&mut self, translation: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_target_translation(self, translation)
    }
    pub fn try_prismatic_set_target_translation(&mut self, translation: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_target_translation(self, translation)
    }
    pub fn prismatic_limit_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_limit_enabled(self)
    }
    pub fn try_prismatic_limit_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_limit_enabled(self)
    }
    pub fn prismatic_enable_limit(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_limit(self, enable)
    }
    pub fn try_prismatic_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_limit(self, enable)
    }
    pub fn prismatic_lower_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_lower_limit(self)
    }
    pub fn try_prismatic_lower_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_lower_limit(self)
    }
    pub fn prismatic_upper_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_upper_limit(self)
    }
    pub fn try_prismatic_upper_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_upper_limit(self)
    }
    pub fn prismatic_set_limits(&mut self, lower: f32, upper: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_limits(self, lower, upper)
    }
    pub fn try_prismatic_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_limits(self, lower, upper)
    }
    pub fn prismatic_motor_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_motor_enabled(self)
    }
    pub fn try_prismatic_motor_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_enabled(self)
    }
    pub fn prismatic_enable_motor(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_motor(self, enable)
    }
    pub fn try_prismatic_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_motor(self, enable)
    }
    pub fn prismatic_motor_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_speed(self)
    }
    pub fn try_prismatic_motor_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_speed(self)
    }
    pub fn prismatic_set_motor_speed(&mut self, speed: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_motor_speed(self, speed)
    }
    pub fn try_prismatic_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_motor_speed(self, speed)
    }
    pub fn prismatic_max_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_max_motor_force(self)
    }
    pub fn try_prismatic_max_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_max_motor_force(self)
    }
    pub fn prismatic_set_max_motor_force(&mut self, force: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_max_motor_force(self, force)
    }
    pub fn try_prismatic_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_max_motor_force(self, force)
    }
    pub fn prismatic_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_force(self)
    }
    pub fn try_prismatic_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_force(self)
    }
    pub fn prismatic_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_translation(self)
    }
    pub fn try_prismatic_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_translation(self)
    }
    pub fn prismatic_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_speed(self)
    }
    pub fn try_prismatic_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_speed(self)
    }

    pub fn revolute_spring_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_spring_enabled(self)
    }
    pub fn try_revolute_spring_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_spring_enabled(self)
    }
    pub fn revolute_enable_spring(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_spring(self, enable)
    }
    pub fn try_revolute_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_spring(self, enable)
    }
    pub fn revolute_spring_hertz(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_hertz(self)
    }
    pub fn try_revolute_spring_hertz(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_hertz(self)
    }
    pub fn revolute_set_spring_hertz(&mut self, hertz: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_hertz(self, hertz)
    }
    pub fn try_revolute_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_hertz(self, hertz)
    }
    pub fn revolute_spring_damping_ratio(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_damping_ratio(self)
    }
    pub fn try_revolute_spring_damping_ratio(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_damping_ratio(self)
    }
    pub fn revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn revolute_target_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_target_angle(self)
    }
    pub fn try_revolute_target_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_target_angle(self)
    }
    pub fn revolute_set_target_angle(&mut self, angle: f32) {
        RevoluteJointRuntimeHandle::revolute_set_target_angle(self, angle)
    }
    pub fn try_revolute_set_target_angle(&mut self, angle: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_target_angle(self, angle)
    }
    pub fn revolute_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_angle(self)
    }
    pub fn try_revolute_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_angle(self)
    }
    pub fn revolute_limit_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_limit_enabled(self)
    }
    pub fn try_revolute_limit_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_limit_enabled(self)
    }
    pub fn revolute_enable_limit(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_limit(self, enable)
    }
    pub fn try_revolute_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_limit(self, enable)
    }
    pub fn revolute_lower_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_lower_limit(self)
    }
    pub fn try_revolute_lower_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_lower_limit(self)
    }
    pub fn revolute_upper_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_upper_limit(self)
    }
    pub fn try_revolute_upper_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_upper_limit(self)
    }
    pub fn revolute_set_limits(&mut self, lower: f32, upper: f32) {
        RevoluteJointRuntimeHandle::revolute_set_limits(self, lower, upper)
    }
    pub fn try_revolute_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_limits(self, lower, upper)
    }
    pub fn revolute_motor_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_motor_enabled(self)
    }
    pub fn try_revolute_motor_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_motor_enabled(self)
    }
    pub fn revolute_enable_motor(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_motor(self, enable)
    }
    pub fn try_revolute_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_motor(self, enable)
    }
    pub fn revolute_motor_speed(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_speed(self)
    }
    pub fn try_revolute_motor_speed(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_speed(self)
    }
    pub fn revolute_set_motor_speed(&mut self, speed: f32) {
        RevoluteJointRuntimeHandle::revolute_set_motor_speed(self, speed)
    }
    pub fn try_revolute_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_motor_speed(self, speed)
    }
    pub fn revolute_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_torque(self)
    }
    pub fn try_revolute_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_torque(self)
    }
    pub fn revolute_max_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_max_motor_torque(self)
    }
    pub fn try_revolute_max_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_max_motor_torque(self)
    }
    pub fn revolute_set_max_motor_torque(&mut self, torque: f32) {
        RevoluteJointRuntimeHandle::revolute_set_max_motor_torque(self, torque)
    }
    pub fn try_revolute_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_max_motor_torque(self, torque)
    }
}

impl<'w> PrismaticJointRuntimeHandle for Joint<'w> {
    fn prismatic_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> Joint<'w> {
    pub fn prismatic_spring_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_spring_enabled(self)
    }
    pub fn try_prismatic_spring_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_enabled(self)
    }
    pub fn prismatic_enable_spring(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_spring(self, enable)
    }
    pub fn try_prismatic_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_spring(self, enable)
    }
    pub fn prismatic_spring_hertz(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_hertz(self)
    }
    pub fn try_prismatic_spring_hertz(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_hertz(self)
    }
    pub fn prismatic_set_spring_hertz(&mut self, hertz: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_hertz(self, hertz)
    }
    pub fn try_prismatic_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_hertz(self, hertz)
    }
    pub fn prismatic_spring_damping_ratio(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_spring_damping_ratio(self)
    }
    pub fn try_prismatic_spring_damping_ratio(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_spring_damping_ratio(self)
    }
    pub fn prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_prismatic_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn prismatic_target_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_target_translation(self)
    }
    pub fn try_prismatic_target_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_target_translation(self)
    }
    pub fn prismatic_set_target_translation(&mut self, translation: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_target_translation(self, translation)
    }
    pub fn try_prismatic_set_target_translation(&mut self, translation: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_target_translation(self, translation)
    }
    pub fn prismatic_limit_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_limit_enabled(self)
    }
    pub fn try_prismatic_limit_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_limit_enabled(self)
    }
    pub fn prismatic_enable_limit(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_limit(self, enable)
    }
    pub fn try_prismatic_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_limit(self, enable)
    }
    pub fn prismatic_lower_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_lower_limit(self)
    }
    pub fn try_prismatic_lower_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_lower_limit(self)
    }
    pub fn prismatic_upper_limit(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_upper_limit(self)
    }
    pub fn try_prismatic_upper_limit(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_upper_limit(self)
    }
    pub fn prismatic_set_limits(&mut self, lower: f32, upper: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_limits(self, lower, upper)
    }
    pub fn try_prismatic_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_limits(self, lower, upper)
    }
    pub fn prismatic_motor_enabled(&self) -> bool {
        PrismaticJointRuntimeHandle::prismatic_motor_enabled(self)
    }
    pub fn try_prismatic_motor_enabled(&self) -> ApiResult<bool> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_enabled(self)
    }
    pub fn prismatic_enable_motor(&mut self, enable: bool) {
        PrismaticJointRuntimeHandle::prismatic_enable_motor(self, enable)
    }
    pub fn try_prismatic_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_enable_motor(self, enable)
    }
    pub fn prismatic_motor_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_speed(self)
    }
    pub fn try_prismatic_motor_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_speed(self)
    }
    pub fn prismatic_set_motor_speed(&mut self, speed: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_motor_speed(self, speed)
    }
    pub fn try_prismatic_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_motor_speed(self, speed)
    }
    pub fn prismatic_max_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_max_motor_force(self)
    }
    pub fn try_prismatic_max_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_max_motor_force(self)
    }
    pub fn prismatic_set_max_motor_force(&mut self, force: f32) {
        PrismaticJointRuntimeHandle::prismatic_set_max_motor_force(self, force)
    }
    pub fn try_prismatic_set_max_motor_force(&mut self, force: f32) -> ApiResult<()> {
        PrismaticJointRuntimeHandle::try_prismatic_set_max_motor_force(self, force)
    }
    pub fn prismatic_motor_force(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_motor_force(self)
    }
    pub fn try_prismatic_motor_force(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_motor_force(self)
    }
    pub fn prismatic_translation(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_translation(self)
    }
    pub fn try_prismatic_translation(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_translation(self)
    }
    pub fn prismatic_speed(&self) -> f32 {
        PrismaticJointRuntimeHandle::prismatic_speed(self)
    }
    pub fn try_prismatic_speed(&self) -> ApiResult<f32> {
        PrismaticJointRuntimeHandle::try_prismatic_speed(self)
    }

    pub fn revolute_spring_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_spring_enabled(self)
    }
    pub fn try_revolute_spring_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_spring_enabled(self)
    }
    pub fn revolute_enable_spring(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_spring(self, enable)
    }
    pub fn try_revolute_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_spring(self, enable)
    }
    pub fn revolute_spring_hertz(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_hertz(self)
    }
    pub fn try_revolute_spring_hertz(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_hertz(self)
    }
    pub fn revolute_set_spring_hertz(&mut self, hertz: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_hertz(self, hertz)
    }
    pub fn try_revolute_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_hertz(self, hertz)
    }
    pub fn revolute_spring_damping_ratio(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_spring_damping_ratio(self)
    }
    pub fn try_revolute_spring_damping_ratio(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_spring_damping_ratio(self)
    }
    pub fn revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        RevoluteJointRuntimeHandle::revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn revolute_target_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_target_angle(self)
    }
    pub fn try_revolute_target_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_target_angle(self)
    }
    pub fn revolute_set_target_angle(&mut self, angle: f32) {
        RevoluteJointRuntimeHandle::revolute_set_target_angle(self, angle)
    }
    pub fn try_revolute_set_target_angle(&mut self, angle: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_target_angle(self, angle)
    }
    pub fn revolute_angle(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_angle(self)
    }
    pub fn try_revolute_angle(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_angle(self)
    }
    pub fn revolute_limit_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_limit_enabled(self)
    }
    pub fn try_revolute_limit_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_limit_enabled(self)
    }
    pub fn revolute_enable_limit(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_limit(self, enable)
    }
    pub fn try_revolute_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_limit(self, enable)
    }
    pub fn revolute_lower_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_lower_limit(self)
    }
    pub fn try_revolute_lower_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_lower_limit(self)
    }
    pub fn revolute_upper_limit(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_upper_limit(self)
    }
    pub fn try_revolute_upper_limit(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_upper_limit(self)
    }
    pub fn revolute_set_limits(&mut self, lower: f32, upper: f32) {
        RevoluteJointRuntimeHandle::revolute_set_limits(self, lower, upper)
    }
    pub fn try_revolute_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_limits(self, lower, upper)
    }
    pub fn revolute_motor_enabled(&self) -> bool {
        RevoluteJointRuntimeHandle::revolute_motor_enabled(self)
    }
    pub fn try_revolute_motor_enabled(&self) -> ApiResult<bool> {
        RevoluteJointRuntimeHandle::try_revolute_motor_enabled(self)
    }
    pub fn revolute_enable_motor(&mut self, enable: bool) {
        RevoluteJointRuntimeHandle::revolute_enable_motor(self, enable)
    }
    pub fn try_revolute_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_enable_motor(self, enable)
    }
    pub fn revolute_motor_speed(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_speed(self)
    }
    pub fn try_revolute_motor_speed(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_speed(self)
    }
    pub fn revolute_set_motor_speed(&mut self, speed: f32) {
        RevoluteJointRuntimeHandle::revolute_set_motor_speed(self, speed)
    }
    pub fn try_revolute_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_motor_speed(self, speed)
    }
    pub fn revolute_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_motor_torque(self)
    }
    pub fn try_revolute_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_motor_torque(self)
    }
    pub fn revolute_max_motor_torque(&self) -> f32 {
        RevoluteJointRuntimeHandle::revolute_max_motor_torque(self)
    }
    pub fn try_revolute_max_motor_torque(&self) -> ApiResult<f32> {
        RevoluteJointRuntimeHandle::try_revolute_max_motor_torque(self)
    }
    pub fn revolute_set_max_motor_torque(&mut self, torque: f32) {
        RevoluteJointRuntimeHandle::revolute_set_max_motor_torque(self, torque)
    }
    pub fn try_revolute_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        RevoluteJointRuntimeHandle::try_revolute_set_max_motor_torque(self, torque)
    }
}

trait RevoluteJointRuntimeHandle {
    fn revolute_joint_id(&self) -> JointId;

    fn revolute_spring_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_enabled_impl,
        )
    }

    fn try_revolute_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_enabled_impl,
        )
    }

    fn revolute_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_spring_impl,
        );
    }

    fn try_revolute_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_spring_impl,
        )
    }

    fn revolute_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_hertz_impl,
        )
    }

    fn try_revolute_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_hertz_impl,
        )
    }

    fn revolute_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            hertz,
            revolute_set_spring_hertz_impl,
        );
    }

    fn try_revolute_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            hertz,
            revolute_set_spring_hertz_impl,
        )
    }

    fn revolute_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_damping_ratio_impl,
        )
    }

    fn try_revolute_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_spring_damping_ratio_impl,
        )
    }

    fn revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            damping_ratio,
            revolute_set_spring_damping_ratio_impl,
        );
    }

    fn try_revolute_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            damping_ratio,
            revolute_set_spring_damping_ratio_impl,
        )
    }

    fn revolute_target_angle(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_target_angle_impl,
        )
    }

    fn try_revolute_target_angle(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_target_angle_impl,
        )
    }

    fn revolute_set_target_angle(&mut self, angle: f32) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            angle,
            revolute_set_target_angle_impl,
        );
    }

    fn try_revolute_set_target_angle(&mut self, angle: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            angle,
            revolute_set_target_angle_impl,
        )
    }

    fn revolute_angle(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_angle_impl,
        )
    }

    fn try_revolute_angle(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_angle_impl,
        )
    }

    fn revolute_limit_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_limit_enabled_impl,
        )
    }

    fn try_revolute_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_limit_enabled_impl,
        )
    }

    fn revolute_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_limit_impl,
        );
    }

    fn try_revolute_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_limit_impl,
        )
    }

    fn revolute_lower_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_lower_limit_impl,
        )
    }

    fn try_revolute_lower_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_lower_limit_impl,
        )
    }

    fn revolute_upper_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_upper_limit_impl,
        )
    }

    fn try_revolute_upper_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_upper_limit_impl,
        )
    }

    fn revolute_set_limits(&mut self, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            lower,
            upper,
            assert_revolute_limits_valid,
            revolute_set_limits_impl,
        );
    }

    fn try_revolute_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            lower,
            upper,
            check_revolute_limits_valid,
            revolute_set_limits_impl,
        )
    }

    fn revolute_motor_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_enabled_impl,
        )
    }

    fn try_revolute_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_enabled_impl,
        )
    }

    fn revolute_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_motor_impl,
        );
    }

    fn try_revolute_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            enable,
            revolute_enable_motor_impl,
        )
    }

    fn revolute_motor_speed(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_speed_impl,
        )
    }

    fn try_revolute_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_speed_impl,
        )
    }

    fn revolute_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            speed,
            revolute_set_motor_speed_impl,
        );
    }

    fn try_revolute_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            speed,
            revolute_set_motor_speed_impl,
        )
    }

    fn revolute_motor_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_torque_impl,
        )
    }

    fn try_revolute_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_motor_torque_impl,
        )
    }

    fn revolute_max_motor_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_max_motor_torque_impl,
        )
    }

    fn try_revolute_max_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            revolute_max_motor_torque_impl,
        )
    }

    fn revolute_set_max_motor_torque(&mut self, torque: f32) {
        joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            torque,
            revolute_set_max_motor_torque_impl,
        );
    }

    fn try_revolute_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.revolute_joint_id(),
            JointType::Revolute,
            torque,
            revolute_set_max_motor_torque_impl,
        )
    }
}

impl RevoluteJointRuntimeHandle for OwnedJoint {
    fn revolute_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> RevoluteJointRuntimeHandle for Joint<'w> {
    fn revolute_joint_id(&self) -> JointId {
        self.id()
    }
}

trait WeldJointRuntimeHandle {
    fn weld_joint_id(&self) -> JointId;

    fn weld_linear_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_hertz_impl,
        )
    }

    fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_hertz_impl,
        )
    }

    fn weld_set_linear_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_linear_hertz_impl,
        );
    }

    fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_linear_hertz_impl,
        )
    }

    fn weld_linear_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_linear_damping_ratio_impl,
        )
    }

    fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        );
    }

    fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        )
    }

    fn weld_angular_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_hertz_impl,
        )
    }

    fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_hertz_impl,
        )
    }

    fn weld_set_angular_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_angular_hertz_impl,
        );
    }

    fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            hertz,
            weld_set_angular_hertz_impl,
        )
    }

    fn weld_angular_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            weld_angular_damping_ratio_impl,
        )
    }

    fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        );
    }

    fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.weld_joint_id(),
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        )
    }
}

impl WeldJointRuntimeHandle for OwnedJoint {
    fn weld_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> WeldJointRuntimeHandle for Joint<'w> {
    fn weld_joint_id(&self) -> JointId {
        self.id()
    }
}

trait WheelJointRuntimeHandle {
    fn wheel_joint_id(&self) -> JointId;

    fn wheel_spring_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_enabled_impl,
        )
    }

    fn try_wheel_spring_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_enabled_impl,
        )
    }

    fn wheel_enable_spring(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_spring_impl,
        );
    }

    fn try_wheel_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_spring_impl,
        )
    }

    fn wheel_spring_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_hertz_impl,
        )
    }

    fn try_wheel_spring_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_hertz_impl,
        )
    }

    fn wheel_set_spring_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            hertz,
            wheel_set_spring_hertz_impl,
        );
    }

    fn try_wheel_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            hertz,
            wheel_set_spring_hertz_impl,
        )
    }

    fn wheel_spring_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_damping_ratio_impl,
        )
    }

    fn try_wheel_spring_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_spring_damping_ratio_impl,
        )
    }

    fn wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            damping_ratio,
            wheel_set_spring_damping_ratio_impl,
        );
    }

    fn try_wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            damping_ratio,
            wheel_set_spring_damping_ratio_impl,
        )
    }

    fn wheel_limit_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_limit_enabled_impl,
        )
    }

    fn try_wheel_limit_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_limit_enabled_impl,
        )
    }

    fn wheel_enable_limit(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_limit_impl,
        );
    }

    fn try_wheel_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_limit_impl,
        )
    }

    fn wheel_lower_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_lower_limit_impl,
        )
    }

    fn try_wheel_lower_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_lower_limit_impl,
        )
    }

    fn wheel_upper_limit(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_upper_limit_impl,
        )
    }

    fn try_wheel_upper_limit(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_upper_limit_impl,
        )
    }

    fn wheel_set_limits(&mut self, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            lower,
            upper,
            assert_wheel_limits_valid,
            wheel_set_limits_impl,
        );
    }

    fn try_wheel_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            lower,
            upper,
            check_wheel_limits_valid,
            wheel_set_limits_impl,
        )
    }

    fn wheel_motor_enabled(&self) -> bool {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_enabled_impl,
        )
    }

    fn try_wheel_motor_enabled(&self) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_enabled_impl,
        )
    }

    fn wheel_enable_motor(&mut self, enable: bool) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_motor_impl,
        );
    }

    fn try_wheel_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            enable,
            wheel_enable_motor_impl,
        )
    }

    fn wheel_motor_speed(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_speed_impl,
        )
    }

    fn try_wheel_motor_speed(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_speed_impl,
        )
    }

    fn wheel_set_motor_speed(&mut self, speed: f32) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            speed,
            wheel_set_motor_speed_impl,
        );
    }

    fn try_wheel_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            speed,
            wheel_set_motor_speed_impl,
        )
    }

    fn wheel_motor_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_torque_impl,
        )
    }

    fn try_wheel_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_motor_torque_impl,
        )
    }

    fn wheel_max_motor_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_max_motor_torque_impl,
        )
    }

    fn try_wheel_max_motor_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            wheel_max_motor_torque_impl,
        )
    }

    fn wheel_set_max_motor_torque(&mut self, torque: f32) {
        joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            torque,
            wheel_set_max_motor_torque_impl,
        );
    }

    fn try_wheel_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.wheel_joint_id(),
            JointType::Wheel,
            torque,
            wheel_set_max_motor_torque_impl,
        )
    }
}

impl WheelJointRuntimeHandle for OwnedJoint {
    fn wheel_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> WheelJointRuntimeHandle for Joint<'w> {
    fn wheel_joint_id(&self) -> JointId {
        self.id()
    }
}

trait MotorJointRuntimeHandle {
    fn motor_joint_id(&self) -> JointId;

    fn motor_linear_velocity(&self) -> Vec2 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_velocity_impl,
        )
    }

    fn try_motor_linear_velocity(&self) -> ApiResult<Vec2> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_velocity_impl,
        )
    }

    fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            v.into(),
            motor_set_linear_velocity_impl,
        );
    }

    fn try_motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            v.into(),
            motor_set_linear_velocity_impl,
        )
    }

    fn motor_angular_velocity(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_velocity_impl,
        )
    }

    fn try_motor_angular_velocity(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_velocity_impl,
        )
    }

    fn motor_set_angular_velocity(&mut self, w: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            w,
            motor_set_angular_velocity_impl,
        );
    }

    fn try_motor_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            w,
            motor_set_angular_velocity_impl,
        )
    }

    fn motor_max_velocity_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_velocity_force_impl,
        )
    }

    fn try_motor_max_velocity_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_velocity_force_impl,
        )
    }

    fn motor_set_max_velocity_force(&mut self, f: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            f,
            motor_set_max_velocity_force_impl,
        );
    }

    fn try_motor_set_max_velocity_force(&mut self, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            f,
            motor_set_max_velocity_force_impl,
        )
    }

    fn motor_max_velocity_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_velocity_torque_impl,
        )
    }

    fn try_motor_max_velocity_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_velocity_torque_impl,
        )
    }

    fn motor_set_max_velocity_torque(&mut self, t: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            t,
            motor_set_max_velocity_torque_impl,
        );
    }

    fn try_motor_set_max_velocity_torque(&mut self, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            t,
            motor_set_max_velocity_torque_impl,
        )
    }

    fn motor_linear_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_hertz_impl,
        )
    }

    fn try_motor_linear_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_hertz_impl,
        )
    }

    fn motor_set_linear_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            hertz,
            motor_set_linear_hertz_impl,
        );
    }

    fn try_motor_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            hertz,
            motor_set_linear_hertz_impl,
        )
    }

    fn motor_linear_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_damping_ratio_impl,
        )
    }

    fn try_motor_linear_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_linear_damping_ratio_impl,
        )
    }

    fn motor_set_linear_damping_ratio(&mut self, damping: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            damping,
            motor_set_linear_damping_ratio_impl,
        );
    }

    fn try_motor_set_linear_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            damping,
            motor_set_linear_damping_ratio_impl,
        )
    }

    fn motor_angular_hertz(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_hertz_impl,
        )
    }

    fn try_motor_angular_hertz(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_hertz_impl,
        )
    }

    fn motor_set_angular_hertz(&mut self, hertz: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            hertz,
            motor_set_angular_hertz_impl,
        );
    }

    fn try_motor_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            hertz,
            motor_set_angular_hertz_impl,
        )
    }

    fn motor_angular_damping_ratio(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_damping_ratio_impl,
        )
    }

    fn try_motor_angular_damping_ratio(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_angular_damping_ratio_impl,
        )
    }

    fn motor_set_angular_damping_ratio(&mut self, damping: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            damping,
            motor_set_angular_damping_ratio_impl,
        );
    }

    fn try_motor_set_angular_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            damping,
            motor_set_angular_damping_ratio_impl,
        )
    }

    fn motor_max_spring_force(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_spring_force_impl,
        )
    }

    fn try_motor_max_spring_force(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_spring_force_impl,
        )
    }

    fn motor_set_max_spring_force(&mut self, f: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            f,
            motor_set_max_spring_force_impl,
        );
    }

    fn try_motor_set_max_spring_force(&mut self, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            f,
            motor_set_max_spring_force_impl,
        )
    }

    fn motor_max_spring_torque(&self) -> f32 {
        joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_spring_torque_impl,
        )
    }

    fn try_motor_max_spring_torque(&self) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            motor_max_spring_torque_impl,
        )
    }

    fn motor_set_max_spring_torque(&mut self, t: f32) {
        joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            t,
            motor_set_max_spring_torque_impl,
        );
    }

    fn try_motor_set_max_spring_torque(&mut self, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            self.motor_joint_id(),
            JointType::Motor,
            t,
            motor_set_max_spring_torque_impl,
        )
    }
}

impl MotorJointRuntimeHandle for OwnedJoint {
    fn motor_joint_id(&self) -> JointId {
        self.id()
    }
}

impl<'w> MotorJointRuntimeHandle for Joint<'w> {
    fn motor_joint_id(&self) -> JointId {
        self.id()
    }
}

impl World {
    pub fn weld_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn try_weld_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_linear_hertz_impl)
    }

    pub fn try_weld_set_linear_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_linear_hertz_impl)
    }

    pub fn weld_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn try_weld_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn weld_set_linear_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        )
    }

    pub fn try_weld_set_linear_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_linear_damping_ratio_impl,
        )
    }

    pub fn weld_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn try_weld_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_angular_hertz_impl)
    }

    pub fn try_weld_set_angular_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Weld, hertz, weld_set_angular_hertz_impl)
    }

    pub fn weld_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn try_weld_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn weld_set_angular_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        )
    }

    pub fn try_weld_set_angular_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Weld,
            damping_ratio,
            weld_set_angular_damping_ratio_impl,
        )
    }

    pub fn wheel_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_enabled_impl)
    }

    pub fn try_wheel_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_enabled_impl)
    }

    pub fn wheel_enable_spring(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_spring_impl)
    }

    pub fn try_wheel_enable_spring(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_spring_impl)
    }

    pub fn wheel_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_hertz_impl)
    }

    pub fn try_wheel_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_hertz_impl)
    }

    pub fn wheel_set_spring_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Wheel, hertz, wheel_set_spring_hertz_impl)
    }

    pub fn try_wheel_set_spring_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Wheel, hertz, wheel_set_spring_hertz_impl)
    }

    pub fn wheel_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_damping_ratio_impl)
    }

    pub fn try_wheel_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_damping_ratio_impl)
    }

    pub fn wheel_set_spring_damping_ratio(&mut self, id: JointId, damping_ratio: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Wheel,
            damping_ratio,
            wheel_set_spring_damping_ratio_impl,
        )
    }

    pub fn try_wheel_set_spring_damping_ratio(
        &mut self,
        id: JointId,
        damping_ratio: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Wheel,
            damping_ratio,
            wheel_set_spring_damping_ratio_impl,
        )
    }

    pub fn wheel_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_limit_enabled_impl)
    }

    pub fn try_wheel_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_limit_enabled_impl)
    }

    pub fn wheel_enable_limit(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_limit_impl)
    }

    pub fn try_wheel_enable_limit(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_limit_impl)
    }

    pub fn wheel_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_lower_limit_impl)
    }

    pub fn try_wheel_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_lower_limit_impl)
    }

    pub fn wheel_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_upper_limit_impl)
    }

    pub fn try_wheel_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_upper_limit_impl)
    }

    pub fn wheel_set_limits(&mut self, id: JointId, lower: f32, upper: f32) {
        joint_kind_set2_checked_validated_impl(
            id,
            JointType::Wheel,
            lower,
            upper,
            assert_wheel_limits_valid,
            wheel_set_limits_impl,
        )
    }

    pub fn try_wheel_set_limits(&mut self, id: JointId, lower: f32, upper: f32) -> ApiResult<()> {
        try_joint_kind_set2_checked_validated_impl(
            id,
            JointType::Wheel,
            lower,
            upper,
            check_wheel_limits_valid,
            wheel_set_limits_impl,
        )
    }

    pub fn wheel_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_enabled_impl)
    }

    pub fn try_wheel_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_enabled_impl)
    }

    pub fn wheel_enable_motor(&mut self, id: JointId, enable: bool) {
        joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_motor_impl)
    }

    pub fn try_wheel_enable_motor(&mut self, id: JointId, enable: bool) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Wheel, enable, wheel_enable_motor_impl)
    }

    pub fn wheel_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_speed_impl)
    }

    pub fn try_wheel_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_speed_impl)
    }

    pub fn wheel_set_motor_speed(&mut self, id: JointId, speed: f32) {
        joint_kind_set_checked_impl(id, JointType::Wheel, speed, wheel_set_motor_speed_impl)
    }

    pub fn try_wheel_set_motor_speed(&mut self, id: JointId, speed: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Wheel, speed, wheel_set_motor_speed_impl)
    }

    pub fn wheel_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_torque_impl)
    }

    pub fn try_wheel_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_torque_impl)
    }

    pub fn wheel_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_max_motor_torque_impl)
    }

    pub fn try_wheel_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_max_motor_torque_impl)
    }

    pub fn wheel_set_max_motor_torque(&mut self, id: JointId, torque: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Wheel,
            torque,
            wheel_set_max_motor_torque_impl,
        )
    }

    pub fn try_wheel_set_max_motor_torque(&mut self, id: JointId, torque: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Wheel,
            torque,
            wheel_set_max_motor_torque_impl,
        )
    }

    pub fn motor_linear_velocity(&self, id: JointId) -> Vec2 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_velocity_impl)
    }

    pub fn try_motor_linear_velocity(&self, id: JointId) -> ApiResult<Vec2> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_velocity_impl)
    }

    pub fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, id: JointId, v: V) {
        joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            v.into(),
            motor_set_linear_velocity_impl,
        )
    }

    pub fn try_motor_set_linear_velocity<V: Into<Vec2>>(
        &mut self,
        id: JointId,
        v: V,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            v.into(),
            motor_set_linear_velocity_impl,
        )
    }

    pub fn motor_angular_velocity(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_velocity_impl)
    }

    pub fn try_motor_angular_velocity(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_velocity_impl)
    }

    pub fn motor_set_angular_velocity(&mut self, id: JointId, w: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, w, motor_set_angular_velocity_impl)
    }

    pub fn try_motor_set_angular_velocity(&mut self, id: JointId, w: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, w, motor_set_angular_velocity_impl)
    }

    pub fn motor_max_velocity_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_force_impl)
    }

    pub fn try_motor_max_velocity_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_force_impl)
    }

    pub fn motor_set_max_velocity_force(&mut self, id: JointId, f: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, f, motor_set_max_velocity_force_impl)
    }

    pub fn try_motor_set_max_velocity_force(&mut self, id: JointId, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, f, motor_set_max_velocity_force_impl)
    }

    pub fn motor_max_velocity_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_torque_impl)
    }

    pub fn try_motor_max_velocity_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_torque_impl)
    }

    pub fn motor_set_max_velocity_torque(&mut self, id: JointId, t: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, t, motor_set_max_velocity_torque_impl)
    }

    pub fn try_motor_set_max_velocity_torque(&mut self, id: JointId, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, t, motor_set_max_velocity_torque_impl)
    }

    pub fn motor_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_hertz_impl)
    }

    pub fn try_motor_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_hertz_impl)
    }

    pub fn motor_set_linear_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, hertz, motor_set_linear_hertz_impl)
    }

    pub fn try_motor_set_linear_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, hertz, motor_set_linear_hertz_impl)
    }

    pub fn motor_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_damping_ratio_impl)
    }

    pub fn try_motor_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_damping_ratio_impl)
    }

    pub fn motor_set_linear_damping_ratio(&mut self, id: JointId, damping: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            damping,
            motor_set_linear_damping_ratio_impl,
        )
    }

    pub fn try_motor_set_linear_damping_ratio(
        &mut self,
        id: JointId,
        damping: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            damping,
            motor_set_linear_damping_ratio_impl,
        )
    }

    pub fn motor_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_hertz_impl)
    }

    pub fn try_motor_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_hertz_impl)
    }

    pub fn motor_set_angular_hertz(&mut self, id: JointId, hertz: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, hertz, motor_set_angular_hertz_impl)
    }

    pub fn try_motor_set_angular_hertz(&mut self, id: JointId, hertz: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, hertz, motor_set_angular_hertz_impl)
    }

    pub fn motor_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_damping_ratio_impl)
    }

    pub fn try_motor_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_damping_ratio_impl)
    }

    pub fn motor_set_angular_damping_ratio(&mut self, id: JointId, damping: f32) {
        joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            damping,
            motor_set_angular_damping_ratio_impl,
        )
    }

    pub fn try_motor_set_angular_damping_ratio(
        &mut self,
        id: JointId,
        damping: f32,
    ) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(
            id,
            JointType::Motor,
            damping,
            motor_set_angular_damping_ratio_impl,
        )
    }

    pub fn motor_max_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_force_impl)
    }

    pub fn try_motor_max_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_force_impl)
    }

    pub fn motor_set_max_spring_force(&mut self, id: JointId, f: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, f, motor_set_max_spring_force_impl)
    }

    pub fn try_motor_set_max_spring_force(&mut self, id: JointId, f: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, f, motor_set_max_spring_force_impl)
    }

    pub fn motor_max_spring_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_torque_impl)
    }

    pub fn try_motor_max_spring_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_torque_impl)
    }

    pub fn motor_set_max_spring_torque(&mut self, id: JointId, t: f32) {
        joint_kind_set_checked_impl(id, JointType::Motor, t, motor_set_max_spring_torque_impl)
    }

    pub fn try_motor_set_max_spring_torque(&mut self, id: JointId, t: f32) -> ApiResult<()> {
        try_joint_kind_set_checked_impl(id, JointType::Motor, t, motor_set_max_spring_torque_impl)
    }
}

impl WorldHandle {
    pub fn weld_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn try_weld_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_hertz_impl)
    }

    pub fn weld_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn try_weld_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_linear_damping_ratio_impl)
    }

    pub fn weld_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn try_weld_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_hertz_impl)
    }

    pub fn weld_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn try_weld_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Weld, weld_angular_damping_ratio_impl)
    }

    pub fn wheel_spring_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_enabled_impl)
    }

    pub fn try_wheel_spring_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_enabled_impl)
    }

    pub fn wheel_spring_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_hertz_impl)
    }

    pub fn try_wheel_spring_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_hertz_impl)
    }

    pub fn wheel_spring_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_damping_ratio_impl)
    }

    pub fn try_wheel_spring_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_spring_damping_ratio_impl)
    }

    pub fn wheel_limit_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_limit_enabled_impl)
    }

    pub fn try_wheel_limit_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_limit_enabled_impl)
    }

    pub fn wheel_lower_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_lower_limit_impl)
    }

    pub fn try_wheel_lower_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_lower_limit_impl)
    }

    pub fn wheel_upper_limit(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_upper_limit_impl)
    }

    pub fn try_wheel_upper_limit(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_upper_limit_impl)
    }

    pub fn wheel_motor_enabled(&self, id: JointId) -> bool {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_enabled_impl)
    }

    pub fn try_wheel_motor_enabled(&self, id: JointId) -> ApiResult<bool> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_enabled_impl)
    }

    pub fn wheel_motor_speed(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_speed_impl)
    }

    pub fn try_wheel_motor_speed(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_speed_impl)
    }

    pub fn wheel_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_torque_impl)
    }

    pub fn try_wheel_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_motor_torque_impl)
    }

    pub fn wheel_max_motor_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Wheel, wheel_max_motor_torque_impl)
    }

    pub fn try_wheel_max_motor_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Wheel, wheel_max_motor_torque_impl)
    }

    pub fn motor_linear_velocity(&self, id: JointId) -> Vec2 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_velocity_impl)
    }

    pub fn try_motor_linear_velocity(&self, id: JointId) -> ApiResult<Vec2> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_velocity_impl)
    }

    pub fn motor_angular_velocity(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_velocity_impl)
    }

    pub fn try_motor_angular_velocity(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_velocity_impl)
    }

    pub fn motor_max_velocity_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_force_impl)
    }

    pub fn try_motor_max_velocity_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_force_impl)
    }

    pub fn motor_max_velocity_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_torque_impl)
    }

    pub fn try_motor_max_velocity_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_velocity_torque_impl)
    }

    pub fn motor_linear_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_hertz_impl)
    }

    pub fn try_motor_linear_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_hertz_impl)
    }

    pub fn motor_linear_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_damping_ratio_impl)
    }

    pub fn try_motor_linear_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_linear_damping_ratio_impl)
    }

    pub fn motor_angular_hertz(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_hertz_impl)
    }

    pub fn try_motor_angular_hertz(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_hertz_impl)
    }

    pub fn motor_angular_damping_ratio(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_damping_ratio_impl)
    }

    pub fn try_motor_angular_damping_ratio(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_angular_damping_ratio_impl)
    }

    pub fn motor_max_spring_force(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_force_impl)
    }

    pub fn try_motor_max_spring_force(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_force_impl)
    }

    pub fn motor_max_spring_torque(&self, id: JointId) -> f32 {
        joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_torque_impl)
    }

    pub fn try_motor_max_spring_torque(&self, id: JointId) -> ApiResult<f32> {
        try_joint_kind_get_checked_impl(id, JointType::Motor, motor_max_spring_torque_impl)
    }
}

impl OwnedJoint {
    pub fn weld_linear_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_hertz(self)
    }
    pub fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_hertz(self)
    }
    pub fn weld_set_linear_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_linear_hertz(self, hertz)
    }
    pub fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_hertz(self, hertz)
    }
    pub fn weld_linear_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_damping_ratio(self)
    }
    pub fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_damping_ratio(self)
    }
    pub fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn weld_angular_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_hertz(self)
    }
    pub fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_hertz(self)
    }
    pub fn weld_set_angular_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_angular_hertz(self, hertz)
    }
    pub fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_hertz(self, hertz)
    }
    pub fn weld_angular_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_damping_ratio(self)
    }
    pub fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_damping_ratio(self)
    }
    pub fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_angular_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_damping_ratio(self, damping_ratio)
    }
    pub fn wheel_spring_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_spring_enabled(self)
    }
    pub fn try_wheel_spring_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_spring_enabled(self)
    }
    pub fn wheel_enable_spring(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_spring(self, enable)
    }
    pub fn try_wheel_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_spring(self, enable)
    }
    pub fn wheel_spring_hertz(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_hertz(self)
    }
    pub fn try_wheel_spring_hertz(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_hertz(self)
    }
    pub fn wheel_set_spring_hertz(&mut self, hertz: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_hertz(self, hertz)
    }
    pub fn try_wheel_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_hertz(self, hertz)
    }
    pub fn wheel_spring_damping_ratio(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_damping_ratio(self)
    }
    pub fn try_wheel_spring_damping_ratio(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_damping_ratio(self)
    }
    pub fn wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn wheel_limit_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_limit_enabled(self)
    }
    pub fn try_wheel_limit_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_limit_enabled(self)
    }
    pub fn wheel_enable_limit(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_limit(self, enable)
    }
    pub fn try_wheel_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_limit(self, enable)
    }
    pub fn wheel_lower_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_lower_limit(self)
    }
    pub fn try_wheel_lower_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_lower_limit(self)
    }
    pub fn wheel_upper_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_upper_limit(self)
    }
    pub fn try_wheel_upper_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_upper_limit(self)
    }
    pub fn wheel_set_limits(&mut self, lower: f32, upper: f32) {
        WheelJointRuntimeHandle::wheel_set_limits(self, lower, upper)
    }
    pub fn try_wheel_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_limits(self, lower, upper)
    }
    pub fn wheel_motor_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_motor_enabled(self)
    }
    pub fn try_wheel_motor_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_motor_enabled(self)
    }
    pub fn wheel_enable_motor(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_motor(self, enable)
    }
    pub fn try_wheel_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_motor(self, enable)
    }
    pub fn wheel_motor_speed(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_speed(self)
    }
    pub fn try_wheel_motor_speed(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_speed(self)
    }
    pub fn wheel_set_motor_speed(&mut self, speed: f32) {
        WheelJointRuntimeHandle::wheel_set_motor_speed(self, speed)
    }
    pub fn try_wheel_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_motor_speed(self, speed)
    }
    pub fn wheel_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_torque(self)
    }
    pub fn try_wheel_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_torque(self)
    }
    pub fn wheel_max_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_max_motor_torque(self)
    }
    pub fn try_wheel_max_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_max_motor_torque(self)
    }
    pub fn wheel_set_max_motor_torque(&mut self, torque: f32) {
        WheelJointRuntimeHandle::wheel_set_max_motor_torque(self, torque)
    }
    pub fn try_wheel_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_max_motor_torque(self, torque)
    }
    pub fn motor_linear_velocity(&self) -> Vec2 {
        MotorJointRuntimeHandle::motor_linear_velocity(self)
    }
    pub fn try_motor_linear_velocity(&self) -> ApiResult<Vec2> {
        MotorJointRuntimeHandle::try_motor_linear_velocity(self)
    }
    pub fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        MotorJointRuntimeHandle::motor_set_linear_velocity(self, v)
    }
    pub fn try_motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_velocity(self, v)
    }
    pub fn motor_angular_velocity(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_velocity(self)
    }
    pub fn try_motor_angular_velocity(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_velocity(self)
    }
    pub fn motor_set_angular_velocity(&mut self, w: f32) {
        MotorJointRuntimeHandle::motor_set_angular_velocity(self, w)
    }
    pub fn try_motor_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_velocity(self, w)
    }
    pub fn motor_max_velocity_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_force(self)
    }
    pub fn try_motor_max_velocity_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_force(self)
    }
    pub fn motor_set_max_velocity_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_force(self, f)
    }
    pub fn try_motor_set_max_velocity_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_force(self, f)
    }
    pub fn motor_max_velocity_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_torque(self)
    }
    pub fn try_motor_max_velocity_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_torque(self)
    }
    pub fn motor_set_max_velocity_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_torque(self, t)
    }
    pub fn try_motor_set_max_velocity_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_torque(self, t)
    }
    pub fn motor_linear_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_hertz(self)
    }
    pub fn try_motor_linear_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_hertz(self)
    }
    pub fn motor_set_linear_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_linear_hertz(self, hertz)
    }
    pub fn try_motor_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_hertz(self, hertz)
    }
    pub fn motor_linear_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_damping_ratio(self)
    }
    pub fn try_motor_linear_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_damping_ratio(self)
    }
    pub fn motor_set_linear_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_linear_damping_ratio(self, damping)
    }
    pub fn try_motor_set_linear_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_damping_ratio(self, damping)
    }
    pub fn motor_angular_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_hertz(self)
    }
    pub fn try_motor_angular_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_hertz(self)
    }
    pub fn motor_set_angular_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_angular_hertz(self, hertz)
    }
    pub fn try_motor_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_hertz(self, hertz)
    }
    pub fn motor_angular_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_damping_ratio(self)
    }
    pub fn try_motor_angular_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_damping_ratio(self)
    }
    pub fn motor_set_angular_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_angular_damping_ratio(self, damping)
    }
    pub fn try_motor_set_angular_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_damping_ratio(self, damping)
    }
    pub fn motor_max_spring_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_force(self)
    }
    pub fn try_motor_max_spring_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_force(self)
    }
    pub fn motor_set_max_spring_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_force(self, f)
    }
    pub fn try_motor_set_max_spring_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_force(self, f)
    }
    pub fn motor_max_spring_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_torque(self)
    }
    pub fn try_motor_max_spring_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_torque(self)
    }
    pub fn motor_set_max_spring_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_torque(self, t)
    }
    pub fn try_motor_set_max_spring_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_torque(self, t)
    }
}

impl<'w> Joint<'w> {
    pub fn weld_linear_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_hertz(self)
    }
    pub fn try_weld_linear_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_hertz(self)
    }
    pub fn weld_set_linear_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_linear_hertz(self, hertz)
    }
    pub fn try_weld_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_hertz(self, hertz)
    }
    pub fn weld_linear_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_linear_damping_ratio(self)
    }
    pub fn try_weld_linear_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_linear_damping_ratio(self)
    }
    pub fn weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_linear_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_linear_damping_ratio(self, damping_ratio)
    }
    pub fn weld_angular_hertz(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_hertz(self)
    }
    pub fn try_weld_angular_hertz(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_hertz(self)
    }
    pub fn weld_set_angular_hertz(&mut self, hertz: f32) {
        WeldJointRuntimeHandle::weld_set_angular_hertz(self, hertz)
    }
    pub fn try_weld_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_hertz(self, hertz)
    }
    pub fn weld_angular_damping_ratio(&self) -> f32 {
        WeldJointRuntimeHandle::weld_angular_damping_ratio(self)
    }
    pub fn try_weld_angular_damping_ratio(&self) -> ApiResult<f32> {
        WeldJointRuntimeHandle::try_weld_angular_damping_ratio(self)
    }
    pub fn weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) {
        WeldJointRuntimeHandle::weld_set_angular_damping_ratio(self, damping_ratio)
    }
    pub fn try_weld_set_angular_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WeldJointRuntimeHandle::try_weld_set_angular_damping_ratio(self, damping_ratio)
    }
    pub fn wheel_spring_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_spring_enabled(self)
    }
    pub fn try_wheel_spring_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_spring_enabled(self)
    }
    pub fn wheel_enable_spring(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_spring(self, enable)
    }
    pub fn try_wheel_enable_spring(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_spring(self, enable)
    }
    pub fn wheel_spring_hertz(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_hertz(self)
    }
    pub fn try_wheel_spring_hertz(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_hertz(self)
    }
    pub fn wheel_set_spring_hertz(&mut self, hertz: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_hertz(self, hertz)
    }
    pub fn try_wheel_set_spring_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_hertz(self, hertz)
    }
    pub fn wheel_spring_damping_ratio(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_spring_damping_ratio(self)
    }
    pub fn try_wheel_spring_damping_ratio(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_spring_damping_ratio(self)
    }
    pub fn wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) {
        WheelJointRuntimeHandle::wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn try_wheel_set_spring_damping_ratio(&mut self, damping_ratio: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_spring_damping_ratio(self, damping_ratio)
    }
    pub fn wheel_limit_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_limit_enabled(self)
    }
    pub fn try_wheel_limit_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_limit_enabled(self)
    }
    pub fn wheel_enable_limit(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_limit(self, enable)
    }
    pub fn try_wheel_enable_limit(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_limit(self, enable)
    }
    pub fn wheel_lower_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_lower_limit(self)
    }
    pub fn try_wheel_lower_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_lower_limit(self)
    }
    pub fn wheel_upper_limit(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_upper_limit(self)
    }
    pub fn try_wheel_upper_limit(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_upper_limit(self)
    }
    pub fn wheel_set_limits(&mut self, lower: f32, upper: f32) {
        WheelJointRuntimeHandle::wheel_set_limits(self, lower, upper)
    }
    pub fn try_wheel_set_limits(&mut self, lower: f32, upper: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_limits(self, lower, upper)
    }
    pub fn wheel_motor_enabled(&self) -> bool {
        WheelJointRuntimeHandle::wheel_motor_enabled(self)
    }
    pub fn try_wheel_motor_enabled(&self) -> ApiResult<bool> {
        WheelJointRuntimeHandle::try_wheel_motor_enabled(self)
    }
    pub fn wheel_enable_motor(&mut self, enable: bool) {
        WheelJointRuntimeHandle::wheel_enable_motor(self, enable)
    }
    pub fn try_wheel_enable_motor(&mut self, enable: bool) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_enable_motor(self, enable)
    }
    pub fn wheel_motor_speed(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_speed(self)
    }
    pub fn try_wheel_motor_speed(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_speed(self)
    }
    pub fn wheel_set_motor_speed(&mut self, speed: f32) {
        WheelJointRuntimeHandle::wheel_set_motor_speed(self, speed)
    }
    pub fn try_wheel_set_motor_speed(&mut self, speed: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_motor_speed(self, speed)
    }
    pub fn wheel_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_motor_torque(self)
    }
    pub fn try_wheel_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_motor_torque(self)
    }
    pub fn wheel_max_motor_torque(&self) -> f32 {
        WheelJointRuntimeHandle::wheel_max_motor_torque(self)
    }
    pub fn try_wheel_max_motor_torque(&self) -> ApiResult<f32> {
        WheelJointRuntimeHandle::try_wheel_max_motor_torque(self)
    }
    pub fn wheel_set_max_motor_torque(&mut self, torque: f32) {
        WheelJointRuntimeHandle::wheel_set_max_motor_torque(self, torque)
    }
    pub fn try_wheel_set_max_motor_torque(&mut self, torque: f32) -> ApiResult<()> {
        WheelJointRuntimeHandle::try_wheel_set_max_motor_torque(self, torque)
    }
    pub fn motor_linear_velocity(&self) -> Vec2 {
        MotorJointRuntimeHandle::motor_linear_velocity(self)
    }
    pub fn try_motor_linear_velocity(&self) -> ApiResult<Vec2> {
        MotorJointRuntimeHandle::try_motor_linear_velocity(self)
    }
    pub fn motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) {
        MotorJointRuntimeHandle::motor_set_linear_velocity(self, v)
    }
    pub fn try_motor_set_linear_velocity<V: Into<Vec2>>(&mut self, v: V) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_velocity(self, v)
    }
    pub fn motor_angular_velocity(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_velocity(self)
    }
    pub fn try_motor_angular_velocity(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_velocity(self)
    }
    pub fn motor_set_angular_velocity(&mut self, w: f32) {
        MotorJointRuntimeHandle::motor_set_angular_velocity(self, w)
    }
    pub fn try_motor_set_angular_velocity(&mut self, w: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_velocity(self, w)
    }
    pub fn motor_max_velocity_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_force(self)
    }
    pub fn try_motor_max_velocity_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_force(self)
    }
    pub fn motor_set_max_velocity_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_force(self, f)
    }
    pub fn try_motor_set_max_velocity_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_force(self, f)
    }
    pub fn motor_max_velocity_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_velocity_torque(self)
    }
    pub fn try_motor_max_velocity_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_velocity_torque(self)
    }
    pub fn motor_set_max_velocity_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_velocity_torque(self, t)
    }
    pub fn try_motor_set_max_velocity_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_velocity_torque(self, t)
    }
    pub fn motor_linear_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_hertz(self)
    }
    pub fn try_motor_linear_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_hertz(self)
    }
    pub fn motor_set_linear_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_linear_hertz(self, hertz)
    }
    pub fn try_motor_set_linear_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_hertz(self, hertz)
    }
    pub fn motor_linear_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_linear_damping_ratio(self)
    }
    pub fn try_motor_linear_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_linear_damping_ratio(self)
    }
    pub fn motor_set_linear_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_linear_damping_ratio(self, damping)
    }
    pub fn try_motor_set_linear_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_linear_damping_ratio(self, damping)
    }
    pub fn motor_angular_hertz(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_hertz(self)
    }
    pub fn try_motor_angular_hertz(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_hertz(self)
    }
    pub fn motor_set_angular_hertz(&mut self, hertz: f32) {
        MotorJointRuntimeHandle::motor_set_angular_hertz(self, hertz)
    }
    pub fn try_motor_set_angular_hertz(&mut self, hertz: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_hertz(self, hertz)
    }
    pub fn motor_angular_damping_ratio(&self) -> f32 {
        MotorJointRuntimeHandle::motor_angular_damping_ratio(self)
    }
    pub fn try_motor_angular_damping_ratio(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_angular_damping_ratio(self)
    }
    pub fn motor_set_angular_damping_ratio(&mut self, damping: f32) {
        MotorJointRuntimeHandle::motor_set_angular_damping_ratio(self, damping)
    }
    pub fn try_motor_set_angular_damping_ratio(&mut self, damping: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_angular_damping_ratio(self, damping)
    }
    pub fn motor_max_spring_force(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_force(self)
    }
    pub fn try_motor_max_spring_force(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_force(self)
    }
    pub fn motor_set_max_spring_force(&mut self, f: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_force(self, f)
    }
    pub fn try_motor_set_max_spring_force(&mut self, f: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_force(self, f)
    }
    pub fn motor_max_spring_torque(&self) -> f32 {
        MotorJointRuntimeHandle::motor_max_spring_torque(self)
    }
    pub fn try_motor_max_spring_torque(&self) -> ApiResult<f32> {
        MotorJointRuntimeHandle::try_motor_max_spring_torque(self)
    }
    pub fn motor_set_max_spring_torque(&mut self, t: f32) {
        MotorJointRuntimeHandle::motor_set_max_spring_torque(self, t)
    }
    pub fn try_motor_set_max_spring_torque(&mut self, t: f32) -> ApiResult<()> {
        MotorJointRuntimeHandle::try_motor_set_max_spring_torque(self, t)
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
