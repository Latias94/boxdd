use super::*;

fn borrow_world_scoped_handle<T, Id: Copy>(
    world: &mut World,
    id: Id,
    invalid: crate::error::ApiError,
    check: impl FnOnce(&WorldCore, Id) -> crate::error::ApiResult<()>,
    wrap: impl FnOnce(Rc<WorldCore>, Id) -> T,
) -> Option<T> {
    crate::core::callback_state::assert_not_in_callback();
    let core = world.core_rc();
    match check(&core, id) {
        Ok(()) => Some(wrap(core, id)),
        Err(error) if error == invalid => None,
        Err(error) => panic!("cannot borrow handle from this world: {error}"),
    }
}

fn try_borrow_world_scoped_handle<T, Id: Copy>(
    world: &mut World,
    id: Id,
    check: impl FnOnce(&WorldCore, Id) -> crate::error::ApiResult<()>,
    wrap: impl FnOnce(Rc<WorldCore>, Id) -> T,
) -> crate::error::ApiResult<T> {
    crate::core::callback_state::check_not_in_callback()?;
    let core = world.core_rc();
    check(&core, id)?;
    Ok(wrap(core, id))
}

impl World {
    /// Borrow a scoped body handle by id (returns `None` if the id is invalid).
    pub fn body<'w>(&'w mut self, id: BodyId) -> Option<Body<'w>> {
        borrow_world_scoped_handle(
            self,
            id,
            crate::error::ApiError::InvalidBodyId,
            WorldCore::check_body,
            Body::new,
        )
    }

    pub fn try_body<'w>(&'w mut self, id: BodyId) -> crate::error::ApiResult<Body<'w>> {
        try_borrow_world_scoped_handle(self, id, WorldCore::check_body, Body::new)
    }

    /// Borrow a scoped joint handle by id (returns `None` if the id is invalid).
    pub fn joint<'w>(&'w mut self, id: JointId) -> Option<crate::joints::Joint<'w>> {
        borrow_world_scoped_handle(
            self,
            id,
            crate::error::ApiError::InvalidJointId,
            WorldCore::check_joint,
            crate::joints::Joint::new,
        )
    }

    pub fn try_joint<'w>(
        &'w mut self,
        id: JointId,
    ) -> crate::error::ApiResult<crate::joints::Joint<'w>> {
        try_borrow_world_scoped_handle(self, id, WorldCore::check_joint, crate::joints::Joint::new)
    }

    /// Borrow a scoped shape handle by id (returns `None` if the id is invalid).
    pub fn shape<'w>(&'w mut self, id: ShapeId) -> Option<crate::shapes::Shape<'w>> {
        borrow_world_scoped_handle(
            self,
            id,
            crate::error::ApiError::InvalidShapeId,
            WorldCore::check_shape,
            crate::shapes::Shape::new,
        )
    }

    pub fn try_shape<'w>(
        &'w mut self,
        id: ShapeId,
    ) -> crate::error::ApiResult<crate::shapes::Shape<'w>> {
        try_borrow_world_scoped_handle(self, id, WorldCore::check_shape, crate::shapes::Shape::new)
    }

    /// Borrow a scoped chain handle by id (returns `None` if the id is invalid).
    pub fn chain<'w>(&'w mut self, id: ChainId) -> Option<crate::shapes::chain::Chain<'w>> {
        borrow_world_scoped_handle(
            self,
            id,
            crate::error::ApiError::InvalidChainId,
            WorldCore::check_chain,
            crate::shapes::chain::Chain::new,
        )
    }

    pub fn try_chain<'w>(
        &'w mut self,
        id: ChainId,
    ) -> crate::error::ApiResult<crate::shapes::chain::Chain<'w>> {
        try_borrow_world_scoped_handle(
            self,
            id,
            WorldCore::check_chain,
            crate::shapes::chain::Chain::new,
        )
    }
}
