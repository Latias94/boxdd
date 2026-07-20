use super::*;
use std::sync::Arc;

type ShapeFilterFn = fn(crate::types::ShapeId, crate::types::ShapeId) -> bool;
type PreSolveFn = fn(
    crate::types::ShapeId,
    crate::types::ShapeId,
    crate::types::Position,
    crate::types::Vec2,
) -> bool;

/// Input passed to world-level friction and restitution mixing callbacks.
///
/// `coefficient` is the shape's friction or restitution coefficient, depending on the callback.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MaterialMixInput {
    pub coefficient: f32,
    pub user_material_id: u64,
}

impl MaterialMixInput {
    #[inline]
    pub const fn new(coefficient: f32, user_material_id: u64) -> Self {
        Self {
            coefficient,
            user_material_id,
        }
    }
}

unsafe extern "C" fn custom_filter_callback(
    a: ffi::b2ShapeId,
    b: ffi::b2ShapeId,
    context: *mut core::ffi::c_void,
) -> bool {
    // SAFETY: context is provided by the custom-filter registration helpers and points to
    // `CustomFilterCtx` for the lifetime of the registered callback.
    let ctx = unsafe { &*(context as *const CustomFilterCtx) };
    if ctx.worker.has_panicked() {
        return true;
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = crate::core::callback_state::CallbackGuard::enter();
        (ctx.cb)(ctx.worker.shape(a), ctx.worker.shape(b))
    })) {
        Ok(v) => v,
        Err(payload) => {
            ctx.worker.record_panic(payload);
            true
        }
    }
}

unsafe extern "C" fn pre_solve_callback(
    a: ffi::b2ShapeId,
    b: ffi::b2ShapeId,
    point: ffi::b2Pos,
    normal: ffi::b2Vec2,
    context: *mut core::ffi::c_void,
) -> bool {
    // SAFETY: context is provided by the pre-solve registration helpers and points to
    // `PreSolveCtx` for the lifetime of the registered callback.
    let ctx = unsafe { &*(context as *const PreSolveCtx) };
    if ctx.worker.has_panicked() {
        return true;
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = crate::core::callback_state::CallbackGuard::enter();
        (ctx.cb)(
            ctx.worker.shape(a),
            ctx.worker.shape(b),
            crate::types::Position::from_raw(point),
            crate::types::Vec2::from_raw(normal),
        )
    })) {
        Ok(v) => v,
        Err(payload) => {
            ctx.worker.record_panic(payload);
            true
        }
    }
}

impl World {
    fn ensure_material_mix_slot(&self) -> crate::error::ApiResult<usize> {
        let mut slot = self
            .core
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned");
        if let Some(slot) = *slot {
            return Ok(slot);
        }

        let Some(new_slot) = crate::core::material_mix_registry::acquire_slot() else {
            return Err(crate::error::ApiError::CallbackSlotsExhausted);
        };
        *slot = Some(new_slot);
        Ok(new_slot)
    }

    fn maybe_release_material_mix_slot(&self) {
        let mut slot = self
            .core
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned");
        if let Some(slot_index) = *slot
            && !crate::core::material_mix_registry::has_any_callback(slot_index)
        {
            crate::core::material_mix_registry::release_slot(slot_index);
            *slot = None;
        }
    }

    fn set_custom_filter_impl<F>(&mut self, f: F)
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        let ctx = Box::new(CustomFilterCtx {
            worker: Arc::clone(&self.core.worker_callbacks),
            cb: Box::new(f),
        });
        self.install_custom_filter_ctx(ctx);
    }

    fn install_custom_filter_ctx(&mut self, ctx: Box<CustomFilterCtx>) {
        let ctx_ptr: *mut core::ffi::c_void = (&*ctx) as *const CustomFilterCtx as *mut _;
        let old = self
            .core
            .custom_filter
            .lock()
            .expect("custom_filter mutex poisoned")
            .replace(ctx);
        unsafe {
            ffi::b2World_SetCustomFilterCallback(self.raw(), Some(custom_filter_callback), ctx_ptr)
        };
        drop(old);
    }

    fn clear_custom_filter_impl(&mut self) {
        unsafe { ffi::b2World_SetCustomFilterCallback(self.raw(), None, core::ptr::null_mut()) };
        let old = self
            .core
            .custom_filter
            .lock()
            .expect("custom_filter mutex poisoned")
            .take();
        drop(old);
    }

    fn set_pre_solve_impl<F>(&mut self, f: F)
    where
        F: Fn(
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Position,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        let ctx = Box::new(PreSolveCtx {
            worker: Arc::clone(&self.core.worker_callbacks),
            cb: Box::new(f),
        });
        self.install_pre_solve_ctx(ctx);
    }

    fn install_pre_solve_ctx(&mut self, ctx: Box<PreSolveCtx>) {
        let ctx_ptr: *mut core::ffi::c_void = (&*ctx) as *const PreSolveCtx as *mut _;
        let old = self
            .core
            .pre_solve
            .lock()
            .expect("pre_solve mutex poisoned")
            .replace(ctx);
        unsafe { ffi::b2World_SetPreSolveCallback(self.raw(), Some(pre_solve_callback), ctx_ptr) };
        drop(old);
    }

    fn clear_pre_solve_impl(&mut self) {
        unsafe { ffi::b2World_SetPreSolveCallback(self.raw(), None, core::ptr::null_mut()) };
        let old = self
            .core
            .pre_solve
            .lock()
            .expect("pre_solve mutex poisoned")
            .take();
        drop(old);
    }

    // --- Collision/solve callbacks ---------------------------------------------------------
    /// Register a thread-safe custom filter closure. This is called when a contact pair is
    /// considered for collision if either shape has custom filtering enabled.
    /// Return false to disable the collision.
    ///
    /// The callback may run on Box2D worker threads and must not call world APIs.
    pub fn set_custom_filter<F>(&mut self, f: F)
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        assert_world_available(&self.core);
        self.set_custom_filter_impl(f)
    }

    pub fn try_set_custom_filter<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        check_world_available(&self.core)?;
        self.set_custom_filter_impl(f);
        Ok(())
    }

    /// Clear the custom filter callback and release associated resources.
    pub fn clear_custom_filter(&mut self) {
        assert_world_available(&self.core);
        self.clear_custom_filter_impl();
    }

    pub fn try_clear_custom_filter(&mut self) -> crate::error::ApiResult<()> {
        check_world_available(&self.core)?;
        self.clear_custom_filter_impl();
        Ok(())
    }

    /// Register a thread-safe pre-solve closure. This is called after contact update (when enabled
    /// on shapes) and before the solver. Return false to disable the contact this step.
    ///
    /// The callback may run on Box2D worker threads and must not call world APIs.
    pub fn set_pre_solve<F>(&mut self, f: F)
    where
        F: Fn(
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Position,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        assert_world_available(&self.core);
        self.set_pre_solve_impl(f)
    }

    pub fn try_set_pre_solve<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Position,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        check_world_available(&self.core)?;
        self.set_pre_solve_impl(f);
        Ok(())
    }

    /// Clear the pre-solve callback and release associated resources.
    pub fn clear_pre_solve(&mut self) {
        assert_world_available(&self.core);
        self.clear_pre_solve_impl();
    }

    pub fn try_clear_pre_solve(&mut self) -> crate::error::ApiResult<()> {
        check_world_available(&self.core)?;
        self.clear_pre_solve_impl();
        Ok(())
    }

    /// Compatibility helper: set or clear the custom filter using a plain function pointer.
    pub fn set_custom_filter_callback(&mut self, cb: Option<ShapeFilterFn>) {
        assert_world_available(&self.core);
        match cb {
            Some(func) => self.set_custom_filter_impl(func),
            None => self.clear_custom_filter_impl(),
        }
    }

    pub fn try_set_custom_filter_callback(
        &mut self,
        cb: Option<ShapeFilterFn>,
    ) -> crate::error::ApiResult<()> {
        check_world_available(&self.core)?;
        match cb {
            Some(func) => self.set_custom_filter_impl(func),
            None => self.clear_custom_filter_impl(),
        }
        Ok(())
    }

    /// Compatibility helper: set or clear the pre-solve using a plain function pointer.
    pub fn set_pre_solve_callback(&mut self, cb: Option<PreSolveFn>) {
        assert_world_available(&self.core);
        match cb {
            Some(func) => self.set_pre_solve_impl(func),
            None => self.clear_pre_solve_impl(),
        }
    }

    pub fn try_set_pre_solve_callback(
        &mut self,
        cb: Option<PreSolveFn>,
    ) -> crate::error::ApiResult<()> {
        check_world_available(&self.core)?;
        match cb {
            Some(func) => self.set_pre_solve_impl(func),
            None => self.clear_pre_solve_impl(),
        }
        Ok(())
    }

    /// Register a thread-safe friction mixing callback.
    ///
    /// This callback may run on Box2D worker threads and intentionally receives no world context.
    /// Use `user_material_id` to implement table-driven material behavior.
    ///
    /// The callback must not attempt to modify Box2D state or unsafely mutate shared application
    /// state.
    pub fn set_friction_callback<F>(&mut self, f: F)
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        assert_world_available(&self.core);
        self.try_set_friction_callback(f)
            .expect("no free callback slot is available for material mixing callbacks");
    }

    pub fn try_set_friction_callback<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        check_world_available(&self.core)?;
        let slot = self.ensure_material_mix_slot()?;
        let ctx = Box::new(MaterialMixCtx {
            worker: Arc::clone(&self.core.worker_callbacks),
            cb: Box::new(f),
        });
        let ptr = (&*ctx) as *const MaterialMixCtx as *mut MaterialMixCtx;
        let old = self
            .core
            .friction_mix
            .lock()
            .expect("friction_mix mutex poisoned")
            .replace(ctx);
        crate::core::material_mix_registry::set_friction_ptr(slot, ptr);
        unsafe {
            ffi::b2World_SetFrictionCallback(
                self.raw(),
                crate::core::material_mix_registry::friction_callback(slot),
            );
        }
        drop(old);
        Ok(())
    }

    /// Clear the friction mixing callback and restore Box2D's default mixing rule.
    pub fn clear_friction_callback(&mut self) {
        assert_world_available(&self.core);
        let slot = *self
            .core
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned");
        if let Some(slot) = slot {
            unsafe { ffi::b2World_SetFrictionCallback(self.raw(), None) };
            crate::core::material_mix_registry::set_friction_ptr(slot, core::ptr::null_mut());
        }
        let old = self
            .core
            .friction_mix
            .lock()
            .expect("friction_mix mutex poisoned")
            .take();
        self.maybe_release_material_mix_slot();
        drop(old);
    }

    pub fn try_clear_friction_callback(&mut self) -> crate::error::ApiResult<()> {
        check_world_available(&self.core)?;
        self.clear_friction_callback();
        Ok(())
    }

    /// Register a thread-safe restitution mixing callback.
    ///
    /// This callback may run on Box2D worker threads and intentionally receives no world context.
    /// Use `user_material_id` to implement table-driven material behavior.
    ///
    /// The callback must not attempt to modify Box2D state or unsafely mutate shared application
    /// state.
    pub fn set_restitution_callback<F>(&mut self, f: F)
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        assert_world_available(&self.core);
        self.try_set_restitution_callback(f)
            .expect("no free callback slot is available for material mixing callbacks");
    }

    pub fn try_set_restitution_callback<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        check_world_available(&self.core)?;
        let slot = self.ensure_material_mix_slot()?;
        let ctx = Box::new(MaterialMixCtx {
            worker: Arc::clone(&self.core.worker_callbacks),
            cb: Box::new(f),
        });
        let ptr = (&*ctx) as *const MaterialMixCtx as *mut MaterialMixCtx;
        let old = self
            .core
            .restitution_mix
            .lock()
            .expect("restitution_mix mutex poisoned")
            .replace(ctx);
        crate::core::material_mix_registry::set_restitution_ptr(slot, ptr);
        unsafe {
            ffi::b2World_SetRestitutionCallback(
                self.raw(),
                crate::core::material_mix_registry::restitution_callback(slot),
            );
        }
        drop(old);
        Ok(())
    }

    /// Clear the restitution mixing callback and restore Box2D's default mixing rule.
    pub fn clear_restitution_callback(&mut self) {
        assert_world_available(&self.core);
        let slot = *self
            .core
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned");
        if let Some(slot) = slot {
            unsafe { ffi::b2World_SetRestitutionCallback(self.raw(), None) };
            crate::core::material_mix_registry::set_restitution_ptr(slot, core::ptr::null_mut());
        }
        let old = self
            .core
            .restitution_mix
            .lock()
            .expect("restitution_mix mutex poisoned")
            .take();
        self.maybe_release_material_mix_slot();
        drop(old);
    }

    pub fn try_clear_restitution_callback(&mut self) -> crate::error::ApiResult<()> {
        check_world_available(&self.core)?;
        self.clear_restitution_callback();
        Ok(())
    }
}
