use super::*;

type ShapeFilterFn = fn(crate::types::ShapeId, crate::types::ShapeId) -> bool;
type PreSolveFn = fn(
    crate::types::ShapeId,
    crate::types::ShapeId,
    crate::types::Vec2,
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
    let core = match ctx.core.upgrade() {
        Some(c) => c,
        None => return true,
    };
    if core
        .callback_panicked
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return true;
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = crate::core::callback_state::CallbackGuard::enter();
        let cw = CallbackWorld::new(Arc::clone(&core));
        (ctx.cb)(&cw, ShapeId::from_raw(a), ShapeId::from_raw(b))
    })) {
        Ok(v) => v,
        Err(payload) => {
            if !core
                .callback_panicked
                .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                *core
                    .callback_panic
                    .lock()
                    .expect("callback_panic mutex poisoned") = Some(payload);
            }
            true
        }
    }
}

unsafe extern "C" fn pre_solve_callback(
    a: ffi::b2ShapeId,
    b: ffi::b2ShapeId,
    point: ffi::b2Vec2,
    normal: ffi::b2Vec2,
    context: *mut core::ffi::c_void,
) -> bool {
    // SAFETY: context is provided by the pre-solve registration helpers and points to
    // `PreSolveCtx` for the lifetime of the registered callback.
    let ctx = unsafe { &*(context as *const PreSolveCtx) };
    let core = match ctx.core.upgrade() {
        Some(c) => c,
        None => return true,
    };
    if core
        .callback_panicked
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return true;
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = crate::core::callback_state::CallbackGuard::enter();
        let cw = CallbackWorld::new(Arc::clone(&core));
        (ctx.cb)(
            &cw,
            ShapeId::from_raw(a),
            ShapeId::from_raw(b),
            crate::types::Vec2::from_raw(point),
            crate::types::Vec2::from_raw(normal),
        )
    })) {
        Ok(v) => v,
        Err(payload) => {
            if !core
                .callback_panicked
                .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                *core
                    .callback_panic
                    .lock()
                    .expect("callback_panic mutex poisoned") = Some(payload);
            }
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

    fn set_custom_filter_with_ctx_impl<F>(&mut self, f: F)
    where
        F: Fn(&CallbackWorld, crate::types::ShapeId, crate::types::ShapeId) -> bool
            + Send
            + Sync
            + 'static,
    {
        let ctx = Box::new(CustomFilterCtx {
            core: Arc::downgrade(&self.core),
            cb: Box::new(f),
        });
        self.install_custom_filter_ctx(ctx);
    }

    fn install_custom_filter_ctx(&mut self, ctx: Box<CustomFilterCtx>) {
        let ctx_ptr: *mut core::ffi::c_void = (&*ctx) as *const CustomFilterCtx as *mut _;
        unsafe {
            ffi::b2World_SetCustomFilterCallback(self.raw(), Some(custom_filter_callback), ctx_ptr)
        };
        *self
            .core
            .custom_filter
            .lock()
            .expect("custom_filter mutex poisoned") = Some(ctx);
    }

    fn clear_custom_filter_impl(&mut self) {
        unsafe { ffi::b2World_SetCustomFilterCallback(self.raw(), None, core::ptr::null_mut()) };
        *self
            .core
            .custom_filter
            .lock()
            .expect("custom_filter mutex poisoned") = None;
    }

    fn set_pre_solve_with_ctx_impl<F>(&mut self, f: F)
    where
        F: Fn(
                &CallbackWorld,
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        let ctx = Box::new(PreSolveCtx {
            core: Arc::downgrade(&self.core),
            cb: Box::new(f),
        });
        self.install_pre_solve_ctx(ctx);
    }

    fn install_pre_solve_ctx(&mut self, ctx: Box<PreSolveCtx>) {
        let ctx_ptr: *mut core::ffi::c_void = (&*ctx) as *const PreSolveCtx as *mut _;
        unsafe { ffi::b2World_SetPreSolveCallback(self.raw(), Some(pre_solve_callback), ctx_ptr) };
        *self
            .core
            .pre_solve
            .lock()
            .expect("pre_solve mutex poisoned") = Some(ctx);
    }

    fn clear_pre_solve_impl(&mut self) {
        unsafe { ffi::b2World_SetPreSolveCallback(self.raw(), None, core::ptr::null_mut()) };
        *self
            .core
            .pre_solve
            .lock()
            .expect("pre_solve mutex poisoned") = None;
    }

    // --- Collision/solve callbacks ---------------------------------------------------------
    /// Register a thread-safe custom filter closure. This is called when a contact pair is
    /// considered for collision if either shape has custom filtering enabled.
    /// Return false to disable the collision.
    ///
    /// Note: Box2D runs this callback while the world is locked. Use the provided `CallbackWorld`
    /// context for operations that must be safe under this constraint (e.g. typed user data).
    pub fn set_custom_filter_with_ctx<F>(&mut self, f: F)
    where
        F: Fn(&CallbackWorld, crate::types::ShapeId, crate::types::ShapeId) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::assert_not_in_callback();
        self.set_custom_filter_with_ctx_impl(f);
    }

    pub fn try_set_custom_filter_with_ctx<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(&CallbackWorld, crate::types::ShapeId, crate::types::ShapeId) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        self.set_custom_filter_with_ctx_impl(f);
        Ok(())
    }

    /// Backwards-compatible custom filter API without a callback context.
    pub fn set_custom_filter<F>(&mut self, f: F)
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        crate::core::callback_state::assert_not_in_callback();
        self.set_custom_filter_with_ctx_impl(move |_, a, b| f(a, b))
    }

    pub fn try_set_custom_filter<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        self.set_custom_filter_with_ctx_impl(move |_, a, b| f(a, b));
        Ok(())
    }

    /// Clear the custom filter callback and release associated resources.
    pub fn clear_custom_filter(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        self.clear_custom_filter_impl();
    }

    pub fn try_clear_custom_filter(&mut self) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.clear_custom_filter_impl();
        Ok(())
    }

    /// Register a thread-safe pre-solve closure. This is called after contact update (when enabled
    /// on shapes) and before the solver. Return false to disable the contact this step.
    ///
    /// Note: Box2D runs this callback while the world is locked. Use the provided `CallbackWorld`
    /// context for operations that must be safe under this constraint (e.g. typed user data).
    pub fn set_pre_solve_with_ctx<F>(&mut self, f: F)
    where
        F: Fn(
                &CallbackWorld,
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::assert_not_in_callback();
        self.set_pre_solve_with_ctx_impl(f);
    }

    pub fn try_set_pre_solve_with_ctx<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(
                &CallbackWorld,
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        self.set_pre_solve_with_ctx_impl(f);
        Ok(())
    }

    /// Backwards-compatible pre-solve API without a callback context.
    pub fn set_pre_solve<F>(&mut self, f: F)
    where
        F: Fn(
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::assert_not_in_callback();
        self.set_pre_solve_with_ctx_impl(move |_, a, b, p, n| f(a, b, p, n))
    }

    pub fn try_set_pre_solve<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        self.set_pre_solve_with_ctx_impl(move |_, a, b, p, n| f(a, b, p, n));
        Ok(())
    }

    /// Clear the pre-solve callback and release associated resources.
    pub fn clear_pre_solve(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        self.clear_pre_solve_impl();
    }

    pub fn try_clear_pre_solve(&mut self) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.clear_pre_solve_impl();
        Ok(())
    }

    /// Compatibility helper: set or clear the custom filter using a plain function pointer.
    pub fn set_custom_filter_callback(&mut self, cb: Option<ShapeFilterFn>) {
        crate::core::callback_state::assert_not_in_callback();
        match cb {
            Some(func) => self.set_custom_filter_with_ctx_impl(move |_, a, b| func(a, b)),
            None => self.clear_custom_filter_impl(),
        }
    }

    pub fn try_set_custom_filter_callback(
        &mut self,
        cb: Option<ShapeFilterFn>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        match cb {
            Some(func) => self.set_custom_filter_with_ctx_impl(move |_, a, b| func(a, b)),
            None => self.clear_custom_filter_impl(),
        }
        Ok(())
    }

    /// Compatibility helper: set or clear the pre-solve using a plain function pointer.
    pub fn set_pre_solve_callback(&mut self, cb: Option<PreSolveFn>) {
        crate::core::callback_state::assert_not_in_callback();
        match cb {
            Some(func) => self.set_pre_solve_with_ctx_impl(move |_, a, b, p, n| func(a, b, p, n)),
            None => self.clear_pre_solve_impl(),
        }
    }

    pub fn try_set_pre_solve_callback(
        &mut self,
        cb: Option<PreSolveFn>,
    ) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        match cb {
            Some(func) => self.set_pre_solve_with_ctx_impl(move |_, a, b, p, n| func(a, b, p, n)),
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
        self.try_set_friction_callback(f)
            .expect("no free callback slot is available for material mixing callbacks");
    }

    pub fn try_set_friction_callback<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        let slot = self.ensure_material_mix_slot()?;
        let ctx = Box::new(MaterialMixCtx {
            core: Arc::downgrade(&self.core),
            cb: Box::new(f),
        });
        let ptr = (&*ctx) as *const MaterialMixCtx as *mut MaterialMixCtx;
        crate::core::material_mix_registry::set_friction_ptr(slot, ptr);
        *self
            .core
            .friction_mix
            .lock()
            .expect("friction_mix mutex poisoned") = Some(ctx);
        unsafe {
            ffi::b2World_SetFrictionCallback(
                self.raw(),
                crate::core::material_mix_registry::friction_callback(slot),
            );
        }
        Ok(())
    }

    /// Clear the friction mixing callback and restore Box2D's default mixing rule.
    pub fn clear_friction_callback(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        if let Some(slot) = *self
            .core
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned")
        {
            unsafe { ffi::b2World_SetFrictionCallback(self.raw(), None) };
            crate::core::material_mix_registry::set_friction_ptr(slot, core::ptr::null_mut());
            *self
                .core
                .friction_mix
                .lock()
                .expect("friction_mix mutex poisoned") = None;
            self.maybe_release_material_mix_slot();
        }
    }

    pub fn try_clear_friction_callback(&mut self) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
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
        self.try_set_restitution_callback(f)
            .expect("no free callback slot is available for material mixing callbacks");
    }

    pub fn try_set_restitution_callback<F>(&mut self, f: F) -> crate::error::ApiResult<()>
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        crate::core::callback_state::check_not_in_callback()?;
        let slot = self.ensure_material_mix_slot()?;
        let ctx = Box::new(MaterialMixCtx {
            core: Arc::downgrade(&self.core),
            cb: Box::new(f),
        });
        let ptr = (&*ctx) as *const MaterialMixCtx as *mut MaterialMixCtx;
        crate::core::material_mix_registry::set_restitution_ptr(slot, ptr);
        *self
            .core
            .restitution_mix
            .lock()
            .expect("restitution_mix mutex poisoned") = Some(ctx);
        unsafe {
            ffi::b2World_SetRestitutionCallback(
                self.raw(),
                crate::core::material_mix_registry::restitution_callback(slot),
            );
        }
        Ok(())
    }

    /// Clear the restitution mixing callback and restore Box2D's default mixing rule.
    pub fn clear_restitution_callback(&mut self) {
        crate::core::callback_state::assert_not_in_callback();
        if let Some(slot) = *self
            .core
            .material_mix_slot
            .lock()
            .expect("material_mix_slot mutex poisoned")
        {
            unsafe { ffi::b2World_SetRestitutionCallback(self.raw(), None) };
            crate::core::material_mix_registry::set_restitution_ptr(slot, core::ptr::null_mut());
            *self
                .core
                .restitution_mix
                .lock()
                .expect("restitution_mix mutex poisoned") = None;
            self.maybe_release_material_mix_slot();
        }
    }

    pub fn try_clear_restitution_callback(&mut self) -> crate::error::ApiResult<()> {
        crate::core::callback_state::check_not_in_callback()?;
        self.clear_restitution_callback();
        Ok(())
    }
}
