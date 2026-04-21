use std::marker::PhantomData;

use crate::body::{Body, OwnedBody};
use crate::error::{ApiError, ApiResult};
use crate::shapes::SurfaceMaterial;
use crate::types::{BodyId, ChainId, ShapeId, Vec2};
use crate::world::World;
use boxdd_sys::ffi;
use std::rc::Rc;
use std::sync::Arc;

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

#[inline]
fn raw_chain_id(id: ChainId) -> ffi::b2ChainId {
    id.into_raw()
}

fn chain_segments_into_impl(id: ChainId, out: &mut Vec<ShapeId>) {
    let id = raw_chain_id(id);
    let count = unsafe { ffi::b2Chain_GetSegmentCount(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::fill_from_ffi(out, count, |ptr, count| {
            ffi::b2Chain_GetSegments(id, ptr.cast(), count)
        });
    }
}

fn chain_segments_impl(id: ChainId) -> Vec<ShapeId> {
    let id = raw_chain_id(id);
    let count = unsafe { ffi::b2Chain_GetSegmentCount(id) }.max(0) as usize;
    unsafe {
        crate::core::ffi_vec::read_from_ffi(count, |ptr: *mut ShapeId, count| {
            ffi::b2Chain_GetSegments(id, ptr.cast(), count)
        })
    }
}

fn chain_segment_count_checked_impl(id: ChainId) -> i32 {
    crate::core::debug_checks::assert_chain_valid(id);
    chain_segment_count_impl(id)
}

fn try_chain_segment_count_impl(id: ChainId) -> ApiResult<i32> {
    crate::core::debug_checks::check_chain_valid(id)?;
    Ok(chain_segment_count_impl(id))
}

fn chain_segments_checked_impl(id: ChainId) -> Vec<ShapeId> {
    crate::core::debug_checks::assert_chain_valid(id);
    chain_segments_impl(id)
}

fn chain_segments_into_checked_impl(id: ChainId, out: &mut Vec<ShapeId>) {
    crate::core::debug_checks::assert_chain_valid(id);
    chain_segments_into_impl(id, out);
}

fn try_chain_segments_impl(id: ChainId) -> ApiResult<Vec<ShapeId>> {
    crate::core::debug_checks::check_chain_valid(id)?;
    Ok(chain_segments_impl(id))
}

fn try_chain_segments_into_impl(id: ChainId, out: &mut Vec<ShapeId>) -> ApiResult<()> {
    crate::core::debug_checks::check_chain_valid(id)?;
    chain_segments_into_impl(id, out);
    Ok(())
}

#[inline]
fn chain_world_id_impl(id: ChainId) -> ffi::b2WorldId {
    unsafe { ffi::b2Chain_GetWorld(raw_chain_id(id)) }
}

#[inline]
fn chain_world_id_checked_impl(id: ChainId) -> ffi::b2WorldId {
    crate::core::debug_checks::assert_chain_valid(id);
    chain_world_id_impl(id)
}

#[inline]
fn try_chain_world_id_raw_impl(id: ChainId) -> ApiResult<ffi::b2WorldId> {
    crate::core::debug_checks::check_chain_valid(id)?;
    Ok(chain_world_id_impl(id))
}

#[inline]
fn chain_is_valid_impl(id: ChainId) -> bool {
    unsafe { ffi::b2Chain_IsValid(raw_chain_id(id)) }
}

#[inline]
fn chain_is_valid_checked_impl(id: ChainId) -> bool {
    crate::core::callback_state::assert_not_in_callback();
    chain_is_valid_impl(id)
}

#[inline]
fn try_chain_is_valid_impl(id: ChainId) -> ApiResult<bool> {
    crate::core::callback_state::check_not_in_callback()?;
    Ok(chain_is_valid_impl(id))
}

#[inline]
fn chain_segment_count_impl(id: ChainId) -> i32 {
    unsafe { ffi::b2Chain_GetSegmentCount(raw_chain_id(id)) }
}

#[inline]
fn chain_surface_material_count_impl(id: ChainId) -> i32 {
    unsafe { ffi::b2Chain_GetRuntimeSurfaceMaterialCount(raw_chain_id(id)) }
}

#[inline]
fn chain_set_surface_material_impl(id: ChainId, index: i32, material: &SurfaceMaterial) {
    unsafe { ffi::b2Chain_SetRuntimeSurfaceMaterial(raw_chain_id(id), &material.0, index) }
}

#[inline]
fn chain_surface_material_impl(id: ChainId, index: i32) -> SurfaceMaterial {
    SurfaceMaterial::from_raw(unsafe {
        ffi::b2Chain_GetRuntimeSurfaceMaterial(raw_chain_id(id), index)
    })
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

fn chain_surface_material_count_checked_impl(id: ChainId) -> i32 {
    crate::core::debug_checks::assert_chain_valid(id);
    chain_surface_material_count_impl(id)
}

fn try_chain_surface_material_count_impl(id: ChainId) -> ApiResult<i32> {
    crate::core::debug_checks::check_chain_valid(id)?;
    Ok(chain_surface_material_count_impl(id))
}

fn chain_set_surface_material_checked_impl(id: ChainId, index: i32, material: &SurfaceMaterial) {
    crate::core::debug_checks::assert_chain_valid(id);
    assert_chain_surface_material_index_in_range(id, index);
    chain_set_surface_material_impl(id, index, material)
}

fn try_chain_set_surface_material_impl(
    id: ChainId,
    index: i32,
    material: &SurfaceMaterial,
) -> ApiResult<()> {
    crate::core::debug_checks::check_chain_valid(id)?;
    check_chain_surface_material_index_in_range(id, index)?;
    chain_set_surface_material_impl(id, index, material);
    Ok(())
}

fn chain_surface_material_checked_impl(id: ChainId, index: i32) -> SurfaceMaterial {
    crate::core::debug_checks::assert_chain_valid(id);
    assert_chain_surface_material_index_in_range(id, index);
    chain_surface_material_impl(id, index)
}

fn try_chain_surface_material_impl(id: ChainId, index: i32) -> ApiResult<SurfaceMaterial> {
    crate::core::debug_checks::check_chain_valid(id)?;
    check_chain_surface_material_index_in_range(id, index)?;
    Ok(chain_surface_material_impl(id, index))
}

#[inline]
fn destroy_chain_now_impl(world_core: &crate::core::world_core::WorldCore, id: ChainId) {
    unsafe { ffi::b2DestroyChain(raw_chain_id(id)) }
    #[cfg(feature = "serialize")]
    world_core.remove_chain(id);
    #[cfg(not(feature = "serialize"))]
    let _ = world_core;
}

fn destroy_owned_chain_if_needed_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ChainId,
) {
    if !chain_is_valid_impl(id) {
        return;
    }

    if crate::core::callback_state::in_callback() || world_core.events_buffers_are_borrowed() {
        world_core.defer_destroy(crate::core::world_core::DeferredDestroy::Chain(id));
    } else {
        destroy_chain_now_impl(world_core, id);
    }
}

fn destroy_scoped_chain_checked_impl(world_core: &crate::core::world_core::WorldCore, id: ChainId) {
    crate::core::callback_state::assert_not_in_callback();
    if chain_is_valid_impl(id) {
        destroy_chain_now_impl(world_core, id);
    }
}

fn try_destroy_scoped_chain_impl(
    world_core: &crate::core::world_core::WorldCore,
    id: ChainId,
) -> ApiResult<()> {
    crate::core::debug_checks::check_chain_valid(id)?;
    if chain_is_valid_impl(id) {
        destroy_chain_now_impl(world_core, id);
    }
    Ok(())
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
        chain_world_id_checked_impl(self.id)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        try_chain_world_id_raw_impl(self.id)
    }

    pub fn is_valid(&self) -> bool {
        chain_is_valid_checked_impl(self.id)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        try_chain_is_valid_impl(self.id)
    }

    /// Borrow the raw id for ID-style APIs.
    pub fn as_id(&self) -> ChainId {
        self.id
    }

    pub fn segment_count(&self) -> i32 {
        chain_segment_count_checked_impl(self.id)
    }

    pub fn try_segment_count(&self) -> ApiResult<i32> {
        try_chain_segment_count_impl(self.id)
    }

    /// Collect all segment shape ids for this chain.
    pub fn segments(&self) -> Vec<ShapeId> {
        chain_segments_checked_impl(self.id)
    }

    pub fn segments_into(&self, out: &mut Vec<ShapeId>) {
        chain_segments_into_checked_impl(self.id, out);
    }

    pub fn try_segments(&self) -> ApiResult<Vec<ShapeId>> {
        try_chain_segments_impl(self.id)
    }

    pub fn try_segments_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        try_chain_segments_into_impl(self.id, out)
    }

    /// Number of runtime-visible material slots on this chain.
    ///
    /// Open chains normalize Box2D's ghost-point placeholder layout down to the number of
    /// live segments. Single-material chains still report `1`.
    pub fn surface_material_count(&self) -> i32 {
        chain_surface_material_count_checked_impl(self.id)
    }
    pub fn try_surface_material_count(&self) -> ApiResult<i32> {
        try_chain_surface_material_count_impl(self.id)
    }
    /// Set a runtime-visible material slot by segment index.
    pub fn set_surface_material(&mut self, index: i32, material: &SurfaceMaterial) {
        chain_set_surface_material_checked_impl(self.id, index, material)
    }
    pub fn try_set_surface_material(
        &mut self,
        index: i32,
        material: &SurfaceMaterial,
    ) -> ApiResult<()> {
        try_chain_set_surface_material_impl(self.id, index, material)
    }
    /// Read a runtime-visible material slot by segment index.
    pub fn surface_material(&self, index: i32) -> SurfaceMaterial {
        chain_surface_material_checked_impl(self.id, index)
    }

    pub fn try_surface_material(&self, index: i32) -> ApiResult<SurfaceMaterial> {
        try_chain_surface_material_impl(self.id, index)
    }

    pub fn into_id(mut self) -> ChainId {
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
        let prev = self
            .core
            .owned_chains
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        debug_assert!(prev > 0, "owned chain counter underflow");
        if self.destroy_on_drop {
            destroy_owned_chain_if_needed_impl(&self.core, self.id);
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

    pub fn id(&self) -> ChainId {
        self.id
    }

    pub fn world_id_raw(&self) -> ffi::b2WorldId {
        chain_world_id_checked_impl(self.id)
    }

    pub fn try_world_id_raw(&self) -> ApiResult<ffi::b2WorldId> {
        try_chain_world_id_raw_impl(self.id)
    }

    pub fn is_valid(&self) -> bool {
        chain_is_valid_checked_impl(self.id)
    }

    pub fn try_is_valid(&self) -> ApiResult<bool> {
        try_chain_is_valid_impl(self.id)
    }

    pub fn segment_count(&self) -> i32 {
        chain_segment_count_checked_impl(self.id)
    }

    pub fn try_segment_count(&self) -> ApiResult<i32> {
        try_chain_segment_count_impl(self.id)
    }

    /// Collect all segment shape ids for this chain.
    pub fn segments(&self) -> Vec<ShapeId> {
        chain_segments_checked_impl(self.id)
    }

    pub fn segments_into(&self, out: &mut Vec<ShapeId>) {
        chain_segments_into_checked_impl(self.id, out);
    }

    pub fn try_segments(&self) -> ApiResult<Vec<ShapeId>> {
        try_chain_segments_impl(self.id)
    }

    pub fn try_segments_into(&self, out: &mut Vec<ShapeId>) -> ApiResult<()> {
        try_chain_segments_into_impl(self.id, out)
    }

    /// Number of runtime-visible material slots on this chain.
    ///
    /// Open chains normalize Box2D's ghost-point placeholder layout down to the number of
    /// live segments. Single-material chains still report `1`.
    pub fn surface_material_count(&self) -> i32 {
        chain_surface_material_count_checked_impl(self.id)
    }
    pub fn try_surface_material_count(&self) -> ApiResult<i32> {
        try_chain_surface_material_count_impl(self.id)
    }

    /// Set a runtime-visible material slot by segment index.
    pub fn set_surface_material(&mut self, index: i32, material: &SurfaceMaterial) {
        chain_set_surface_material_checked_impl(self.id, index, material)
    }

    pub fn try_set_surface_material(
        &mut self,
        index: i32,
        material: &SurfaceMaterial,
    ) -> ApiResult<()> {
        try_chain_set_surface_material_impl(self.id, index, material)
    }

    /// Read a runtime-visible material slot by segment index.
    pub fn surface_material(&self, index: i32) -> SurfaceMaterial {
        chain_surface_material_checked_impl(self.id, index)
    }

    pub fn try_surface_material(&self, index: i32) -> ApiResult<SurfaceMaterial> {
        try_chain_surface_material_impl(self.id, index)
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

    #[cfg(feature = "serialize")]
    pub(crate) fn points_raw_slice(&self) -> &[ffi::b2Vec2] {
        &self.points
    }
    #[cfg(feature = "serialize")]
    pub(crate) fn materials_raw_slice(&self) -> &[ffi::b2SurfaceMaterial] {
        &self.materials
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

pub(crate) fn create_chain_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ChainDef,
) -> ChainId {
    crate::core::debug_checks::assert_body_valid(body);
    assert_chain_def_valid(def);
    let id = ChainId::from_raw(unsafe { ffi::b2CreateChain(body.into_raw(), &def.def) });
    #[cfg(feature = "serialize")]
    {
        let meta = crate::core::serialize_registry::ChainCreateMeta::from_def(body, def);
        core.record_chain(id, meta);
    }
    #[cfg(not(feature = "serialize"))]
    let _ = core;
    id
}

pub(crate) fn try_create_chain_for_body_impl(
    core: &crate::core::world_core::WorldCore,
    body: BodyId,
    def: &ChainDef,
) -> ApiResult<ChainId> {
    crate::core::debug_checks::check_body_valid(body)?;
    check_chain_def_valid(def)?;
    let id = ChainId::from_raw(unsafe { ffi::b2CreateChain(body.into_raw(), &def.def) });
    #[cfg(feature = "serialize")]
    {
        let meta = crate::core::serialize_registry::ChainCreateMeta::from_def(body, def);
        core.record_chain(id, meta);
    }
    #[cfg(not(feature = "serialize"))]
    let _ = core;
    Ok(id)
}

impl ChainDef {
    pub fn validate(&self) -> ApiResult<()> {
        check_chain_def_valid(self)
    }
}

impl<'w> Body<'w> {
    /// Create a chain shape attached to this body. Points/materials are cloned internally by Box2D.
    pub fn create_chain(&mut self, def: &ChainDef) -> Chain<'w> {
        let id = create_chain_for_body_impl(self.core.as_ref(), self.id, def);
        Chain::new(Arc::clone(&self.core), id)
    }

    pub fn try_create_chain(&mut self, def: &ChainDef) -> ApiResult<Chain<'w>> {
        let id = try_create_chain_for_body_impl(self.core.as_ref(), self.id, def)?;
        Ok(Chain::new(Arc::clone(&self.core), id))
    }
}

impl OwnedBody {
    /// Create a chain shape attached to this body. Points/materials are cloned internally by Box2D.
    pub fn create_chain(&mut self, def: &ChainDef) -> OwnedChain {
        let core = self.core_arc();
        let id = create_chain_for_body_impl(core.as_ref(), self.id(), def);
        OwnedChain::new(core, id)
    }

    pub fn try_create_chain(&mut self, def: &ChainDef) -> ApiResult<OwnedChain> {
        let core = self.core_arc();
        let id = try_create_chain_for_body_impl(core.as_ref(), self.id(), def)?;
        Ok(OwnedChain::new(core, id))
    }
}
