use boxdd_sys::ffi;
use std::any::Any;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

pub(crate) struct CustomFilterCtx {
    pub(crate) core: Weak<WorldCore>,
    pub(crate) cb:
        Box<dyn Fn(crate::types::ShapeId, crate::types::ShapeId) -> bool + Send + Sync + 'static>,
}

pub(crate) struct PreSolveCtx {
    pub(crate) core: Weak<WorldCore>,
    pub(crate) cb: Box<
        dyn Fn(
                crate::types::ShapeId,
                crate::types::ShapeId,
                crate::types::Vec2,
                crate::types::Vec2,
            ) -> bool
            + Send
            + Sync
            + 'static,
    >,
}

pub(crate) struct WorldCore {
    pub(crate) id: ffi::b2WorldId,
    pub(crate) custom_filter: Mutex<Option<Box<CustomFilterCtx>>>,
    pub(crate) pre_solve: Mutex<Option<Box<PreSolveCtx>>>,
    pub(crate) callback_panicked: AtomicBool,
    pub(crate) callback_panic: Mutex<Option<Box<dyn Any + Send + 'static>>>,
    pub(crate) deferred_destroys: Mutex<Vec<DeferredDestroy>>,
    #[cfg(feature = "serialize")]
    pub(crate) registries: Mutex<crate::core::serialize_registry::Registries>,
    pub(crate) owned_bodies: AtomicUsize,
    pub(crate) owned_shapes: AtomicUsize,
    pub(crate) owned_joints: AtomicUsize,
    pub(crate) owned_chains: AtomicUsize,
}

#[derive(Clone, Debug)]
pub(crate) enum DeferredDestroy {
    Body(ffi::b2BodyId),
    Shape {
        id: ffi::b2ShapeId,
        update_body_mass: bool,
    },
    Joint {
        id: ffi::b2JointId,
        wake_bodies: bool,
    },
    Chain(ffi::b2ChainId),
}

impl WorldCore {
    pub(crate) fn new(id: ffi::b2WorldId) -> Arc<Self> {
        Arc::new(Self {
            id,
            custom_filter: Mutex::new(None),
            pre_solve: Mutex::new(None),
            callback_panicked: AtomicBool::new(false),
            callback_panic: Mutex::new(None),
            deferred_destroys: Mutex::new(Vec::new()),
            #[cfg(feature = "serialize")]
            registries: Mutex::new(crate::core::serialize_registry::Registries::default()),
            owned_bodies: AtomicUsize::new(0),
            owned_shapes: AtomicUsize::new(0),
            owned_joints: AtomicUsize::new(0),
            owned_chains: AtomicUsize::new(0),
        })
    }

    pub(crate) fn owned_counts(&self) -> (usize, usize, usize, usize) {
        (
            self.owned_bodies.load(Ordering::Relaxed),
            self.owned_shapes.load(Ordering::Relaxed),
            self.owned_joints.load(Ordering::Relaxed),
            self.owned_chains.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn defer_destroy(&self, d: DeferredDestroy) {
        self.deferred_destroys
            .lock()
            .expect("deferred_destroys mutex poisoned")
            .push(d);
    }

    pub(crate) fn process_deferred_destroys(&self) {
        crate::core::callback_state::assert_not_in_callback();
        let mut pending = self
            .deferred_destroys
            .lock()
            .expect("deferred_destroys mutex poisoned");
        if pending.is_empty() {
            return;
        }
        let items = core::mem::take(&mut *pending);
        drop(pending);

        for item in items {
            match item {
                DeferredDestroy::Body(id) => {
                    if unsafe { ffi::b2Body_IsValid(id) } {
                        #[cfg(feature = "serialize")]
                        {
                            let mut r = self.registries.lock().expect("registries mutex poisoned");
                            r.remove_shape_flags_for_body(id);
                            r.remove_chains_for_body(id);
                            r.remove_body(id);
                        }
                        unsafe { ffi::b2DestroyBody(id) };
                    }
                }
                DeferredDestroy::Shape {
                    id,
                    update_body_mass,
                } => {
                    if unsafe { ffi::b2Shape_IsValid(id) } {
                        unsafe { ffi::b2DestroyShape(id, update_body_mass) };
                        #[cfg(feature = "serialize")]
                        {
                            self.registries
                                .lock()
                                .expect("registries mutex poisoned")
                                .remove_shape_flags(id);
                        }
                    }
                }
                DeferredDestroy::Joint { id, wake_bodies } => {
                    if unsafe { ffi::b2Joint_IsValid(id) } {
                        unsafe { ffi::b2DestroyJoint(id, wake_bodies) };
                    }
                }
                DeferredDestroy::Chain(id) => {
                    if unsafe { ffi::b2Chain_IsValid(id) } {
                        unsafe { ffi::b2DestroyChain(id) };
                        #[cfg(feature = "serialize")]
                        {
                            self.registries
                                .lock()
                                .expect("registries mutex poisoned")
                                .remove_chain(id);
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn record_body(&self, id: ffi::b2BodyId) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .record_body(id);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn record_chain(
        &self,
        id: crate::types::ChainId,
        meta: crate::core::serialize_registry::ChainCreateMeta,
    ) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .record_chain(id, meta);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn record_shape_flags(&self, sid: ffi::b2ShapeId, def: &ffi::b2ShapeDef) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .record_shape_flags(sid, def);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn remove_chain(&self, id: crate::types::ChainId) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .remove_chain(id);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn remove_shape_flags(&self, sid: ffi::b2ShapeId) {
        self.registries
            .lock()
            .expect("registries mutex poisoned")
            .remove_shape_flags(sid);
    }

    #[cfg(feature = "serialize")]
    pub(crate) fn cleanup_before_destroy_body(&self, id: ffi::b2BodyId) {
        crate::core::callback_state::assert_not_in_callback();
        let mut r = self.registries.lock().expect("registries mutex poisoned");
        r.remove_shape_flags_for_body(id);
        r.remove_chains_for_body(id);
        r.remove_body(id);
    }
}

impl Drop for WorldCore {
    fn drop(&mut self) {
        let _guard = crate::core::box2d_lock::lock();
        // SAFETY: `WorldCore` owns the Box2D world id; only the last Arc drops it.
        unsafe { ffi::b2DestroyWorld(self.id) };
    }
}
