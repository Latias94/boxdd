use std::marker::PhantomData;

use crate::body::{Body, OwnedBody};
use crate::error::{ApiError, ApiResult};
use crate::shapes::SurfaceMaterial;
use crate::types::{BodyId, ChainId, ShapeId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;
use std::rc::Rc;

const _: () = {
    assert!(core::mem::size_of::<Vec2>() == core::mem::size_of::<ffi::b2Vec2>());
    assert!(core::mem::align_of::<Vec2>() == core::mem::align_of::<ffi::b2Vec2>());
    assert!(
        core::mem::size_of::<SurfaceMaterial>() == core::mem::size_of::<ffi::b2SurfaceMaterial>()
    );
    assert!(
        core::mem::align_of::<SurfaceMaterial>() == core::mem::align_of::<ffi::b2SurfaceMaterial>()
    );
};

/// How a `ChainDef` provides surface materials to Box2D.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ChainDefMaterialLayout<'a> {
    /// Use Box2D's default chain material.
    Default(SurfaceMaterial),
    /// Use one material for the entire chain.
    Single(SurfaceMaterial),
    /// Use one material entry for every stored chain point.
    Multiple(&'a [SurfaceMaterial]),
}

impl<'a> ChainDefMaterialLayout<'a> {
    /// Number of material entries visible to Box2D.
    pub const fn count(&self) -> usize {
        match self {
            Self::Default(_) | Self::Single(_) => 1,
            Self::Multiple(materials) => materials.len(),
        }
    }
}

/// A scoped chain handle tied to a mutable borrow of the world.
pub struct Chain<'w> {
    pub(crate) id: ChainId,
    pub(crate) core: Rc<crate::core::world_core::WorldCore>,
    _world: PhantomData<&'w World>,
}

/// A RAII-owned chain that is destroyed on drop.
pub struct OwnedChain {
    id: ChainId,
    core: Rc<crate::core::world_core::WorldCore>,
    destroy_on_drop: bool,
}

#[inline]
fn raw_chain_id(id: ChainId) -> ffi::b2ChainId {
    id.into_raw()
}

unsafe fn try_fill_segment_output(
    out: &mut Vec<ShapeId>,
    brand: crate::id::IdBrand,
    requested: i32,
    fill: impl FnOnce(*mut ffi::b2ShapeId, i32) -> i32,
) -> ApiResult<()> {
    unsafe {
        crate::core::ffi_vec::try_fill_mapped_from_ffi(out, requested, fill, |raw| {
            brand.try_shape(raw)
        })
    }
}

unsafe fn try_read_segment_output(
    brand: crate::id::IdBrand,
    requested: i32,
    fill: impl FnOnce(*mut ffi::b2ShapeId, i32) -> i32,
) -> ApiResult<Vec<ShapeId>> {
    unsafe {
        crate::core::ffi_vec::try_read_mapped_from_ffi(requested, fill, |raw| brand.try_shape(raw))
    }
}

fn chain_segments_into_in_impl(
    brand: crate::id::IdBrand,
    id: ChainId,
    out: &mut Vec<ShapeId>,
) -> ApiResult<()> {
    let id = raw_chain_id(id);
    let count = unsafe { ffi::b2Chain_GetSegmentCount(id) };
    unsafe {
        try_fill_segment_output(out, brand, count, |ptr, count| {
            ffi::b2Chain_GetSegments(id, ptr, count)
        })
    }
}

fn chain_segments_in_impl(brand: crate::id::IdBrand, id: ChainId) -> ApiResult<Vec<ShapeId>> {
    let id = raw_chain_id(id);
    let count = unsafe { ffi::b2Chain_GetSegmentCount(id) };
    unsafe {
        try_read_segment_output(brand, count, |ptr, count| {
            ffi::b2Chain_GetSegments(id, ptr, count)
        })
    }
}

#[cfg(test)]
mod segment_output_tests {
    use super::*;

    fn test_registry() -> (
        crate::id::IdBrand,
        std::sync::Arc<crate::core::identity_registry::ActiveIdentityRegistry>,
    ) {
        let brand = crate::id::IdBrand::new(
            ffi::b2WorldId {
                index1: 4,
                generation: 7,
            },
            crate::id::WorldToken::allocate().unwrap(),
        )
        .unwrap();
        let registry = crate::core::identity_registry::ActiveIdentityRegistry::new(brand);
        let body = registry
            .register_body(ffi::b2BodyId {
                index1: 1,
                world0: brand.world0(),
                generation: 1,
            })
            .unwrap();
        for index1 in [2, 9] {
            registry
                .register_shape(raw_shape(index1, brand.world0()), body)
                .unwrap();
        }
        (brand, registry)
    }

    fn raw_shape(index1: i32, world0: u16) -> ffi::b2ShapeId {
        ffi::b2ShapeId {
            index1,
            world0,
            generation: 1,
        }
    }

    #[test]
    fn segment_output_rejects_invalid_ids_and_reuses_safe_output_allocation() {
        let (brand, _registry) = test_registry();
        let mut out = Vec::<ShapeId>::with_capacity(4);
        out.push(brand.try_shape(raw_shape(9, brand.world0())).unwrap());
        let expected_ptr = out.as_ptr();
        let expected_capacity = out.capacity();

        let error = unsafe {
            try_fill_segment_output(&mut out, brand, 1, |ptr, _capacity| {
                ptr.write(raw_shape(1, brand.world0().wrapping_add(1)));
                1
            })
        }
        .unwrap_err();
        assert_eq!(error, ApiError::WrongWorld);
        assert!(out.is_empty());
        assert_eq!(out.as_ptr(), expected_ptr);
        assert_eq!(out.capacity(), expected_capacity);

        let error = unsafe {
            try_fill_segment_output(&mut out, brand, 1, |ptr, _capacity| {
                ptr.write(raw_shape(0, brand.world0()));
                1
            })
        }
        .unwrap_err();
        assert_eq!(error, ApiError::InvalidShapeId);
        assert!(out.is_empty());
        assert_eq!(out.as_ptr(), expected_ptr);

        unsafe {
            try_fill_segment_output(&mut out, brand, 1, |ptr, _capacity| {
                ptr.write(raw_shape(2, brand.world0()));
                1
            })
            .unwrap();
        }
        assert_eq!(out.as_ptr(), expected_ptr);
        assert_eq!(out.capacity(), expected_capacity);
        let raw = out[0].into_raw();
        assert_eq!(raw.index1, 2);
        assert_eq!(raw.world0, brand.world0());
        assert_eq!(raw.generation, 1);
    }
}

#[inline]
fn chain_world_id_impl(id: ChainId) -> ffi::b2WorldId {
    unsafe { ffi::b2Chain_GetWorld(raw_chain_id(id)) }
}

#[inline]
fn chain_segment_count_impl(id: ChainId) -> i32 {
    unsafe { ffi::b2Chain_GetSegmentCount(raw_chain_id(id)) }
}

#[inline]
fn chain_raw_surface_material_count_impl(id: ChainId) -> i32 {
    unsafe { ffi::b2Chain_GetSurfaceMaterialCount(raw_chain_id(id)) }
}

#[inline]
fn chain_runtime_surface_material_layout_impl(id: ChainId) -> (i32, i32) {
    let raw_count = chain_raw_surface_material_count_impl(id);
    let segment_count = chain_segment_count_impl(id);
    debug_assert!(
        raw_count == 1 || raw_count == segment_count || raw_count == segment_count + 3,
        "unexpected chain material layout: raw_count={raw_count}, segment_count={segment_count}"
    );
    (raw_count, segment_count)
}

#[inline]
fn chain_surface_material_count_impl(id: ChainId) -> i32 {
    let (raw_count, segment_count) = chain_runtime_surface_material_layout_impl(id);
    if raw_count == 1 { 1 } else { segment_count }
}

#[inline]
fn chain_set_surface_material_impl(
    brand: crate::id::IdBrand,
    id: ChainId,
    index: i32,
    material: &SurfaceMaterial,
) -> ApiResult<()> {
    let (raw_count, segment_count) = chain_runtime_surface_material_layout_impl(id);
    if raw_count == 1 {
        unsafe { ffi::b2Chain_SetSurfaceMaterial(raw_chain_id(id), &material.0, 0) }
    } else if raw_count == segment_count {
        unsafe { ffi::b2Chain_SetSurfaceMaterial(raw_chain_id(id), &material.0, index) }
    } else {
        let segment = chain_segments_in_impl(brand, id)?[index as usize];
        crate::shapes::shape_set_surface_material_impl(segment, material);
    }
    Ok(())
}

#[inline]
fn chain_surface_material_impl(
    brand: crate::id::IdBrand,
    id: ChainId,
    index: i32,
) -> ApiResult<SurfaceMaterial> {
    let (raw_count, segment_count) = chain_runtime_surface_material_layout_impl(id);
    if raw_count == 1 {
        Ok(SurfaceMaterial::from_raw(unsafe {
            ffi::b2Chain_GetSurfaceMaterial(raw_chain_id(id), 0)
        }))
    } else if raw_count == segment_count {
        Ok(SurfaceMaterial::from_raw(unsafe {
            ffi::b2Chain_GetSurfaceMaterial(raw_chain_id(id), index)
        }))
    } else {
        let segment = chain_segments_in_impl(brand, id)?[index as usize];
        Ok(crate::shapes::shape_surface_material_impl(segment))
    }
}

#[track_caller]
fn assert_chain_surface_material_index_in_range(id: ChainId, index: i32) {
    let count = chain_surface_material_count_impl(id);
    assert!(
        0 <= index && index < count,
        "chain surface material index out of range: index={index}, visible_count={count}"
    );
}

fn check_chain_surface_material_index_in_range(id: ChainId, index: i32) -> ApiResult<()> {
    let count = chain_surface_material_count_impl(id);
    if 0 <= index && index < count {
        Ok(())
    } else {
        Err(ApiError::IndexOutOfRange)
    }
}

pub(crate) fn try_chain_set_surface_material_with_access(
    core: &crate::core::world_core::WorldCore,
    id: ChainId,
    index: i32,
    material: &SurfaceMaterial,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<()> {
    crate::shapes::check_surface_material_valid(material)?;
    crate::core::callback_state::check_not_in_callback()?;
    core.check_chain_with_access(id, access)?;
    check_chain_surface_material_index_in_range(id, index)?;
    chain_set_surface_material_impl(core.brand(), id, index, material)
}

#[inline]
fn destroy_chain_now_impl(world_core: &crate::core::world_core::WorldCore, id: ChainId) {
    crate::core::world_core::WorldCore::destroy_chain_now(world_core, id)
        .expect("invalid or foreign ChainId");
}

fn destroy_owned_chain_if_needed_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ChainId,
) {
    crate::core::world_core::WorldCore::destroy_owned_or_defer(
        world_core,
        crate::core::world_core::DeferredDestroy::Chain(id),
    );
}

fn destroy_scoped_chain_checked_impl(world_core: &crate::core::world_core::WorldCore, id: ChainId) {
    crate::core::callback_state::assert_not_in_callback();
    match world_core.check_chain(id) {
        Ok(()) => destroy_chain_now_impl(world_core, id),
        Err(ApiError::InvalidChainId) => {}
        Err(error) => panic!("chain handle is unavailable or foreign: {error}"),
    }
}

fn try_destroy_scoped_chain_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ChainId,
) -> ApiResult<()> {
    try_destroy_chain_with_access(world_core, id, crate::core::world_core::WorldAccess::Idle)
}

pub(crate) fn try_destroy_chain_with_access(
    world_core: &crate::core::world_core::WorldCore,
    id: ChainId,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<()> {
    crate::core::callback_state::check_not_in_callback()?;
    world_core.destroy_chain_now_with_access(id, access)
}

trait ChainRuntimeHandle {
    fn chain_id(&self) -> ChainId;
    fn chain_world_core(&self) -> &crate::core::world_core::WorldCore;

    #[inline]
    #[track_caller]
    fn assert_valid(&self) {
        self.check_valid()
            .expect("chain handle is unavailable, foreign, or invalid");
    }

    #[inline]
    fn check_valid(&self) -> ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.chain_world_core().check_chain(self.chain_id())
    }

    fn handle_world_id_raw(&self) -> ffi::b2WorldId {
        self.assert_valid();
        chain_world_id_impl(self.chain_id())
    }

    fn try_handle_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        self.check_valid()?;
        Ok(chain_world_id_impl(self.chain_id()))
    }

    fn handle_is_valid(&self) -> bool {
        self.try_handle_is_valid()
            .expect("chain handle is unavailable or foreign")
    }

    fn try_handle_is_valid(&self) -> ApiResult<bool> {
        crate::core::callback_state::check_not_in_callback()?;
        let core = self.chain_world_core();
        core.check_available()?;
        core.chain_is_valid(self.chain_id())
    }

    fn handle_segment_count(&self) -> i32 {
        self.assert_valid();
        chain_segment_count_impl(self.chain_id())
    }

    fn try_handle_segment_count(&self) -> ApiResult<i32> {
        self.check_valid()?;
        Ok(chain_segment_count_impl(self.chain_id()))
    }

    fn handle_segments(&self) -> Vec<ShapeId> {
        self.assert_valid();
        chain_segments_in_impl(self.chain_world_core().brand(), self.chain_id())
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    fn handle_segments_into(&self, out: &mut Vec<ShapeId>) {
        self.assert_valid();
        chain_segments_into_in_impl(self.chain_world_core().brand(), self.chain_id(), out)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT);
    }

    fn try_handle_segments(&self) -> ApiResult<Vec<ShapeId>> {
        self.check_valid()?;
        chain_segments_in_impl(self.chain_world_core().brand(), self.chain_id())
    }

    fn try_handle_segments_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        self.check_valid()?;
        chain_segments_into_in_impl(self.chain_world_core().brand(), self.chain_id(), out)
    }

    fn handle_surface_material_count(&self) -> i32 {
        self.assert_valid();
        chain_surface_material_count_impl(self.chain_id())
    }

    fn try_handle_surface_material_count(&self) -> ApiResult<i32> {
        self.check_valid()?;
        Ok(chain_surface_material_count_impl(self.chain_id()))
    }

    fn handle_set_surface_material(&mut self, index: i32, material: &SurfaceMaterial) {
        self.try_handle_set_surface_material(index, material)
            .expect("invalid chain surface material parameters")
    }

    fn try_handle_set_surface_material(
        &mut self,
        index: i32,
        material: &SurfaceMaterial,
    ) -> ApiResult<()> {
        try_chain_set_surface_material_with_access(
            self.chain_world_core(),
            self.chain_id(),
            index,
            material,
            crate::core::world_core::WorldAccess::Idle,
        )
    }

    fn handle_surface_material(&self, index: i32) -> SurfaceMaterial {
        self.assert_valid();
        assert_chain_surface_material_index_in_range(self.chain_id(), index);
        chain_surface_material_impl(self.chain_world_core().brand(), self.chain_id(), index)
            .expect(crate::core::ffi_vec::FFI_OUTPUT_EXPECT)
    }

    fn try_handle_surface_material(&self, index: i32) -> ApiResult<SurfaceMaterial> {
        self.check_valid()?;
        check_chain_surface_material_index_in_range(self.chain_id(), index)?;
        chain_surface_material_impl(self.chain_world_core().brand(), self.chain_id(), index)
    }
}

impl ChainRuntimeHandle for OwnedChain {
    fn chain_id(&self) -> ChainId {
        self.id
    }

    fn chain_world_core(&self) -> &crate::core::world_core::WorldCore {
        self.core.as_ref()
    }
}

impl<'w> ChainRuntimeHandle for Chain<'w> {
    fn chain_id(&self) -> ChainId {
        self.id
    }

    fn chain_world_core(&self) -> &crate::core::world_core::WorldCore {
        self.core.as_ref()
    }
}

impl OwnedChain {
    pub(crate) fn new(core: Rc<crate::core::world_core::WorldCore>, id: ChainId) -> Self {
        core.owned_chains
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            id,
            core,
            destroy_on_drop: true,
        }
    }

    pub fn id(&self) -> ChainId {
        self.id
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        ChainRuntimeHandle::handle_world_id_raw(self)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        ChainRuntimeHandle::try_handle_world_id_raw(self)
    }

    pub fn is_valid(&self) -> bool {
        ChainRuntimeHandle::handle_is_valid(self)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        ChainRuntimeHandle::try_handle_is_valid(self)
    }

    /// Borrow the world-bound branded ID for ID-style APIs.
    pub fn as_id(&self) -> ChainId {
        self.id
    }

    pub fn segment_count(&self) -> i32 {
        ChainRuntimeHandle::handle_segment_count(self)
    }

    pub fn try_segment_count(&self) -> ApiResult<i32> {
        ChainRuntimeHandle::try_handle_segment_count(self)
    }

    /// Collect all segment shape ids for this chain.
    pub fn segments(&self) -> Vec<ShapeId> {
        ChainRuntimeHandle::handle_segments(self)
    }

    pub fn segments_into(&self, out: &mut Vec<ShapeId>) {
        ChainRuntimeHandle::handle_segments_into(self, out);
    }

    pub fn try_segments(&self) -> ApiResult<Vec<ShapeId>> {
        ChainRuntimeHandle::try_handle_segments(self)
    }

    pub fn try_segments_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        ChainRuntimeHandle::try_handle_segments_into(self, out)
    }

    /// Number of runtime-visible material slots on this chain.
    ///
    /// Open chains normalize Box2D's ghost-point placeholder layout down to the number of
    /// live segments. Single-material chains still report `1`.
    pub fn surface_material_count(&self) -> i32 {
        ChainRuntimeHandle::handle_surface_material_count(self)
    }
    pub fn try_surface_material_count(&self) -> ApiResult<i32> {
        ChainRuntimeHandle::try_handle_surface_material_count(self)
    }
    /// Set a runtime-visible material slot by segment index.
    pub fn set_surface_material(&mut self, index: i32, material: &SurfaceMaterial) {
        ChainRuntimeHandle::handle_set_surface_material(self, index, material)
    }
    pub fn try_set_surface_material(
        &mut self,
        index: i32,
        material: &SurfaceMaterial,
    ) -> ApiResult<()> {
        ChainRuntimeHandle::try_handle_set_surface_material(self, index, material)
    }
    /// Read a runtime-visible material slot by segment index.
    pub fn surface_material(&self, index: i32) -> SurfaceMaterial {
        ChainRuntimeHandle::handle_surface_material(self, index)
    }

    pub fn try_surface_material(&self, index: i32) -> ApiResult<SurfaceMaterial> {
        ChainRuntimeHandle::try_handle_surface_material(self, index)
    }

    pub fn into_id(mut self) -> ChainId {
        self.core
            .check_owned_policy_change()
            .expect("owned chain cannot be disarmed while its world is unavailable");
        self.destroy_on_drop = false;
        self.id
    }

    pub fn destroy(mut self) {
        if self.destroy_on_drop {
            destroy_owned_chain_if_needed_impl(&self.core, self.id);
        }
        self.destroy_on_drop = false;
    }
}

impl Drop for OwnedChain {
    fn drop(&mut self) {
        let _ = self.core.id;
        let _ = self.core.owned_chains.fetch_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |count| Some(count.saturating_sub(1)),
        );
        if self.destroy_on_drop {
            destroy_owned_chain_if_needed_impl(&self.core, self.id);
        }
    }
}

impl<'w> Chain<'w> {
    pub(crate) fn new(core: Rc<crate::core::world_core::WorldCore>, id: ChainId) -> Self {
        Self {
            id,
            core,
            _world: PhantomData,
        }
    }

    pub fn id(&self) -> ChainId {
        self.id
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        ChainRuntimeHandle::handle_world_id_raw(self)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        ChainRuntimeHandle::try_handle_world_id_raw(self)
    }

    pub fn is_valid(&self) -> bool {
        ChainRuntimeHandle::handle_is_valid(self)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        ChainRuntimeHandle::try_handle_is_valid(self)
    }

    pub fn segment_count(&self) -> i32 {
        ChainRuntimeHandle::handle_segment_count(self)
    }

    pub fn try_segment_count(&self) -> ApiResult<i32> {
        ChainRuntimeHandle::try_handle_segment_count(self)
    }

    /// Collect all segment shape ids for this chain.
    pub fn segments(&self) -> Vec<ShapeId> {
        ChainRuntimeHandle::handle_segments(self)
    }

    pub fn segments_into(&self, out: &mut Vec<ShapeId>) {
        ChainRuntimeHandle::handle_segments_into(self, out);
    }

    pub fn try_segments(&self) -> ApiResult<Vec<ShapeId>> {
        ChainRuntimeHandle::try_handle_segments(self)
    }

    pub fn try_segments_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        ChainRuntimeHandle::try_handle_segments_into(self, out)
    }

    /// Number of runtime-visible material slots on this chain.
    ///
    /// Open chains normalize Box2D's ghost-point placeholder layout down to the number of
    /// live segments. Single-material chains still report `1`.
    pub fn surface_material_count(&self) -> i32 {
        ChainRuntimeHandle::handle_surface_material_count(self)
    }
    pub fn try_surface_material_count(&self) -> ApiResult<i32> {
        ChainRuntimeHandle::try_handle_surface_material_count(self)
    }

    /// Set a runtime-visible material slot by segment index.
    pub fn set_surface_material(&mut self, index: i32, material: &SurfaceMaterial) {
        ChainRuntimeHandle::handle_set_surface_material(self, index, material)
    }

    pub fn try_set_surface_material(
        &mut self,
        index: i32,
        material: &SurfaceMaterial,
    ) -> ApiResult<()> {
        ChainRuntimeHandle::try_handle_set_surface_material(self, index, material)
    }

    /// Read a runtime-visible material slot by segment index.
    pub fn surface_material(&self, index: i32) -> SurfaceMaterial {
        ChainRuntimeHandle::handle_surface_material(self, index)
    }

    pub fn try_surface_material(&self, index: i32) -> ApiResult<SurfaceMaterial> {
        ChainRuntimeHandle::try_handle_surface_material(self, index)
    }

    /// Destroy this chain immediately.
    pub fn destroy(self) {
        destroy_scoped_chain_checked_impl(&self.core, self.id);
    }

    pub fn try_destroy(self) -> ApiResult<()> {
        try_destroy_scoped_chain_impl(&self.core, self.id)
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
            let _lease = crate::core::foundation::assert_transient_native_lease();
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
        let _lease = crate::core::foundation::assert_transient_native_lease();
        Self {
            def: unsafe { ffi::b2DefaultChainDef() },
            points: Vec::new(),
            materials: Vec::new(),
        }
    }
}

impl ChainDef {
    /// Start building a new `ChainDef` from defaults.
    pub fn builder() -> ChainDefBuilder {
        ChainDefBuilder {
            inner: Self::default(),
        }
    }

    /// Stored chain points, including Box2D's ghost points.
    pub fn points(&self) -> &[Vec2] {
        unsafe {
            core::slice::from_raw_parts(self.points.as_ptr().cast::<Vec2>(), self.points.len())
        }
    }

    /// Whether the chain is closed into a loop.
    pub const fn is_loop(&self) -> bool {
        self.def.isLoop
    }

    /// Collision filter used by the chain.
    pub const fn filter(&self) -> crate::filter::Filter {
        crate::filter::Filter::from_raw(self.def.filter)
    }

    /// Whether sensor begin/end events are enabled for the chain.
    pub const fn sensor_events_enabled(&self) -> bool {
        self.def.enableSensorEvents
    }

    /// Inspect the material layout supplied to the chain definition.
    pub fn material_layout(&self) -> ChainDefMaterialLayout<'_> {
        match self.materials.len() {
            0 => ChainDefMaterialLayout::Default(SurfaceMaterial::from_raw(unsafe {
                *self.def.materials
            })),
            1 => ChainDefMaterialLayout::Single(SurfaceMaterial::from_raw(self.materials[0])),
            _ => ChainDefMaterialLayout::Multiple(unsafe {
                core::slice::from_raw_parts(
                    self.materials.as_ptr().cast::<SurfaceMaterial>(),
                    self.materials.len(),
                )
            }),
        }
    }

    /// Number of material entries visible to Box2D.
    pub fn material_count(&self) -> usize {
        self.material_layout().count()
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
        self.inner.points = points.into_iter().map(|p| p.into().into_raw()).collect();
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
            let _lease = crate::core::foundation::assert_transient_native_lease();
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

impl From<ChainDef> for ChainDefBuilder {
    fn from(def: ChainDef) -> Self {
        Self { inner: def }
    }
}

#[inline]
#[track_caller]
pub(crate) fn assert_chain_def_valid(def: &ChainDef) {
    check_chain_def_valid(def).expect("invalid ChainDef");
}

pub(crate) fn check_chain_def_valid(def: &ChainDef) -> ApiResult<()> {
    let count = def.def.count;
    if count < 4 || usize::try_from(count) != Ok(def.points.len()) {
        return Err(ApiError::InvalidChainDef);
    }
    if def.def.points.is_null() || def.def.points != def.points.as_ptr() {
        return Err(ApiError::InvalidChainDef);
    }

    let mc = def.def.materialCount;
    if mc != 1 && mc != count {
        return Err(ApiError::InvalidChainDef);
    }
    if def.def.materials.is_null() {
        return Err(ApiError::InvalidChainDef);
    }

    let expected_material_count = if def.materials.is_empty() {
        1
    } else {
        i32::try_from(def.materials.len()).map_err(|_| ApiError::InvalidChainDef)?
    };
    if mc != expected_material_count
        || (!def.materials.is_empty() && def.def.materials != def.materials.as_ptr())
    {
        return Err(ApiError::InvalidChainDef);
    }

    if !def
        .points
        .iter()
        .copied()
        .map(Vec2::from_raw)
        .all(Vec2::is_valid)
    {
        return Err(ApiError::InvalidChainDef);
    }

    if def.materials.is_empty() {
        // `ChainDef` has no raw constructor. Its empty-material layout always retains the
        // non-null static material returned by `b2DefaultChainDef`.
        let material = SurfaceMaterial::from_raw(unsafe { *def.def.materials });
        crate::shapes::check_surface_material_valid(&material)
            .map_err(|_| ApiError::InvalidChainDef)?;
    } else {
        for &material in &def.materials {
            crate::shapes::check_surface_material_valid(&SurfaceMaterial::from_raw(material))
                .map_err(|_| ApiError::InvalidChainDef)?;
        }
    }

    Ok(())
}

fn finish_chain_creation(
    core: &crate::core::world_core::WorldCore,
    raw: ffi::b2ChainId,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<ChainId> {
    core.finish_created_chain_with_access(raw, access)
}

pub(crate) fn create_chain_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ChainDef,
) -> ChainId {
    assert_chain_def_valid(def);
    crate::core::callback_state::assert_not_in_callback();
    core.check_body(body).expect("invalid or foreign BodyId");
    let raw = unsafe { ffi::b2CreateChain(body.into_raw(), &def.def) };
    finish_chain_creation(core, raw, crate::core::world_core::WorldAccess::Idle)
        .expect("Box2D returned an invalid ChainId")
}

pub(crate) fn try_create_chain_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ChainDef,
) -> ApiResult<ChainId> {
    try_create_chain_for_body_with_access(
        core,
        body,
        def,
        crate::core::world_core::WorldAccess::Idle,
    )
}

pub(crate) fn try_create_chain_for_body_with_access(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ChainDef,
    access: crate::core::world_core::WorldAccess,
) -> ApiResult<ChainId> {
    check_chain_def_valid(def)?;
    crate::core::callback_state::check_not_in_callback()?;
    core.check_body_with_access(body, access)?;
    let raw = unsafe { ffi::b2CreateChain(body.into_raw(), &def.def) };
    finish_chain_creation(core, raw, access)
}

fn create_body_attached_chain_handle<T>(
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ChainDef,
    create: impl FnOnce(&crate::core::world_core::WorldCore, BodyId, &ChainDef) -> ChainId,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ChainId) -> T,
) -> T {
    let id = create(core.as_ref(), body, def);
    wrap(Rc::clone(core), id)
}

fn try_create_body_attached_chain_handle<T>(
    core: &Rc<crate::core::world_core::WorldCore>,
    body: BodyId,
    def: &ChainDef,
    create: impl FnOnce(&crate::core::world_core::WorldCore, BodyId, &ChainDef) -> ApiResult<ChainId>,
    wrap: impl FnOnce(Rc<crate::core::world_core::WorldCore>, ChainId) -> T,
) -> ApiResult<T> {
    let id = create(core.as_ref(), body, def)?;
    Ok(wrap(Rc::clone(core), id))
}

impl ChainDef {
    pub fn validate(&self) -> ApiResult<()> {
        check_chain_def_valid(self)
    }
}

impl<'w> Body<'w> {
    /// Create a chain shape attached to this body. Points/materials are cloned internally by Box2D.
    pub fn create_chain(&mut self, def: &ChainDef) -> Chain<'w> {
        create_body_attached_chain_handle(
            &self.core,
            self.id,
            def,
            create_chain_for_body_impl,
            Chain::new,
        )
    }

    pub fn try_create_chain(&mut self, def: &ChainDef) -> ApiResult<Chain<'w>> {
        try_create_body_attached_chain_handle(
            &self.core,
            self.id,
            def,
            try_create_chain_for_body_impl,
            Chain::new,
        )
    }
}

impl OwnedBody {
    /// Create a chain shape attached to this body. Points/materials are cloned internally by Box2D.
    pub fn create_chain(&mut self, def: &ChainDef) -> OwnedChain {
        create_body_attached_chain_handle(
            &self.core_rc(),
            self.id(),
            def,
            create_chain_for_body_impl,
            OwnedChain::new,
        )
    }

    pub fn try_create_chain(&mut self, def: &ChainDef) -> ApiResult<OwnedChain> {
        try_create_body_attached_chain_handle(
            &self.core_rc(),
            self.id(),
            def,
            try_create_chain_for_body_impl,
            OwnedChain::new,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_creation_registers_chain_and_segment_identities_before_returning() {
        let mut world = World::new(crate::WorldDef::default()).unwrap();
        let body = world.create_body_id(crate::BodyBuilder::new().build());
        let core = world.core();
        let def = ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build();
        let chain = try_create_chain_for_body_impl(core, body, &def).unwrap();
        let segments = chain_segments_in_impl(core.brand(), chain).unwrap();

        assert_eq!(core.check_chain(chain), Ok(()));
        assert!(!segments.is_empty());
        assert!(
            segments
                .iter()
                .all(|&segment| core.check_shape(segment).is_ok())
        );
        assert_eq!(
            core.finish_created_chain(chain.into_raw()),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(core.check_available(), Err(ApiError::WorldPoisoned));
    }
}
