//! Safe wrapper for Box2D's standalone dynamic AABB tree.
//!
//! The dynamic tree can organize spatial data that is not part of a Box2D world.
//! Proxies store an AABB, category bits, and an opaque `u64` user data value.

use std::{collections::BTreeSet, panic::AssertUnwindSafe};

use boxdd_sys::ffi;

use crate::{
    collision::ShapeProxy,
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
    #[inline]
    fn from_raw(raw: ffi::b2TreeStats) -> Self {
        Self {
            node_visits: raw.nodeVisits,
            leaf_visits: raw.leafVisits,
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

    #[inline]
    fn from_raw(raw: ffi::b2RayCastInput) -> Self {
        Self {
            origin: Vec2::from_raw(raw.origin),
            translation: Vec2::from_raw(raw.translation),
            max_fraction: raw.maxFraction,
        }
    }

    #[inline]
    fn into_raw(self) -> ffi::b2RayCastInput {
        ffi::b2RayCastInput {
            origin: self.origin.into_raw(),
            translation: self.translation.into_raw(),
            maxFraction: self.max_fraction,
        }
    }
}

/// Shape-cast input for [`DynamicTree::shape_cast`].
#[derive(Copy, Clone, Debug)]
pub struct TreeShapeCastInput {
    pub proxy: ShapeProxy,
    pub translation: Vec2,
    pub max_fraction: f32,
    pub can_encroach: bool,
}

impl TreeShapeCastInput {
    /// Build a shape cast over `proxy` moving by `translation`.
    #[inline]
    pub fn new<T: Into<Vec2>>(proxy: ShapeProxy, translation: T) -> Self {
        Self {
            proxy,
            translation: translation.into(),
            max_fraction: 1.0,
            can_encroach: false,
        }
    }

    /// Limit the cast to a fraction of the translation.
    #[inline]
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Self {
        self.max_fraction = max_fraction;
        self
    }

    /// Allow encroachment when initially touching.
    #[inline]
    pub fn with_can_encroach(mut self, can_encroach: bool) -> Self {
        self.can_encroach = can_encroach;
        self
    }

    /// Validate this input before crossing the FFI boundary.
    pub fn validate(&self) -> ApiResult<()> {
        self.proxy.validate()?;
        check_vec2(self.translation)?;
        check_fraction(self.max_fraction)
    }

    #[inline]
    fn from_raw(raw: ffi::b2ShapeCastInput) -> Self {
        Self {
            proxy: ShapeProxy::from_raw(raw.proxy),
            translation: Vec2::from_raw(raw.translation),
            max_fraction: raw.maxFraction,
            can_encroach: raw.canEncroach,
        }
    }

    #[inline]
    fn into_raw(self) -> ffi::b2ShapeCastInput {
        ffi::b2ShapeCastInput {
            proxy: self.proxy.into_raw(),
            translation: self.translation.into_raw(),
            maxFraction: self.max_fraction,
            canEncroach: self.can_encroach,
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
    /// Create an empty dynamic tree.
    #[inline]
    pub fn new() -> Self {
        Self {
            raw: unsafe { ffi::b2DynamicTree_Create() },
            proxies: BTreeSet::new(),
        }
    }

    /// Create a proxy and return its tree-local id.
    pub fn create_proxy(&mut self, aabb: Aabb, category_bits: u64, user_data: u64) -> TreeProxyId {
        assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
        self.try_create_proxy(aabb, category_bits, user_data)
            .expect("validated dynamic tree proxy creation")
    }

    /// Create a proxy with recoverable validation.
    pub fn try_create_proxy(
        &mut self,
        aabb: Aabb,
        category_bits: u64,
        user_data: u64,
    ) -> ApiResult<TreeProxyId> {
        check_aabb(aabb)?;
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
        assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
        self.try_move_proxy(proxy, aabb)
            .expect("proxy id must belong to this dynamic tree");
    }

    /// Move a proxy with recoverable validation.
    pub fn try_move_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) -> ApiResult<()> {
        self.check_proxy(proxy)?;
        check_aabb(aabb)?;
        unsafe {
            ffi::b2DynamicTree_MoveProxy(&mut self.raw, proxy.into_raw(), aabb.into_raw());
        }
        Ok(())
    }

    /// Enlarge a proxy and its ancestors as necessary.
    pub fn enlarge_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) {
        assert!(aabb.is_valid(), "aabb must be valid, got {:?}", aabb);
        self.try_enlarge_proxy(proxy, aabb)
            .expect("proxy id must belong to this dynamic tree");
    }

    /// Enlarge a proxy with recoverable validation.
    pub fn try_enlarge_proxy(&mut self, proxy: TreeProxyId, aabb: Aabb) -> ApiResult<()> {
        self.check_proxy(proxy)?;
        check_aabb(aabb)?;
        unsafe {
            ffi::b2DynamicTree_EnlargeProxy(&mut self.raw, proxy.into_raw(), aabb.into_raw());
        }
        Ok(())
    }

    /// Set the category bits on a proxy.
    pub fn set_category_bits(&mut self, proxy: TreeProxyId, category_bits: u64) {
        self.try_set_category_bits(proxy, category_bits)
            .expect("proxy id must belong to this dynamic tree");
    }

    /// Set the category bits on a proxy with recoverable validation.
    pub fn try_set_category_bits(
        &mut self,
        proxy: TreeProxyId,
        category_bits: u64,
    ) -> ApiResult<()> {
        self.check_proxy(proxy)?;
        unsafe {
            ffi::b2DynamicTree_SetCategoryBits(&mut self.raw, proxy.into_raw(), category_bits);
        }
        Ok(())
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

    /// Shape cast against tree proxies.
    pub fn shape_cast<F>(
        &self,
        input: TreeShapeCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> TreeStats
    where
        F: FnMut(TreeShapeCastInput, TreeProxyId, u64) -> f32,
    {
        self.try_shape_cast(input, mask_bits, callback)
            .expect("validated dynamic tree shape cast")
    }

    /// Shape cast against tree proxies with recoverable validation.
    pub fn try_shape_cast<F>(
        &self,
        input: TreeShapeCastInput,
        mask_bits: u64,
        callback: &mut F,
    ) -> ApiResult<TreeStats>
    where
        F: FnMut(TreeShapeCastInput, TreeProxyId, u64) -> f32,
    {
        input.validate()?;
        let raw_input = input.into_raw();
        let mut ctx = ShapeCastCtx::new(callback);
        let stats = unsafe {
            ffi::b2DynamicTree_ShapeCast(
                &self.raw,
                &raw_input,
                mask_bits,
                Some(shape_cast_cb::<F>),
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

    /// Validate the tree using Box2D's internal test hook.
    #[inline]
    pub fn validate(&self) {
        unsafe { ffi::b2DynamicTree_Validate(&self.raw) }
    }

    /// Validate that the tree has no enlarged AABBs using Box2D's internal test hook.
    #[inline]
    pub fn validate_no_enlarged(&self) {
        unsafe { ffi::b2DynamicTree_ValidateNoEnlarged(&self.raw) }
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
type ShapeCastCtx<'a, F> = CastCtx<'a, F, TreeShapeCastInput>;

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

unsafe extern "C" fn shape_cast_cb<F>(
    input: *const ffi::b2ShapeCastInput,
    proxy_id: i32,
    user_data: u64,
    context: *mut core::ffi::c_void,
) -> f32
where
    F: FnMut(TreeShapeCastInput, TreeProxyId, u64) -> f32,
{
    let ctx = unsafe { &mut *(context as *mut ShapeCastCtx<'_, F>) };
    if ctx.panic.is_some() {
        return 0.0;
    }
    let input = TreeShapeCastInput::from_raw(unsafe { *input });
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
