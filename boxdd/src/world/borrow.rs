use super::*;

fn borrow_world_scoped_handle<'w, T, Id: Copy>(
    world: &'w mut World,
    id: Id,
    is_valid: impl FnOnce(Id) -> bool,
    wrap: impl FnOnce(Arc<WorldCore>, Id) -> T,
) -> Option<T> {
    crate::core::callback_state::assert_not_in_callback();
    if is_valid(id) {
        Some(wrap(world.core_arc(), id))
    } else {
        None
    }
}

fn try_borrow_world_scoped_handle<'w, T, Id: Copy>(
    world: &'w mut World,
    id: Id,
    invalid: crate::error::ApiError,
    is_valid: impl FnOnce(Id) -> bool,
    wrap: impl FnOnce(Arc<WorldCore>, Id) -> T,
) -> crate::error::ApiResult<T> {
    crate::core::callback_state::check_not_in_callback()?;
    if is_valid(id) {
        Ok(wrap(world.core_arc(), id))
    } else {
        Err(invalid)
    }
}

impl World {
    /// Borrow a scoped body handle by id (returns `None` if the id is invalid).
    pub fn body<'w>(&'w mut self, id: BodyId) -> Option<Body<'w>> {
        borrow_world_scoped_handle(
            self,
            id,
            |id| unsafe { ffi::b2Body_IsValid(raw_body_id(id)) },
            Body::new,
        )
    }

    pub fn try_body<'w>(&'w mut self, id: BodyId) -> crate::error::ApiResult<Body<'w>> {
        try_borrow_world_scoped_handle(
            self,
            id,
            crate::error::ApiError::InvalidBodyId,
            |id| unsafe { ffi::b2Body_IsValid(raw_body_id(id)) },
            Body::new,
        )
    }

    /// Borrow a scoped joint handle by id (returns `None` if the id is invalid).
    pub fn joint<'w>(&'w mut self, id: JointId) -> Option<crate::joints::Joint<'w>> {
        borrow_world_scoped_handle(
            self,
            id,
            |id| unsafe { ffi::b2Joint_IsValid(raw_joint_id(id)) },
            crate::joints::Joint::new,
        )
    }

    pub fn try_joint<'w>(
        &'w mut self,
        id: JointId,
    ) -> crate::error::ApiResult<crate::joints::Joint<'w>> {
        try_borrow_world_scoped_handle(
            self,
            id,
            crate::error::ApiError::InvalidJointId,
            |id| unsafe { ffi::b2Joint_IsValid(raw_joint_id(id)) },
            crate::joints::Joint::new,
        )
    }

    /// Borrow a scoped shape handle by id (returns `None` if the id is invalid).
    pub fn shape<'w>(&'w mut self, id: ShapeId) -> Option<crate::shapes::Shape<'w>> {
        borrow_world_scoped_handle(
            self,
            id,
            |id| unsafe { ffi::b2Shape_IsValid(raw_shape_id(id)) },
            crate::shapes::Shape::new,
        )
    }

    pub fn try_shape<'w>(
        &'w mut self,
        id: ShapeId,
    ) -> crate::error::ApiResult<crate::shapes::Shape<'w>> {
        try_borrow_world_scoped_handle(
            self,
            id,
            crate::error::ApiError::InvalidShapeId,
            |id| unsafe { ffi::b2Shape_IsValid(raw_shape_id(id)) },
            crate::shapes::Shape::new,
        )
    }

    /// Borrow a scoped chain handle by id (returns `None` if the id is invalid).
    pub fn chain<'w>(&'w mut self, id: ChainId) -> Option<crate::shapes::chain::Chain<'w>> {
        borrow_world_scoped_handle(
            self,
            id,
            |id| unsafe { ffi::b2Chain_IsValid(raw_chain_id(id)) },
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
            crate::error::ApiError::InvalidChainId,
            |id| unsafe { ffi::b2Chain_IsValid(raw_chain_id(id)) },
            crate::shapes::chain::Chain::new,
        )
    }
}
