use boxdd_sys::ffi;

use crate::error::Result;
use crate::types::{BodyId, MassData, Position, Vec2};

use super::super::{
    definition::check_mass_data_valid,
    scoped::Body,
    validation::{check_valid_native_body_position, check_valid_native_body_vec2},
};
use super::{check_native_body_non_negative, raw_body_id};

#[inline]
pub(crate) fn body_mass_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetMass(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_rotational_inertia_impl(id: BodyId) -> f32 {
    unsafe { ffi::b2Body_GetRotationalInertia(raw_body_id(id)) }
}

#[inline]
pub(crate) fn body_local_center_of_mass_impl(id: BodyId) -> Vec2 {
    Vec2::from_raw(unsafe { ffi::b2Body_GetLocalCenter(raw_body_id(id)) })
}

#[inline]
pub(crate) fn body_world_center_of_mass_impl(id: BodyId) -> Position {
    Position::from_raw(unsafe { ffi::b2Body_GetWorldCenter(raw_body_id(id)) })
}

#[inline]
pub(crate) fn body_mass_data_impl(id: BodyId) -> Result<MassData> {
    MassData::from_native("Body::mass_data", unsafe {
        ffi::b2Body_GetMassData(raw_body_id(id))
    })
}

#[inline]
fn body_set_mass_data_impl(id: BodyId, mass_data: MassData) {
    unsafe { ffi::b2Body_SetMassData(raw_body_id(id), mass_data.into_raw()) };
}

#[inline]
fn body_apply_mass_from_shapes_impl(id: BodyId) {
    unsafe { ffi::b2Body_ApplyMassFromShapes(raw_body_id(id)) };
}

impl Body<'_> {
    pub fn mass(&self) -> Result<f32> {
        self.body_access().call(|_| {
            check_native_body_non_negative("Body::mass", "mass", body_mass_impl(self.body_id()))
        })
    }

    pub fn rotational_inertia(&self) -> Result<f32> {
        self.body_access().call(|_| {
            check_native_body_non_negative(
                "Body::rotational_inertia",
                "rotational_inertia",
                body_rotational_inertia_impl(self.body_id()),
            )
        })
    }

    pub fn local_center_of_mass(&self) -> Result<Vec2> {
        self.body_access().call(|_| {
            check_valid_native_body_vec2(
                "Body::local_center_of_mass",
                "local_center_of_mass",
                body_local_center_of_mass_impl(self.body_id()),
            )
        })
    }

    pub fn world_center_of_mass(&self) -> Result<Position> {
        self.body_access().call(|_| {
            check_valid_native_body_position(
                "Body::world_center_of_mass",
                "world_center_of_mass",
                body_world_center_of_mass_impl(self.body_id()),
            )
        })
    }

    pub fn mass_data(&self) -> Result<MassData> {
        self.body_access()
            .call(|_| body_mass_data_impl(self.body_id()))
    }

    pub fn set_mass_data(&mut self, mass_data: MassData) -> Result<()> {
        self.body_access().call(|_| {
            check_mass_data_valid(mass_data)?;
            body_set_mass_data_impl(self.body_id(), mass_data);
            Ok(())
        })
    }

    pub fn apply_mass_from_shapes(&mut self) -> Result<()> {
        self.body_access().call(|_| {
            body_apply_mass_from_shapes_impl(self.body_id());
            Ok(())
        })
    }
}
