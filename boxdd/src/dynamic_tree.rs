//! Safe wrapper for Box2D's standalone dynamic AABB tree.
//!
//! The dynamic tree can organize spatial data that is not part of a Box2D world.
//! Proxies store an AABB, category bits, and an opaque `u64` user data value.

use core::{
    fmt,
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};
use std::collections::HashMap;

use boxdd_sys::ffi;

#[cfg(not(target_arch = "wasm32"))]
use crate::core::callback_state::{OwnerCallScope, PanicSlot, invoke_owner_callback};
use crate::{
    error::{ApiError, ApiResult},
    query::Aabb,
    types::Vec2,
};

static NEXT_TREE_TOKEN: AtomicU64 = AtomicU64::new(1);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct TreeToken(NonZeroU64);

impl TreeToken {
    fn allocate() -> ApiResult<Self> {
        Self::allocate_from(&NEXT_TREE_TOKEN)
    }

    fn allocate_from(next: &AtomicU64) -> ApiResult<Self> {
        let value = next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| ApiError::TreeIdentityExhausted)?;
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(ApiError::TreeIdentityExhausted)
    }
}

impl fmt::Debug for TreeToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TreeToken(..)")
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct TreeProxyNonce(NonZeroU64);

impl TreeProxyNonce {
    fn allocate(last: &mut u64) -> ApiResult<Self> {
        let value = last
            .checked_add(1)
            .ok_or(ApiError::ObjectIdentityExhausted)?;
        let nonce = NonZeroU64::new(value).ok_or(ApiError::ObjectIdentityExhausted)?;
        *last = value;
        Ok(Self(nonce))
    }
}

impl fmt::Debug for TreeProxyNonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TreeProxyNonce(..)")
    }
}

/// A live proxy identifier bound to one [`DynamicTree`] registration.
///
/// Values are issued only by [`DynamicTree::create_proxy`] and traversal callbacks. An identifier
/// becomes stale when its proxy is destroyed or replaced, and native slot reuse does not revive
/// it. Raw native proxy integers cannot be converted into this safe capability.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TreeProxyId {
    tree: TreeToken,
    slot: i32,
    nonce: TreeProxyNonce,
}

impl TreeProxyId {
    #[inline]
    const fn bind(tree: TreeToken, slot: i32, nonce: TreeProxyNonce) -> Self {
        Self { tree, slot, nonce }
    }

    #[inline]
    const fn raw_slot(self) -> i32 {
        self.slot
    }
}

impl fmt::Debug for TreeProxyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeProxyId")
            .field("slot", &self.slot)
            .field("tree", &self.tree)
            .field("nonce", &self.nonce)
            .finish()
    }
}

/// Dynamic tree traversal performance counters.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct TreeStats {
    pub node_visits: i32,
    pub leaf_visits: i32,
}

/// Controls how a dynamic-tree cast proceeds after visiting one proxy.
///
/// This mirrors Box2D's callback protocol without exposing its magic `f32` sentinels. A
/// [`TreeCastControl::Clip`] fraction must be finite, greater than zero, and no greater than the
/// clipped input's `max_fraction`. Invalid clip values panic after the native traversal has
/// stopped, so they never enter Box2D.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TreeCastControl {
    /// Stop the cast immediately.
    Terminate,
    /// Ignore this proxy and continue without changing the current cast fraction.
    Skip,
    /// Continue without changing the current cast fraction.
    Continue,
    /// Clip subsequent traversal to this fraction of the original translation.
    Clip(f32),
}

impl TreeCastControl {
    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    fn into_raw(self, max_fraction: f32) -> f32 {
        match self {
            Self::Terminate => 0.0,
            Self::Skip => -1.0,
            Self::Continue => max_fraction,
            Self::Clip(fraction) => {
                assert!(
                    fraction.is_finite() && fraction > 0.0 && fraction <= max_fraction,
                    "dynamic-tree clip fraction must be finite and in (0, {max_fraction}], got {fraction}"
                );
                fraction
            }
        }
    }
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
    identity: TreeToken,
    proxies: HashMap<i32, TreeProxyNonce>,
    last_proxy_nonce: u64,
    // The native tree owns allocator-backed state between calls. Keep a shared foundation lease
    // for its entire lifetime so replay cannot mutate process-global state before this owner drops.
    foundation_lease: Option<crate::core::foundation::TransientFoundationLease>,
    #[cfg(test)]
    destroy_probe: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
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
            .expect("dynamic tree creation must satisfy capacity and identity constraints")
    }

    /// Create an empty dynamic tree with recoverable capacity validation.
    #[inline]
    pub fn try_with_capacity(proxy_capacity: usize) -> ApiResult<Self> {
        let proxy_capacity = check_proxy_capacity(proxy_capacity)?;
        let foundation_lease = crate::core::foundation::transient_native_lease()?;
        let identity = TreeToken::allocate()?;
        Ok(Self {
            raw: unsafe { ffi::b2DynamicTree_Create(proxy_capacity) },
            identity,
            proxies: HashMap::new(),
            last_proxy_nonce: 0,
            foundation_lease: Some(foundation_lease),
            #[cfg(test)]
            destroy_probe: None,
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
        self.proxies
            .try_reserve(1)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        let nonce = TreeProxyNonce::allocate(&mut self.last_proxy_nonce)?;
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        let slot = unsafe {
            ffi::b2DynamicTree_CreateProxy(&mut self.raw, aabb.into_raw(), category_bits, user_data)
        };
        let previous = self.proxies.insert(slot, nonce);
        assert!(
            previous.is_none(),
            "Box2D returned a proxy slot that is already live"
        );
        Ok(TreeProxyId::bind(self.identity, slot, nonce))
    }

    /// Destroy a proxy owned by this tree.
    pub fn destroy_proxy(&mut self, proxy: TreeProxyId) {
        self.try_destroy_proxy(proxy)
            .expect("proxy id must belong to this dynamic tree");
    }

    /// Destroy a proxy with recoverable validation.
    pub fn try_destroy_proxy(&mut self, proxy: TreeProxyId) -> ApiResult<()> {
        let slot = self.check_proxy(proxy)?;
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        unsafe {
            ffi::b2DynamicTree_DestroyProxy(&mut self.raw, slot);
        }
        let removed = self.proxies.remove(&slot);
        debug_assert_eq!(removed, Some(proxy.nonce));
        Ok(())
    }

    /// Move a proxy to a new AABB by removing and reinserting it.
    pub fn move_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) {
        self.try_move_proxy(proxy, aabb)
            .expect("proxy id and AABB must satisfy Box2D dynamic-tree constraints");
    }

    /// Move a proxy with recoverable validation.
    pub fn try_move_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) -> ApiResult<()> {
        let slot = self.check_proxy(proxy)?;
        check_dynamic_tree_update_aabb(aabb)?;
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        unsafe {
            ffi::b2DynamicTree_MoveProxy(&mut self.raw, slot, aabb.into_raw());
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
        let slot = self.check_proxy(proxy)?;
        check_dynamic_tree_update_aabb(aabb)?;
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        let current = Aabb::from_raw(unsafe { ffi::b2DynamicTree_GetAABB(&self.raw, slot) });
        if aabb_contains(current, aabb) {
            return Err(ApiError::InvalidArgument);
        }
        unsafe {
            ffi::b2DynamicTree_EnlargeProxy(&mut self.raw, slot, aabb.into_raw());
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
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
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
        let slot = self.check_proxy(proxy)?;
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        Ok(unsafe { ffi::b2DynamicTree_GetCategoryBits(&mut self.raw, slot) })
    }

    /// Get proxy user data.
    pub fn user_data(&self, proxy: TreeProxyId) -> u64 {
        self.try_user_data(proxy)
            .expect("proxy id must belong to this dynamic tree")
    }

    /// Get proxy user data with recoverable validation.
    pub fn try_user_data(&self, proxy: TreeProxyId) -> ApiResult<u64> {
        let slot = self.check_proxy(proxy)?;
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        Ok(unsafe { ffi::b2DynamicTree_GetUserData(&self.raw, slot) })
    }

    /// Get a proxy's current AABB.
    pub fn aabb(&self, proxy: TreeProxyId) -> Aabb {
        self.try_aabb(proxy)
            .expect("proxy id must belong to this dynamic tree")
    }

    /// Get a proxy's current AABB with recoverable validation.
    pub fn try_aabb(&self, proxy: TreeProxyId) -> ApiResult<Aabb> {
        let slot = self.check_proxy(proxy)?;
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        Ok(Aabb::from_raw(unsafe {
            ffi::b2DynamicTree_GetAABB(&self.raw, slot)
        }))
    }

    /// Query proxies overlapping `aabb`, applying category mask bits.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn query<F>(&self, aabb: Aabb, mask_bits: u64, visit: &mut F) -> TreeStats
    where
        F: FnMut(TreeProxyId, u64) -> bool,
    {
        assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
        self.try_query(aabb, mask_bits, visit)
            .expect("validated dynamic tree query")
    }

    /// Query proxies overlapping `aabb` with recoverable validation.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_query<F>(&self, aabb: Aabb, mask_bits: u64, visit: &mut F) -> ApiResult<TreeStats>
    where
        F: FnMut(TreeProxyId, u64) -> bool,
    {
        check_aabb(aabb)?;
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        let owner_scope = OwnerCallScope::enter();
        let mut ctx = QueryCtx::new(self.proxy_resolver(), visit);
        let stats = unsafe {
            ffi::b2DynamicTree_Query(
                &self.raw,
                aabb.into_raw(),
                mask_bits,
                Some(query_cb::<F>),
                &mut ctx as *mut _ as *mut _,
            )
        };
        let value = TreeStats::from_raw(stats);
        Ok(owner_scope.finish_captured(Some(value), ctx.into_panic(), std::iter::empty()))
    }

    /// Query proxies overlapping `aabb` without category filtering.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn query_all<F>(&self, aabb: Aabb, visit: &mut F) -> TreeStats
    where
        F: FnMut(TreeProxyId, u64) -> bool,
    {
        assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
        self.try_query_all(aabb, visit)
            .expect("validated dynamic tree query")
    }

    /// Query proxies overlapping `aabb` without category filtering and with recoverable validation.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_query_all<F>(&self, aabb: Aabb, visit: &mut F) -> ApiResult<TreeStats>
    where
        F: FnMut(TreeProxyId, u64) -> bool,
    {
        check_aabb(aabb)?;
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        let owner_scope = OwnerCallScope::enter();
        let mut ctx = QueryCtx::new(self.proxy_resolver(), visit);
        let stats = unsafe {
            ffi::b2DynamicTree_QueryAll(
                &self.raw,
                aabb.into_raw(),
                Some(query_cb::<F>),
                &mut ctx as *mut _ as *mut _,
            )
        };
        let value = TreeStats::from_raw(stats);
        Ok(owner_scope.finish_captured(Some(value), ctx.into_panic(), std::iter::empty()))
    }

    /// Ray cast against tree proxies.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn ray_cast<F>(
        &self,
        input: TreeRayCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> TreeStats
    where
        F: FnMut(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl,
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
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_ray_cast<F>(
        &self,
        input: TreeRayCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> ApiResult<TreeStats>
    where
        F: FnMut(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl,
    {
        input.validate()?;
        let raw_input = input.into_raw();
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        let owner_scope = OwnerCallScope::enter();
        let mut ctx = RayCastCtx::new(self.proxy_resolver(), callback);
        let stats = unsafe {
            ffi::b2DynamicTree_RayCast(
                &self.raw,
                &raw_input,
                mask_bits,
                Some(ray_cast_cb::<F>),
                &mut ctx as *mut _ as *mut _,
            )
        };
        let value = TreeStats::from_raw(stats);
        Ok(owner_scope.finish_captured(Some(value), ctx.into_panic(), std::iter::empty()))
    }

    /// Cast a swept AABB against tree proxies.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn box_cast<F>(
        &self,
        input: TreeBoxCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> TreeStats
    where
        F: FnMut(TreeBoxCastInput, TreeProxyId, u64) -> TreeCastControl,
    {
        self.try_box_cast(input, mask_bits, callback)
            .expect("validated dynamic tree box cast")
    }

    /// Cast a swept AABB against tree proxies with recoverable validation.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_box_cast<F>(
        &self,
        input: TreeBoxCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> ApiResult<TreeStats>
    where
        F: FnMut(TreeBoxCastInput, TreeProxyId, u64) -> TreeCastControl,
    {
        input.validate()?;
        let raw_input = input.into_raw();
        let _foundation_lease = crate::core::foundation::transient_native_lease()?;
        let owner_scope = OwnerCallScope::enter();
        let mut ctx = BoxCastCtx::new(self.proxy_resolver(), callback);
        let stats = unsafe {
            ffi::b2DynamicTree_BoxCast(
                &self.raw,
                &raw_input,
                mask_bits,
                Some(box_cast_cb::<F>),
                &mut ctx as *mut _ as *mut _,
            )
        };
        let value = TreeStats::from_raw(stats);
        Ok(owner_scope.finish_captured(Some(value), ctx.into_panic(), std::iter::empty()))
    }

    /// Get the binary tree height.
    #[inline]
    pub fn height(&self) -> i32 {
        let _foundation_lease = crate::core::foundation::assert_transient_native_lease();
        unsafe { ffi::b2DynamicTree_GetHeight(&self.raw) }
    }

    /// Get the ratio of summed node areas to root area.
    #[inline]
    pub fn area_ratio(&self) -> f32 {
        let _foundation_lease = crate::core::foundation::assert_transient_native_lease();
        unsafe { ffi::b2DynamicTree_GetAreaRatio(&self.raw) }
    }

    /// Get the root bounds for the full tree.
    #[inline]
    pub fn root_bounds(&self) -> Aabb {
        let _foundation_lease = crate::core::foundation::assert_transient_native_lease();
        Aabb::from_raw(unsafe { ffi::b2DynamicTree_GetRootBounds(&self.raw) })
    }

    /// Get the number of proxies currently created in the tree.
    #[inline]
    pub fn proxy_count(&self) -> i32 {
        let _foundation_lease = crate::core::foundation::assert_transient_native_lease();
        unsafe { ffi::b2DynamicTree_GetProxyCount(&self.raw) }
    }

    /// Rebuild the tree and return the number of boxes sorted.
    #[inline]
    pub fn rebuild(&mut self, full_build: bool) -> i32 {
        let _foundation_lease = crate::core::foundation::assert_transient_native_lease();
        unsafe { ffi::b2DynamicTree_Rebuild(&mut self.raw, full_build) }
    }

    /// Get the number of bytes used by this tree.
    #[inline]
    pub fn byte_count(&self) -> i32 {
        let _foundation_lease = crate::core::foundation::assert_transient_native_lease();
        unsafe { ffi::b2DynamicTree_GetByteCount(&self.raw) }
    }

    /// Validate the native tree invariants.
    ///
    /// This is primarily useful in tests and diagnostics. Box2D reports invariant failures
    /// through its configured assertion callback.
    #[inline]
    pub fn validate(&self) {
        let _foundation_lease = crate::core::foundation::assert_transient_native_lease();
        unsafe { ffi::b2DynamicTree_Validate(&self.raw) };
    }

    /// Validate that no tree node remains marked as enlarged.
    ///
    /// This is primarily useful after rebuild operations in tests and diagnostics.
    #[inline]
    pub fn validate_no_enlarged(&self) {
        let _foundation_lease = crate::core::foundation::assert_transient_native_lease();
        unsafe { ffi::b2DynamicTree_ValidateNoEnlarged(&self.raw) };
    }

    /// Return whether a proxy id is currently owned by this tree.
    #[inline]
    pub fn contains_proxy(&self, proxy: TreeProxyId) -> bool {
        self.check_proxy(proxy).is_ok()
    }

    #[inline]
    fn check_proxy(&self, proxy: TreeProxyId) -> ApiResult<i32> {
        if proxy.tree != self.identity {
            return Err(ApiError::WrongTree);
        }

        self.proxies
            .get(&proxy.raw_slot())
            .is_some_and(|nonce| *nonce == proxy.nonce)
            .then_some(proxy.raw_slot())
            .ok_or(ApiError::InvalidTreeProxyId)
    }

    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    fn proxy_resolver(&self) -> ProxyResolver<'_> {
        ProxyResolver {
            tree: self.identity,
            proxies: &self.proxies,
        }
    }
}

impl Drop for DynamicTree {
    fn drop(&mut self) {
        let mut raw = self.raw;
        let foundation_lease = self
            .foundation_lease
            .take()
            .expect("a dynamic tree owns exactly one foundation lease");
        #[cfg(test)]
        let destroy_probe = self.destroy_probe.take();
        let cleanup = move || {
            unsafe {
                ffi::b2DynamicTree_Destroy(&mut raw);
            }
            #[cfg(test)]
            if let Some(probe) = destroy_probe {
                probe.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            drop(foundation_lease);
        };

        if crate::core::callback_state::in_callback() {
            crate::core::callback_state::defer_callback_cleanup_or_forget(cleanup);
        } else {
            cleanup();
        }
    }
}

#[derive(Copy, Clone)]
#[cfg(not(target_arch = "wasm32"))]
struct ProxyResolver<'a> {
    tree: TreeToken,
    proxies: &'a HashMap<i32, TreeProxyNonce>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ProxyResolver<'_> {
    #[inline]
    fn resolve(self, slot: i32) -> Option<TreeProxyId> {
        self.proxies
            .get(&slot)
            .copied()
            .map(|nonce| TreeProxyId::bind(self.tree, slot, nonce))
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct QueryCtx<'tree, 'callback, F> {
    proxies: ProxyResolver<'tree>,
    callback: &'callback mut F,
    stopped_early: bool,
    panic: PanicSlot,
}

#[cfg(not(target_arch = "wasm32"))]
impl<'tree, 'callback, F> QueryCtx<'tree, 'callback, F>
where
    F: FnMut(TreeProxyId, u64) -> bool,
{
    fn new(proxies: ProxyResolver<'tree>, callback: &'callback mut F) -> Self {
        Self {
            proxies,
            callback,
            stopped_early: false,
            panic: PanicSlot::default(),
        }
    }

    fn visit(&mut self, slot: i32, user_data: u64) -> bool {
        if self.stopped_early || self.panic.has_panicked() {
            return false;
        }
        let proxies = self.proxies;
        let keep_going = invoke_owner_callback(&mut self.panic, false, || {
            let proxy = proxies
                .resolve(slot)
                .expect("Box2D returned an unregistered dynamic-tree proxy");
            (self.callback)(proxy, user_data)
        });
        if !keep_going {
            self.stopped_early = true;
        }
        keep_going
    }

    fn into_panic(self) -> PanicSlot {
        self.panic
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct CastCtx<'tree, 'callback, F, I> {
    proxies: ProxyResolver<'tree>,
    callback: &'callback mut F,
    panic: PanicSlot,
    _input: core::marker::PhantomData<I>,
}

#[cfg(not(target_arch = "wasm32"))]
type RayCastCtx<'tree, 'callback, F> = CastCtx<'tree, 'callback, F, TreeRayCastInput>;
#[cfg(not(target_arch = "wasm32"))]
type BoxCastCtx<'tree, 'callback, F> = CastCtx<'tree, 'callback, F, TreeBoxCastInput>;

#[cfg(not(target_arch = "wasm32"))]
impl<'tree, 'callback, F, I> CastCtx<'tree, 'callback, F, I> {
    fn new(proxies: ProxyResolver<'tree>, callback: &'callback mut F) -> Self {
        Self {
            proxies,
            callback,
            panic: PanicSlot::default(),
            _input: core::marker::PhantomData,
        }
    }

    fn into_panic(self) -> PanicSlot {
        self.panic
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn query_cb<F>(
    proxy_id: i32,
    user_data: u64,
    context: *mut core::ffi::c_void,
) -> bool
where
    F: FnMut(TreeProxyId, u64) -> bool,
{
    let ctx = unsafe { &mut *(context as *mut QueryCtx<'_, '_, F>) };
    ctx.visit(proxy_id, user_data)
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn ray_cast_cb<F>(
    input: *const ffi::b2RayCastInput,
    proxy_id: i32,
    user_data: u64,
    context: *mut core::ffi::c_void,
) -> f32
where
    F: FnMut(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl,
{
    let ctx = unsafe { &mut *(context as *mut RayCastCtx<'_, '_, F>) };
    let input = TreeRayCastInput::from_raw(unsafe { *input });
    let proxies = ctx.proxies;
    let control = invoke_owner_callback(&mut ctx.panic, TreeCastControl::Terminate, || {
        let proxy = proxies
            .resolve(proxy_id)
            .expect("Box2D returned an unregistered dynamic-tree proxy");
        (ctx.callback)(input, proxy, user_data)
    });
    invoke_owner_callback(&mut ctx.panic, 0.0, || control.into_raw(input.max_fraction))
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn box_cast_cb<F>(
    input: *const ffi::b2BoxCastInput,
    proxy_id: i32,
    user_data: u64,
    context: *mut core::ffi::c_void,
) -> f32
where
    F: FnMut(TreeBoxCastInput, TreeProxyId, u64) -> TreeCastControl,
{
    let ctx = unsafe { &mut *(context as *mut BoxCastCtx<'_, '_, F>) };
    let input = TreeBoxCastInput::from_raw(unsafe { *input });
    let proxies = ctx.proxies;
    let control = invoke_owner_callback(&mut ctx.panic, TreeCastControl::Terminate, || {
        let proxy = proxies
            .resolve(proxy_id)
            .expect("Box2D returned an unregistered dynamic-tree proxy");
        (ctx.callback)(input, proxy, user_data)
    });
    invoke_owner_callback(&mut ctx.panic, 0.0, || control.into_raw(input.max_fraction))
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
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    static_assertions::assert_not_impl_any!(DynamicTree: Send, Sync);

    fn callback_proxy_resolver(
        proxies: &mut HashMap<i32, TreeProxyNonce>,
        slot: i32,
    ) -> ProxyResolver<'_> {
        let mut last_nonce = 0;
        let nonce = TreeProxyNonce::allocate(&mut last_nonce).unwrap();
        proxies.insert(slot, nonce);
        ProxyResolver {
            tree: TreeToken::allocate().unwrap(),
            proxies,
        }
    }

    fn recycled_proxy_fixture() -> (DynamicTree, TreeProxyId, TreeProxyId, Aabb) {
        let aabb = Aabb::from_center_half_extents([0.0, 0.0], [1.0, 1.0]);
        let mut tree = DynamicTree::new();
        let stale = tree.create_proxy(aabb, u64::MAX, 7);
        tree.destroy_proxy(stale);
        let live = tree.create_proxy(aabb, u64::MAX, 11);
        assert_eq!(stale.raw_slot(), live.raw_slot());
        assert_ne!(stale, live);
        (tree, stale, live, aabb)
    }

    fn equal_slot_foreign_fixture() -> (DynamicTree, TreeProxyId, TreeProxyId, Aabb) {
        let aabb = Aabb::from_center_half_extents([0.0, 0.0], [1.0, 1.0]);
        let mut first = DynamicTree::new();
        let foreign = first.create_proxy(aabb, u64::MAX, 7);
        let mut second = DynamicTree::new();
        let local = second.create_proxy(aabb, u64::MAX, 11);
        assert_eq!(foreign.raw_slot(), local.raw_slot());
        assert_ne!(foreign.tree, local.tree);
        drop(first);
        (second, foreign, local, aabb)
    }

    fn assert_live_proxy_unchanged(tree: &mut DynamicTree, proxy: TreeProxyId, aabb: Aabb) {
        assert!(tree.contains_proxy(proxy));
        assert_eq!(tree.try_user_data(proxy), Ok(11));
        assert_eq!(tree.try_aabb(proxy), Ok(aabb));
        assert_eq!(tree.proxy_count(), 1);
    }

    #[test]
    fn tree_identity_and_proxy_nonce_exhaustion_do_not_wrap() {
        let next_tree = AtomicU64::new(u64::MAX);
        assert_eq!(
            TreeToken::allocate_from(&next_tree),
            Err(ApiError::TreeIdentityExhausted)
        );
        assert_eq!(next_tree.load(Ordering::Relaxed), u64::MAX);

        let mut last_nonce = u64::MAX;
        assert_eq!(
            TreeProxyNonce::allocate(&mut last_nonce),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(last_nonce, u64::MAX);
    }

    #[test]
    fn equal_native_slots_from_another_tree_are_rejected_before_native_access() {
        let replacement_aabb = Aabb::from_center_half_extents([4.0, 4.0], [1.0, 1.0]);

        let (mut tree, foreign, live, original_aabb) = equal_slot_foreign_fixture();
        assert!(!tree.contains_proxy(foreign));
        assert_eq!(tree.try_destroy_proxy(foreign), Err(ApiError::WrongTree));
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, foreign, live, original_aabb) = equal_slot_foreign_fixture();
        assert_eq!(
            tree.try_move_proxy(foreign, replacement_aabb),
            Err(ApiError::WrongTree)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, foreign, live, original_aabb) = equal_slot_foreign_fixture();
        assert_eq!(
            tree.try_enlarge_proxy(foreign, replacement_aabb),
            Err(ApiError::WrongTree)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, foreign, live, original_aabb) = equal_slot_foreign_fixture();
        assert_eq!(
            tree.try_replace_category_bits(foreign, 0b10),
            Err(ApiError::WrongTree)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, foreign, live, original_aabb) = equal_slot_foreign_fixture();
        assert_eq!(tree.try_category_bits(foreign), Err(ApiError::WrongTree));
        assert_eq!(tree.try_user_data(foreign), Err(ApiError::WrongTree));
        assert_eq!(tree.try_aabb(foreign), Err(ApiError::WrongTree));
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);
    }

    #[test]
    fn recycled_native_slots_do_not_revive_stale_proxy_ids() {
        let replacement_aabb = Aabb::from_center_half_extents([4.0, 4.0], [1.0, 1.0]);

        let (mut tree, stale, live, original_aabb) = recycled_proxy_fixture();
        assert!(!tree.contains_proxy(stale));
        assert_eq!(
            tree.try_destroy_proxy(stale),
            Err(ApiError::InvalidTreeProxyId)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, stale, live, original_aabb) = recycled_proxy_fixture();
        assert_eq!(
            tree.try_move_proxy(stale, replacement_aabb),
            Err(ApiError::InvalidTreeProxyId)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, stale, live, original_aabb) = recycled_proxy_fixture();
        assert_eq!(
            tree.try_enlarge_proxy(stale, replacement_aabb),
            Err(ApiError::InvalidTreeProxyId)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, stale, live, original_aabb) = recycled_proxy_fixture();
        assert_eq!(
            tree.try_replace_category_bits(stale, 0b10),
            Err(ApiError::InvalidTreeProxyId)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, stale, live, original_aabb) = recycled_proxy_fixture();
        assert_eq!(
            tree.try_category_bits(stale),
            Err(ApiError::InvalidTreeProxyId)
        );
        assert_eq!(tree.try_user_data(stale), Err(ApiError::InvalidTreeProxyId));
        assert_eq!(tree.try_aabb(stale), Err(ApiError::InvalidTreeProxyId));
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);
    }

    #[test]
    fn dropping_a_tree_cannot_rebind_its_proxy_to_a_new_tree() {
        let aabb = Aabb::from_center_half_extents([0.0, 0.0], [1.0, 1.0]);
        let (stale, stale_slot) = {
            let mut tree = DynamicTree::new();
            let proxy = tree.create_proxy(aabb, u64::MAX, 7);
            (proxy, proxy.raw_slot())
        };

        let mut replacement = DynamicTree::new();
        let live = replacement.create_proxy(aabb, u64::MAX, 11);
        assert_eq!(stale_slot, live.raw_slot());
        assert_eq!(replacement.try_user_data(stale), Err(ApiError::WrongTree));
        assert_live_proxy_unchanged(&mut replacement, live, aabb);
    }

    #[test]
    fn query_defers_another_tree_destruction_until_the_native_callback_returns() {
        let aabb = Aabb::from_center_half_extents([0.0, 0.0], [1.0, 1.0]);
        let mut query_tree = DynamicTree::new();
        query_tree.create_proxy(aabb, u64::MAX, 7);

        let destroyed = Arc::new(AtomicBool::new(false));
        let mut doomed_tree = DynamicTree::new();
        doomed_tree.destroy_probe = Some(Arc::clone(&destroyed));
        let mut doomed_tree = Some(doomed_tree);

        let mut visit = |_: TreeProxyId, _: u64| {
            drop(doomed_tree.take());
            assert!(!destroyed.load(Ordering::SeqCst));
            false
        };
        query_tree.query_all(aabb, &mut visit);

        assert!(destroyed.load(Ordering::SeqCst));
    }

    #[test]
    fn panicking_tree_callbacks_return_native_stop_sentinels() {
        fn panic_query(_: TreeProxyId, _: u64) -> bool {
            panic!("tree query test panic");
        }
        fn panic_ray(_: TreeRayCastInput, _: TreeProxyId, _: u64) -> TreeCastControl {
            panic!("tree ray-cast test panic");
        }
        fn panic_box(_: TreeBoxCastInput, _: TreeProxyId, _: u64) -> TreeCastControl {
            panic!("tree box-cast test panic");
        }

        let mut proxies = HashMap::new();
        let resolver = callback_proxy_resolver(&mut proxies, 1);

        let mut query: fn(TreeProxyId, u64) -> bool = panic_query;
        let mut query_context = QueryCtx::new(resolver, &mut query);
        let query_context_ptr = core::ptr::from_mut(&mut query_context).cast::<core::ffi::c_void>();
        // SAFETY: the callback context remains live for both synchronous invocations.
        let first = unsafe { query_cb::<fn(TreeProxyId, u64) -> bool>(1, 7, query_context_ptr) };
        let second = unsafe { query_cb::<fn(TreeProxyId, u64) -> bool>(1, 7, query_context_ptr) };
        assert!(!first);
        assert!(!second);
        assert!(query_context.panic.has_panicked());

        let ray_input = TreeRayCastInput::new([-4.0, 1.0], [10.0, 0.0]).into_raw();
        let mut ray: fn(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl = panic_ray;
        let mut ray_context: RayCastCtx<'_, '_, _> = CastCtx::new(resolver, &mut ray);
        let ray_context_ptr = core::ptr::from_mut(&mut ray_context).cast::<core::ffi::c_void>();
        let first = unsafe {
            ray_cast_cb::<fn(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl>(
                &ray_input,
                1,
                7,
                ray_context_ptr,
            )
        };
        let second = unsafe {
            ray_cast_cb::<fn(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl>(
                &ray_input,
                1,
                7,
                ray_context_ptr,
            )
        };
        assert_eq!(first, 0.0);
        assert_eq!(second, 0.0);
        assert!(ray_context.panic.has_panicked());

        let box_input =
            TreeBoxCastInput::new(Aabb::new([-4.0, 0.5], [-3.0, 1.5]), [8.0, 0.0]).into_raw();
        let mut box_cast: fn(TreeBoxCastInput, TreeProxyId, u64) -> TreeCastControl = panic_box;
        let mut box_context: BoxCastCtx<'_, '_, _> = CastCtx::new(resolver, &mut box_cast);
        let box_context_ptr = core::ptr::from_mut(&mut box_context).cast::<core::ffi::c_void>();
        let first = unsafe {
            box_cast_cb::<fn(TreeBoxCastInput, TreeProxyId, u64) -> TreeCastControl>(
                &box_input,
                1,
                7,
                box_context_ptr,
            )
        };
        let second = unsafe {
            box_cast_cb::<fn(TreeBoxCastInput, TreeProxyId, u64) -> TreeCastControl>(
                &box_input,
                1,
                7,
                box_context_ptr,
            )
        };
        assert_eq!(first, 0.0);
        assert_eq!(second, 0.0);
        assert!(box_context.panic.has_panicked());
    }

    #[test]
    fn invalid_tree_cast_controls_stop_before_reaching_native_code() {
        fn invalid_clip(_: TreeRayCastInput, _: TreeProxyId, _: u64) -> TreeCastControl {
            TreeCastControl::Clip(f32::NAN)
        }

        let input = TreeRayCastInput::new([-4.0, 1.0], [10.0, 0.0])
            .with_max_fraction(0.5)
            .into_raw();
        let mut proxies = HashMap::new();
        let resolver = callback_proxy_resolver(&mut proxies, 1);
        let mut invalid: fn(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl = invalid_clip;
        let mut context: RayCastCtx<'_, '_, _> = CastCtx::new(resolver, &mut invalid);
        let context_ptr = core::ptr::from_mut(&mut context).cast::<core::ffi::c_void>();

        // SAFETY: `context` remains live for this synchronous trampoline invocation.
        let result = unsafe {
            ray_cast_cb::<fn(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl>(
                &input,
                1,
                7,
                context_ptr,
            )
        };

        assert_eq!(result, 0.0);
        assert!(context.panic.has_panicked());
        assert_eq!(TreeCastControl::Terminate.into_raw(0.5), 0.0);
        assert_eq!(TreeCastControl::Skip.into_raw(0.5), -1.0);
        assert_eq!(TreeCastControl::Continue.into_raw(0.5), 0.5);
        assert_eq!(TreeCastControl::Clip(0.25).into_raw(0.5), 0.25);
    }

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
