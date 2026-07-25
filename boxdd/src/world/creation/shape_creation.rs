use super::*;

fn wrap_world_owned_handle<T, Id>(
    core: &Rc<WorldCore>,
    id: Id,
    wrap: impl FnOnce(Rc<WorldCore>, Id) -> T,
) -> T {
    wrap(Rc::clone(core), id)
}

fn try_wrap_world_owned_handle<T, Id, E>(
    core: &Rc<WorldCore>,
    id: Result<Id, E>,
    wrap: impl FnOnce(Rc<WorldCore>, Id) -> T,
) -> Result<T, E> {
    id.map(|id| wrap(Rc::clone(core), id))
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

    /// Create a chain segment that is owned directly by the body rather than by a `Chain`.
    pub fn create_chain_segment_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        chain_segment: &crate::shapes::ChainSegment,
    ) -> ShapeId {
        crate::shapes::create_chain_segment_shape_for_body_impl(
            self.core.as_ref(),
            body,
            def,
            chain_segment,
        )
    }

    /// Create an RAII-owned chain segment that has no parent `Chain`.
    pub fn create_chain_segment_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        chain_segment: &crate::shapes::ChainSegment,
    ) -> crate::shapes::OwnedShape {
        wrap_world_owned_handle(
            &self.core,
            crate::shapes::create_chain_segment_shape_for_body_impl(
                self.core.as_ref(),
                body,
                def,
                chain_segment,
            ),
            crate::shapes::OwnedShape::new,
        )
    }

    pub fn try_create_chain_segment_shape_for(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        chain_segment: &crate::shapes::ChainSegment,
    ) -> crate::error::ApiResult<ShapeId> {
        crate::shapes::try_create_chain_segment_shape_for_body_impl(
            self.core.as_ref(),
            body,
            def,
            chain_segment,
        )
    }

    pub fn try_create_chain_segment_shape_for_owned(
        &mut self,
        body: BodyId,
        def: &ShapeDef,
        chain_segment: &crate::shapes::ChainSegment,
    ) -> crate::error::ApiResult<crate::shapes::OwnedShape> {
        try_wrap_world_owned_handle(
            &self.core,
            crate::shapes::try_create_chain_segment_shape_for_body_impl(
                self.core.as_ref(),
                body,
                def,
                chain_segment,
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
        self.core
            .destroy_shape_now(shape, update_body_mass)
            .expect("invalid or foreign ShapeId");
    }

    pub fn try_destroy_shape_id(
        &mut self,
        shape: ShapeId,
        update_body_mass: bool,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core.destroy_shape_now(shape, update_body_mass)
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
        let core = Rc::clone(&self.core);
        let id = self.create_chain_for_id(body, def);
        wrap_world_owned_handle(&core, id, crate::shapes::chain::OwnedChain::new)
    }

    pub fn try_create_chain_for_owned(
        &mut self,
        body: BodyId,
        def: &crate::shapes::chain::ChainDef,
    ) -> crate::error::ApiResult<crate::shapes::chain::OwnedChain> {
        let core = Rc::clone(&self.core);
        let id = self.try_create_chain_for_id(body, def);
        try_wrap_world_owned_handle(&core, id, crate::shapes::chain::OwnedChain::new)
    }

    pub fn destroy_chain_id(&mut self, chain: ChainId) {
        crate::core::callback_state::assert_not_in_callback();
        self.core
            .destroy_chain_now(chain)
            .expect("invalid or foreign ChainId");
    }

    pub fn try_destroy_chain_id(&mut self, chain: ChainId) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.core.destroy_chain_now(chain)
    }
}
