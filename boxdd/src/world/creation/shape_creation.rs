use super::*;

fn wrap_world_owned_handle<T, Id>(
    core: &Arc<WorldCore>,
    id: Id,
    wrap: impl FnOnce(Arc<WorldCore>, Id) -> T,
) -> T {
    wrap(Arc::clone(core), id)
}

fn try_wrap_world_owned_handle<T, Id, E>(
    core: &Arc<WorldCore>,
    id: Result<Id, E>,
    wrap: impl FnOnce(Arc<WorldCore>, Id) -> T,
) -> Result<T, E> {
    id.map(|id| wrap(Arc::clone(core), id))
}

impl World {
    // ID-based shape helpers (world-anchored)
    pub fn create_circle_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        circle: &crate::shapes::Circle,
    ) -> ShapeId {
        crate::shapes::create_circle_shape_for_body_impl(self.core.as_ref(), body, def, circle)
    }

    pub fn create_circle_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        circle: &crate::shapes::Circle,
    ) -> crate::shapes::OwnedShape {
        wrap_world_owned_handle(
            &self.core,
            crate::shapes::create_circle_shape_for_body_impl(self.core.as_ref(), body, def, circle),
            crate::shapes::OwnedShape::new,
        )
    }

    pub fn try_create_circle_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        circle: &crate::shapes::Circle,
    ) -> crate::error::ApiResult<ShapeId> {
        crate::shapes::try_create_circle_shape_for_body_impl(self.core.as_ref(), body, def, circle)
    }

    pub fn try_create_circle_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        circle: &crate::shapes::Circle,
    ) -> crate::error::ApiResult<crate::shapes::OwnedShape> {
        try_wrap_world_owned_handle(
            &self.core,
            crate::shapes::try_create_circle_shape_for_body_impl(
                self.core.as_ref(),
                body,
                def,
                circle,
            ),
            crate::shapes::OwnedShape::new,
        )
    }

    pub fn create_segment_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        segment: &crate::shapes::Segment,
    ) -> ShapeId {
        crate::shapes::create_segment_shape_for_body_impl(self.core.as_ref(), body, def, segment)
    }

    pub fn create_segment_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        segment: &crate::shapes::Segment,
    ) -> crate::shapes::OwnedShape {
        wrap_world_owned_handle(
            &self.core,
            crate::shapes::create_segment_shape_for_body_impl(
                self.core.as_ref(),
                body,
                def,
                segment,
            ),
            crate::shapes::OwnedShape::new,
        )
    }

    pub fn try_create_segment_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        segment: &crate::shapes::Segment,
    ) -> crate::error::ApiResult<ShapeId> {
        crate::shapes::try_create_segment_shape_for_body_impl(
            self.core.as_ref(),
            body,
            def,
            segment,
        )
    }

    pub fn try_create_segment_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        segment: &crate::shapes::Segment,
    ) -> crate::error::ApiResult<crate::shapes::OwnedShape> {
        try_wrap_world_owned_handle(
            &self.core,
            crate::shapes::try_create_segment_shape_for_body_impl(
                self.core.as_ref(),
                body,
                def,
                segment,
            ),
            crate::shapes::OwnedShape::new,
        )
    }

    pub fn create_capsule_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        capsule: &crate::shapes::Capsule,
    ) -> ShapeId {
        crate::shapes::create_capsule_shape_for_body_impl(self.core.as_ref(), body, def, capsule)
    }

    pub fn create_capsule_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        capsule: &crate::shapes::Capsule,
    ) -> crate::shapes::OwnedShape {
        wrap_world_owned_handle(
            &self.core,
            crate::shapes::create_capsule_shape_for_body_impl(
                self.core.as_ref(),
                body,
                def,
                capsule,
            ),
            crate::shapes::OwnedShape::new,
        )
    }

    pub fn try_create_capsule_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        capsule: &crate::shapes::Capsule,
    ) -> crate::error::ApiResult<ShapeId> {
        crate::shapes::try_create_capsule_shape_for_body_impl(
            self.core.as_ref(),
            body,
            def,
            capsule,
        )
    }

    pub fn try_create_capsule_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        capsule: &crate::shapes::Capsule,
    ) -> crate::error::ApiResult<crate::shapes::OwnedShape> {
        try_wrap_world_owned_handle(
            &self.core,
            crate::shapes::try_create_capsule_shape_for_body_impl(
                self.core.as_ref(),
                body,
                def,
                capsule,
            ),
            crate::shapes::OwnedShape::new,
        )
    }

    pub fn create_polygon_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        polygon: &crate::shapes::Polygon,
    ) -> ShapeId {
        crate::shapes::create_polygon_shape_for_body_impl(self.core.as_ref(), body, def, polygon)
    }

    pub fn create_polygon_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        polygon: &crate::shapes::Polygon,
    ) -> crate::shapes::OwnedShape {
        wrap_world_owned_handle(
            &self.core,
            crate::shapes::create_polygon_shape_for_body_impl(
                self.core.as_ref(),
                body,
                def,
                polygon,
            ),
            crate::shapes::OwnedShape::new,
        )
    }

    pub fn try_create_polygon_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        polygon: &crate::shapes::Polygon,
    ) -> crate::error::ApiResult<ShapeId> {
        crate::shapes::try_create_polygon_shape_for_body_impl(
            self.core.as_ref(),
            body,
            def,
            polygon,
        )
    }

    pub fn try_create_polygon_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        polygon: &crate::shapes::Polygon,
    ) -> crate::error::ApiResult<crate::shapes::OwnedShape> {
        try_wrap_world_owned_handle(
            &self.core,
            crate::shapes::try_create_polygon_shape_for_body_impl(
                self.core.as_ref(),
                body,
                def,
                polygon,
            ),
            crate::shapes::OwnedShape::new,
        )
    }

    pub fn destroy_shape_id(&mut self, shape: ShapeId, update_body_mass: bool) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Shape_IsValid(raw_shape_id(shape)) } {
            unsafe { ffi::b2DestroyShape(raw_shape_id(shape), update_body_mass) };
            let _ = self.core.clear_shape_user_data(shape);
        }
        #[cfg(feature = "serialize")]
        {
            self.core.remove_shape_flags(shape);
        }
    }

    // Chain API (ID-style)
    pub fn create_chain_for_id(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> ChainId {
        crate::shapes::chain::create_chain_for_body_impl(self.core.as_ref(), body, def)
    }

    pub fn try_create_chain_for_id(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> crate::error::ApiResult<ChainId> {
        crate::shapes::chain::try_create_chain_for_body_impl(self.core.as_ref(), body, def)
    }

    pub fn create_chain_for_owned(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> crate::shapes::chain::OwnedChain {
        let core = Arc::clone(&self.core);
        let id = self.create_chain_for_id(body, def);
        wrap_world_owned_handle(&core, id, crate::shapes::chain::OwnedChain::new)
    }

    pub fn try_create_chain_for_owned(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> crate::error::ApiResult<crate::shapes::chain::OwnedChain> {
        let core = Arc::clone(&self.core);
        let id = self.try_create_chain_for_id(body, def);
        try_wrap_world_owned_handle(&core, id, crate::shapes::chain::OwnedChain::new)
    }

    pub fn destroy_chain_id(&mut self, chain: ChainId) {
        crate::core::debug_checks::assert_chain_valid(chain);
        if unsafe { ffi::b2Chain_IsValid(raw_chain_id(chain)) } {
            unsafe { ffi::b2DestroyChain(raw_chain_id(chain)) };
        }
        #[cfg(feature = "serialize")]
        {
            self.core.remove_chain(chain);
        }
    }

    pub fn try_destroy_chain_id(&mut self, chain: ChainId) -> crate::error::ApiResult<()> {
        crate::core::debug_checks::check_chain_valid(chain)?;
        unsafe { ffi::b2DestroyChain(raw_chain_id(chain)) };
        #[cfg(feature = "serialize")]
        {
            self.core.remove_chain(chain);
        }
        Ok(())
    }
}
