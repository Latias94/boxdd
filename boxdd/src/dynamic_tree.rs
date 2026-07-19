//! Safe wrapper for Box2D's standalone dynamic AABB tree.
//!
//! The dynamic tree can organize spatial data that is not part of a Box2D world.
//! Proxies store an AABB, category bits, and an opaque `u64` user data value.

use std::{collections::BTreeSet, panic::AssertUnwindSafe};

use boxdd_sys::ffi;

use crate::{
    error::{ApiError, ApiResult},
    query::Aabb,
    types::Vec2,
};

type PanicPayload = Box<dyn std::any::Any + Send + 'static>;

/// Opaque proxy identifier owned by a [`DynamicTree`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TreeProxyId(i32);

impl TreeProxyId {
    /// Build a proxy id from its raw integer value.
    #[inline]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// Return the raw Box2D proxy id.
    #[inline]
    pub const fn into_raw(self) -> i32 {
        self.0
    }
}

/// Dynamic tree traversal performance counters.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct TreeStats {
    pub node_visits: i32,
    pub leaf_visits: i32,
}

impl TreeStats {
    /// Convert Box2D traversal counters into the safe value type.
    #[inline]
    pub fn from_raw(raw: ffi::b2TreeStats) -> Self {
        Self {
            node_visits: raw.nodeVisits,
            leaf_visits: raw.leafVisits,
        }
    }

    /// Convert these traversal counters into their raw Box2D representation.
    #[inline]
    pub fn into_raw(self) -> ffi::b2TreeStats {
        ffi::b2TreeStats {
            nodeVisits: self.node_visits,
            leafVisits: self.leaf_visits,
        }
    }
}

/// Ray-cast input for [`DynamicTree::ray_cast`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TreeRayCastInput {
    pub origin: Vec2,
    pub translation: Vec2,
    pub max_fraction: f32,
}

impl TreeRayCastInput {
    /// Build a ray cast over `origin + translation * max_fraction`.
    #[inline]
    pub fn new<O: Into<Vec2>, T: Into<Vec2>>(origin: O, translation: T) -> Self {
        Self {
            origin: origin.into(),
            translation: translation.into(),
            max_fraction: 1.0,
        }
    }

    /// Limit the cast to a fraction of the translation.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Self {
        self.max_fraction = max_fraction;
        self
    }

    /// Validate this input before crossing the FFI boundary.
    pub fn validate(&self) -> ApiResult<()> {
        check_vec2(self.origin)?;
        check_vec2(self.translation)?;
        check_fraction(self.max_fraction)
    }

    /// Convert a raw Box2D ray-cast input into the safe value type.
    #[inline]
    pub fn from_raw(raw: ffi::b2RayCastInput) -> Self {
        Self {
            origin: Vec2::from_raw(raw.origin),
            translation: Vec2::from_raw(raw.translation),
            max_fraction: raw.maxFraction,
        }
    }

    /// Convert this ray-cast input into its raw Box2D representation.
    #[inline]
    pub fn into_raw(self) -> ffi::b2RayCastInput {
        ffi::b2RayCastInput {
            origin: self.origin.into_raw(),
            translation: self.translation.into_raw(),
            maxFraction: self.max_fraction,
        }
    }
}

/// Swept AABB input for [`DynamicTree::box_cast`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TreeBoxCastInput {
    pub aabb: Aabb,
    pub translation: Vec2,
    pub max_fraction: f32,
}

impl TreeBoxCastInput {
    /// Build an AABB cast over `aabb` moving by `translation`.
    #[inline]
    pub fn new<T: Into<Vec2>>(aabb: Aabb, translation: T) -> Self {
        Self {
            aabb,
            translation: translation.into(),
            max_fraction: 1.0,
        }
    }

    /// Limit the cast to a fraction of the translation.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Self {
        self.max_fraction = max_fraction;
        self
    }

    /// Validate this input before crossing the FFI boundary.
    pub fn validate(&self) -> ApiResult<()> {
        check_aabb(self.aabb)?;
        check_vec2(self.translation)?;
        check_fraction(self.max_fraction)
    }

    /// Convert a raw Box2D box-cast input into the safe value type.
    #[inline]
    pub fn from_raw(raw: ffi::b2BoxCastInput) -> Self {
        Self {
            aabb: Aabb::from_raw(raw.box_),
            translation: Vec2::from_raw(raw.translation),
            max_fraction: raw.maxFraction,
        }
    }

    /// Convert this box-cast input into its raw Box2D representation.
    #[inline]
    pub fn into_raw(self) -> ffi::b2BoxCastInput {
        ffi::b2BoxCastInput {
            box_: self.aabb.into_raw(),
            translation: self.translation.into_raw(),
            maxFraction: self.max_fraction,
        }
    }
}

/// RAII owner for a Box2D dynamic tree.
pub struct DynamicTree {
    raw: ffi::b2DynamicTree,
    proxies: BTreeSet<i32>,
}

impl Default for DynamicTree {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicTree {
    /// Initial proxy capacity used by [`DynamicTree::new`].
    pub const DEFAULT_PROXY_CAPACITY: usize = 16;

    /// Largest proxy capacity whose native node count and allocation fit this platform.
    pub const MAX_PROXY_CAPACITY: usize =
        maximum_proxy_capacity_for(isize::MAX as usize, core::mem::size_of::<ffi::b2TreeNode>());

    /// Create an empty dynamic tree.
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_PROXY_CAPACITY)
    }

    /// Create an empty dynamic tree with an initial proxy capacity hint.
    ///
    /// Box2D currently rounds capacities below 16 up to 16 and grows the tree as needed.
    #[inline]
    pub fn with_capacity(proxy_capacity: usize) -> Self {
        Self::try_with_capacity(proxy_capacity)
            .expect("dynamic tree proxy capacity exceeds Box2D's supported range")
    }

    /// Create an empty dynamic tree with recoverable capacity validation.
    #[inline]
    pub fn try_with_capacity(proxy_capacity: usize) -> ApiResult<Self> {
        let proxy_capacity = check_proxy_capacity(proxy_capacity)?;
        Ok(Self {
            raw: unsafe { ffi::b2DynamicTree_Create(proxy_capacity) },
            proxies: BTreeSet::new(),
        })
    }

    /// Create a proxy and return its tree-local id.
    pub fn create_proxy(&mut self, aabb: Aabb, category_bits: u64, user_data: u64) -> TreeProxyId {
        assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
        self.try_create_proxy(aabb, category_bits, user_data)
            .expect("dynamic tree cannot safely grow for another proxy")
    }

    /// Create a proxy with recoverable validation.
    pub fn try_create_proxy(
        &mut self,
        aabb: Aabb,
        category_bits: u64,
        user_data: u64,
    ) -> ApiResult<TreeProxyId> {
        check_aabb(aabb)?;
        check_proxy_node_reserve(&self.raw)?;
        let id = unsafe {
            ffi::b2DynamicTree_CreateProxy(&mut self.raw, aabb.into_raw(), category_bits, user_data)
        };
        self.proxies.insert(id);
        Ok(TreeProxyId(id))
    }

    /// Destroy a proxy owned by this tree.
    pub fn destroy_proxy(&mut self, proxy: TreeProxyId) {
        self.try_destroy_proxy(proxy)
            .expect("proxy id must belong to this dynamic tree");
    }

    /// Destroy a proxy with recoverable validation.
    pub fn try_destroy_proxy(&mut self, proxy: TreeProxyId) -> ApiResult<()> {
        self.check_proxy(proxy)?;
        unsafe {
            ffi::b2DynamicTree_DestroyProxy(&mut self.raw, proxy.into_raw());
        }
        self.proxies.remove(&proxy.into_raw());
        Ok(())
    }

    /// Move a proxy to a new AABB by removing and reinserting it.
    pub fn move_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) {
        self.try_move_proxy(proxy, aabb)
            .expect("proxy id and AABB must satisfy Box2D dynamic-tree constraints");
    }

    /// Move a proxy with recoverable validation.
    pub fn try_move_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) -> ApiResult<()> {
        self.check_proxy(proxy)?;
        check_dynamic_tree_update_aabb(aabb)?;
        unsafe {
            ffi::b2DynamicTree_MoveProxy(&mut self.raw, proxy.into_raw(), aabb.into_raw());
        }
        Ok(())
    }

    /// Enlarge a proxy and its ancestors as necessary.
    pub fn enlarge_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) {
        self.try_enlarge_proxy(proxy, aabb)
            .expect("proxy id and enlarged AABB must satisfy Box2D dynamic-tree constraints");
    }

    /// Enlarge a proxy with recoverable validation.
    pub fn try_enlarge_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) -> ApiResult<()> {
        self.check_proxy(proxy)?;
        check_dynamic_tree_update_aabb(aabb)?;
        let current =
            Aabb::from_raw(unsafe { ffi::b2DynamicTree_GetAABB(&self.raw, proxy.into_raw()) });
        if aabb_contains(current, aabb) {
            return Err(ApiError::InvalidArgument);
        }
        unsafe {
            ffi::b2DynamicTree_EnlargeProxy(&mut self.raw, proxy.into_raw(), aabb.into_raw());
        }
        Ok(())
    }

    /// Replace a proxy with equivalent state and new category bits.
    ///
    /// The returned id identifies the replacement; `proxy` is invalid after this call.
    pub fn replace_category_bits(&mut self, proxy: TreeProxyId, category_bits: u64) -> TreeProxyId {
        self.try_replace_category_bits(proxy, category_bits)
            .expect("proxy id must belong to this dynamic tree")
    }

    /// Replace a proxy with recoverable validation.
    ///
    /// The pinned Box2D revision cannot call its in-place category setter for arbitrary user data
    /// in assertion-enabled builds because the setter reads aliased internal union storage. Creating
    /// a replacement preserves the documented AABB and user-data behavior without depending on that
    /// private representation.
    pub fn try_replace_category_bits(
        &mut self,
        proxy: TreeProxyId,
        category_bits: u64,
    ) -> ApiResult<TreeProxyId> {
        self.check_proxy(proxy)?;
        let aabb = self.try_aabb(proxy)?;
        let user_data = self.try_user_data(proxy)?;
        let replacement = self.try_create_proxy(aabb, category_bits, user_data)?;
        if let Err(error) = self.try_destroy_proxy(proxy) {
            let _ = self.try_destroy_proxy(replacement);
            return Err(error);
        }
        Ok(replacement)
    }

    /// Get the category bits on a proxy.
    pub fn category_bits(&mut self, proxy: TreeProxyId) -> u64 {
        self.try_category_bits(proxy)
            .expect("proxy id must belong to this dynamic tree")
    }

    /// Get the category bits on a proxy with recoverable validation.
    pub fn try_category_bits(&mut self, proxy: TreeProxyId) -> ApiResult<u64> {
        self.check_proxy(proxy)?;
        Ok(unsafe { ffi::b2DynamicTree_GetCategoryBits(&mut self.raw, proxy.into_raw()) })
    }

    /// Get proxy user data.
    pub fn user_data(&self, proxy: TreeProxyId) -> u64 {
        self.try_user_data(proxy)
            .expect("proxy id must belong to this dynamic tree")
    }

    /// Get proxy user data with recoverable validation.
    pub fn try_user_data(&self, proxy: TreeProxyId) -> ApiResult<u64> {
        self.check_proxy(proxy)?;
        Ok(unsafe { ffi::b2DynamicTree_GetUserData(&self.raw, proxy.into_raw()) })
    }

    /// Get a proxy's current AABB.
    pub fn aabb(&self, proxy: TreeProxyId) -> Aabb {
        self.try_aabb(proxy)
            .expect("proxy id must belong to this dynamic tree")
    }

    /// Get a proxy's current AABB with recoverable validation.
    pub fn try_aabb(&self, proxy: TreeProxyId) -> ApiResult<Aabb> {
        self.check_proxy(proxy)?;
        Ok(Aabb::from_raw(unsafe {
            ffi::b2DynamicTree_GetAABB(&self.raw, proxy.into_raw())
        }))
    }

    /// Query proxies overlapping `aabb`, applying category mask bits.
    pub fn query<F>(&self, aabb: Aabb, mask_bits: u64, visit: &mut F) -> TreeStats
    where
        F: FnMut(TreeProxyId, u64) -> bool,
    {
        assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
        self.try_query(aabb, mask_bits, visit)
            .expect("validated dynamic tree query")
    }

    /// Query proxies overlapping `aabb` with recoverable validation.
    pub fn try_query<F>(&self, aabb: Aabb, mask_bits: u64, visit: &mut F) -> ApiResult<TreeStats>
    where
        F: FnMut(TreeProxyId, u64) -> bool,
    {
        check_aabb(aabb)?;
        let mut ctx = QueryCtx::new(visit);
        let stats = unsafe {
            ffi::b2DynamicTree_Query(
                &self.raw,
                aabb.into_raw(),
                mask_bits,
                Some(query_cb::<F>),
                &mut ctx as *mut _ as *mut _,
            )
        };
        ctx.finish();
        Ok(TreeStats::from_raw(stats))
    }

    /// Query proxies overlapping `aabb` without category filtering.
    pub fn query_all<F>(&self, aabb: Aabb, visit: &mut F) -> TreeStats
    where
        F: FnMut(TreeProxyId, u64) -> bool,
    {
        assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
        self.try_query_all(aabb, visit)
            .expect("validated dynamic tree query")
    }

    /// Query proxies overlapping `aabb` without category filtering and with recoverable validation.
    pub fn try_query_all<F>(&self, aabb: Aabb, visit: &mut F) -> ApiResult<TreeStats>
    where
        F: FnMut(TreeProxyId, u64) -> bool,
    {
        check_aabb(aabb)?;
        let mut ctx = QueryCtx::new(visit);
        let stats = unsafe {
            ffi::b2DynamicTree_QueryAll(
                &self.raw,
                aabb.into_raw(),
                Some(query_cb::<F>),
                &mut ctx as *mut _ as *mut _,
            )
        };
        ctx.finish();
        Ok(TreeStats::from_raw(stats))
    }

    /// Ray cast against tree proxies.
    pub fn ray_cast<F>(
        &self,
        input: TreeRayCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> TreeStats
    where
        F: FnMut(TreeRayCastInput, TreeProxyId, u64) -> f32,
    {
        assert!(
            input.validate().is_ok(),
            "ray cast input must be valid, got {:?}",
            input
        );
        self.try_ray_cast(input, mask_bits, callback)
            .expect("validated dynamic tree ray cast")
    }

    /// Ray cast against tree proxies with recoverable validation.
    pub fn try_ray_cast<F>(
        &self,
        input: TreeRayCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> ApiResult<TreeStats>
    where
        F: FnMut(TreeRayCastInput, TreeProxyId, u64) -> f32,
    {
        input.validate()?;
        let raw_input = input.into_raw();
        let mut ctx = RayCastCtx::new(callback);
        let stats = unsafe {
            ffi::b2DynamicTree_RayCast(
                &self.raw,
                &raw_input,
                mask_bits,
                Some(ray_cast_cb::<F>),
                &mut ctx as *mut _ as *mut _,
            )
        };
        ctx.finish();
        Ok(TreeStats::from_raw(stats))
    }

    /// Cast a swept AABB against tree proxies.
    pub fn box_cast<F>(
        &self,
        input: TreeBoxCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> TreeStats
    where
        F: FnMut(TreeBoxCastInput, TreeProxyId, u64) -> f32,
    {
        self.try_box_cast(input, mask_bits, callback)
            .expect("validated dynamic tree box cast")
    }

    /// Cast a swept AABB against tree proxies with recoverable validation.
    pub fn try_box_cast<F>(
        &self,
        input: TreeBoxCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> ApiResult<TreeStats>
    where
        F: FnMut(TreeBoxCastInput, TreeProxyId, u64) -> f32,
    {
        input.validate()?;
        let raw_input = input.into_raw();
        let mut ctx = BoxCastCtx::new(callback);
        let stats = unsafe {
            ffi::b2DynamicTree_BoxCast(
                &self.raw,
                &raw_input,
                mask_bits,
                Some(box_cast_cb::<F>),
                &mut ctx as *mut _ as *mut _,
            )
        };
        ctx.finish();
        Ok(TreeStats::from_raw(stats))
    }

    /// Get the binary tree height.
    #[inline]
    pub fn height(&self) -> i32 {
        unsafe { ffi::b2DynamicTree_GetHeight(&self.raw) }
    }

    /// Get the ratio of summed node areas to root area.
    #[inline]
    pub fn area_ratio(&self) -> f32 {
        unsafe { ffi::b2DynamicTree_GetAreaRatio(&self.raw) }
    }

    /// Get the root bounds for the full tree.
    #[inline]
    pub fn root_bounds(&self) -> Aabb {
        Aabb::from_raw(unsafe { ffi::b2DynamicTree_GetRootBounds(&self.raw) })
    }

    /// Get the number of proxies currently created in the tree.
    #[inline]
    pub fn proxy_count(&self) -> i32 {
        unsafe { ffi::b2DynamicTree_GetProxyCount(&self.raw) }
    }

    /// Rebuild the tree and return the number of boxes sorted.
    #[inline]
    pub fn rebuild(&mut self, full_build: bool) -> i32 {
        unsafe { ffi::b2DynamicTree_Rebuild(&mut self.raw, full_build) }
    }

    /// Get the number of bytes used by this tree.
    #[inline]
    pub fn byte_count(&self) -> i32 {
        unsafe { ffi::b2DynamicTree_GetByteCount(&self.raw) }
    }

    /// Return whether a proxy id is currently owned by this tree.
    #[inline]
    pub fn contains_proxy(&self, proxy: TreeProxyId) -> bool {
        self.proxies.contains(&proxy.into_raw())
    }

    #[inline]
    fn check_proxy(&self, proxy: TreeProxyId) -> ApiResult<()> {
        self.contains_proxy(proxy)
            .then_some(())
            .ok_or(ApiError::InvalidArgument)
    }
}

impl Drop for DynamicTree {
    fn drop(&mut self) {
        unsafe {
            ffi::b2DynamicTree_Destroy(&mut self.raw);
        }
    }
}

struct QueryCtx<'a, F> {
    callback: &'a mut F,
    stopped_early: bool,
    panic: Option<PanicPayload>,
}

impl<'a, F> QueryCtx<'a, F>
where
    F: FnMut(TreeProxyId, u64) -> bool,
{
    fn new(callback: &'a mut F) -> Self {
        Self {
            callback,
            stopped_early: false,
            panic: None,
        }
    }

    fn visit(&mut self, proxy: TreeProxyId, user_data: u64) -> bool {
        if self.stopped_early || self.panic.is_some() {
            return false;
        }
        match std::panic::catch_unwind(AssertUnwindSafe(|| (self.callback)(proxy, user_data))) {
            Ok(true) => true,
            Ok(false) => {
                self.stopped_early = true;
                false
            }
            Err(panic) => {
                self.panic = Some(panic);
                false
            }
        }
    }

    fn finish(self) {
        if let Some(panic) = self.panic {
            std::panic::resume_unwind(panic);
        }
    }
}

struct CastCtx<'a, F, I> {
    callback: &'a mut F,
    panic: Option<PanicPayload>,
    _input: core::marker::PhantomData<I>,
}

type RayCastCtx<'a, F> = CastCtx<'a, F, TreeRayCastInput>;
type BoxCastCtx<'a, F> = CastCtx<'a, F, TreeBoxCastInput>;

impl<'a, F, I> CastCtx<'a, F, I> {
    fn new(callback: &'a mut F) -> Self {
        Self {
            callback,
            panic: None,
            _input: core::marker::PhantomData,
        }
    }

    fn finish(self) {
        if let Some(panic) = self.panic {
            std::panic::resume_unwind(panic);
        }
    }
}

unsafe extern "C" fn query_cb<F>(
    proxy_id: i32,
    user_data: u64,
    context: *mut core::ffi::c_void,
) -> bool
where
    F: FnMut(TreeProxyId, u64) -> bool,
{
    let ctx = unsafe { &mut *(context as *mut QueryCtx<'_, F>) };
    ctx.visit(TreeProxyId(proxy_id), user_data)
}

unsafe extern "C" fn ray_cast_cb<F>(
    input: *const ffi::b2RayCastInput,
    proxy_id: i32,
    user_data: u64,
    context: *mut core::ffi::c_void,
) -> f32
where
    F: FnMut(TreeRayCastInput, TreeProxyId, u64) -> f32,
{
    let ctx = unsafe { &mut *(context as *mut RayCastCtx<'_, F>) };
    if ctx.panic.is_some() {
        return 0.0;
    }
    let input = TreeRayCastInput::from_raw(unsafe { *input });
    match std::panic::catch_unwind(AssertUnwindSafe(|| {
        (ctx.callback)(input, TreeProxyId(proxy_id), user_data)
    })) {
        Ok(fraction) => fraction,
        Err(panic) => {
            ctx.panic = Some(panic);
            0.0
        }
    }
}

unsafe extern "C" fn box_cast_cb<F>(
    input: *const ffi::b2BoxCastInput,
    proxy_id: i32,
    user_data: u64,
    context: *mut core::ffi::c_void,
) -> f32
where
    F: FnMut(TreeBoxCastInput, TreeProxyId, u64) -> f32,
{
    let ctx = unsafe { &mut *(context as *mut BoxCastCtx<'_, F>) };
    if ctx.panic.is_some() {
        return 0.0;
    }
    let input = TreeBoxCastInput::from_raw(unsafe { *input });
    match std::panic::catch_unwind(AssertUnwindSafe(|| {
        (ctx.callback)(input, TreeProxyId(proxy_id), user_data)
    })) {
        Ok(fraction) => fraction,
        Err(panic) => {
            ctx.panic = Some(panic);
            0.0
        }
    }
}

#[inline]
fn check_aabb(aabb: Aabb) -> ApiResult<()> {
    if aabb.is_valid() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
fn check_dynamic_tree_update_aabb(aabb: Aabb) -> ApiResult<()> {
    check_aabb(aabb)?;
    #[cfg(feature = "double-precision")]
    let huge_factor = 1.0e9_f32;
    #[cfg(not(feature = "double-precision"))]
    let huge_factor = 1.0e5_f32;
    let huge = huge_factor * crate::length_units_per_meter();
    let width = aabb.upper.x - aabb.lower.x;
    let height = aabb.upper.y - aabb.lower.y;
    if width < huge && height < huge {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
fn aabb_contains(outer: Aabb, inner: Aabb) -> bool {
    outer.lower.x <= inner.lower.x
        && outer.lower.y <= inner.lower.y
        && inner.upper.x <= outer.upper.x
        && inner.upper.y <= outer.upper.y
}

#[inline]
fn check_vec2(value: Vec2) -> ApiResult<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
fn check_fraction(value: f32) -> ApiResult<()> {
    if crate::is_valid_float(value) && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

const fn maximum_tree_node_capacity_for(address_limit: usize, node_size: usize) -> usize {
    let byte_limited = address_limit / node_size;
    if byte_limited < i32::MAX as usize {
        byte_limited
    } else {
        i32::MAX as usize
    }
}

const fn maximum_proxy_capacity_for(address_limit: usize, node_size: usize) -> usize {
    let max_nodes = maximum_tree_node_capacity_for(address_limit, node_size);
    let byte_limited = max_nodes.div_ceil(2);
    let arithmetic_limited = (i32::MAX as usize) / 2;
    if byte_limited < arithmetic_limited {
        byte_limited
    } else {
        arithmetic_limited
    }
}

#[inline]
fn check_proxy_node_reserve(tree: &ffi::b2DynamicTree) -> ApiResult<()> {
    check_proxy_node_reserve_fields(tree.nodeCapacity, tree.nodeCount, tree.proxyCount)
}

fn check_proxy_node_reserve_fields(
    node_capacity: i32,
    node_count: i32,
    proxy_count: i32,
) -> ApiResult<()> {
    let node_capacity = usize::try_from(node_capacity).map_err(|_| ApiError::InvalidArgument)?;
    let node_count = usize::try_from(node_count).map_err(|_| ApiError::InvalidArgument)?;
    let available = node_capacity
        .checked_sub(node_count)
        .ok_or(ApiError::InvalidArgument)?;
    let required = if proxy_count == 0 { 1 } else { 2 };
    if available >= required {
        return Ok(());
    }

    let grown = node_capacity
        .checked_add(node_capacity / 2)
        .ok_or(ApiError::InvalidArgument)?;
    let maximum = maximum_tree_node_capacity_for(
        isize::MAX as usize,
        core::mem::size_of::<ffi::b2TreeNode>(),
    );
    if grown <= maximum {
        Ok(())
    } else {
        Err(ApiError::InvalidArgument)
    }
}

#[inline]
fn check_proxy_capacity(proxy_capacity: usize) -> ApiResult<i32> {
    if proxy_capacity > DynamicTree::MAX_PROXY_CAPACITY {
        return Err(ApiError::InvalidArgument);
    }
    i32::try_from(proxy_capacity).map_err(|_| ApiError::InvalidArgument)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_capacity_guard_accounts_for_native_arithmetic_and_allocation() {
        let maximum = DynamicTree::MAX_PROXY_CAPACITY;
        assert_eq!(check_proxy_capacity(maximum), Ok(maximum as i32));
        assert_eq!(
            check_proxy_capacity(maximum + 1),
            Err(ApiError::InvalidArgument)
        );
        assert!(2_i32.checked_mul(maximum as i32).is_some());

        let node_size = core::mem::size_of::<ffi::b2TreeNode>();
        let node_count = 2 * maximum - 1;
        assert!(node_count <= i32::MAX as usize);
        assert!(
            node_count
                .checked_mul(node_size)
                .is_some_and(|bytes| { bytes <= isize::MAX as usize })
        );
    }

    #[test]
    fn proxy_capacity_formula_is_safe_for_a_32_bit_address_model() {
        let address_limit = i32::MAX as usize;
        let node_size = core::mem::size_of::<ffi::b2TreeNode>();
        let maximum = maximum_proxy_capacity_for(address_limit, node_size);
        let max_nodes = 2 * maximum - 1;

        assert!(max_nodes * node_size <= address_limit);
        let next_nodes = 2 * (maximum + 1) - 1;
        assert!(
            next_nodes > i32::MAX as usize || next_nodes * node_size > address_limit,
            "the formula must select the largest safe proxy capacity"
        );
    }

    #[test]
    fn proxy_creation_rejects_an_unsafe_native_growth_step() {
        let maximum = maximum_tree_node_capacity_for(
            isize::MAX as usize,
            core::mem::size_of::<ffi::b2TreeNode>(),
        );
        assert_eq!(
            check_proxy_node_reserve_fields(maximum as i32, maximum as i32, 1),
            Err(ApiError::InvalidArgument)
        );
    }
}
