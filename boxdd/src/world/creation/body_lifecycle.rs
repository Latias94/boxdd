use super::*;

pub(crate) fn create_body_id(
    owner: &dyn crate::world::OwnerAdapter,
    def: BodyDef,
) -> crate::error::Result<BodyId> {
    let creation = crate::world::OwnerCreation::begin(owner)?;
    if let Err(error) = crate::body::check_body_def_valid(&def) {
        return creation.abort(error);
    }
    if let Err(error) = creation
        .core()
        .check_definition_length_scale("World::create_body", def.length_scale())
    {
        return creation.abort(error);
    }
    let pending = match creation.core().reserve_body_creation() {
        Ok(pending) => pending,
        Err(error) => return creation.abort(error),
    };
    let raw = def.prepare();
    let raw_id = unsafe { ffi::b2CreateBody(creation.core().id, raw.as_raw()) };
    let mut native = match creation.core().claim_created_body(raw_id) {
        Ok(native) => native,
        Err(error) => return creation.abort(error),
    };
    let bound = match creation.core().bind_created_body(pending, raw_id) {
        Ok(bound) => bound,
        Err(error) => return creation.abort(error),
    };
    creation.finish(|| {
        let id = bound.publish();
        native.commit();
        id
    })
}

impl World {
    /// Create a body and return its process-local storage id.
    pub fn create_body(&mut self, def: BodyDef) -> crate::error::Result<BodyId> {
        create_body_id(self, def)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn body_creation_registers_identity_before_returning() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let id = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();

        assert_eq!(world.core.check_body(id), Ok(()));
    }
}
