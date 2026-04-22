use super::*;

fn create_body_id_impl(world: &mut World, def: BodyDef) -> BodyId {
    let raw = def.0;
    let id = BodyId::from_raw(unsafe { ffi::b2CreateBody(world.raw(), &raw) });
    #[cfg(feature = "serialize")]
    {
        world.core.record_body(id);
    }
    id
}

impl World {
    /// Create a body owned by this world.
    pub fn create_body<'w>(&'w mut self, def: BodyDef) -> Body<'w> {
        crate::core::callback_state::assert_not_in_callback();
        crate::body::assert_body_def_valid(&def);
        let id = create_body_id_impl(self, def);
        Body::new(self.core_arc(), id)
    }

    pub fn try_create_body<'w>(&'w mut self, def: BodyDef) -> crate::error::ApiResult<Body<'w>> {
        crate::core::callback_state::check_not_in_callback()?;
        crate::body::check_body_def_valid(&def)?;
        let id = create_body_id_impl(self, def);
        Ok(Body::new(self.core_arc(), id))
    }

    /// Create a RAII-owned body. Dropping the returned handle destroys the body.
    pub fn create_body_owned(&mut self, def: BodyDef) -> crate::body::OwnedBody {
        crate::core::callback_state::assert_not_in_callback();
        crate::body::assert_body_def_valid(&def);
        let id = create_body_id_impl(self, def);
        crate::body::OwnedBody::new(self.core_arc(), id)
    }

    pub fn try_create_body_owned(
        &mut self,
        def: BodyDef,
    ) -> crate::error::ApiResult<crate::body::OwnedBody> {
        crate::core::callback_state::check_not_in_callback()?;
        crate::body::check_body_def_valid(&def)?;
        let id = create_body_id_impl(self, def);
        Ok(crate::body::OwnedBody::new(self.core_arc(), id))
    }

    /// ID-style body creation. Prefer when you want to store/pass ids without borrowing the world.
    pub fn create_body_id(&mut self, def: BodyDef) -> BodyId {
        crate::core::callback_state::assert_not_in_callback();
        crate::body::assert_body_def_valid(&def);
        create_body_id_impl(self, def)
    }

    pub fn try_create_body_id(&mut self, def: BodyDef) -> crate::error::ApiResult<BodyId> {
        crate::core::callback_state::check_not_in_callback()?;
        crate::body::check_body_def_valid(&def)?;
        Ok(create_body_id_impl(self, def))
    }

    /// Destroy a body by id.
    pub fn destroy_body_id(&mut self, id: BodyId) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Body_IsValid(raw_body_id(id)) } {
            #[cfg(feature = "serialize")]
            self.core.cleanup_before_destroy_body(id);
            unsafe { ffi::b2DestroyBody(raw_body_id(id)) };
            let _ = self.core.clear_body_user_data(id);
        }
    }

    pub fn try_destroy_body_id(&mut self, id: BodyId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_body_valid(id)?;
        #[cfg(feature = "serialize")]
        self.core.cleanup_before_destroy_body(id);
        unsafe { ffi::b2DestroyBody(raw_body_id(id)) };
        let _ = self.core.clear_body_user_data(id);
        Ok(())
    }
}
