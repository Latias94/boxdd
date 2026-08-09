#[cfg(not(target_arch = "wasm32"))]
use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::core::callback_state::{
    CustomFilterCb, CustomFilterCtx, MaterialMixCtx, PreSolveCb, PreSolveCtx,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::core::identity_registry::StepShapeResolver;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::{marker::PhantomData, rc::Rc};

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

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn custom_filter_callback(
    a: ffi::b2ShapeId,
    b: ffi::b2ShapeId,
    context: *mut core::ffi::c_void,
) -> bool {
    // SAFETY: Box2D receives this pointer only while the active step owns the stable boxed context.
    let Some(ctx) = (unsafe { worker_callback_context::<CustomFilterCtx>(context) }) else {
        return true;
    };
    if ctx.worker.has_failed() {
        return true;
    }
    let Some((shape_a, shape_b)) = resolve_callback_shapes(
        &ctx.worker,
        &ctx.shapes,
        a,
        b,
        "World::step/custom_filter_callback",
    ) else {
        return true;
    };
    crate::core::callback_state::invoke_worker_callback(&ctx.worker, true, || {
        (ctx.cb)(shape_a, shape_b)
    })
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn pre_solve_callback(
    a: ffi::b2ShapeId,
    b: ffi::b2ShapeId,
    point: ffi::b2Pos,
    normal: ffi::b2Vec2,
    context: *mut core::ffi::c_void,
) -> bool {
    // SAFETY: Box2D receives this pointer only while the active step owns the stable boxed context.
    let Some(ctx) = (unsafe { worker_callback_context::<PreSolveCtx>(context) }) else {
        return true;
    };
    if ctx.worker.has_failed() {
        return true;
    }
    let Some((shape_a, shape_b)) = resolve_callback_shapes(
        &ctx.worker,
        &ctx.shapes,
        a,
        b,
        "World::step/pre_solve_callback",
    ) else {
        return true;
    };
    let point = crate::types::Position::from_raw(point);
    if !point.is_valid() {
        ctx.worker.record_error(crate::Error::InvalidNativeOutput {
            operation: "World::step/pre_solve_callback",
            output: "contact point",
            constraint: "finite coordinates",
        });
        return true;
    }
    let normal = crate::types::Vec2::from_raw(normal);
    if !callback_normal_is_valid(normal) {
        ctx.worker.record_error(crate::Error::InvalidNativeOutput {
            operation: "World::step/pre_solve_callback",
            output: "contact normal",
            constraint: "a finite unit vector within Box2D's length tolerance",
        });
        return true;
    }
    crate::core::callback_state::invoke_worker_callback(&ctx.worker, true, || {
        (ctx.cb)(shape_a, shape_b, point, normal)
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[inline]
unsafe fn worker_callback_context<'a, T>(context: *mut core::ffi::c_void) -> Option<&'a T> {
    if context.is_null() || !(context as usize).is_multiple_of(core::mem::align_of::<T>()) {
        return None;
    }
    // SAFETY: callers install a live `T` allocation for the complete synchronous native call.
    Some(unsafe { &*context.cast::<T>() })
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_callback_shapes(
    worker: &crate::core::callback_state::WorkerCallbackState,
    resolver: &StepShapeResolver,
    a: ffi::b2ShapeId,
    b: ffi::b2ShapeId,
    operation: &'static str,
) -> Option<(crate::types::ShapeId, crate::types::ShapeId)> {
    match (resolver.shape(a), resolver.shape(b)) {
        (Ok(a), Ok(b)) => Some((a, b)),
        _ => {
            worker.record_error(crate::Error::InvalidNativeOutput {
                operation,
                output: "shape ids",
                constraint: "active shapes in the step identity snapshot",
            });
            None
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[inline]
fn callback_normal_is_valid(normal: crate::types::Vec2) -> bool {
    normal.is_valid()
        && (1.0 - (normal.x * normal.x + normal.y * normal.y)).abs() < 100.0 * f32::EPSILON
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct PreparedStepCallbacks {
    custom_filter: Option<Box<CustomFilterCtx>>,
    pre_solve: Option<Box<PreSolveCtx>>,
    material_mix: Option<crate::core::material_mix_registry::ActiveMaterialMixSnapshot>,
}

#[cfg(not(target_arch = "wasm32"))]
impl PreparedStepCallbacks {
    pub(crate) fn install(self, world: ffi::b2WorldId) -> ActiveStepCallbacks {
        let mut active = ActiveStepCallbacks {
            world,
            custom_filter: self.custom_filter,
            pre_solve: self.pre_solve,
            material_mix: self.material_mix,
            _owner_thread: PhantomData,
        };
        unsafe {
            match active.custom_filter.as_deref_mut() {
                Some(context) => ffi::b2World_SetCustomFilterCallback(
                    world,
                    Some(custom_filter_callback),
                    crate::core::callback_state::worker_context_ptr(context).cast(),
                ),
                None => ffi::b2World_SetCustomFilterCallback(world, None, core::ptr::null_mut()),
            }
            match active.pre_solve.as_deref_mut() {
                Some(context) => ffi::b2World_SetPreSolveCallback(
                    world,
                    Some(pre_solve_callback),
                    crate::core::callback_state::worker_context_ptr(context).cast(),
                ),
                None => ffi::b2World_SetPreSolveCallback(world, None, core::ptr::null_mut()),
            }
        }
        active
    }
}

/// Owns native callback pointers for exactly one synchronous Box2D step.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct ActiveStepCallbacks {
    world: ffi::b2WorldId,
    custom_filter: Option<Box<CustomFilterCtx>>,
    pre_solve: Option<Box<PreSolveCtx>>,
    material_mix: Option<crate::core::material_mix_registry::ActiveMaterialMixSnapshot>,
    _owner_thread: PhantomData<Rc<()>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ActiveStepCallbacks {
    pub(crate) fn finish(self) {
        drop(self);
    }

    fn uninstall(&self) {
        unsafe {
            ffi::b2World_SetCustomFilterCallback(self.world, None, core::ptr::null_mut());
            ffi::b2World_SetPreSolveCallback(self.world, None, core::ptr::null_mut());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for ActiveStepCallbacks {
    fn drop(&mut self) {
        self.uninstall();
        if let Some(material_mix) = self.material_mix.take() {
            material_mix.finish();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn retire_callback_registration<T>(
    retired: Option<crate::core::world_core::CallbackRegistration<T>>,
) {
    let mut panic = crate::core::callback_state::PanicSlot::default();
    panic.run_cleanup(|| drop(retired));
    panic.resume_or_forget();
}

#[cfg(not(target_arch = "wasm32"))]
impl World {
    fn material_mix_state(
        &self,
    ) -> crate::error::Result<
        std::sync::MutexGuard<'_, crate::core::material_mix_registry::OwnedMaterialMixSlot>,
    > {
        match self.core.material_mix.lock() {
            Ok(state) => Ok(state),
            Err(_) => {
                self.core.poison();
                Err(crate::error::Error::WorldPoisoned)
            }
        }
    }

    fn material_mix_registry_error(
        &self,
        error: crate::core::material_mix_registry::MaterialMixRegistryError,
    ) -> crate::error::Error {
        match error {
            crate::core::material_mix_registry::MaterialMixRegistryError::SlotsExhausted
            | crate::core::material_mix_registry::MaterialMixRegistryError::PublicationGenerationExhausted => {
                crate::error::Error::CallbackSlotsExhausted
            }
            crate::core::material_mix_registry::MaterialMixRegistryError::InvalidSlot
            | crate::core::material_mix_registry::MaterialMixRegistryError::SlotPoisoned
            | crate::core::material_mix_registry::MaterialMixRegistryError::StaleLease
            | crate::core::material_mix_registry::MaterialMixRegistryError::InvalidOwnerState => {
                self.core.poison();
                crate::error::Error::WorldPoisoned
            }
        }
    }

    fn material_mix_operation_error(
        &self,
        failure: crate::core::material_mix_registry::MaterialMixOperationFailure,
    ) -> crate::error::Error {
        let public_error = self.material_mix_registry_error(failure.error());
        failure.into_retired().resume_drop_panics();
        public_error
    }

    fn set_custom_filter_impl<F>(&mut self, f: F)
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        let callback: Arc<CustomFilterCb> = Arc::new(f);
        let registration = crate::core::world_core::CallbackRegistration::new(Box::new(callback));
        let old = self
            .core
            .custom_filter
            .lock()
            .expect("custom_filter mutex poisoned")
            .replace(registration);
        retire_callback_registration(old);
    }

    fn clear_custom_filter_impl(&mut self) {
        let old = self
            .core
            .custom_filter
            .lock()
            .expect("custom_filter mutex poisoned")
            .take();
        retire_callback_registration(old);
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
        let callback: Arc<PreSolveCb> = Arc::new(f);
        let registration = crate::core::world_core::CallbackRegistration::new(Box::new(callback));
        let old = self
            .core
            .pre_solve
            .lock()
            .expect("pre_solve mutex poisoned")
            .replace(registration);
        retire_callback_registration(old);
    }

    fn clear_pre_solve_impl(&mut self) {
        let old = self
            .core
            .pre_solve
            .lock()
            .expect("pre_solve mutex poisoned")
            .take();
        retire_callback_registration(old);
    }

    pub(crate) fn prepare_step_callbacks(&self) -> crate::error::Result<PreparedStepCallbacks> {
        let material_mix = {
            let state = self.material_mix_state()?;
            state.activate_snapshot()
        };
        let material_mix = match material_mix {
            Ok(snapshot) => snapshot,
            Err(error) => return Err(self.material_mix_registry_error(error)),
        };
        let custom_filter = self
            .core
            .custom_filter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(|registration| Arc::clone(registration.context()));
        let pre_solve = self
            .core
            .pre_solve
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(|registration| Arc::clone(registration.context()));
        if custom_filter.is_none() && pre_solve.is_none() {
            return Ok(PreparedStepCallbacks {
                custom_filter: None,
                pre_solve: None,
                material_mix,
            });
        }
        let shapes = self.core.step_shape_resolver()?;
        let custom_filter = custom_filter.map(|cb| {
            Box::new(CustomFilterCtx {
                worker: Arc::clone(&self.core.worker_callbacks),
                shapes: Arc::clone(&shapes),
                cb,
            })
        });
        let pre_solve = pre_solve.map(|cb| {
            Box::new(PreSolveCtx {
                worker: Arc::clone(&self.core.worker_callbacks),
                shapes: Arc::clone(&shapes),
                cb,
            })
        });
        Ok(PreparedStepCallbacks {
            custom_filter,
            pre_solve,
            material_mix,
        })
    }

    // --- Collision/solve callbacks ---------------------------------------------------------
    /// Register a thread-safe custom filter closure. This is called when a contact pair is
    /// considered for collision if either shape has custom filtering enabled.
    /// Return false to disable the collision.
    ///
    /// The callback may run on Box2D worker threads and must not call world APIs.
    /// Its native context is published only for the synchronous duration of each step.
    pub fn set_custom_filter<F>(&mut self, f: F) -> crate::error::Result<()>
    where
        F: Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static,
    {
        let f = crate::core::callback_state::PendingUserValue::new(f);
        check_world_available(self)?;
        self.set_custom_filter_impl(f.into_inner());
        Ok(())
    }

    /// Clear the custom filter callback and release associated resources.
    pub fn clear_custom_filter(&mut self) -> crate::error::Result<()> {
        check_world_available(self)?;
        self.clear_custom_filter_impl();
        Ok(())
    }

    /// Register a thread-safe pre-solve closure. This is called after contact update (when enabled
    /// on shapes) and before the solver. Return false to disable the contact this step.
    ///
    /// The callback may run on Box2D worker threads and must not call world APIs.
    /// Its native context is published only for the synchronous duration of each step.
    pub fn set_pre_solve<F>(&mut self, f: F) -> crate::error::Result<()>
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
        let f = crate::core::callback_state::PendingUserValue::new(f);
        check_world_available(self)?;
        self.set_pre_solve_impl(f.into_inner());
        Ok(())
    }

    /// Clear the pre-solve callback and release associated resources.
    pub fn clear_pre_solve(&mut self) -> crate::error::Result<()> {
        check_world_available(self)?;
        self.clear_pre_solve_impl();
        Ok(())
    }

    /// Register a thread-safe friction mixing callback.
    ///
    /// This callback may run on Box2D worker threads and intentionally receives no world context.
    /// Use `user_material_id` to implement table-driven material behavior.
    ///
    /// The callback must return a finite, non-negative coefficient. An invalid result or panic is
    /// contained at the C boundary and resumed from the owning [`World::step`] call.
    ///
    /// The callback must not attempt to modify Box2D state or unsafely mutate shared application
    /// state.
    pub fn set_friction_callback<F>(
        &mut self,
        identity: crate::MixerId,
        f: F,
    ) -> crate::error::Result<()>
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        let f = crate::core::callback_state::PendingUserValue::new(f);
        check_world_available(self)?;
        let mut state = self.material_mix_state()?;
        let context = Arc::new(MaterialMixCtx {
            worker: Arc::clone(&self.core.worker_callbacks),
            cb: Box::new(f.into_inner()),
        });
        let registration =
            crate::core::material_mix_registry::MaterialMixerRegistration::new(identity, context);
        let update = state.set_friction(registration);
        drop(state);
        let update = match update {
            Ok(update) => update,
            Err(failure) => return Err(self.material_mix_operation_error(failure)),
        };
        unsafe {
            ffi::b2World_SetFrictionCallback(
                self.raw(),
                crate::core::material_mix_registry::friction_callback(update.slot()),
            );
        }
        update.into_retired().resume_drop_panics();
        Ok(())
    }

    /// Clear the friction mixing callback and restore Box2D's default mixing rule.
    fn clear_friction_callback_impl(&mut self) -> crate::error::Result<()> {
        let releases_slot = self.material_mix_state()?.clearing_friction_releases_slot();
        if releases_slot {
            // A trampoline encodes only its slot index. Remove the last native reference before
            // allowing that slot to be leased by another owner; entered calls already own an Arc.
            unsafe { ffi::b2World_SetFrictionCallback(self.raw(), None) };
        }
        let retired = {
            let mut state = self.material_mix_state()?;
            state.clear_friction()
        };
        let retired = match retired {
            Ok(retired) => retired,
            Err(failure) => return Err(self.material_mix_operation_error(failure)),
        };
        if !releases_slot {
            unsafe { ffi::b2World_SetFrictionCallback(self.raw(), None) };
        }
        retired.resume_drop_panics();
        Ok(())
    }

    pub fn clear_friction_callback(&mut self) -> crate::error::Result<()> {
        check_world_available(self)?;
        self.clear_friction_callback_impl()
    }

    /// Register a thread-safe restitution mixing callback.
    ///
    /// This callback may run on Box2D worker threads and intentionally receives no world context.
    /// Use `user_material_id` to implement table-driven material behavior.
    ///
    /// The callback must return a finite, non-negative coefficient. An invalid result or panic is
    /// contained at the C boundary and resumed from the owning [`World::step`] call.
    ///
    /// The callback must not attempt to modify Box2D state or unsafely mutate shared application
    /// state.
    pub fn set_restitution_callback<F>(
        &mut self,
        identity: crate::MixerId,
        f: F,
    ) -> crate::error::Result<()>
    where
        F: Fn(MaterialMixInput, MaterialMixInput) -> f32 + Send + Sync + 'static,
    {
        let f = crate::core::callback_state::PendingUserValue::new(f);
        check_world_available(self)?;
        let mut state = self.material_mix_state()?;
        let context = Arc::new(MaterialMixCtx {
            worker: Arc::clone(&self.core.worker_callbacks),
            cb: Box::new(f.into_inner()),
        });
        let registration =
            crate::core::material_mix_registry::MaterialMixerRegistration::new(identity, context);
        let update = state.set_restitution(registration);
        drop(state);
        let update = match update {
            Ok(update) => update,
            Err(failure) => return Err(self.material_mix_operation_error(failure)),
        };
        unsafe {
            ffi::b2World_SetRestitutionCallback(
                self.raw(),
                crate::core::material_mix_registry::restitution_callback(update.slot()),
            );
        }
        update.into_retired().resume_drop_panics();
        Ok(())
    }

    /// Clear the restitution mixing callback and restore Box2D's default mixing rule.
    fn clear_restitution_callback_impl(&mut self) -> crate::error::Result<()> {
        let releases_slot = self
            .material_mix_state()?
            .clearing_restitution_releases_slot();
        if releases_slot {
            // See the friction path: native uninstallation must precede final slot release.
            unsafe { ffi::b2World_SetRestitutionCallback(self.raw(), None) };
        }
        let retired = {
            let mut state = self.material_mix_state()?;
            state.clear_restitution()
        };
        let retired = match retired {
            Ok(retired) => retired,
            Err(failure) => return Err(self.material_mix_operation_error(failure)),
        };
        if !releases_slot {
            unsafe { ffi::b2World_SetRestitutionCallback(self.raw(), None) };
        }
        retired.resume_drop_panics();
        Ok(())
    }

    pub fn clear_restitution_callback(&mut self) -> crate::error::Result<()> {
        check_world_available(self)?;
        self.clear_restitution_callback_impl()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::{BodyType, ShapeDef};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    static_assertions::assert_not_impl_any!(ActiveStepCallbacks: Send, Sync);

    fn callback_shapes(world: &mut World) -> (ffi::b2ShapeId, ffi::b2ShapeId) {
        let body_a = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let body_b = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let shape_def = ShapeDef::builder().density(1.0).build().unwrap();
        let polygon = crate::shapes::box_polygon(0.5, 0.5).unwrap();
        let shape_a = world
            .body(body_a)
            .unwrap()
            .create_polygon(&shape_def, &polygon)
            .unwrap();
        let shape_b = world
            .body(body_b)
            .unwrap()
            .create_polygon(&shape_def, &polygon)
            .unwrap();
        (shape_a.into_raw(), shape_b.into_raw())
    }

    #[test]
    fn invalid_callback_contexts_use_the_conservative_fallback() {
        let shape = ffi::b2ShapeId {
            index1: 0,
            world0: 0,
            generation: 0,
        };
        let invalid_context = core::ptr::dangling_mut::<core::ffi::c_void>();

        // SAFETY: these calls intentionally exercise the callback's pre-dereference validation.
        unsafe {
            assert!(custom_filter_callback(shape, shape, core::ptr::null_mut()));
            assert!(custom_filter_callback(shape, shape, invalid_context));
            assert!(pre_solve_callback(
                shape,
                shape,
                Position::ZERO.into_raw(),
                Vec2::new(0.0, 1.0).into_raw(),
                core::ptr::null_mut(),
            ));
            assert!(pre_solve_callback(
                shape,
                shape,
                Position::ZERO.into_raw(),
                Vec2::new(0.0, 1.0).into_raw(),
                invalid_context,
            ));
        }
    }

    #[test]
    fn invalid_callback_shape_is_reported_without_invoking_user_code() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let (shape_a, mut invalid_shape) = callback_shapes(&mut world);
        invalid_shape.index1 = 0;
        let calls = Arc::new(AtomicUsize::new(0));
        let worker = Arc::clone(&world.core().worker_callbacks);
        let context = Box::new(CustomFilterCtx {
            worker: Arc::clone(&worker),
            shapes: world.core().step_shape_resolver().unwrap(),
            cb: Arc::new({
                let calls = Arc::clone(&calls);
                move |_, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    false
                }
            }),
        });
        let context_ptr = crate::core::callback_state::worker_context_ptr(context.as_ref()).cast();

        // SAFETY: the callback context remains allocated and `shape_a` belongs to `world`.
        assert!(unsafe { custom_filter_callback(shape_a, invalid_shape, context_ptr) });
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(matches!(
            worker.begin_call(),
            Err(crate::Error::InvalidNativeOutput {
                operation: "World::step/custom_filter_callback",
                output: "shape ids",
                ..
            })
        ));
        assert!(!worker.has_failed());
    }

    #[test]
    fn invalid_pre_solve_geometry_is_reported_without_invoking_user_code() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let (shape_a, shape_b) = callback_shapes(&mut world);
        let calls = Arc::new(AtomicUsize::new(0));
        let worker = Arc::clone(&world.core().worker_callbacks);
        let context = Box::new(PreSolveCtx {
            worker: Arc::clone(&worker),
            shapes: world.core().step_shape_resolver().unwrap(),
            cb: Arc::new({
                let calls = Arc::clone(&calls);
                move |_, _, _, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    false
                }
            }),
        });
        let context_ptr = crate::core::callback_state::worker_context_ptr(context.as_ref()).cast();
        let mut non_finite_point = Position::ZERO.into_raw();
        non_finite_point.x = crate::types::WorldScalar::NAN;
        let cases = [
            (non_finite_point, Vec2::new(0.0, 1.0).into_raw()),
            (Position::ZERO.into_raw(), Vec2::new(2.0, 0.0).into_raw()),
            (
                Position::ZERO.into_raw(),
                Vec2::new(f32::NAN, 0.0).into_raw(),
            ),
        ];

        for (point, normal) in cases {
            // SAFETY: the callback context remains allocated and both shapes belong to `world`.
            assert!(unsafe { pre_solve_callback(shape_a, shape_b, point, normal, context_ptr) });
            assert!(matches!(
                worker.begin_call(),
                Err(crate::Error::InvalidNativeOutput {
                    operation: "World::step/pre_solve_callback",
                    ..
                })
            ));
            assert!(!worker.has_failed());
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn custom_filter_panic_uses_true_fallback_and_reuses_worker_state() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let (shape_a, shape_b) = callback_shapes(&mut world);
        let worker = Arc::clone(&world.core().worker_callbacks);
        let shapes = world.core().step_shape_resolver().unwrap();
        let context = Box::new(CustomFilterCtx {
            worker: Arc::clone(&worker),
            shapes: Arc::clone(&shapes),
            cb: Arc::new(|_, _| -> bool { panic!("custom filter test panic") }),
        });
        let context_ptr = crate::core::callback_state::worker_context_ptr(context.as_ref()).cast();

        // SAFETY: the callback context remains allocated and the raw shape IDs belong to `world`.
        let first = unsafe { custom_filter_callback(shape_a, shape_b, context_ptr) };
        // A worker callback is disabled after its first panic; the fallback must remain stable.
        let second = unsafe { custom_filter_callback(shape_a, shape_b, context_ptr) };
        assert!(first);
        assert!(second);

        let mut panic = crate::core::callback_state::PanicSlot::default();
        worker.drain_panics(&mut panic);
        assert!(panic.into_result(()).is_err());
        worker.begin_call().unwrap();
        drop(context);
    }

    #[test]
    fn pre_solve_panic_uses_true_fallback_and_reuses_worker_state() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let (shape_a, shape_b) = callback_shapes(&mut world);
        let worker = Arc::clone(&world.core().worker_callbacks);
        let shapes = world.core().step_shape_resolver().unwrap();
        let context = Box::new(PreSolveCtx {
            worker: Arc::clone(&worker),
            shapes: Arc::clone(&shapes),
            cb: Arc::new(|_, _, _, _| -> bool { panic!("pre-solve test panic") }),
        });
        let context_ptr = crate::core::callback_state::worker_context_ptr(context.as_ref()).cast();

        // SAFETY: the callback context remains allocated and the raw shape IDs belong to `world`.
        let first = unsafe {
            pre_solve_callback(
                shape_a,
                shape_b,
                Position::ZERO.into_raw(),
                Vec2::new(0.0, 1.0).into_raw(),
                context_ptr,
            )
        };
        let second = unsafe {
            pre_solve_callback(
                shape_a,
                shape_b,
                Position::ZERO.into_raw(),
                Vec2::new(0.0, 1.0).into_raw(),
                context_ptr,
            )
        };
        assert!(first);
        assert!(second);

        let mut panic = crate::core::callback_state::PanicSlot::default();
        worker.drain_panics(&mut panic);
        assert!(panic.into_result(()).is_err());
        worker.begin_call().unwrap();
        drop(context);
    }

    #[test]
    fn callback_resolution_does_not_wait_for_the_owner_registry_lock() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let (shape_a, shape_b) = callback_shapes(&mut world);
        let shapes = world.core().step_shape_resolver().unwrap();
        let worker = Arc::clone(&world.core().worker_callbacks);
        let owner_lock = world.core().hold_identity_lock_for_test();
        let context = Box::new(CustomFilterCtx {
            worker,
            shapes: Arc::clone(&shapes),
            cb: Arc::new(|_, _| true),
        });
        let (send, receive) = mpsc::sync_channel(1);

        std::thread::scope(|scope| {
            let handle = scope.spawn(move || {
                let context_ptr =
                    crate::core::callback_state::worker_context_ptr(context.as_ref()).cast();
                // SAFETY: the boxed context and immutable resolver outlive this scoped worker.
                let result = unsafe { custom_filter_callback(shape_a, shape_b, context_ptr) };
                send.send(result).unwrap();
            });

            let result = receive.recv_timeout(Duration::from_secs(1));
            drop(owner_lock);
            handle.join().unwrap();
            assert_eq!(result, Ok(true));
        });
    }

    #[test]
    fn snapshot_restore_republishes_registered_filter_on_the_next_step() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_builder()
                    .gravity([0.0_f32, 0.0])
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let shape_def = ShapeDef::builder()
            .density(1.0)
            .enable_custom_filtering(true)
            .build()
            .unwrap();
        let polygon = crate::shapes::box_polygon(0.5, 0.5).unwrap();
        for x in [0.0_f32, 0.25] {
            let body = world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_builder()
                        .body_type(BodyType::Dynamic)
                        .position([x, 0.0])
                        .build()
                        .unwrap(),
                )
                .unwrap();
            world
                .body(body)
                .unwrap()
                .create_polygon(&shape_def, &polygon)
                .unwrap();
        }

        let calls = Arc::new(AtomicUsize::new(0));
        world
            .set_custom_filter({
                let calls = Arc::clone(&calls);
                move |_, _| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    false
                }
            })
            .unwrap();
        let snapshot = world.snapshot().unwrap();

        drop(world.step(1.0 / 60.0, 1).unwrap());
        assert!(calls.load(Ordering::SeqCst) > 0);
        world.restore(&snapshot).unwrap();
        calls.store(0, Ordering::SeqCst);

        drop(world.step(1.0 / 60.0, 1).unwrap());
        assert!(calls.load(Ordering::SeqCst) > 0);
    }
}
