use crate::core::world_core::WorldCore;
use crate::error::Result;
use crate::types::{ContactData, ContactId};
use crate::world::World;
use boxdd_sys::ffi;

#[inline]
fn contact_is_valid_impl(core: &WorldCore, id: ContactId) -> Result<bool> {
    WorldCore::contact_is_valid(core, id)
}

#[inline]
fn contact_data_raw_impl(core: &WorldCore, id: ContactId) -> Result<ffi::b2ContactData> {
    core.check_contact(id)?;
    Ok(unsafe { ffi::b2Contact_GetData(id.into_raw()) })
}

impl World {
    /// Return whether a contact id from the current completed-step epoch is still live.
    #[inline]
    pub fn contact_is_valid(&self, id: ContactId) -> Result<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        contact_is_valid_impl(self.core(), id)
    }

    /// Fetch an owned contact snapshot after validating world ownership and liveness.
    #[inline]
    pub fn contact_data(&self, id: ContactId) -> Result<ContactData> {
        crate::core::callback_state::check_not_in_callback()?;
        self.contact_data_impl(id)
    }

    #[inline]
    fn contact_data_impl(&self, id: ContactId) -> Result<ContactData> {
        let raw = contact_data_raw_impl(self.core(), id)?;
        self.core().with_output_identity_resolver(|resolver| {
            ContactData::from_raw_in(resolver, self.core().contact_epoch(), raw)
        })
    }
}

#[cfg(test)]
mod tests {
    use boxdd_sys::ffi;

    use crate::{BodyId, BodyType, ContactId, Error, ShapeDef, World, shapes};

    #[cfg(feature = "double-precision")]
    const CONTACT_ORIGIN: crate::WorldScalar = 10_000_000.0;
    #[cfg(not(feature = "double-precision"))]
    const CONTACT_ORIGIN: crate::WorldScalar = 0.0;

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
        let body_a = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .position([CONTACT_ORIGIN - 1.0, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let body_b = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .position([CONTACT_ORIGIN + 1.0, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let shape_def = ShapeDef::builder()
            .density(1.0)
            .enable_contact_events(true)
            .build()
            .unwrap();
        let polygon = shapes::box_polygon(0.5_f32, 0.5).unwrap();
        world
            .body(body_a)
            .unwrap()
            .create_polygon(&shape_def, &polygon)
            .unwrap();
        world
            .body(body_b)
            .unwrap()
            .create_polygon(&shape_def, &polygon)
            .unwrap();
        world
            .body(body_a)
            .unwrap()
            .set_linear_velocity([2.0_f32, 0.0])
            .unwrap();
        world
            .body(body_b)
            .unwrap()
            .set_linear_velocity([-2.0_f32, 0.0])
            .unwrap();

        for _ in 0..180 {
            let completed = world.step(1.0 / 60.0, 4).unwrap();
            let contact = completed
                .contact_events()
                .unwrap()
                .begin()
                .first()
                .map(|event| event.contact_id);
            if let Some(contact) = contact {
                return (body_a, contact);
            }
        }
        panic!("expected a live contact id from a contact-begin event");
    }

    #[test]
    fn runtime_contact_manifold_reconstructs_world_points_from_body_centers() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_builder()
                    .gravity([0.0_f32, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let (_, contact) = create_live_contact(&mut world);
        let data = world.contact_data(contact).unwrap();
        assert!(!data.manifold.is_empty());

        let body_a = world.shape(data.shape_id_a).unwrap().body_id().unwrap();
        let body_b = world.shape(data.shape_id_b).unwrap().body_id().unwrap();
        let center_a = world.body(body_a).unwrap().world_center_of_mass().unwrap();
        let center_b = world.body(body_b).unwrap().world_center_of_mass().unwrap();

        for point in data.manifold.points() {
            let world_a = point.world_point_a(center_a);
            let world_b = point.world_point_b(center_b);
            let delta = world_b.checked_relative_to(world_a).unwrap();
            let contact_origin = crate::Position::new(CONTACT_ORIGIN, 0.0);
            let local_a = world_a.checked_relative_to(contact_origin).unwrap();
            let local_b = world_b.checked_relative_to(contact_origin).unwrap();

            assert!(local_a.x.abs() <= 0.01 && local_b.x.abs() <= 0.01);
            assert!(local_a.y.abs() <= 0.51 && local_b.y.abs() <= 0.51);
            assert!(delta.x.abs() <= 0.01 && delta.y.abs() <= 0.01);
        }
    }

    #[test]
    fn contact_helpers_return_in_callback() {
        let world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let contact = invalid_contact_id(&world);
        let _guard = crate::core::callback_state::CallbackGuard::enter();

        assert_eq!(
            world.contact_is_valid(contact).unwrap_err(),
            Error::InCallback
        );
        assert_eq!(world.contact_data(contact).unwrap_err(), Error::InCallback);
    }

    #[test]
    fn contact_ids_expire_at_the_next_valid_step() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_builder()
                    .gravity([0.0_f32, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let (body, contact) = create_live_contact(&mut world);

        assert!(world.contact_is_valid(contact).unwrap());
        assert!(
            world
                .body(body)
                .unwrap()
                .contact_data()
                .unwrap()
                .iter()
                .any(|data| data.contact_id == contact)
        );

        drop(world.step(1.0 / 60.0, 4).unwrap());

        assert!(!world.contact_is_valid(contact).unwrap());
        assert_eq!(
            world.contact_data(contact).unwrap_err(),
            Error::InvalidContactId
        );
        let current_contact = world
            .body(body)
            .unwrap()
            .contact_data()
            .unwrap()
            .into_iter()
            .next()
            .expect("the active contact should receive a fresh id after the next step")
            .contact_id;
        assert_ne!(current_contact, contact);
        assert!(world.contact_is_valid(current_contact).unwrap());
    }
}
