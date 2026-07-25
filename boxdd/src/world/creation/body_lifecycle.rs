use super::*;

fn finish_body_creation(
    world: &World,
    raw: ffi::b2BodyId,
    access: crate::core::world_core::WorldAccess,
) -> crate::error::ApiResult<BodyId> {
    world.core.finish_created_body_with_access(raw, access)
}

pub(crate) fn try_create_body_id_with_access(
    world: &mut World,
    def: BodyDef,
    access: crate::core::world_core::WorldAccess,
) -> crate::error::ApiResult<BodyId> {
    world.core.check_access(access)?;
    let raw = def.into_raw_guard();
    let raw_id = unsafe { ffi::b2CreateBody(world.raw(), raw.as_raw()) };
    finish_body_creation(world, raw_id, access)
}

fn try_create_body_id_impl(world: &mut World, def: BodyDef) -> crate::error::ApiResult<BodyId> {
    try_create_body_id_with_access(world, def, crate::core::world_core::WorldAccess::Idle)
}

fn create_body_id_impl(world: &mut World, def: BodyDef) -> BodyId {
    try_create_body_id_impl(world, def)
        .expect("world unavailable or Box2D returned an invalid BodyId")
}

impl World {
    /// Create a body owned by this world.
    pub fn create_body<'w>(&'w mut self, def: BodyDef) -> Body<'w> {
        crate::core::callback_state::assert_not_in_callback();
        crate::body::assert_body_def_valid(&def);
        let id = create_body_id_impl(self, def);
        Body::new(self.core_rc(), id)
    }

    pub fn try_create_body<'w>(&'w mut self, def: BodyDef) -> crate::error::ApiResult<Body<'w>> {
        crate::core::callback_state::check_not_in_callback()?;
        crate::body::check_body_def_valid(&def)?;
        let id = try_create_body_id_impl(self, def)?;
        Ok(Body::new(self.core_rc(), id))
    }

    /// Create a RAII-owned body. Dropping the returned handle destroys the body.
    pub fn create_body_owned(&mut self, def: BodyDef) -> crate::body::OwnedBody {
        crate::core::callback_state::assert_not_in_callback();
        crate::body::assert_body_def_valid(&def);
        let id = create_body_id_impl(self, def);
        crate::body::OwnedBody::new(self.core_rc(), id)
    }

    pub fn try_create_body_owned(
        &mut self,
        def: BodyDef,
    ) -> crate::error::ApiResult<crate::body::OwnedBody> {
        crate::core::callback_state::check_not_in_callback()?;
        crate::body::check_body_def_valid(&def)?;
        let id = try_create_body_id_impl(self, def)?;
        Ok(crate::body::OwnedBody::new(self.core_rc(), id))
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
        try_create_body_id_impl(self, def)
    }

    /// Destroy a body by id.
    pub fn destroy_body_id(&mut self, id: BodyId) {
        crate::core::callback_state::assert_not_in_callback();
        self.core
            .destroy_body_now(id)
            .expect("invalid or foreign BodyId");
    }

    pub fn try_destroy_body_id(&mut self, id: BodyId) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core.destroy_body_now(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_creation_registers_identity_before_returning() {
        let mut world = World::new(WorldDef::default()).unwrap();
        let id = world.try_create_body_id(BodyDef::default()).unwrap();

        assert_eq!(world.core.check_body(id), Ok(()));

        assert_eq!(
            world.core.finish_created_body_with_access(
                id.into_raw(),
                crate::core::world_core::WorldAccess::Idle,
            ),
            Err(crate::error::ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(
            world.core.check_available(),
            Err(crate::error::ApiError::WorldPoisoned)
        );
    }
}
