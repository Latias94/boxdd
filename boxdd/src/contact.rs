use crate::core::world_core::WorldCore;
use crate::error::ApiResult;
use crate::types::{ContactData, ContactId};
use crate::world::{World, WorldHandle};
use boxdd_sys::ffi;

#[inline]
fn contact_is_valid_impl(core: &WorldCore, id: ContactId) -> ApiResult<bool> {
    WorldCore::contact_is_valid(core, id)
}

#[inline]
fn contact_data_raw_impl(core: &WorldCore, id: ContactId) -> ApiResult<ffi::b2ContactData> {
    core.check_contact(id)?;
    Ok(unsafe { ffi::b2Contact_GetData(id.into_raw()) })
}

impl World {
    /// Return whether a contact id from the current completed-step epoch is still live.
    #[inline]
    pub fn contact_is_valid(&self, id: ContactId) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        contact_is_valid_impl(self.core(), id).expect("contact id belongs to a different world")
    }

    /// Recoverable version of [`Self::contact_is_valid`].
    #[inline]
    pub fn try_contact_is_valid(&self, id: ContactId) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        contact_is_valid_impl(self.core(), id)
    }

    /// Fetch an owned contact snapshot after validating world ownership and liveness.
    #[inline]
    pub fn contact_data(&self, id: ContactId) -> ContactData {
        crate::core::callback_state::assert_not_in_callback();
        self.contact_data_impl(id)
            .expect("invalid contact id or contact belongs to a different world")
    }

    /// Recoverable version of [`Self::contact_data`].
    #[inline]
    pub fn try_contact_data(&self, id: ContactId) -> ApiResult<ContactData> {
        crate::core::callback_state::check_not_in_callback()?;
        self.contact_data_impl(id)
    }

    /// Fetch the raw Box2D contact snapshot after validating ownership and liveness.
    #[inline]
    pub fn contact_data_raw(&self, id: ContactId) -> ffi::b2ContactData {
        crate::core::callback_state::assert_not_in_callback();
        contact_data_raw_impl(self.core(), id)
            .expect("invalid contact id or contact belongs to a different world")
    }

    /// Recoverable version of [`Self::contact_data_raw`].
    #[inline]
    pub fn try_contact_data_raw(&self, id: ContactId) -> ApiResult<ffi::b2ContactData> {
        crate::core::callback_state::check_not_in_callback()?;
        contact_data_raw_impl(self.core(), id)
    }

    #[inline]
    fn contact_data_impl(&self, id: ContactId) -> ApiResult<ContactData> {
        let raw = contact_data_raw_impl(self.core(), id)?;
        ContactData::try_from_raw_in(self.brand(), self.core().contact_epoch(), raw)
    }
}

impl WorldHandle {
    /// Return whether a contact id from the current completed-step epoch is still live.
    #[inline]
    pub fn contact_is_valid(&self, id: ContactId) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        contact_is_valid_impl(self.core(), id).expect("contact id belongs to a different world")
    }

    /// Recoverable version of [`Self::contact_is_valid`].
    #[inline]
    pub fn try_contact_is_valid(&self, id: ContactId) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        contact_is_valid_impl(self.core(), id)
    }

    /// Fetch an owned contact snapshot after validating world ownership and liveness.
    #[inline]
    pub fn contact_data(&self, id: ContactId) -> ContactData {
        crate::core::callback_state::assert_not_in_callback();
        self.contact_data_impl(id)
            .expect("invalid contact id or contact belongs to a different world")
    }

    /// Recoverable version of [`Self::contact_data`].
    #[inline]
    pub fn try_contact_data(&self, id: ContactId) -> ApiResult<ContactData> {
        crate::core::callback_state::check_not_in_callback()?;
        self.contact_data_impl(id)
    }

    /// Fetch the raw Box2D contact snapshot after validating ownership and liveness.
    #[inline]
    pub fn contact_data_raw(&self, id: ContactId) -> ffi::b2ContactData {
        crate::core::callback_state::assert_not_in_callback();
        contact_data_raw_impl(self.core(), id)
            .expect("invalid contact id or contact belongs to a different world")
    }

    /// Recoverable version of [`Self::contact_data_raw`].
    #[inline]
    pub fn try_contact_data_raw(&self, id: ContactId) -> ApiResult<ffi::b2ContactData> {
        crate::core::callback_state::check_not_in_callback()?;
        contact_data_raw_impl(self.core(), id)
    }

    #[inline]
    fn contact_data_impl(&self, id: ContactId) -> ApiResult<ContactData> {
        let raw = contact_data_raw_impl(self.core(), id)?;
        ContactData::try_from_raw_in(self.brand(), self.core().contact_epoch(), raw)
    }
}

#[cfg(test)]
mod tests {
    use boxdd_sys::ffi;

    use crate::{
        ApiError, BodyBuilder, BodyId, BodyType, ContactId, ShapeDef, World, WorldDef, shapes,
    };

    fn invalid_contact_id(world: &World) -> crate::ContactId {
        world.brand().contact(
            ffi::b2ContactId {
                index1: 0,
                world0: world.brand().world0(),
                padding: 0,
                generation: 0,
            },
            world.core().contact_epoch(),
        )
    }

    fn create_live_contact(world: &mut World) -> (BodyId, ContactId) {
        let body_a = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([-1.0_f32, 0.0])
                .build(),
        );
        let body_b = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([1.0_f32, 0.0])
                .build(),
        );
        let shape_def = ShapeDef::builder()
            .density(1.0)
            .enable_contact_events(true)
            .build();
        world.create_polygon_shape_for(body_a, &shape_def, &shapes::box_polygon(0.5_f32, 0.5));
        world.create_polygon_shape_for(body_b, &shape_def, &shapes::box_polygon(0.5_f32, 0.5));
        world.set_body_linear_velocity(body_a, [2.0_f32, 0.0]);
        world.set_body_linear_velocity(body_b, [-2.0_f32, 0.0]);

        for _ in 0..180 {
            world.step(1.0 / 60.0, 4);
            if let Some(event) = world.contact_events().begin.first() {
                return (body_a, event.contact_id);
            }
        }
        panic!("expected a live contact id from a contact-begin event");
    }

    #[test]
    fn try_contact_helpers_return_in_callback() {
        let world = World::new(WorldDef::default()).unwrap();
        let contact = invalid_contact_id(&world);
        let _guard = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(
            world.try_contact_is_valid(contact).unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            world.try_contact_data(contact).unwrap_err(),
            ApiError::InCallback
        );
        assert_eq!(
            world.try_contact_data_raw(contact).unwrap_err(),
            ApiError::InCallback
        );
    }

    #[test]
    fn contact_ids_and_raw_ids_expire_at_the_next_valid_step() {
        let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
        let (body, contact) = create_live_contact(&mut world);
        let raw_contact = contact.unbind();

        assert_eq!(world.bind_contact_id(raw_contact).unwrap(), contact);
        assert!(world.try_contact_is_valid(contact).unwrap());
        assert!(
            world
                .body(body)
                .unwrap()
                .contact_data()
                .iter()
                .any(|data| data.contact_id == contact)
        );

        world.step(1.0 / 60.0, 4);

        assert!(!world.try_contact_is_valid(contact).unwrap());
        assert_eq!(
            world.try_contact_data(contact).unwrap_err(),
            ApiError::InvalidContactId
        );
        assert_eq!(
            world.bind_contact_id(raw_contact).unwrap_err(),
            ApiError::InvalidContactId
        );

        let current_contact = world
            .body(body)
            .unwrap()
            .contact_data()
            .into_iter()
            .next()
            .expect("the active contact should receive a fresh id after the next step")
            .contact_id;
        assert_ne!(current_contact, contact);
        assert!(world.try_contact_is_valid(current_contact).unwrap());
        assert_eq!(
            world.bind_contact_id(current_contact.unbind()).unwrap(),
            current_contact
        );
    }
}
