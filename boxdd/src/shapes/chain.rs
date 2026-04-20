use std::marker::PhantomData;

use crate::body::Body;
use crate::error::{ApiError, ApiResult};
use crate::shapes::SurfaceMaterial;
use crate::types::{ChainId, ShapeId};
use crate::world::World;
use boxdd_sys::ffi;
use std::rc::Rc;
use std::sync::Arc;

/// A scoped chain handle tied to a mutable borrow of the world.
pub struct Chain<'w> {
    pub(crate) id: ChainId,
    #[allow(dead_code)]
    pub(crate) core: Arc<crate::core::world_core::WorldCore>,
    _world: PhantomData<&'w World>,
}

/// A RAII-owned chain that is destroyed on drop.
pub struct OwnedChain {
    id: ChainId,
    core: Arc<crate::core::world_core::WorldCore>,
    destroy_on_drop: bool,
    _not_send: PhantomData<Rc<()>>,
}

fn chain_segments_into_impl(id: ChainId, out: &mut Vec<ShapeId>) {
    let count = unsafe { ffi::b2Chain_GetSegmentCount(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, count, |ptr, count| {
            ffi::b2Chain_GetSegments(id, ptr, count)
        });
    }
}

fn chain_segments_impl(id: ChainId) -> Vec<ShapeId> {
    let count = unsafe { ffi::b2Chain_GetSegmentCount(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::read_from_ffi(count, |ptr, count| {
            ffi::b2Chain_GetSegments(id, ptr, count)
        })
    }
}

impl OwnedChain {
    pub(crate) fn new(core: Arc<crate::core::world_core::WorldCore>, id: ChainId) -> Self {
        core.owned_chains
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            id,
            core,
            destroy_on_drop: true,
            _not_send: PhantomData,
        }
    }

    pub fn id(&self) -> ChainId {
        self.id
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        self.assert_valid();
        unsafe { ffi::b2Chain_GetWorld(self.id) }
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Chain_GetWorld(self.id) })
    }

    pub fn is_valid(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2Chain_IsValid(self.id) }
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(unsafe { ffi::b2Chain_IsValid(self.id) })
    }

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_chain_valid(self.id);
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_chain_valid(self.id)
    }

    /// Borrow the raw id for ID-style APIs.
    pub fn as_id(&self) -> ChainId {
        self.id
    }

    pub fn segment_count(&self) -> i32 {
        self.assert_valid();
        unsafe { ffi::b2Chain_GetSegmentCount(self.id) }
    }

    pub fn try_segment_count(&self) -> ApiResult<i32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Chain_GetSegmentCount(self.id) })
    }

    pub fn surface_material_count(&self) -> i32 {
        self.assert_valid();
        unsafe { ffi::b2Chain_GetSurfaceMaterialCount(self.id) }
    }
    pub fn try_surface_material_count(&self) -> ApiResult<i32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Chain_GetSurfaceMaterialCount(self.id) })
    }
    pub fn segments(&self) -> Vec<ShapeId> {
        self.assert_valid();
        chain_segments_impl(self.id)
    }

    pub fn segments_into(&self, out: &mut Vec<ShapeId>) {
        self.assert_valid();
        chain_segments_into_impl(self.id, out);
    }

    pub fn try_segments(&self) -> ApiResult<Vec<ShapeId>> {
        self.check_valid()?;
        Ok(chain_segments_impl(self.id))
    }

    pub fn try_segments_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        self.check_valid()?;
        chain_segments_into_impl(self.id, out);
        Ok(())
    }
    pub fn set_surface_material(&mut self, index: i32, material: &SurfaceMaterial) {
        self.assert_valid();
        unsafe { ffi::b2Chain_SetSurfaceMaterial(self.id, &material.0, index) }
    }
    pub fn try_set_surface_material(
        &mut self,
        index: i32,
        material: &SurfaceMaterial,
    ) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Chain_SetSurfaceMaterial(self.id, &material.0, index) }
        Ok(())
    }
    pub fn surface_material(&self, index: i32) -> SurfaceMaterial {
        self.assert_valid();
        SurfaceMaterial(unsafe { ffi::b2Chain_GetSurfaceMaterial(self.id, index) })
    }

    pub fn try_surface_material(&self, index: i32) -> ApiResult<SurfaceMaterial> {
        self.check_valid()?;
        Ok(SurfaceMaterial(unsafe {
            ffi::b2Chain_GetSurfaceMaterial(self.id, index)
        }))
    }

    pub fn into_id(mut self) -> ChainId {
        self.destroy_on_drop = false;
        self.id
    }

    pub fn destroy(mut self) {
        if self.destroy_on_drop && unsafe { ffi::b2Chain_IsValid(self.id) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Chain(self.id));
            } else {
                unsafe { ffi::b2DestroyChain(self.id) }
                #[cfg(feature = "serialize")]
                self.core.remove_chain(self.id);
            }
        }
        self.destroy_on_drop = false;
    }
}

impl Drop for OwnedChain {
    fn drop(&mut self) {
        let _ = self.core.id;
        let prev = self
            .core
            .owned_chains
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        debug_assert!(prev > 0, "owned chain counter underflow");
        if self.destroy_on_drop && unsafe { ffi::b2Chain_IsValid(self.id) } {
            if crate::core::callback_state::in_callback() || self.core.events_buffers_are_borrowed()
            {
                self.core
                    .defer_destroy(crate::core::world_core::DeferredDestroy::Chain(self.id));
            } else {
                unsafe { ffi::b2DestroyChain(self.id) }
                #[cfg(feature = "serialize")]
                self.core.remove_chain(self.id);
            }
        }
    }
}

impl<'w> Chain<'w> {
    pub(crate) fn new(core: Arc<crate::core::world_core::WorldCore>, id: ChainId) -> Self {
        Self {
            id,
            core,
            _world: PhantomData,
        }
    }

    #[inline]
    fn assert_valid(&self) {
        crate::core::debug_checks::assert_chain_valid(self.id);
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::debug_checks::check_chain_valid(self.id)
    }

    pub fn id(&self) -> ChainId {
        self.id
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        self.assert_valid();
        unsafe { ffi::b2Chain_GetWorld(self.id) }
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Chain_GetWorld(self.id) })
    }

    pub fn is_valid(&self) -> bool {
        crate::core::callback_state::assert_not_in_callback();
        unsafe { ffi::b2Chain_IsValid(self.id) }
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        Ok(unsafe { ffi::b2Chain_IsValid(self.id) })
    }
    pub fn segment_count(&self) -> i32 {
        self.assert_valid();
        unsafe { ffi::b2Chain_GetSegmentCount(self.id) }
    }

    pub fn try_segment_count(&self) -> ApiResult<i32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Chain_GetSegmentCount(self.id) })
    }
    pub fn surface_material_count(&self) -> i32 {
        self.assert_valid();
        unsafe { ffi::b2Chain_GetSurfaceMaterialCount(self.id) }
    }
    pub fn try_surface_material_count(&self) -> ApiResult<i32> {
        self.check_valid()?;
        Ok(unsafe { ffi::b2Chain_GetSurfaceMaterialCount(self.id) })
    }

    /// Collect all segment shape ids for this chain.
    pub fn segments(&self) -> Vec<ShapeId> {
        self.assert_valid();
        chain_segments_impl(self.id)
    }

    pub fn segments_into(&self, out: &mut Vec<ShapeId>) {
        self.assert_valid();
        chain_segments_into_impl(self.id, out);
    }

    pub fn try_segments(&self) -> ApiResult<Vec<ShapeId>> {
        self.check_valid()?;
        Ok(chain_segments_impl(self.id))
    }

    pub fn try_segments_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        self.check_valid()?;
        chain_segments_into_impl(self.id, out);
        Ok(())
    }

    pub fn set_surface_material(&mut self, index: i32, material: &SurfaceMaterial) {
        self.assert_valid();
        unsafe { ffi::b2Chain_SetSurfaceMaterial(self.id, &material.0, index) }
    }

    pub fn try_set_surface_material(
        &mut self,
        index: i32,
        material: &SurfaceMaterial,
    ) -> ApiResult<()> {
        self.check_valid()?;
        unsafe { ffi::b2Chain_SetSurfaceMaterial(self.id, &material.0, index) }
        Ok(())
    }

    pub fn surface_material(&self, index: i32) -> SurfaceMaterial {
        self.assert_valid();
        SurfaceMaterial(unsafe { ffi::b2Chain_GetSurfaceMaterial(self.id, index) })
    }

    pub fn try_surface_material(&self, index: i32) -> ApiResult<SurfaceMaterial> {
        self.check_valid()?;
        Ok(SurfaceMaterial(unsafe {
            ffi::b2Chain_GetSurfaceMaterial(self.id, index)
        }))
    }

    /// Destroy this chain immediately.
    pub fn destroy(self) {
        crate::core::callback_state::assert_not_in_callback();
        if unsafe { ffi::b2Chain_IsValid(self.id) } {
            unsafe { ffi::b2DestroyChain(self.id) }
            #[cfg(feature = "serialize")]
            self.core.remove_chain(self.id);
        }
    }

    pub fn try_destroy(self) -> ApiResult<()> {
        self.check_valid()?;
        if unsafe { ffi::b2Chain_IsValid(self.id) } {
            unsafe { ffi::b2DestroyChain(self.id) }
            #[cfg(feature = "serialize")]
            self.core.remove_chain(self.id);
        }
        Ok(())
    }
}

/// Chain shape definition. Holds optional owned data for points and materials.
#[derive(Debug)]
pub struct ChainDef {
    pub(crate) def: ffi::b2ChainDef,
    points: Vec<ffi::b2Vec2>,
    materials: Vec<ffi::b2SurfaceMaterial>,
}

impl Clone for ChainDef {
    fn clone(&self) -> Self {
        let mut def = self.def;
        let points = self.points.clone();
        let materials = self.materials.clone();

        if points.is_empty() {
            def.points = core::ptr::null();
            def.count = 0;
        } else {
            def.points = points.as_ptr();
            def.count = points.len() as i32;
        }

        if materials.is_empty() {
            // Keep default material pointer/count stable.
            let default_def = unsafe { ffi::b2DefaultChainDef() };
            def.materials = default_def.materials;
            def.materialCount = default_def.materialCount;
        } else {
            def.materials = materials.as_ptr();
            def.materialCount = materials.len() as i32;
        }

        Self {
            def,
            points,
            materials,
        }
    }
}

impl Default for ChainDef {
    fn default() -> Self {
        Self {
            def: unsafe { ffi::b2DefaultChainDef() },
            points: Vec::new(),
            materials: Vec::new(),
        }
    }
}

impl ChainDef {
    pub fn builder() -> ChainDefBuilder {
        ChainDefBuilder {
            inner: Self::default(),
        }
    }
    #[cfg(feature = "serialize")]
    pub fn points_vec(&self) -> Vec<ffi::b2Vec2> {
        self.points.clone()
    }
    #[cfg(feature = "serialize")]
    pub fn materials_vec(&self) -> Vec<ffi::b2SurfaceMaterial> {
        self.materials.clone()
    }
}

#[derive(Clone, Debug)]
pub struct ChainDefBuilder {
    inner: ChainDef,
}

impl ChainDefBuilder {
    pub fn points<I, P>(mut self, points: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<crate::types::Vec2>,
    {
        self.inner.points = points
            .into_iter()
            .map(|p| ffi::b2Vec2::from(p.into()))
            .collect();
        self.inner.def.points = if self.inner.points.is_empty() {
            core::ptr::null()
        } else {
            self.inner.points.as_ptr()
        };
        self.inner.def.count = self.inner.points.len() as i32;
        self
    }
    pub fn is_loop(mut self, v: bool) -> Self {
        self.inner.def.isLoop = v;
        self
    }
    pub fn filter(mut self, f: crate::filter::Filter) -> Self {
        self.inner.def.filter = f.into_raw();
        self
    }
    pub fn filter_raw(mut self, f: ffi::b2Filter) -> Self {
        self.inner.def.filter = f;
        self
    }
    pub fn enable_sensor_events(mut self, v: bool) -> Self {
        self.inner.def.enableSensorEvents = v;
        self
    }
    pub fn single_material(mut self, m: &SurfaceMaterial) -> Self {
        self.inner.materials.clear();
        self.inner.materials.push(m.0);
        self.inner.def.materials = self.inner.materials.as_ptr();
        self.inner.def.materialCount = 1;
        self
    }
    pub fn materials(mut self, mats: &[SurfaceMaterial]) -> Self {
        if mats.is_empty() {
            self.inner.materials.clear();
            // Reset to the upstream default material (static storage on the C side).
            let default_def = unsafe { ffi::b2DefaultChainDef() };
            self.inner.def.materials = default_def.materials;
            self.inner.def.materialCount = default_def.materialCount;
        } else {
            self.inner.materials = mats.iter().map(|m| m.0).collect();
            self.inner.def.materials = self.inner.materials.as_ptr();
            self.inner.def.materialCount = self.inner.materials.len() as i32;
        }
        self
    }
    #[must_use]
    pub fn build(mut self) -> ChainDef {
        if self.inner.def.count == 0 {
            // ensure sane default
            self.inner.points.clear();
            self.inner.def.points = core::ptr::null();
        }
        self.inner
    }
}

#[inline]
#[track_caller]
pub(crate) fn assert_chain_def_valid(def: &ChainDef) {
    let count = def.def.count;
    assert!(
        count >= 4,
        "invalid ChainDef: expected at least 4 points (including ghosts), got {count}"
    );
    assert!(
        !def.def.points.is_null(),
        "invalid ChainDef: points pointer is null"
    );
    let mc = def.def.materialCount;
    assert!(
        mc == 1 || mc == count,
        "invalid ChainDef: materialCount must be 1 or equal to count (materialCount={mc}, count={count})"
    );
    assert!(
        !def.def.materials.is_null(),
        "invalid ChainDef: materials pointer is null"
    );
}

pub(crate) fn check_chain_def_valid(def: &ChainDef) -> ApiResult<()> {
    let count = def.def.count;
    if count < 4 {
        return Err(ApiError::InvalidChainDef);
    }
    if def.def.points.is_null() {
        return Err(ApiError::InvalidChainDef);
    }
    let mc = def.def.materialCount;
    if mc != 1 && mc != count {
        return Err(ApiError::InvalidChainDef);
    }
    if def.def.materials.is_null() {
        return Err(ApiError::InvalidChainDef);
    }
    Ok(())
}

impl ChainDef {
    pub fn validate(&self) -> ApiResult<()> {
        check_chain_def_valid(self)
    }
}

impl<'w> Body<'w> {
    /// Create a chain shape attached to this body. Points/materials are cloned internally by Box2D.
    pub fn create_chain(&mut self, def: &ChainDef) -> Chain<'w> {
        crate::core::debug_checks::assert_body_valid(self.id);
        assert_chain_def_valid(def);
        let id = unsafe { ffi::b2CreateChain(self.id, &def.def) };
        #[cfg(feature = "serialize")]
        {
            let meta = crate::core::serialize_registry::ChainCreateMeta::from_def(self.id, def);
            self.core.record_chain(id, meta);
        }
        Chain::new(Arc::clone(&self.core), id)
    }
}
