//! Safe wrapper for Box2D's standalone dynamic AABB tree.
//!
//! The dynamic tree can organize spatial data that is not part of a Box2D world.
//! Proxies store an AABB, category bits, and an opaque `u64` user data value.

use core::{
    cell::Cell,
    fmt,
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};
use std::collections::HashMap;

use boxdd_sys::ffi;

use crate::core::callback_state::CallbackOwnerToken;
#[cfg(not(target_arch = "wasm32"))]
use crate::core::callback_state::{
    PanicSlot, invoke_owner_callback, run_dynamic_tree_box_cast_boundary,
    run_dynamic_tree_query_all_boundary, run_dynamic_tree_query_boundary,
    run_dynamic_tree_ray_cast_boundary,
};
use crate::{
    error::{Error, Result},
    query::Aabb,
    types::Vec2,
};

static NEXT_TREE_TOKEN: AtomicU64 = AtomicU64::new(1);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct TreeToken(NonZeroU64);

impl TreeToken {
    fn allocate() -> Result<Self> {
        Self::allocate_from(&NEXT_TREE_TOKEN)
    }

    fn allocate_from(next: &AtomicU64) -> Result<Self> {
        let value = next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| Error::TreeIdentityExhausted)?;
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(Error::TreeIdentityExhausted)
    }

    const fn callback_owner(self) -> CallbackOwnerToken {
        CallbackOwnerToken::dynamic_tree(self.0)
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
    fn allocate(last: &mut u64) -> Result<Self> {
        let value = last.checked_add(1).ok_or(Error::ObjectIdentityExhausted)?;
        let nonce = NonZeroU64::new(value).ok_or(Error::ObjectIdentityExhausted)?;
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
    node_visits: i32,
    leaf_visits: i32,
}

/// Controls how a dynamic-tree cast proceeds after visiting one proxy.
///
/// This mirrors Box2D's callback protocol without exposing its magic `f32` sentinels. A
/// [`TreeCastControl::Clip`] fraction must be finite, greater than zero, and no greater than the
/// clipped input's `max_fraction`. Invalid clip values stop the native traversal and make the
/// enclosing cast return [`Error::InvalidArgument`].
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
    fn into_raw(self, max_fraction: f32) -> Result<f32> {
        match self {
            Self::Terminate => Ok(0.0),
            Self::Skip => Ok(-1.0),
            Self::Continue => Ok(max_fraction),
            Self::Clip(fraction)
                if fraction.is_finite() && fraction > 0.0 && fraction <= max_fraction =>
            {
                Ok(fraction)
            }
            Self::Clip(_) => Err(Error::invalid_argument(
                "TreeCastControl::Clip",
                "fraction",
                "a finite value greater than zero and no greater than max_fraction",
            )),
        }
    }
}

impl TreeStats {
    /// Construct traversal counters after validating their relationships.
    #[inline]
    pub fn new(node_visits: i32, leaf_visits: i32) -> Result<Self> {
        let stats = Self {
            node_visits,
            leaf_visits,
        };
        stats.validate_for("TreeStats::new")?;
        Ok(stats)
    }

    /// Convert Box2D traversal counters into the safe value type.
    #[inline]
    pub fn from_raw(raw: ffi::b2TreeStats) -> Result<Self> {
        let stats = Self {
            node_visits: raw.nodeVisits,
            leaf_visits: raw.leafVisits,
        };
        stats.validate_for("TreeStats::from_raw")?;
        Ok(stats)
    }

    /// Validate the traversal-counter relationships.
    #[inline]
    pub fn validate(&self) -> Result<()> {
        self.validate_for("TreeStats::validate")
    }

    fn validate_for(&self, operation: &'static str) -> Result<()> {
        if self.node_visits < 0 {
            return Err(Error::invalid_argument(
                operation,
                "node_visits",
                "a non-negative native int",
            ));
        }
        if self.leaf_visits < 0 {
            return Err(Error::invalid_argument(
                operation,
                "leaf_visits",
                "a non-negative native int no greater than node_visits",
            ));
        }
        if self.leaf_visits > self.node_visits {
            return Err(Error::invalid_argument(
                operation,
                "leaf_visits",
                "a count no greater than node_visits",
            ));
        }
        Ok(())
    }

    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    fn from_native(
        operation: &'static str,
        raw: ffi::b2TreeStats,
        proxy_count: usize,
    ) -> Result<Self> {
        let stats = Self::from_raw(raw).map_err(|_| Error::InvalidNativeOutput {
            operation,
            output: "tree_stats",
            constraint: "non-negative visit counts with leaf_visits no greater than node_visits",
        })?;
        let maximum_node_visits = if proxy_count == 0 {
            0
        } else {
            proxy_count
                .checked_mul(2)
                .and_then(|count| count.checked_sub(1))
                .ok_or(Error::InvalidNativeOutput {
                    operation,
                    output: "tree_stats.node_visits",
                    constraint: "a count bounded by the tree's representable node capacity",
                })?
        };
        let node_visits =
            usize::try_from(stats.node_visits).map_err(|_| Error::InvalidNativeOutput {
                operation,
                output: "tree_stats.node_visits",
                constraint: "a non-negative count bounded by the tree's node count",
            })?;
        let leaf_visits =
            usize::try_from(stats.leaf_visits).map_err(|_| Error::InvalidNativeOutput {
                operation,
                output: "tree_stats.leaf_visits",
                constraint: "a non-negative count bounded by the tree's proxy count",
            })?;
        if (proxy_count == 0) != (node_visits == 0) {
            return Err(Error::InvalidNativeOutput {
                operation,
                output: "tree_stats.node_visits",
                constraint: "zero exactly when the tree has no proxies",
            });
        }
        if node_visits > maximum_node_visits {
            return Err(Error::InvalidNativeOutput {
                operation,
                output: "tree_stats.node_visits",
                constraint: "a count no greater than the tree's node count",
            });
        }
        if leaf_visits > proxy_count {
            return Err(Error::InvalidNativeOutput {
                operation,
                output: "tree_stats.leaf_visits",
                constraint: "a count no greater than the tree's proxy count",
            });
        }
        Ok(stats)
    }

    /// Return the number of visited tree nodes.
    #[inline]
    pub const fn node_visits(self) -> i32 {
        self.node_visits
    }

    /// Return the number of visited leaf nodes.
    #[inline]
    pub const fn leaf_visits(self) -> i32 {
        self.leaf_visits
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
    pub(crate) origin: Vec2,
    pub(crate) translation: Vec2,
    pub(crate) max_fraction: f32,
}

impl TreeRayCastInput {
    /// Build a ray cast over `origin + translation * max_fraction`.
    #[inline]
    pub fn new<O: Into<Vec2>, T: Into<Vec2>>(origin: O, translation: T) -> Result<Self> {
        let input = Self {
            origin: origin.into(),
            translation: translation.into(),
            max_fraction: 1.0,
        };
        input.validate()?;
        Ok(input)
    }

    /// Limit the cast to a fraction of the translation.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Result<Self> {
        check_fraction(
            "TreeRayCastInput::with_max_fraction",
            "max_fraction",
            max_fraction,
        )?;
        self.max_fraction = max_fraction;
        Ok(self)
    }

    /// Validate this input before crossing the FFI boundary.
    pub fn validate(&self) -> Result<()> {
        check_vec2("TreeRayCastInput::validate", "origin", self.origin)?;
        check_vec2(
            "TreeRayCastInput::validate",
            "translation",
            self.translation,
        )?;
        check_fraction(
            "TreeRayCastInput::validate",
            "max_fraction",
            self.max_fraction,
        )
    }

    #[inline]
    /// Construct from a raw Box2D ray-cast input after validating its invariants.
    pub fn from_raw(raw: ffi::b2RayCastInput) -> Result<Self> {
        let input = Self {
            origin: Vec2::from_raw(raw.origin),
            translation: Vec2::from_raw(raw.translation),
            max_fraction: raw.maxFraction,
        };
        input.validate()?;
        Ok(input)
    }

    #[inline]
    pub const fn origin(self) -> Vec2 {
        self.origin
    }

    #[inline]
    pub const fn translation(self) -> Vec2 {
        self.translation
    }

    #[inline]
    pub const fn max_fraction(self) -> f32 {
        self.max_fraction
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
    pub(crate) aabb: Aabb,
    pub(crate) translation: Vec2,
    pub(crate) max_fraction: f32,
}

impl TreeBoxCastInput {
    /// Build an AABB cast over `aabb` moving by `translation`.
    #[inline]
    pub fn new<T: Into<Vec2>>(aabb: Aabb, translation: T) -> Result<Self> {
        let input = Self {
            aabb,
            translation: translation.into(),
            max_fraction: 1.0,
        };
        input.validate()?;
        Ok(input)
    }

    /// Limit the cast to a fraction of the translation.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Result<Self> {
        check_fraction(
            "TreeBoxCastInput::with_max_fraction",
            "max_fraction",
            max_fraction,
        )?;
        self.max_fraction = max_fraction;
        Ok(self)
    }

    /// Validate this input before crossing the FFI boundary.
    pub fn validate(&self) -> Result<()> {
        check_aabb("TreeBoxCastInput::validate", "aabb", self.aabb)?;
        check_vec2(
            "TreeBoxCastInput::validate",
            "translation",
            self.translation,
        )?;
        check_fraction(
            "TreeBoxCastInput::validate",
            "max_fraction",
            self.max_fraction,
        )
    }

    #[inline]
    /// Construct from a raw Box2D box-cast input after validating its invariants.
    pub fn from_raw(raw: ffi::b2BoxCastInput) -> Result<Self> {
        let input = Self::from_raw_unvalidated(raw);
        input.validate()?;
        Ok(input)
    }

    #[inline]
    pub(crate) fn from_raw_unvalidated(raw: ffi::b2BoxCastInput) -> Self {
        Self {
            aabb: Aabb::from_raw_unvalidated(raw.box_),
            translation: Vec2::from_raw(raw.translation),
            max_fraction: raw.maxFraction,
        }
    }

    #[inline]
    pub const fn aabb(self) -> Aabb {
        self.aabb
    }

    #[inline]
    pub const fn translation(self) -> Vec2 {
        self.translation
    }

    #[inline]
    pub const fn max_fraction(self) -> f32 {
        self.max_fraction
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
    terminal_error: Cell<Option<Error>>,
    // The native tree owns allocator-backed state between calls. Keep a shared foundation lease
    // for its entire lifetime so replay cannot mutate process-global state before this owner drops.
    foundation_lease: Option<crate::core::foundation::TransientFoundationLease>,
    #[cfg(test)]
    destroy_probe: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl DynamicTree {
    /// Initial proxy capacity used by [`DynamicTree::new`].
    pub const DEFAULT_PROXY_CAPACITY: usize = 16;

    /// Largest proxy capacity whose native node count and allocation fit this platform.
    pub const MAX_PROXY_CAPACITY: usize =
        maximum_proxy_capacity_for(isize::MAX as usize, core::mem::size_of::<ffi::b2TreeNode>());

    /// Create an empty dynamic tree.
    #[inline]
    pub fn new() -> Result<Self> {
        Self::with_capacity(Self::DEFAULT_PROXY_CAPACITY)
    }

    /// Create an empty dynamic tree with an initial proxy capacity hint.
    ///
    /// Box2D currently rounds capacities below 16 up to 16 and grows the tree as needed.
    #[inline]
    pub fn with_capacity(proxy_capacity: usize) -> Result<Self> {
        let proxy_capacity = check_proxy_capacity(proxy_capacity)?;
        let foundation_lease = crate::core::foundation::transient_native_lease()?;
        let identity = TreeToken::allocate()?;
        let raw = unsafe { ffi::b2DynamicTree_Create(proxy_capacity) };
        // Rejected pointer fields are never passed back to a potentially incompatible provider.
        validate_initial_dynamic_tree("DynamicTree::with_capacity", &raw, proxy_capacity)?;
        Ok(Self {
            raw,
            identity,
            proxies: HashMap::new(),
            last_proxy_nonce: 0,
            terminal_error: Cell::new(None),
            foundation_lease: Some(foundation_lease),
            #[cfg(test)]
            destroy_probe: None,
        })
    }

    /// Create a proxy and return its tree-local id.
    pub fn create_proxy(
        &mut self,
        aabb: Aabb,
        category_bits: u64,
        user_data: u64,
    ) -> Result<TreeProxyId> {
        check_aabb("DynamicTree::create_proxy", "aabb", aabb)?;
        self.check_available()?;
        check_proxy_node_reserve(&self.raw)?;
        self.proxies
            .try_reserve(1)
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        let nonce = TreeProxyNonce::allocate(&mut self.last_proxy_nonce)?;
        let expected_proxy_count =
            self.proxies
                .len()
                .checked_add(1)
                .ok_or(Error::InvalidNativeDynamicTreeState {
                    operation: "DynamicTree::create_proxy",
                    field: "proxyCount",
                    value: i64::MAX,
                    constraint: "the exact safe proxy registry length",
                })?;
        let slot = unsafe {
            ffi::b2DynamicTree_CreateProxy(&mut self.raw, aabb.into_raw(), category_bits, user_data)
        };
        if let Err(error) = validate_created_proxy_state(
            &self.raw,
            slot,
            expected_proxy_count,
            self.proxies.contains_key(&slot),
        ) {
            self.terminal_error.set(Some(error));
            return Err(error);
        }
        let previous = self.proxies.insert(slot, nonce);
        debug_assert!(previous.is_none());
        Ok(TreeProxyId::bind(self.identity, slot, nonce))
    }

    /// Destroy a proxy owned by this tree.
    pub fn destroy_proxy(&mut self, proxy: TreeProxyId) -> Result<()> {
        let slot = self.check_proxy(proxy)?;
        self.check_available()?;
        let expected_proxy_count = self.proxies.len() - 1;
        unsafe {
            ffi::b2DynamicTree_DestroyProxy(&mut self.raw, slot);
        }
        if let Err(error) = checked_native_proxy_count(
            "DynamicTree::destroy_proxy",
            unsafe { ffi::b2DynamicTree_GetProxyCount(&self.raw) },
            expected_proxy_count,
        ) {
            self.terminal_error.set(Some(error));
            return Err(error);
        }
        let removed = self.proxies.remove(&slot);
        debug_assert_eq!(removed, Some(proxy.nonce));
        Ok(())
    }

    /// Move a proxy to a new AABB by removing and reinserting it.
    pub fn move_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) -> Result<()> {
        let slot = self.check_proxy(proxy)?;
        check_dynamic_tree_update_aabb("DynamicTree::move_proxy", aabb)?;
        self.check_available()?;
        unsafe {
            ffi::b2DynamicTree_MoveProxy(&mut self.raw, slot, aabb.into_raw());
        }
        Ok(())
    }

    /// Enlarge a proxy and its ancestors as necessary.
    pub fn enlarge_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) -> Result<()> {
        let slot = self.check_proxy(proxy)?;
        check_dynamic_tree_update_aabb("DynamicTree::enlarge_proxy", aabb)?;
        self.check_available()?;
        let current = self.aabb(proxy)?;
        if aabb_contains(current, aabb) {
            return Err(Error::invalid_argument(
                "DynamicTree::enlarge_proxy",
                "aabb",
                "bounds that extend beyond the proxy's current AABB",
            ));
        }
        unsafe {
            ffi::b2DynamicTree_EnlargeProxy(&mut self.raw, slot, aabb.into_raw());
        }
        Ok(())
    }

    /// Replace a proxy with equivalent state and new category bits.
    ///
    /// The returned id identifies the replacement; `proxy` is invalid after this call.
    ///
    /// The pinned Box2D revision cannot call its in-place category setter for arbitrary user data
    /// in assertion-enabled builds because the setter reads aliased internal union storage. Creating
    /// a replacement preserves the documented AABB and user-data behavior without depending on that
    /// private representation.
    pub fn replace_category_bits(
        &mut self,
        proxy: TreeProxyId,
        category_bits: u64,
    ) -> Result<TreeProxyId> {
        self.check_proxy(proxy)?;
        self.check_available()?;
        let aabb = self.aabb(proxy)?;
        let user_data = self.user_data(proxy)?;
        let replacement = self.create_proxy(aabb, category_bits, user_data)?;
        if let Err(error) = self.destroy_proxy(proxy) {
            let _ = self.destroy_proxy(replacement);
            return Err(error);
        }
        Ok(replacement)
    }

    /// Get the category bits on a proxy.
    pub fn category_bits(&mut self, proxy: TreeProxyId) -> Result<u64> {
        let slot = self.check_proxy(proxy)?;
        self.check_available()?;
        Ok(unsafe { ffi::b2DynamicTree_GetCategoryBits(&mut self.raw, slot) })
    }

    /// Get proxy user data.
    pub fn user_data(&self, proxy: TreeProxyId) -> Result<u64> {
        let slot = self.check_proxy(proxy)?;
        self.check_available()?;
        Ok(unsafe { ffi::b2DynamicTree_GetUserData(&self.raw, slot) })
    }

    /// Get a proxy's current AABB.
    pub fn aabb(&self, proxy: TreeProxyId) -> Result<Aabb> {
        let slot = self.check_proxy(proxy)?;
        self.check_available()?;
        // SAFETY: the value is validated immediately below before publication.
        let aabb =
            Aabb::from_raw_unvalidated(unsafe { ffi::b2DynamicTree_GetAABB(&self.raw, slot) });
        if aabb.is_valid() {
            Ok(aabb)
        } else {
            Err(Error::InvalidNativeOutput {
                operation: "DynamicTree::aabb",
                output: "aabb",
                constraint: "finite ordered lower and upper bounds",
            })
        }
    }

    /// Query proxies overlapping `aabb`, applying category mask bits.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn query<F>(&self, aabb: Aabb, mask_bits: u64, visit: &mut F) -> Result<TreeStats>
    where
        F: FnMut(TreeProxyId, u64) -> bool,
    {
        check_aabb("DynamicTree::query", "aabb", aabb)?;
        self.check_available()?;
        self.checked_native_proxy_count("DynamicTree::query")?;
        let proxy_count = self.proxies.len();
        let mut ctx = QueryCtx::new(self.proxy_resolver(), visit);
        let (value, error) = run_dynamic_tree_query_boundary(
            self.identity.callback_owner(),
            move || {
                let stats = unsafe {
                    ffi::b2DynamicTree_Query(
                        &self.raw,
                        aabb.into_raw(),
                        mask_bits,
                        Some(query_cb::<F>),
                        &mut ctx as *mut _ as *mut _,
                    )
                };
                (stats, ctx)
            },
            |native, panic| {
                native.map(|(stats, ctx)| {
                    let error = ctx.error();
                    panic.absorb(ctx.into_panic());
                    (
                        TreeStats::from_native("DynamicTree::query", stats, proxy_count),
                        error,
                    )
                })
            },
        );
        error.map_or(value, Err)
    }

    /// Query proxies overlapping `aabb` without category filtering.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn query_all<F>(&self, aabb: Aabb, visit: &mut F) -> Result<TreeStats>
    where
        F: FnMut(TreeProxyId, u64) -> bool,
    {
        check_aabb("DynamicTree::query_all", "aabb", aabb)?;
        self.check_available()?;
        self.checked_native_proxy_count("DynamicTree::query_all")?;
        let proxy_count = self.proxies.len();
        let mut ctx = QueryCtx::new(self.proxy_resolver(), visit);
        let (value, error) = run_dynamic_tree_query_all_boundary(
            self.identity.callback_owner(),
            move || {
                let stats = unsafe {
                    ffi::b2DynamicTree_QueryAll(
                        &self.raw,
                        aabb.into_raw(),
                        Some(query_cb::<F>),
                        &mut ctx as *mut _ as *mut _,
                    )
                };
                (stats, ctx)
            },
            |native, panic| {
                native.map(|(stats, ctx)| {
                    let error = ctx.error();
                    panic.absorb(ctx.into_panic());
                    (
                        TreeStats::from_native("DynamicTree::query_all", stats, proxy_count),
                        error,
                    )
                })
            },
        );
        error.map_or(value, Err)
    }

    /// Ray cast against tree proxies.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn ray_cast<F>(
        &self,
        input: TreeRayCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> Result<TreeStats>
    where
        F: FnMut(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl,
    {
        input.validate()?;
        let raw_input = input.into_raw();
        self.check_available()?;
        self.checked_native_proxy_count("DynamicTree::ray_cast")?;
        let proxy_count = self.proxies.len();
        let mut ctx = RayCastCtx::new(self.proxy_resolver(), callback);
        let (value, error) = run_dynamic_tree_ray_cast_boundary(
            self.identity.callback_owner(),
            move || {
                let stats = unsafe {
                    ffi::b2DynamicTree_RayCast(
                        &self.raw,
                        &raw_input,
                        mask_bits,
                        Some(ray_cast_cb::<F>),
                        &mut ctx as *mut _ as *mut _,
                    )
                };
                (stats, ctx)
            },
            |native, panic| {
                native.map(|(stats, ctx)| {
                    let error = ctx.error();
                    panic.absorb(ctx.into_panic());
                    (
                        TreeStats::from_native("DynamicTree::ray_cast", stats, proxy_count),
                        error,
                    )
                })
            },
        );
        error.map_or(value, Err)
    }

    /// Cast a swept AABB against tree proxies.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn box_cast<F>(
        &self,
        input: TreeBoxCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> Result<TreeStats>
    where
        F: FnMut(TreeBoxCastInput, TreeProxyId, u64) -> TreeCastControl,
    {
        input.validate()?;
        let raw_input = input.into_raw();
        self.check_available()?;
        self.checked_native_proxy_count("DynamicTree::box_cast")?;
        let proxy_count = self.proxies.len();
        let mut ctx = BoxCastCtx::new(self.proxy_resolver(), callback);
        let (value, error) = run_dynamic_tree_box_cast_boundary(
            self.identity.callback_owner(),
            move || {
                let stats = unsafe {
                    ffi::b2DynamicTree_BoxCast(
                        &self.raw,
                        &raw_input,
                        mask_bits,
                        Some(box_cast_cb::<F>),
                        &mut ctx as *mut _ as *mut _,
                    )
                };
                (stats, ctx)
            },
            |native, panic| {
                native.map(|(stats, ctx)| {
                    let error = ctx.error();
                    panic.absorb(ctx.into_panic());
                    (
                        TreeStats::from_native("DynamicTree::box_cast", stats, proxy_count),
                        error,
                    )
                })
            },
        );
        error.map_or(value, Err)
    }

    /// Get the binary tree height.
    #[inline]
    pub fn height(&self) -> Result<i32> {
        self.check_available()?;
        let proxy_count = self.checked_native_proxy_count("DynamicTree::height")?;
        checked_native_tree_height(
            "DynamicTree::height",
            unsafe { ffi::b2DynamicTree_GetHeight(&self.raw) },
            proxy_count,
        )
    }

    /// Get the ratio of summed node areas to root area.
    #[inline]
    pub fn area_ratio(&self) -> Result<f32> {
        self.check_available()?;
        checked_native_non_negative_finite_f32("DynamicTree::area_ratio", "area_ratio", unsafe {
            ffi::b2DynamicTree_GetAreaRatio(&self.raw)
        })
    }

    /// Get the root bounds for the full tree.
    #[inline]
    pub fn root_bounds(&self) -> Result<Aabb> {
        self.check_available()?;
        Aabb::from_raw(unsafe { ffi::b2DynamicTree_GetRootBounds(&self.raw) }).map_err(|_| {
            Error::InvalidNativeOutput {
                operation: "DynamicTree::root_bounds",
                output: "root_bounds",
                constraint: "finite ordered lower and upper bounds",
            }
        })
    }

    /// Get the number of proxies currently created in the tree.
    #[inline]
    pub fn proxy_count(&self) -> Result<i32> {
        self.check_available()?;
        self.checked_native_proxy_count("DynamicTree::proxy_count")
    }

    /// Rebuild the tree and return the number of boxes sorted.
    #[inline]
    pub fn rebuild(&mut self, full_build: bool) -> Result<i32> {
        self.check_available()?;
        let proxy_count = self.checked_native_proxy_count("DynamicTree::rebuild")?;
        let rebuilt = unsafe { ffi::b2DynamicTree_Rebuild(&mut self.raw, full_build) };
        let result = self
            .checked_native_proxy_count("DynamicTree::rebuild")
            .and_then(|_| {
                checked_native_rebuild_count(
                    "DynamicTree::rebuild",
                    rebuilt,
                    proxy_count,
                    full_build,
                )
            });
        if let Err(error) = result {
            self.terminal_error.set(Some(error));
        }
        result
    }

    /// Get the number of bytes used by this tree.
    #[inline]
    pub fn byte_count(&self) -> Result<i32> {
        self.check_available()?;
        checked_native_tree_byte_count(
            "DynamicTree::byte_count",
            self.raw.nodeCapacity,
            self.raw.rebuildCapacity,
            unsafe { ffi::b2DynamicTree_GetByteCount(&self.raw) },
        )
    }

    /// Validate the native tree invariants.
    ///
    /// This is primarily useful in tests and diagnostics. Box2D reports invariant failures
    /// through its configured assertion callback.
    #[inline]
    pub fn validate(&self) -> Result<()> {
        self.check_available()?;
        unsafe { ffi::b2DynamicTree_Validate(&self.raw) };
        Ok(())
    }

    /// Return whether a proxy id is currently owned by this tree.
    #[inline]
    pub fn contains_proxy(&self, proxy: TreeProxyId) -> bool {
        self.check_proxy(proxy).is_ok()
    }

    #[inline]
    fn check_available(&self) -> Result<()> {
        debug_assert!(self.foundation_lease.is_some());
        crate::core::callback_state::check_not_in_callback()?;
        self.terminal_error.get().map_or(Ok(()), Err)
    }

    #[inline]
    fn checked_native_proxy_count(&self, operation: &'static str) -> Result<i32> {
        checked_native_proxy_count(
            operation,
            unsafe { ffi::b2DynamicTree_GetProxyCount(&self.raw) },
            self.proxies.len(),
        )
    }

    #[inline]
    fn check_proxy(&self, proxy: TreeProxyId) -> Result<i32> {
        if proxy.tree != self.identity {
            return Err(Error::WrongTree);
        }

        self.proxies
            .get(&proxy.raw_slot())
            .is_some_and(|nonce| *nonce == proxy.nonce)
            .then_some(proxy.raw_slot())
            .ok_or(Error::InvalidTreeProxyId)
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

fn validate_initial_dynamic_tree(
    operation: &'static str,
    tree: &ffi::b2DynamicTree,
    proxy_capacity: i32,
) -> Result<()> {
    let invalid = |field: &'static str, value: i64, constraint: &'static str| {
        Error::InvalidNativeDynamicTreeState {
            operation,
            field,
            value,
            constraint,
        }
    };
    let expected_node_capacity = proxy_capacity
        .max(16)
        .checked_mul(2)
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| {
            invalid(
                "nodeCapacity",
                i64::from(tree.nodeCapacity),
                "the exact initial capacity derived from proxy_capacity",
            )
        })?;

    if tree.nodes.is_null() || !tree.nodes.is_aligned() {
        return Err(invalid(
            "nodes",
            0,
            "a non-null aligned initial node allocation",
        ));
    }
    for (field, value, expected, constraint) in [
        (
            "root",
            tree.root,
            -1,
            "the null root index for an empty tree",
        ),
        ("nodeCount", tree.nodeCount, 0, "zero for an empty tree"),
        (
            "nodeCapacity",
            tree.nodeCapacity,
            expected_node_capacity,
            "the exact initial capacity derived from proxy_capacity",
        ),
        ("freeList", tree.freeList, 0, "the first initial free node"),
        ("proxyCount", tree.proxyCount, 0, "zero for an empty tree"),
        (
            "rebuildCapacity",
            tree.rebuildCapacity,
            0,
            "zero before the first rebuild",
        ),
    ] {
        if value != expected {
            return Err(invalid(field, i64::from(value), constraint));
        }
    }
    if !tree.leafIndices.is_null()
        || !tree.leafBoxes.is_null()
        || !tree.leafCenters.is_null()
        || !tree.binIndices.is_null()
    {
        return Err(invalid(
            "rebuild_buffers",
            1,
            "null pointers before the first rebuild",
        ));
    }
    Ok(())
}

fn validate_created_proxy_state(
    tree: &ffi::b2DynamicTree,
    slot: i32,
    expected_proxy_count: usize,
    slot_already_registered: bool,
) -> Result<()> {
    const OPERATION: &str = "DynamicTree::create_proxy";
    let invalid = |field: &'static str, value: i64, constraint: &'static str| {
        Error::InvalidNativeDynamicTreeState {
            operation: OPERATION,
            field,
            value,
            constraint,
        }
    };
    let expected_proxy_count = i32::try_from(expected_proxy_count).map_err(|_| {
        invalid(
            "proxyCount",
            i64::MAX,
            "the exact safe proxy registry length representable by a native int",
        )
    })?;
    if slot < 0 || slot_already_registered {
        return Err(invalid(
            "proxy_slot",
            i64::from(slot),
            "a non-negative newly allocated slot",
        ));
    }
    if tree.nodes.is_null() || !tree.nodes.is_aligned() {
        return Err(invalid("nodes", 0, "a non-null aligned node allocation"));
    }
    if tree.nodeCapacity <= 0 || slot >= tree.nodeCapacity {
        return Err(invalid(
            "proxy_slot",
            i64::from(slot),
            "a newly allocated slot within nodeCapacity",
        ));
    }
    if tree.proxyCount != expected_proxy_count {
        return Err(invalid(
            "proxyCount",
            i64::from(tree.proxyCount),
            "the exact safe proxy registry length after creation",
        ));
    }
    let expected_node_count = expected_proxy_count
        .checked_mul(2)
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| {
            invalid(
                "nodeCount",
                i64::from(tree.nodeCount),
                "twice proxyCount minus one",
            )
        })?;
    if tree.nodeCount != expected_node_count || tree.nodeCount > tree.nodeCapacity {
        return Err(invalid(
            "nodeCount",
            i64::from(tree.nodeCount),
            "twice proxyCount minus one and no greater than nodeCapacity",
        ));
    }
    if tree.root < 0 || tree.root >= tree.nodeCapacity {
        return Err(invalid(
            "root",
            i64::from(tree.root),
            "an allocated node index within nodeCapacity",
        ));
    }
    if (expected_proxy_count == 1 && tree.root != slot)
        || (expected_proxy_count > 1 && tree.root == slot)
    {
        return Err(invalid(
            "root",
            i64::from(tree.root),
            "the created leaf for the first proxy, otherwise a distinct internal node",
        ));
    }
    if tree.freeList < -1 || tree.freeList >= tree.nodeCapacity {
        return Err(invalid(
            "freeList",
            i64::from(tree.freeList),
            "the null index or a free node index within nodeCapacity",
        ));
    }
    if tree.freeList == slot {
        return Err(invalid(
            "freeList",
            i64::from(tree.freeList),
            "a free node distinct from the newly allocated slot",
        ));
    }
    Ok(())
}

#[inline]
fn checked_native_proxy_count(operation: &'static str, value: i32, expected: usize) -> Result<i32> {
    let expected = i32::try_from(expected).map_err(|_| Error::InvalidNativeOutput {
        operation,
        output: "proxy_count",
        constraint: "a non-negative count representable by a native int",
    })?;
    if value == expected {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output: "proxy_count",
            constraint: "a non-negative count equal to the safe proxy registry length",
        })
    }
}

#[inline]
fn checked_native_tree_height(
    operation: &'static str,
    height: i32,
    proxy_count: i32,
) -> Result<i32> {
    let valid = if proxy_count <= 1 {
        height == 0
    } else {
        height > 0 && height < proxy_count
    };
    if valid {
        Ok(height)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output: "height",
            constraint: "zero for at most one proxy, otherwise positive and less than proxy_count",
        })
    }
}

#[inline]
fn checked_native_non_negative_finite_f32(
    operation: &'static str,
    output: &'static str,
    value: f32,
) -> Result<f32> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output,
            constraint: "a finite value greater than or equal to zero",
        })
    }
}

#[inline]
fn checked_native_rebuild_count(
    operation: &'static str,
    rebuilt: i32,
    proxy_count: i32,
    full_build: bool,
) -> Result<i32> {
    let valid = if full_build {
        rebuilt == proxy_count
    } else if proxy_count == 0 {
        rebuilt == 0
    } else {
        rebuilt > 0 && rebuilt <= proxy_count
    };
    if valid {
        Ok(rebuilt)
    } else {
        Err(Error::InvalidNativeOutput {
            operation,
            output: "rebuilt_proxy_count",
            constraint: "a count within proxy_count, equal to proxy_count for a full rebuild",
        })
    }
}

fn checked_native_tree_byte_count(
    operation: &'static str,
    node_capacity: i32,
    rebuild_capacity: i32,
    value: i32,
) -> Result<i32> {
    let invalid = || Error::InvalidNativeOutput {
        operation,
        output: "byte_count",
        constraint: "the exact non-negative allocation size representable by a native int",
    };
    let node_capacity = usize::try_from(node_capacity).map_err(|_| invalid())?;
    if node_capacity == 0 {
        return Err(invalid());
    }
    let rebuild_capacity = usize::try_from(rebuild_capacity).map_err(|_| invalid())?;
    let rebuild_stride = core::mem::size_of::<i32>()
        .checked_add(core::mem::size_of::<ffi::b2AABB>())
        .and_then(|size| size.checked_add(core::mem::size_of::<ffi::b2Vec2>()))
        .and_then(|size| size.checked_add(core::mem::size_of::<i32>()))
        .ok_or_else(invalid)?;
    let expected = core::mem::size_of::<ffi::b2DynamicTree>()
        .checked_add(
            core::mem::size_of::<ffi::b2TreeNode>()
                .checked_mul(node_capacity)
                .ok_or_else(invalid)?,
        )
        .and_then(|size| {
            rebuild_stride
                .checked_mul(rebuild_capacity)
                .and_then(|rebuild_size| size.checked_add(rebuild_size))
        })
        .ok_or_else(invalid)?;
    let expected = i32::try_from(expected).map_err(|_| invalid())?;
    if value == expected {
        Ok(value)
    } else {
        Err(invalid())
    }
}

impl Drop for DynamicTree {
    fn drop(&mut self) {
        let mut raw = self.raw;
        let skip_native_destroy = self.terminal_error.get().is_some();
        let Some(foundation_lease) = self.foundation_lease.take() else {
            return;
        };
        #[cfg(test)]
        let destroy_probe = self.destroy_probe.take();
        let cleanup = move || {
            if !skip_native_destroy {
                unsafe {
                    ffi::b2DynamicTree_Destroy(&mut raw);
                }
                #[cfg(test)]
                if let Some(probe) = destroy_probe {
                    probe.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
            drop(foundation_lease);
        };

        if crate::core::callback_state::in_callback() {
            crate::core::callback_state::defer_callback_cleanup_or_forget(
                self.identity.callback_owner(),
                cleanup,
            );
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
    error: Option<Error>,
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
            error: None,
        }
    }

    fn visit(&mut self, slot: i32, user_data: u64) -> bool {
        if self.stopped_early || self.panic.has_panicked() || self.error.is_some() {
            return false;
        }
        let Some(proxy) = self.proxies.resolve(slot) else {
            self.error = Some(Error::InvalidTreeProxyId);
            return false;
        };
        let keep_going =
            invoke_owner_callback(&mut self.panic, false, || (self.callback)(proxy, user_data));
        if !keep_going {
            self.stopped_early = true;
        }
        keep_going
    }

    fn into_panic(self) -> PanicSlot {
        self.panic
    }

    fn error(&self) -> Option<Error> {
        self.error
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct CastCtx<'tree, 'callback, F, I> {
    proxies: ProxyResolver<'tree>,
    callback: &'callback mut F,
    panic: PanicSlot,
    error: Option<Error>,
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
            error: None,
            _input: core::marker::PhantomData,
        }
    }

    fn into_panic(self) -> PanicSlot {
        self.panic
    }

    fn error(&self) -> Option<Error> {
        self.error
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
    let context = context.cast::<QueryCtx<'_, '_, F>>();
    if context.is_null() || !context.is_aligned() {
        return false;
    }
    let ctx = unsafe { &mut *context };
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
    let context = context.cast::<RayCastCtx<'_, '_, F>>();
    if context.is_null() || !context.is_aligned() {
        return 0.0;
    }
    let ctx = unsafe { &mut *context };
    if ctx.error.is_some() {
        return 0.0;
    }
    if input.is_null() || !input.is_aligned() {
        ctx.error = Some(Error::InvalidNativeOutput {
            operation: "DynamicTree::ray_cast",
            output: "input",
            constraint: "a non-null aligned ray-cast input pointer",
        });
        return 0.0;
    }
    let input = match TreeRayCastInput::from_raw(unsafe { *input }) {
        Ok(input) => input,
        Err(_) => {
            ctx.error = Some(Error::InvalidNativeOutput {
                operation: "DynamicTree::ray_cast",
                output: "input",
                constraint: "finite ray input with max_fraction in [0, 1]",
            });
            return 0.0;
        }
    };
    let Some(proxy) = ctx.proxies.resolve(proxy_id) else {
        ctx.error = Some(Error::InvalidTreeProxyId);
        return 0.0;
    };
    let control = invoke_owner_callback(&mut ctx.panic, TreeCastControl::Terminate, || {
        (ctx.callback)(input, proxy, user_data)
    });
    match control.into_raw(input.max_fraction) {
        Ok(value) => value,
        Err(error) => {
            ctx.error = Some(error);
            0.0
        }
    }
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
    let context = context.cast::<BoxCastCtx<'_, '_, F>>();
    if context.is_null() || !context.is_aligned() {
        return 0.0;
    }
    let ctx = unsafe { &mut *context };
    if ctx.error.is_some() {
        return 0.0;
    }
    if input.is_null() || !input.is_aligned() {
        ctx.error = Some(Error::InvalidNativeOutput {
            operation: "DynamicTree::box_cast",
            output: "input",
            constraint: "a non-null aligned box-cast input pointer",
        });
        return 0.0;
    }
    let input = match TreeBoxCastInput::from_raw(unsafe { *input }) {
        Ok(input) => input,
        Err(_) => {
            ctx.error = Some(Error::InvalidNativeOutput {
                operation: "DynamicTree::box_cast",
                output: "input",
                constraint: "finite box-cast input with max_fraction in [0, 1]",
            });
            return 0.0;
        }
    };
    let Some(proxy) = ctx.proxies.resolve(proxy_id) else {
        ctx.error = Some(Error::InvalidTreeProxyId);
        return 0.0;
    };
    let control = invoke_owner_callback(&mut ctx.panic, TreeCastControl::Terminate, || {
        (ctx.callback)(input, proxy, user_data)
    });
    match control.into_raw(input.max_fraction) {
        Ok(value) => value,
        Err(error) => {
            ctx.error = Some(error);
            0.0
        }
    }
}

#[inline]
fn check_aabb(operation: &'static str, argument: &'static str, aabb: Aabb) -> Result<()> {
    if aabb.is_valid() {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "finite ordered lower and upper bounds",
        ))
    }
}

#[inline]
fn check_dynamic_tree_update_aabb(operation: &'static str, aabb: Aabb) -> Result<()> {
    check_aabb(operation, "aabb", aabb)?;
    #[cfg(feature = "double-precision")]
    let huge_factor = 1.0e9_f32;
    #[cfg(not(feature = "double-precision"))]
    let huge_factor = 1.0e5_f32;
    let huge = huge_factor * crate::core::foundation::current_length_units_per_meter()?;
    let width = aabb.upper.x - aabb.lower.x;
    let height = aabb.upper.y - aabb.lower.y;
    if width < huge && height < huge {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            "aabb",
            "finite bounds within Box2D's supported dynamic-tree extent",
        ))
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
fn check_vec2(operation: &'static str, argument: &'static str, value: Vec2) -> Result<()> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite vector",
        ))
    }
}

#[inline]
fn check_fraction(operation: &'static str, argument: &'static str, value: f32) -> Result<()> {
    if crate::is_valid_float(value) && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(Error::invalid_argument(
            operation,
            argument,
            "a finite value in 0.0..=1.0",
        ))
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
fn check_proxy_node_reserve(tree: &ffi::b2DynamicTree) -> Result<()> {
    check_proxy_node_reserve_fields(tree.nodeCapacity, tree.nodeCount, tree.proxyCount)
}

fn check_proxy_node_reserve_fields(
    node_capacity: i32,
    node_count: i32,
    proxy_count: i32,
) -> Result<()> {
    const OPERATION: &str = "DynamicTree::create_proxy";
    if node_capacity == 0 {
        return Err(Error::InvalidNativeDynamicTreeState {
            operation: OPERATION,
            field: "nodeCapacity",
            value: i64::from(node_capacity),
            constraint: "a positive native capacity",
        });
    }
    let node_capacity =
        usize::try_from(node_capacity).map_err(|_| Error::InvalidNativeDynamicTreeState {
            operation: OPERATION,
            field: "nodeCapacity",
            value: i64::from(node_capacity),
            constraint: "a non-negative native int",
        })?;
    let node_count =
        usize::try_from(node_count).map_err(|_| Error::InvalidNativeDynamicTreeState {
            operation: OPERATION,
            field: "nodeCount",
            value: i64::from(node_count),
            constraint: "a non-negative native int no greater than nodeCapacity",
        })?;
    if proxy_count < 0 {
        return Err(Error::InvalidNativeDynamicTreeState {
            operation: OPERATION,
            field: "proxyCount",
            value: i64::from(proxy_count),
            constraint: "a non-negative native int",
        });
    }
    if proxy_count as usize > node_count {
        return Err(Error::InvalidNativeDynamicTreeState {
            operation: OPERATION,
            field: "proxyCount",
            value: i64::from(proxy_count),
            constraint: "a count no greater than nodeCount",
        });
    }
    let available =
        node_capacity
            .checked_sub(node_count)
            .ok_or(Error::InvalidNativeDynamicTreeState {
                operation: OPERATION,
                field: "nodeCount",
                value: node_count as i64,
                constraint: "a value no greater than nodeCapacity",
            })?;
    let required = if proxy_count == 0 { 1 } else { 2 };
    if available >= required {
        return Ok(());
    }

    let grown = node_capacity.checked_add(node_capacity / 2).ok_or(
        Error::InvalidNativeDynamicTreeState {
            operation: OPERATION,
            field: "nodeCapacity",
            value: node_capacity as i64,
            constraint: "a value whose native growth calculation fits usize",
        },
    )?;
    if grown <= node_capacity {
        return Err(Error::InvalidNativeDynamicTreeState {
            operation: OPERATION,
            field: "nodeCapacity",
            value: node_capacity as i64,
            constraint: "a capacity whose native growth step increases capacity",
        });
    }
    let maximum = maximum_tree_node_capacity_for(
        isize::MAX as usize,
        core::mem::size_of::<ffi::b2TreeNode>(),
    );
    if grown <= maximum {
        Ok(())
    } else {
        Err(Error::InvalidNativeDynamicTreeState {
            operation: OPERATION,
            field: "grown nodeCapacity",
            value: grown as i64,
            constraint: "a value supported by native pointer and signed-index arithmetic",
        })
    }
}

#[inline]
fn check_proxy_capacity(proxy_capacity: usize) -> Result<i32> {
    if proxy_capacity > DynamicTree::MAX_PROXY_CAPACITY {
        return Err(Error::invalid_argument(
            "DynamicTree::with_capacity",
            "proxy_capacity",
            "a capacity supported by native allocation and signed-index arithmetic",
        ));
    }
    i32::try_from(proxy_capacity).map_err(|_| {
        Error::invalid_argument(
            "DynamicTree::with_capacity",
            "proxy_capacity",
            "a capacity representable by a native int",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    static_assertions::assert_not_impl_any!(DynamicTree: Send, Sync);

    fn initialize_foundation() {
        crate::Foundation::initialize_default().unwrap();
    }

    fn initial_tree_raw(proxy_capacity: i32) -> ffi::b2DynamicTree {
        ffi::b2DynamicTree {
            nodes: core::ptr::NonNull::<ffi::b2TreeNode>::dangling().as_ptr(),
            root: -1,
            nodeCount: 0,
            nodeCapacity: proxy_capacity.max(16) * 2 - 1,
            freeList: 0,
            proxyCount: 0,
            leafIndices: core::ptr::null_mut(),
            leafBoxes: core::ptr::null_mut(),
            leafCenters: core::ptr::null_mut(),
            binIndices: core::ptr::null_mut(),
            rebuildCapacity: 0,
        }
    }

    #[test]
    fn initial_native_tree_layout_is_validated_before_owner_publication() {
        let valid = initial_tree_raw(16);
        assert!(validate_initial_dynamic_tree("test", &valid, 16).is_ok());

        let mut invalid_pointer = valid;
        invalid_pointer.nodes = core::ptr::null_mut();
        assert!(matches!(
            validate_initial_dynamic_tree("test", &invalid_pointer, 16),
            Err(Error::InvalidNativeDynamicTreeState { field: "nodes", .. })
        ));

        let mut invalid_capacity = valid;
        invalid_capacity.nodeCapacity += 1;
        assert!(matches!(
            validate_initial_dynamic_tree("test", &invalid_capacity, 16),
            Err(Error::InvalidNativeDynamicTreeState {
                field: "nodeCapacity",
                ..
            })
        ));

        let mut invalid_rebuild_pointer = valid;
        invalid_rebuild_pointer.leafIndices = core::ptr::NonNull::<i32>::dangling().as_ptr();
        assert!(matches!(
            validate_initial_dynamic_tree("test", &invalid_rebuild_pointer, 16),
            Err(Error::InvalidNativeDynamicTreeState {
                field: "rebuild_buffers",
                ..
            })
        ));

        let mut created = valid;
        created.root = 0;
        created.nodeCount = 1;
        created.proxyCount = 1;
        created.freeList = 1;
        assert!(validate_created_proxy_state(&created, 0, 1, false).is_ok());
        assert!(matches!(
            validate_created_proxy_state(&created, 0, 1, true),
            Err(Error::InvalidNativeDynamicTreeState {
                field: "proxy_slot",
                ..
            })
        ));

        let mut invalid_proxy_count = created;
        invalid_proxy_count.proxyCount = 2;
        assert!(matches!(
            validate_created_proxy_state(&invalid_proxy_count, 0, 1, false),
            Err(Error::InvalidNativeDynamicTreeState {
                field: "proxyCount",
                ..
            })
        ));
    }

    #[test]
    fn terminal_tree_never_reenters_native_destroy() {
        initialize_foundation();

        let destroyed = Arc::new(AtomicBool::new(false));
        {
            let mut tree = DynamicTree::new().unwrap();
            tree.destroy_probe = Some(Arc::clone(&destroyed));
            tree.terminal_error.set(Some(Error::InvalidNativeOutput {
                operation: "test",
                output: "tree",
                constraint: "a valid native owner",
            }));
        }

        assert!(!destroyed.load(Ordering::SeqCst));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn native_tree_outputs_reject_invalid_scalars_and_relationships() {
        assert!(matches!(
            TreeStats::from_native(
                "test",
                ffi::b2TreeStats {
                    nodeVisits: 0,
                    leafVisits: 0,
                },
                1,
            ),
            Err(Error::InvalidNativeOutput {
                output: "tree_stats.node_visits",
                ..
            })
        ));
        assert!(matches!(
            TreeStats::from_native(
                "test",
                ffi::b2TreeStats {
                    nodeVisits: 4,
                    leafVisits: 1,
                },
                2,
            ),
            Err(Error::InvalidNativeOutput {
                output: "tree_stats.node_visits",
                ..
            })
        ));
        assert!(matches!(
            TreeStats::from_native(
                "test",
                ffi::b2TreeStats {
                    nodeVisits: 3,
                    leafVisits: 3,
                },
                2,
            ),
            Err(Error::InvalidNativeOutput {
                output: "tree_stats.leaf_visits",
                ..
            })
        ));
        for value in [-1.0, f32::INFINITY, f32::NAN] {
            assert!(matches!(
                checked_native_non_negative_finite_f32("test", "value", value),
                Err(Error::InvalidNativeOutput {
                    output: "value",
                    ..
                })
            ));
        }
        assert!(checked_native_tree_height("test", 0, 0).is_ok());
        assert!(checked_native_tree_height("test", 0, 1).is_ok());
        assert!(checked_native_tree_height("test", 1, 2).is_ok());
        assert!(checked_native_tree_height("test", -1, 0).is_err());
        assert!(checked_native_tree_height("test", 0, 2).is_err());
        assert!(checked_native_tree_height("test", 2, 2).is_err());
        assert!(checked_native_proxy_count("test", -1, 0).is_err());
        assert!(checked_native_proxy_count("test", 1, 0).is_err());
        assert!(checked_native_rebuild_count("test", -1, 0, false).is_err());
        assert!(checked_native_rebuild_count("test", 0, 1, false).is_err());
        assert!(checked_native_rebuild_count("test", 2, 1, false).is_err());
        assert!(checked_native_rebuild_count("test", 0, 1, true).is_err());
    }

    #[test]
    fn native_tree_byte_count_rejects_signed_overflow_and_mismatches() {
        assert!(checked_native_tree_byte_count("test", -1, 0, 0).is_err());
        assert!(checked_native_tree_byte_count("test", 16, -1, 0).is_err());
        assert!(checked_native_tree_byte_count("test", i32::MAX, i32::MAX, 0).is_err());

        let node_capacity = 16;
        let rebuild_capacity = 4;
        let expected = core::mem::size_of::<ffi::b2DynamicTree>()
            + core::mem::size_of::<ffi::b2TreeNode>() * node_capacity as usize
            + (core::mem::size_of::<i32>()
                + core::mem::size_of::<ffi::b2AABB>()
                + core::mem::size_of::<ffi::b2Vec2>()
                + core::mem::size_of::<i32>())
                * rebuild_capacity as usize;
        let expected = i32::try_from(expected).unwrap();
        assert_eq!(
            checked_native_tree_byte_count("test", node_capacity, rebuild_capacity, expected,),
            Ok(expected)
        );
        assert!(
            checked_native_tree_byte_count("test", node_capacity, rebuild_capacity, expected + 1,)
                .is_err()
        );
    }

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
        let aabb = Aabb::from_center_half_extents([0.0, 0.0], [1.0, 1.0]).unwrap();
        let mut tree = DynamicTree::new().unwrap();
        let stale = tree.create_proxy(aabb, u64::MAX, 7).unwrap();
        tree.destroy_proxy(stale).unwrap();
        let live = tree.create_proxy(aabb, u64::MAX, 11).unwrap();
        assert_eq!(stale.raw_slot(), live.raw_slot());
        assert_ne!(stale, live);
        (tree, stale, live, aabb)
    }

    fn equal_slot_foreign_fixture() -> (DynamicTree, TreeProxyId, TreeProxyId, Aabb) {
        let aabb = Aabb::from_center_half_extents([0.0, 0.0], [1.0, 1.0]).unwrap();
        let mut first = DynamicTree::new().unwrap();
        let foreign = first.create_proxy(aabb, u64::MAX, 7).unwrap();
        let mut second = DynamicTree::new().unwrap();
        let local = second.create_proxy(aabb, u64::MAX, 11).unwrap();
        assert_eq!(foreign.raw_slot(), local.raw_slot());
        assert_ne!(foreign.tree, local.tree);
        drop(first);
        (second, foreign, local, aabb)
    }

    fn assert_live_proxy_unchanged(tree: &mut DynamicTree, proxy: TreeProxyId, aabb: Aabb) {
        assert!(tree.contains_proxy(proxy));
        assert_eq!(tree.user_data(proxy), Ok(11));
        assert_eq!(tree.aabb(proxy), Ok(aabb));
        assert_eq!(tree.proxy_count(), Ok(1));
    }

    #[test]
    fn tree_identity_and_proxy_nonce_exhaustion_do_not_wrap() {
        let next_tree = AtomicU64::new(u64::MAX);
        assert_eq!(
            TreeToken::allocate_from(&next_tree),
            Err(Error::TreeIdentityExhausted)
        );
        assert_eq!(next_tree.load(Ordering::Relaxed), u64::MAX);

        let mut last_nonce = u64::MAX;
        assert_eq!(
            TreeProxyNonce::allocate(&mut last_nonce),
            Err(Error::ObjectIdentityExhausted)
        );
        assert_eq!(last_nonce, u64::MAX);
    }

    #[test]
    fn equal_native_slots_from_another_tree_are_rejected_before_native_access() {
        initialize_foundation();

        let replacement_aabb = Aabb::from_center_half_extents([4.0, 4.0], [1.0, 1.0]).unwrap();

        let (mut tree, foreign, live, original_aabb) = equal_slot_foreign_fixture();
        assert!(!tree.contains_proxy(foreign));
        assert_eq!(tree.destroy_proxy(foreign), Err(Error::WrongTree));
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, foreign, live, original_aabb) = equal_slot_foreign_fixture();
        assert_eq!(
            tree.move_proxy(foreign, replacement_aabb),
            Err(Error::WrongTree)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, foreign, live, original_aabb) = equal_slot_foreign_fixture();
        assert_eq!(
            tree.enlarge_proxy(foreign, replacement_aabb),
            Err(Error::WrongTree)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, foreign, live, original_aabb) = equal_slot_foreign_fixture();
        assert_eq!(
            tree.replace_category_bits(foreign, 0b10),
            Err(Error::WrongTree)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, foreign, live, original_aabb) = equal_slot_foreign_fixture();
        assert_eq!(tree.category_bits(foreign), Err(Error::WrongTree));
        assert_eq!(tree.user_data(foreign), Err(Error::WrongTree));
        assert_eq!(tree.aabb(foreign), Err(Error::WrongTree));
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);
    }

    #[test]
    fn recycled_native_slots_do_not_revive_stale_proxy_ids() {
        initialize_foundation();

        let replacement_aabb = Aabb::from_center_half_extents([4.0, 4.0], [1.0, 1.0]).unwrap();

        let (mut tree, stale, live, original_aabb) = recycled_proxy_fixture();
        assert!(!tree.contains_proxy(stale));
        assert_eq!(tree.destroy_proxy(stale), Err(Error::InvalidTreeProxyId));
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, stale, live, original_aabb) = recycled_proxy_fixture();
        assert_eq!(
            tree.move_proxy(stale, replacement_aabb),
            Err(Error::InvalidTreeProxyId)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, stale, live, original_aabb) = recycled_proxy_fixture();
        assert_eq!(
            tree.enlarge_proxy(stale, replacement_aabb),
            Err(Error::InvalidTreeProxyId)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, stale, live, original_aabb) = recycled_proxy_fixture();
        assert_eq!(
            tree.replace_category_bits(stale, 0b10),
            Err(Error::InvalidTreeProxyId)
        );
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);

        let (mut tree, stale, live, original_aabb) = recycled_proxy_fixture();
        assert_eq!(tree.category_bits(stale), Err(Error::InvalidTreeProxyId));
        assert_eq!(tree.user_data(stale), Err(Error::InvalidTreeProxyId));
        assert_eq!(tree.aabb(stale), Err(Error::InvalidTreeProxyId));
        assert_live_proxy_unchanged(&mut tree, live, original_aabb);
    }

    #[test]
    fn dropping_a_tree_cannot_rebind_its_proxy_to_a_new_tree() {
        initialize_foundation();

        let aabb = Aabb::from_center_half_extents([0.0, 0.0], [1.0, 1.0]).unwrap();
        let (stale, stale_slot) = {
            let mut tree = DynamicTree::new().unwrap();
            let proxy = tree.create_proxy(aabb, u64::MAX, 7).unwrap();
            (proxy, proxy.raw_slot())
        };

        let mut replacement = DynamicTree::new().unwrap();
        let live = replacement.create_proxy(aabb, u64::MAX, 11).unwrap();
        assert_eq!(stale_slot, live.raw_slot());
        assert_eq!(replacement.user_data(stale), Err(Error::WrongTree));
        assert_live_proxy_unchanged(&mut replacement, live, aabb);
    }

    #[test]
    fn query_defers_another_tree_destruction_until_the_native_callback_returns() {
        initialize_foundation();

        let aabb = Aabb::from_center_half_extents([0.0, 0.0], [1.0, 1.0]).unwrap();
        let mut query_tree = DynamicTree::new().unwrap();
        query_tree.create_proxy(aabb, u64::MAX, 7).unwrap();

        let destroyed = Arc::new(AtomicBool::new(false));
        let mut doomed_tree = DynamicTree::new().unwrap();
        doomed_tree.destroy_probe = Some(Arc::clone(&destroyed));
        let mut doomed_tree = Some(doomed_tree);

        let mut visit = |_: TreeProxyId, _: u64| {
            drop(doomed_tree.take());
            assert!(!destroyed.load(Ordering::SeqCst));
            false
        };
        query_tree.query_all(aabb, &mut visit).unwrap();

        assert!(destroyed.load(Ordering::SeqCst));
    }

    #[test]
    fn tree_callback_trampolines_fail_closed_on_invalid_native_pointers() {
        fn query(_: TreeProxyId, _: u64) -> bool {
            true
        }
        fn ray(_: TreeRayCastInput, _: TreeProxyId, _: u64) -> TreeCastControl {
            TreeCastControl::Continue
        }
        fn box_cast(_: TreeBoxCastInput, _: TreeProxyId, _: u64) -> TreeCastControl {
            TreeCastControl::Continue
        }

        assert!(!unsafe { query_cb::<fn(TreeProxyId, u64) -> bool>(1, 7, core::ptr::null_mut()) });
        assert_eq!(
            unsafe {
                ray_cast_cb::<fn(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl>(
                    core::ptr::null(),
                    1,
                    7,
                    core::ptr::null_mut(),
                )
            },
            0.0
        );

        let mut proxies = HashMap::new();
        let resolver = callback_proxy_resolver(&mut proxies, 1);
        let mut ray_callback: fn(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl = ray;
        let mut ray_context: RayCastCtx<'_, '_, _> = CastCtx::new(resolver, &mut ray_callback);
        let ray_context_ptr = core::ptr::from_mut(&mut ray_context).cast::<core::ffi::c_void>();
        assert_eq!(
            unsafe {
                ray_cast_cb::<fn(TreeRayCastInput, TreeProxyId, u64) -> TreeCastControl>(
                    core::ptr::null(),
                    1,
                    7,
                    ray_context_ptr,
                )
            },
            0.0
        );
        assert_eq!(
            ray_context.error(),
            Some(Error::InvalidNativeOutput {
                operation: "DynamicTree::ray_cast",
                output: "input",
                constraint: "a non-null aligned ray-cast input pointer",
            })
        );

        let mut box_callback: fn(TreeBoxCastInput, TreeProxyId, u64) -> TreeCastControl = box_cast;
        let mut box_context: BoxCastCtx<'_, '_, _> = CastCtx::new(resolver, &mut box_callback);
        let box_context_ptr = core::ptr::from_mut(&mut box_context).cast::<core::ffi::c_void>();
        assert_eq!(
            unsafe {
                box_cast_cb::<fn(TreeBoxCastInput, TreeProxyId, u64) -> TreeCastControl>(
                    core::ptr::null(),
                    1,
                    7,
                    box_context_ptr,
                )
            },
            0.0
        );
        assert_eq!(
            box_context.error(),
            Some(Error::InvalidNativeOutput {
                operation: "DynamicTree::box_cast",
                output: "input",
                constraint: "a non-null aligned box-cast input pointer",
            })
        );

        let _ = query;
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

        let ray_input = TreeRayCastInput::new([-4.0, 1.0], [10.0, 0.0])
            .unwrap()
            .into_raw();
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
            TreeBoxCastInput::new(Aabb::new([-4.0, 0.5], [-3.0, 1.5]).unwrap(), [8.0, 0.0])
                .unwrap()
                .into_raw();
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
            .unwrap()
            .with_max_fraction(0.5)
            .unwrap()
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
        assert!(!context.panic.has_panicked());
        assert_eq!(
            context.error(),
            Some(Error::invalid_argument(
                "TreeCastControl::Clip",
                "fraction",
                "a finite value greater than zero and no greater than max_fraction",
            ))
        );
        assert_eq!(TreeCastControl::Terminate.into_raw(0.5), Ok(0.0));
        assert_eq!(TreeCastControl::Skip.into_raw(0.5), Ok(-1.0));
        assert_eq!(TreeCastControl::Continue.into_raw(0.5), Ok(0.5));
        assert_eq!(TreeCastControl::Clip(0.25).into_raw(0.5), Ok(0.25));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn invalid_tree_cast_controls_are_reported_by_the_safe_api() {
        initialize_foundation();

        let bounds = Aabb::from_center_half_extents([0.0, 0.0], [1.0, 1.0]).unwrap();
        let mut tree = DynamicTree::new().unwrap();
        tree.create_proxy(bounds, u64::MAX, 7).unwrap();
        let input = TreeRayCastInput::new([-4.0, 0.0], [8.0, 0.0]).unwrap();

        assert_eq!(
            tree.ray_cast(input, u64::MAX, &mut |_, _, _| {
                TreeCastControl::Clip(f32::NAN)
            }),
            Err(Error::invalid_argument(
                "TreeCastControl::Clip",
                "fraction",
                "a finite value greater than zero and no greater than max_fraction",
            ))
        );
    }

    #[test]
    fn proxy_capacity_guard_accounts_for_native_arithmetic_and_allocation() {
        let maximum = DynamicTree::MAX_PROXY_CAPACITY;
        assert_eq!(check_proxy_capacity(maximum), Ok(maximum as i32));
        assert_eq!(
            check_proxy_capacity(maximum + 1),
            Err(Error::invalid_argument(
                "DynamicTree::with_capacity",
                "proxy_capacity",
                "a capacity supported by native allocation and signed-index arithmetic",
            ))
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
        assert!(matches!(
            check_proxy_node_reserve_fields(maximum as i32, maximum as i32, 1),
            Err(Error::InvalidNativeDynamicTreeState {
                operation: "DynamicTree::create_proxy",
                field: "grown nodeCapacity",
                constraint: "a value supported by native pointer and signed-index arithmetic",
                ..
            })
        ));
    }

    #[test]
    fn proxy_creation_rejects_inconsistent_native_capacity_state() {
        assert_eq!(
            check_proxy_node_reserve_fields(0, 0, 0),
            Err(Error::InvalidNativeDynamicTreeState {
                operation: "DynamicTree::create_proxy",
                field: "nodeCapacity",
                value: 0,
                constraint: "a positive native capacity",
            })
        );
        assert_eq!(
            check_proxy_node_reserve_fields(16, 1, 2),
            Err(Error::InvalidNativeDynamicTreeState {
                operation: "DynamicTree::create_proxy",
                field: "proxyCount",
                value: 2,
                constraint: "a count no greater than nodeCount",
            })
        );
        assert_eq!(
            check_proxy_node_reserve_fields(1, 1, 1),
            Err(Error::InvalidNativeDynamicTreeState {
                operation: "DynamicTree::create_proxy",
                field: "nodeCapacity",
                value: 1,
                constraint: "a capacity whose native growth step increases capacity",
            })
        );
    }
}
