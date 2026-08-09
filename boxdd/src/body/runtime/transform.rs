use boxdd_sys::ffi;

use crate::error::{Error, Result};
use crate::query::Aabb;
use crate::types::{BodyId, Position, Vec2, WorldTransform};

use super::super::{
    scoped::Body,
    validation::{
        check_body_world_point_in_local_range, check_valid_body_float, check_valid_body_position,
        check_valid_body_target_motion, check_valid_body_vec2, check_valid_native_body_position,
        check_valid_native_body_vec2,
    },
};
use super::{
    check_native_body_finite,
    mass::{body_local_center_of_mass_impl, body_world_center_of_mass_impl},
    raw_body_id,
};

#[inline]
pub(crate) fn body_position_impl(id: BodyId) -> Position {
    Position::from_raw(unsafe { ffi::b2Body_GetPosition(raw_body_id(id)) })
}

#[inline]
pub(crate) fn body_linear_velocity_impl(id: BodyId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2Body_GetLinearVelocity(raw_body_id(id)) })
}

#[inline]
pub(crate) fn body_angular_velocity_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetAngularVelocity(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_rotation_raw_impl(id: BodyId) -> ffi::b2Rot {
    unsafe { ffi::b2Body_GetRotation(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_rotation_impl(operation: &'static str, id: BodyId) -> Result<crate::Rot> {
    crate::Rot::from_raw(body_rotation_raw_impl(id)).map_err(|_| Error::InvalidNativeOutput {
        operation,
        output: "rotation",
        constraint: "a normalized finite rotation",
    })
}

#[inline]
fn body_transform_raw_impl(id: BodyId) -> ffi::b2WorldTransform {
    unsafe { ffi::b2Body_GetTransform(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_transform_impl(id: BodyId) -> Result<WorldTransform> {
    WorldTransform::from_raw(body_transform_raw_impl(id)).map_err(|_| Error::InvalidNativeOutput {
        operation: "Body::transform",
        output: "transform",
        constraint: "a finite rigid world transform",
    })
}

#[inline]
pub(crate) fn body_aabb_impl(id: BodyId) -> Result<Aabb> {
    Aabb::from_raw(unsafe { ffi::b2Body_ComputeAABB(raw_body_id(id)) }).map_err(|_| {
        Error::InvalidNativeOutput {
            operation: "Body::aabb",
            output: "aabb",
            constraint: "finite ordered lower and upper bounds",
        }
    })
}

#[inline]
pub(crate) fn body_local_point_impl<V: Into<Position>>(id: BodyId, world_point: V) -> Vec2 {
    let point: ffi::b2Pos = world_point.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetLocalPoint(raw_body_id(id), point) })
}

#[inline]
pub(crate) fn body_world_point_impl<V: Into<Vec2>>(id: BodyId, local_point: V) -> Position {
    let point: ffi::b2Vec2 = local_point.into().into_raw();
    Position::from_raw(unsafe { ffi::b2Body_GetWorldPoint(raw_body_id(id), point) })
}

#[inline]
pub(crate) fn body_local_vector_impl<V: Into<Vec2>>(id: BodyId, world_vector: V) -> Vec2 {
    let vector: ffi::b2Vec2 = world_vector.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetLocalVector(raw_body_id(id), vector) })
}

#[inline]
pub(crate) fn body_world_vector_impl<V: Into<Vec2>>(id: BodyId, local_vector: V) -> Vec2 {
    let vector: ffi::b2Vec2 = local_vector.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetWorldVector(raw_body_id(id), vector) })
}

#[inline]
pub(crate) fn body_local_point_velocity_impl<V: Into<Vec2>>(id: BodyId, local_point: V) -> Vec2 {
    let point: ffi::b2Vec2 = local_point.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetLocalPointVelocity(raw_body_id(id), point) })
}

#[inline]
pub(crate) fn body_world_point_velocity_impl<V: Into<Position>>(
    id: BodyId,
    world_point: V,
) -> Vec2 {
    let point: ffi::b2Pos = world_point.into().into_raw();
    Vec2::from_raw(unsafe { ffi::b2Body_GetWorldPointVelocity(raw_body_id(id), point) })
}

#[inline]
fn body_set_position_and_rotation_impl<V: Into<Position>>(
    id: BodyId,
    position: V,
    angle_radians: f32,
) {
    let (s, c) = angle_radians.sin_cos();
    let rotation = ffi::b2Rot { c, s };
    let position: ffi::b2Pos = position.into().into_raw();
    unsafe { ffi::b2Body_SetTransform(raw_body_id(id), position, rotation) };
}

#[inline]
fn body_set_linear_velocity_impl<V: Into<Vec2>>(id: BodyId, velocity: V) {
    let velocity: ffi::b2Vec2 = velocity.into().into_raw();
    unsafe { ffi::b2Body_SetLinearVelocity(raw_body_id(id), velocity) }
}

#[inline]
fn body_set_angular_velocity_impl(id: BodyId, angular_velocity: f32) {
    unsafe { ffi::b2Body_SetAngularVelocity(raw_body_id(id), angular_velocity) }
}

#[inline]
fn body_set_target_transform_impl(id: BodyId, target: WorldTransform, time_step: f32, wake: bool) {
    unsafe { ffi::b2Body_SetTargetTransform(raw_body_id(id), target.into_raw(), time_step, wake) };
}

impl Body<'_> {
    pub fn position(&self) -> Result<Position> {
        self.body_access().call(|_| {
            check_valid_native_body_position(
                "Body::position",
                "position",
                body_position_impl(self.body_id()),
            )
        })
    }

    pub fn linear_velocity(&self) -> Result<Vec2> {
        self.body_access().call(|_| {
            check_valid_native_body_vec2(
                "Body::linear_velocity",
                "linear_velocity",
                body_linear_velocity_impl(self.body_id()),
            )
        })
    }

    pub fn angular_velocity(&self) -> Result<f32> {
        self.body_access().call(|_| {
            check_native_body_finite(
                "Body::angular_velocity",
                "angular_velocity",
                body_angular_velocity_impl(self.body_id()),
            )
        })
    }

    pub fn rotation(&self) -> Result<crate::Rot> {
        self.body_access()
            .call(|_| body_rotation_impl("Body::rotation", self.body_id()))
    }

    pub fn transform(&self) -> Result<WorldTransform> {
        self.body_access()
            .call(|_| body_transform_impl(self.body_id()))
    }

    pub fn aabb(&self) -> Result<Aabb> {
        self.body_access().call(|_| body_aabb_impl(self.body_id()))
    }

    pub fn local_point<V: Into<Position>>(&self, world_point: V) -> Result<Vec2> {
        self.body_access().call(|_| {
            let world_point =
                check_valid_body_position("Body::local_point", "world_point", world_point.into())?;
            let id = self.body_id();
            let body_position = check_valid_native_body_position(
                "Body::local_point",
                "body_position",
                body_position_impl(id),
            )?;
            let world_point = check_body_world_point_in_local_range(
                "Body::local_point",
                "world_point",
                world_point,
                body_position,
            )?;
            check_valid_native_body_vec2(
                "Body::local_point",
                "local_point",
                body_local_point_impl(id, world_point),
            )
        })
    }

    pub fn world_point<V: Into<Vec2>>(&self, local_point: V) -> Result<Position> {
        self.body_access().call(|_| {
            let local_point =
                check_valid_body_vec2("Body::world_point", "local_point", local_point.into())?;
            check_valid_native_body_position(
                "Body::world_point",
                "world_point",
                body_world_point_impl(self.body_id(), local_point),
            )
        })
    }

    pub fn local_vector<V: Into<Vec2>>(&self, world_vector: V) -> Result<Vec2> {
        self.body_access().call(|_| {
            let world_vector =
                check_valid_body_vec2("Body::local_vector", "world_vector", world_vector.into())?;
            check_valid_native_body_vec2(
                "Body::local_vector",
                "local_vector",
                body_local_vector_impl(self.body_id(), world_vector),
            )
        })
    }

    pub fn world_vector<V: Into<Vec2>>(&self, local_vector: V) -> Result<Vec2> {
        self.body_access().call(|_| {
            let local_vector =
                check_valid_body_vec2("Body::world_vector", "local_vector", local_vector.into())?;
            check_valid_native_body_vec2(
                "Body::world_vector",
                "world_vector",
                body_world_vector_impl(self.body_id(), local_vector),
            )
        })
    }

    pub fn local_point_velocity<V: Into<Vec2>>(&self, local_point: V) -> Result<Vec2> {
        self.body_access().call(|_| {
            let local_point = check_valid_body_vec2(
                "Body::local_point_velocity",
                "local_point",
                local_point.into(),
            )?;
            check_valid_native_body_vec2(
                "Body::local_point_velocity",
                "velocity",
                body_local_point_velocity_impl(self.body_id(), local_point),
            )
        })
    }

    pub fn world_point_velocity<V: Into<Position>>(&self, world_point: V) -> Result<Vec2> {
        self.body_access().call(|_| {
            let world_point = check_valid_body_position(
                "Body::world_point_velocity",
                "world_point",
                world_point.into(),
            )?;
            let id = self.body_id();
            let center = check_valid_native_body_position(
                "Body::world_point_velocity",
                "world_center_of_mass",
                body_world_center_of_mass_impl(id),
            )?;
            let world_point = check_body_world_point_in_local_range(
                "Body::world_point_velocity",
                "world_point",
                world_point,
                center,
            )?;
            check_valid_native_body_vec2(
                "Body::world_point_velocity",
                "velocity",
                body_world_point_velocity_impl(id, world_point),
            )
        })
    }

    pub fn set_position_and_rotation<V: Into<Position>>(
        &mut self,
        position: V,
        angle_radians: f32,
    ) -> Result<()> {
        self.body_access().call(|_| {
            let position = check_valid_body_position(
                "Body::set_position_and_rotation",
                "position",
                position.into(),
            )?;
            let angle_radians = check_valid_body_float(
                "Body::set_position_and_rotation",
                "angle_radians",
                angle_radians,
            )?;
            body_set_position_and_rotation_impl(self.body_id(), position, angle_radians);
            Ok(())
        })
    }

    pub fn set_linear_velocity<V: Into<Vec2>>(&mut self, velocity: V) -> Result<()> {
        self.body_access().call(|_| {
            let velocity =
                check_valid_body_vec2("Body::set_linear_velocity", "velocity", velocity.into())?;
            body_set_linear_velocity_impl(self.body_id(), velocity);
            Ok(())
        })
    }

    pub fn set_angular_velocity(&mut self, angular_velocity: f32) -> Result<()> {
        self.body_access().call(|_| {
            let angular_velocity = check_valid_body_float(
                "Body::set_angular_velocity",
                "angular_velocity",
                angular_velocity,
            )?;
            body_set_angular_velocity_impl(self.body_id(), angular_velocity);
            Ok(())
        })
    }

    pub fn set_target_transform(
        &mut self,
        target: WorldTransform,
        time_step: f32,
        wake: bool,
    ) -> Result<()> {
        self.body_access().call(|_| {
            let id = self.body_id();
            let current_center = check_valid_native_body_position(
                "Body::set_target_transform",
                "world_center_of_mass",
                body_world_center_of_mass_impl(id),
            )?;
            let current_rotation = body_rotation_impl("Body::set_target_transform", id)?;
            let local_center = check_valid_native_body_vec2(
                "Body::set_target_transform",
                "local_center_of_mass",
                body_local_center_of_mass_impl(id),
            )?;
            let (target, time_step) = check_valid_body_target_motion(
                target,
                time_step,
                current_center,
                current_rotation,
                local_center,
            )?;
            body_set_target_transform_impl(id, target, time_step, wake);
            Ok(())
        })
    }
}
