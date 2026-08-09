use crate::core::world_core::WorldCore;
use crate::error::Result;
use core::cell::Cell;

/// Crate-private authority behind every borrow-scoped object capability.
///
/// The trait lives in a private module, so external code cannot mint an access proof. Implementors
/// must keep `core` alive and preserve the selected activity for the complete borrow.
pub(crate) trait OwnerAdapter {
    fn capability_core(&self) -> &WorldCore;
    fn capability_completed_step(&self) -> &crate::events::CompletedStepState;
    /// Reject callbacks, unavailable activity, and any sticky owner failure before user input is
    /// converted or validated.
    fn capability_preflight(&self) -> Result<()>;
    /// Surface failures produced out-of-band by the operation which just completed.
    fn capability_postflight(&self) -> Result<()>;
}

struct OwnerPostflightGuard<'owner> {
    owner: &'owner dyn OwnerAdapter,
    finished: bool,
}

impl<'owner> OwnerPostflightGuard<'owner> {
    fn new(owner: &'owner dyn OwnerAdapter) -> Self {
        Self {
            owner,
            finished: false,
        }
    }

    fn finish<R>(mut self, result: Result<R>) -> Result<R> {
        self.finished = true;
        self.owner.capability_postflight()?;
        result
    }
}

impl Drop for OwnerPostflightGuard<'_> {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.owner.capability_postflight();
        }
    }
}

fn run_owner_operation<R>(
    owner: &dyn OwnerAdapter,
    operation: impl FnOnce(&WorldCore) -> Result<R>,
) -> Result<R> {
    let operation = crate::core::callback_state::PendingUserValue::new(operation);
    owner.capability_preflight()?;
    let guard = OwnerPostflightGuard::new(owner);
    let operation = operation.into_inner();
    guard.finish(operation(owner.capability_core()))
}

/// Run one whole-world operation through the selected owner's activity gate.
///
/// The call object deliberately exposes only the native world identity. Domain modules cannot
/// select an activity mode or bypass owner-specific postflight checks.
pub(crate) fn run_owner_call<R>(
    owner: &dyn OwnerAdapter,
    operation: impl FnOnce(WorldCall<'_>) -> Result<R>,
) -> Result<R> {
    run_owner_operation(owner, |core| operation(WorldCall { core }))
}

/// Construct a joint base only after the selected owner has authenticated both body ids.
///
/// This keeps the length-scale provenance and the body-id provenance coupled to the same active
/// owner. Recording owners additionally run their writer status postflight before the definition
/// becomes observable.
pub(crate) fn joint_base_for_owner(
    owner: &dyn OwnerAdapter,
    body_a: crate::BodyId,
    body_b: crate::BodyId,
) -> Result<crate::JointBase> {
    run_owner_operation(owner, |core| {
        core.check_body_after_preflight(body_a)?;
        core.check_body_after_preflight(body_b)?;
        let base = core.joint_base(body_a, body_b);
        base.validate()?;
        Ok(base)
    })
}

/// Narrow authority for one whole-world operation after owner preflight.
#[derive(Copy, Clone)]
pub(crate) struct WorldCall<'call> {
    core: &'call WorldCore,
}

impl WorldCall<'_> {
    pub(crate) fn raw_world(&self) -> boxdd_sys::ffi::b2WorldId {
        self.core.id
    }
}

/// Owner transaction for native creation followed by infallible identity publication.
///
/// Creation code reserves every Rust resource before entering native code. This guard then keeps
/// owner postflight ahead of publication, so a recording writer failure cannot leave behind a live
/// object whose returned ID was discarded.
#[must_use = "a creation transaction must publish or abort"]
pub(crate) struct OwnerCreation<'owner> {
    owner: &'owner dyn OwnerAdapter,
    finished: bool,
}

impl<'owner> OwnerCreation<'owner> {
    pub(crate) fn begin(owner: &'owner dyn OwnerAdapter) -> Result<Self> {
        owner.capability_preflight()?;
        Ok(Self {
            owner,
            finished: false,
        })
    }

    pub(crate) fn core(&self) -> &'owner WorldCore {
        self.owner.capability_core()
    }

    pub(crate) fn finish<R>(mut self, publish: impl FnOnce() -> R) -> Result<R> {
        self.finished = true;
        self.owner.capability_postflight()?;
        Ok(publish())
    }

    pub(crate) fn abort<R>(mut self, error: crate::Error) -> Result<R> {
        self.finished = true;
        self.owner.capability_postflight()?;
        Err(error)
    }
}

impl Drop for OwnerCreation<'_> {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.owner.capability_postflight();
        }
    }
}

/// A shared owner proof dedicated to world queries.
pub(crate) struct QueryProof<'owner> {
    owner: &'owner dyn OwnerAdapter,
}

#[derive(Copy, Clone)]
struct BodyKey(crate::BodyId);

#[derive(Copy, Clone)]
struct ShapeKey {
    id: crate::ShapeId,
    kind: crate::ShapeType,
}

#[derive(Copy, Clone)]
struct ChainKey(crate::ChainId);

#[derive(Copy, Clone)]
struct JointKey {
    id: crate::JointId,
    kind: crate::JointType,
}

/// An owner borrow plus the body identity validated when the capability was acquired.
pub(crate) struct BodyProof<'owner> {
    owner: &'owner dyn OwnerAdapter,
    key: BodyKey,
}

/// An owner borrow plus identity and geometry kind authenticated at acquisition.
///
/// Geometry setters update the cell immediately after native mutation and before owner postflight.
pub(crate) struct ShapeProof<'owner> {
    owner: &'owner dyn OwnerAdapter,
    key: Cell<ShapeKey>,
}

pub(crate) struct ChainProof<'owner> {
    owner: &'owner dyn OwnerAdapter,
    key: ChainKey,
}

pub(crate) struct JointProof<'owner> {
    owner: &'owner dyn OwnerAdapter,
    key: JointKey,
}

impl<'owner> BodyProof<'owner> {
    pub(crate) fn acquire(owner: &'owner mut impl OwnerAdapter, id: crate::BodyId) -> Result<Self> {
        owner.capability_preflight()?;
        owner.capability_core().check_body_after_preflight(id)?;
        Ok(Self {
            owner,
            key: BodyKey(id),
        })
    }

    pub(crate) const fn id(&self) -> crate::BodyId {
        self.key.0
    }

    pub(crate) fn call<R>(&self, operation: impl FnOnce(BodyCall<'_>) -> Result<R>) -> Result<R> {
        run_owner_operation(self.owner, |core| {
            operation(BodyCall {
                core,
                key: self.key,
            })
        })
    }

    pub(crate) fn begin_creation(&self) -> Result<(OwnerCreation<'_>, BodyCall<'_>)> {
        let creation = OwnerCreation::begin(self.owner)?;
        let call = BodyCall {
            core: self.owner.capability_core(),
            key: self.key,
        };
        Ok((creation, call))
    }

    /// Run an owned shape-creation operation after the proof and owner gates accept it.
    pub(crate) fn run_creation<R>(
        &self,
        operation: impl FnOnce(OwnerCreation<'_>, BodyCall<'_>) -> Result<R>,
    ) -> Result<R> {
        let operation = crate::core::callback_state::PendingUserValue::new(operation);
        let (creation, body) = self.begin_creation()?;
        operation.into_inner()(creation, body)
    }
}

impl<'owner> ShapeProof<'owner> {
    pub(crate) fn acquire(
        owner: &'owner mut impl OwnerAdapter,
        id: crate::ShapeId,
    ) -> Result<Self> {
        owner.capability_preflight()?;
        let core = owner.capability_core();
        core.check_shape_after_preflight(id)?;
        let raw_kind = crate::shapes::shape_type_raw_impl(id);
        let kind = crate::ShapeType::decode_native(raw_kind).inspect_err(|_| core.poison())?;
        Ok(Self {
            owner,
            key: Cell::new(ShapeKey { id, kind }),
        })
    }

    pub(crate) fn id(&self) -> crate::ShapeId {
        self.key.get().id
    }

    pub(crate) fn set_kind(&self, kind: crate::ShapeType) {
        self.key.set(ShapeKey {
            id: self.id(),
            kind,
        });
    }

    pub(crate) fn call<R>(&self, operation: impl FnOnce(ShapeCall<'_>) -> Result<R>) -> Result<R> {
        run_owner_operation(self.owner, |core| {
            operation(ShapeCall {
                core,
                key: self.key.get(),
            })
        })
    }
}

impl<'owner> ChainProof<'owner> {
    pub(crate) fn acquire(
        owner: &'owner mut impl OwnerAdapter,
        id: crate::ChainId,
    ) -> Result<Self> {
        owner.capability_preflight()?;
        owner.capability_core().check_chain_after_preflight(id)?;
        Ok(Self {
            owner,
            key: ChainKey(id),
        })
    }

    pub(crate) const fn id(&self) -> crate::ChainId {
        self.key.0
    }

    pub(crate) fn call<R>(&self, operation: impl FnOnce(ChainCall<'_>) -> Result<R>) -> Result<R> {
        run_owner_operation(self.owner, |core| {
            operation(ChainCall {
                core,
                key: self.key,
            })
        })
    }
}

impl<'owner> JointProof<'owner> {
    pub(crate) fn acquire(
        owner: &'owner mut impl OwnerAdapter,
        id: crate::JointId,
    ) -> Result<Self> {
        owner.capability_preflight()?;
        let kind = owner.capability_core().check_joint_after_preflight(id)?;
        Ok(Self {
            owner,
            key: JointKey { id, kind },
        })
    }

    pub(crate) const fn id(&self) -> crate::JointId {
        self.key.id
    }

    pub(crate) const fn kind(&self) -> crate::JointType {
        self.key.kind
    }

    pub(crate) fn call<R>(&self, operation: impl FnOnce(JointCall<'_>) -> Result<R>) -> Result<R> {
        run_owner_operation(self.owner, |core| {
            operation(JointCall {
                core,
                key: self.key,
            })
        })
    }
}

/// Narrow authority for one previously validated body.
#[derive(Copy, Clone)]
pub(crate) struct BodyCall<'call> {
    core: &'call WorldCore,
    key: BodyKey,
}

impl BodyCall<'_> {
    pub(crate) const fn id(&self) -> crate::BodyId {
        self.key.0
    }

    pub(crate) fn contact_epoch(&self) -> crate::id::ContactEpoch {
        self.core.contact_epoch()
    }

    pub(crate) fn with_output_identity_resolver<T>(
        &self,
        resolve: impl FnOnce(&crate::core::identity_registry::OutputIdentityResolver<'_>) -> Result<T>,
    ) -> Result<T> {
        self.core.with_output_identity_resolver(resolve)
    }

    pub(crate) fn poison(&self) {
        self.core.poison();
    }

    pub(crate) fn set_user_data<T: 'static>(
        &self,
        value: crate::core::callback_state::PendingUserValue<T>,
    ) -> Result<crate::core::user_data::UserDataUpdate> {
        self.core.set_body_user_data(self.id(), value)
    }

    pub(crate) fn clear_user_data(&self) -> Result<crate::core::user_data::RetiredUserData> {
        self.core.clear_body_user_data(self.id())
    }

    pub(crate) fn with_user_data<T: 'static, R, F>(
        &self,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> Result<Option<R>>
    where
        F: FnOnce(&T) -> R,
    {
        self.core.borrow_body_user_data(self.id(), f)
    }

    pub(crate) fn with_user_data_mut<T: 'static, R, F>(
        &self,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> Result<Option<R>>
    where
        F: FnOnce(&mut T) -> R,
    {
        self.core.borrow_body_user_data_mut(self.id(), f)
    }

    pub(crate) fn take_user_data<T: 'static>(&self) -> Result<Option<T>> {
        self.core.take_body_user_data(self.id())
    }

    pub(crate) fn reserve_shape_creation(
        &self,
    ) -> Result<crate::core::identity_registry::PendingShape> {
        self.core.reserve_shape_creation(self.id())
    }

    pub(crate) fn claim_created_shape(
        &self,
        raw: boxdd_sys::ffi::b2ShapeId,
        update_body_mass: bool,
    ) -> Result<crate::core::world_core::NativeCreationGuard<'_>> {
        self.core.claim_created_shape(raw, update_body_mass)
    }

    pub(crate) fn bind_created_shape(
        &self,
        pending: crate::core::identity_registry::PendingShape,
        raw: boxdd_sys::ffi::b2ShapeId,
    ) -> Result<crate::core::identity_registry::BoundShape> {
        self.core.bind_created_shape(pending, raw)
    }

    pub(crate) fn reserve_chain_creation(
        &self,
        segment_count: usize,
    ) -> Result<crate::core::identity_registry::PendingChain> {
        self.core.reserve_chain_creation(self.id(), segment_count)
    }

    pub(crate) fn claim_created_chain(
        &self,
        raw: boxdd_sys::ffi::b2ChainId,
    ) -> Result<crate::core::world_core::NativeCreationGuard<'_>> {
        self.core.claim_created_chain(raw)
    }

    pub(crate) fn bind_created_chain(
        &self,
        pending: crate::core::identity_registry::PendingChain,
        raw: boxdd_sys::ffi::b2ChainId,
    ) -> Result<crate::core::identity_registry::BoundChain> {
        self.core.bind_created_chain(pending, raw)
    }

    pub(crate) fn destroy(self) -> Result<()> {
        self.core.destroy_acquired_body(self.id())
    }
}

/// Narrow authority for one previously validated shape.
#[derive(Copy, Clone)]
pub(crate) struct ShapeCall<'call> {
    core: &'call WorldCore,
    key: ShapeKey,
}

impl ShapeCall<'_> {
    pub(crate) const fn id(&self) -> crate::ShapeId {
        self.key.id
    }

    pub(crate) const fn kind(&self) -> crate::ShapeType {
        self.key.kind
    }

    pub(crate) fn require_kind(&self, expected: crate::ShapeType) -> Result<()> {
        let actual = self.kind();
        if actual == expected {
            Ok(())
        } else {
            Err(crate::Error::WrongShapeType { expected, actual })
        }
    }

    pub(crate) fn contact_epoch(&self) -> crate::id::ContactEpoch {
        self.core.contact_epoch()
    }

    pub(crate) fn with_output_identity_resolver<T>(
        &self,
        resolve: impl FnOnce(&crate::core::identity_registry::OutputIdentityResolver<'_>) -> Result<T>,
    ) -> Result<T> {
        self.core.with_output_identity_resolver(resolve)
    }

    pub(crate) fn set_user_data<T: 'static>(
        &self,
        value: crate::core::callback_state::PendingUserValue<T>,
    ) -> Result<crate::core::user_data::UserDataUpdate> {
        self.core.set_shape_user_data(self.id(), value)
    }

    pub(crate) fn clear_user_data(&self) -> Result<crate::core::user_data::RetiredUserData> {
        self.core.clear_shape_user_data(self.id())
    }

    pub(crate) fn with_user_data<T: 'static, R, F>(
        &self,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> Result<Option<R>>
    where
        F: FnOnce(&T) -> R,
    {
        self.core.borrow_shape_user_data(self.id(), f)
    }

    pub(crate) fn with_user_data_mut<T: 'static, R, F>(
        &self,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> Result<Option<R>>
    where
        F: FnOnce(&mut T) -> R,
    {
        self.core.borrow_shape_user_data_mut(self.id(), f)
    }

    pub(crate) fn take_user_data<T: 'static>(&self) -> Result<Option<T>> {
        self.core.take_shape_user_data(self.id())
    }

    pub(crate) fn destroy(self, update_body_mass: bool) -> Result<()> {
        self.core
            .destroy_acquired_shape(self.id(), update_body_mass)
    }
}

/// Narrow authority for one previously validated chain.
#[derive(Copy, Clone)]
pub(crate) struct ChainCall<'call> {
    core: &'call WorldCore,
    key: ChainKey,
}

impl ChainCall<'_> {
    pub(crate) const fn id(&self) -> crate::ChainId {
        self.key.0
    }

    pub(crate) fn with_output_identity_resolver<T>(
        &self,
        resolve: impl FnOnce(&crate::core::identity_registry::OutputIdentityResolver<'_>) -> Result<T>,
    ) -> Result<T> {
        self.core.with_output_identity_resolver(resolve)
    }

    pub(crate) fn destroy(self) -> Result<()> {
        self.core.destroy_acquired_chain(self.id())
    }
}

/// Narrow authority for one previously validated joint and its cached kind.
#[derive(Copy, Clone)]
pub(crate) struct JointCall<'call> {
    core: &'call WorldCore,
    key: JointKey,
}

impl JointCall<'_> {
    pub(crate) const fn id(&self) -> crate::JointId {
        self.key.id
    }

    pub(crate) const fn kind(&self) -> crate::JointType {
        self.key.kind
    }

    pub(crate) fn with_output_identity_resolver<T>(
        &self,
        resolve: impl FnOnce(&crate::core::identity_registry::OutputIdentityResolver<'_>) -> Result<T>,
    ) -> Result<T> {
        self.core.with_output_identity_resolver(resolve)
    }

    pub(crate) fn set_user_data<T: 'static>(
        &self,
        value: crate::core::callback_state::PendingUserValue<T>,
    ) -> Result<crate::core::user_data::UserDataUpdate> {
        self.core.set_joint_user_data(self.id(), value)
    }

    pub(crate) fn clear_user_data(&self) -> Result<crate::core::user_data::RetiredUserData> {
        self.core.clear_joint_user_data(self.id())
    }

    pub(crate) fn with_user_data<T: 'static, R, F>(
        &self,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> Result<Option<R>>
    where
        F: FnOnce(&T) -> R,
    {
        self.core.borrow_joint_user_data(self.id(), f)
    }

    pub(crate) fn with_user_data_mut<T: 'static, R, F>(
        &self,
        f: crate::core::callback_state::PendingUserValue<F>,
    ) -> Result<Option<R>>
    where
        F: FnOnce(&mut T) -> R,
    {
        self.core.borrow_joint_user_data_mut(self.id(), f)
    }

    pub(crate) fn take_user_data<T: 'static>(&self) -> Result<Option<T>> {
        self.core.take_joint_user_data(self.id())
    }

    pub(crate) fn destroy(self, wake_bodies: bool) -> Result<()> {
        self.core.destroy_acquired_joint(self.id(), wake_bodies)
    }
}

impl<'owner> QueryProof<'owner> {
    pub(crate) fn acquire(owner: &'owner impl OwnerAdapter) -> Result<Self> {
        owner.capability_preflight()?;
        Ok(Self { owner })
    }

    pub(crate) fn begin(&self) -> Result<QueryCallGuard<'_, 'owner>> {
        self.owner.capability_preflight()?;
        Ok(QueryCallGuard { proof: self })
    }
}

/// A query operation which crossed its owner gate but has not entered the native boundary yet.
#[must_use = "an authorized query operation must be consumed by invoke"]
pub(crate) struct QueryCallGuard<'proof, 'owner> {
    proof: &'proof QueryProof<'owner>,
}

impl QueryCallGuard<'_, '_> {
    pub(crate) fn invoke<R>(self, operation: impl FnOnce(QueryCall<'_>) -> Result<R>) -> Result<R> {
        let owner = self.proof.owner;
        let postflight = OwnerPostflightGuard::new(owner);
        crate::core::callback_state::run_query_boundary(
            crate::core::callback_state::CallbackOwnerToken::world(
                owner.capability_core().brand.token(),
            ),
            || {
                operation(QueryCall {
                    core: owner.capability_core(),
                })
            },
            |native, _panic| native.map(|result| postflight.finish(result)),
        )
    }
}

/// Narrow authority available only while one query is crossing its native boundary.
#[derive(Copy, Clone)]
pub(crate) struct QueryCall<'call> {
    core: &'call WorldCore,
}

impl QueryCall<'_> {
    pub(crate) fn raw_world(&self) -> boxdd_sys::ffi::b2WorldId {
        self.core.id
    }

    pub(crate) fn resolve_shape(&self, raw: boxdd_sys::ffi::b2ShapeId) -> Result<crate::ShapeId> {
        self.core.resolve_query_shape(raw)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn with_output_identity_resolver<T>(
        &self,
        resolve: impl FnOnce(&crate::core::identity_registry::OutputIdentityResolver<'_>) -> Result<T>,
    ) -> Result<T> {
        self.core.with_output_identity_resolver(resolve)
    }

    #[cfg(test)]
    pub(crate) fn for_test(core: &WorldCore) -> QueryCall<'_> {
        QueryCall { core }
    }
}

impl OwnerAdapter for super::World {
    fn capability_core(&self) -> &WorldCore {
        self.core()
    }

    fn capability_completed_step(&self) -> &crate::events::CompletedStepState {
        self.completed_step_state()
    }

    fn capability_preflight(&self) -> Result<()> {
        super::check_world_available(self)
    }

    fn capability_postflight(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    struct CountingOwner {
        world: crate::World,
        preflights: Rc<Cell<usize>>,
        postflights: Rc<Cell<usize>>,
    }

    impl OwnerAdapter for CountingOwner {
        fn capability_core(&self) -> &WorldCore {
            self.world.core()
        }

        fn capability_completed_step(&self) -> &crate::events::CompletedStepState {
            self.world.completed_step_state()
        }

        fn capability_preflight(&self) -> Result<()> {
            self.preflights.set(self.preflights.get() + 1);
            super::super::check_world_available(&self.world)
        }

        fn capability_postflight(&self) -> Result<()> {
            self.postflights.set(self.postflights.get() + 1);
            Ok(())
        }
    }

    #[test]
    fn validated_proof_acquisition_uses_one_access_gate_and_no_postflight() {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        let body_id = world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_def(),
            )
            .unwrap();
        let access_checks_before = world.core().access_check_count_for_test();
        let native_checks_before = world.core().native_object_check_count_for_test();
        let preflights = Rc::new(Cell::new(0));
        let postflights = Rc::new(Cell::new(0));
        let mut owner = CountingOwner {
            world,
            preflights: Rc::clone(&preflights),
            postflights: Rc::clone(&postflights),
        };

        let _proof = BodyProof::acquire(&mut owner, body_id).unwrap();
        assert_eq!(preflights.get(), 1);
        assert_eq!(postflights.get(), 0);
        assert_eq!(
            owner.world.core().access_check_count_for_test(),
            access_checks_before + 1
        );
        assert_eq!(
            owner.world.core().native_object_check_count_for_test(),
            native_checks_before + 1
        );
    }

    #[test]
    fn validated_object_operations_do_not_repeat_identity_or_native_validity_checks() {
        let foundation = crate::Foundation::initialize_default().unwrap();
        let mut world = foundation.create_world(foundation.world_def()).unwrap();
        let body_a = world.create_body(foundation.body_def()).unwrap();
        let body_b = world.create_body(foundation.body_def()).unwrap();
        let shape = world
            .body(body_a)
            .unwrap()
            .create_centered_circle(&crate::ShapeDef::default(), 0.5)
            .unwrap();
        let chain_def = crate::ChainDef::builder()
            .points([
                crate::Vec2::new(-2.0, 0.0),
                crate::Vec2::new(-1.0, 0.0),
                crate::Vec2::new(1.0, 0.0),
                crate::Vec2::new(2.0, 0.0),
            ])
            .build()
            .unwrap();
        let chain = world
            .body(body_a)
            .unwrap()
            .create_chain(&chain_def)
            .unwrap();
        let joint_base = world.joint_base(body_a, body_b).unwrap();
        let joint = world
            .create_distance_joint(&crate::DistanceJointDef::new(joint_base))
            .unwrap();

        macro_rules! assert_one_capability_authentication {
            ($operations:block) => {{
                let native_checks = world.core().native_object_check_count_for_test();
                let identity_locks = world.core().identity_lock_count_for_test();
                $operations
                assert_eq!(
                    world.core().native_object_check_count_for_test(),
                    native_checks + 1
                );
                assert_eq!(
                    world.core().identity_lock_count_for_test(),
                    identity_locks + 1
                );
            }};
        }

        assert_one_capability_authentication!({
            let mut body = world.body(body_a).unwrap();
            for _ in 0..8 {
                let _position = body.position().unwrap();
                let awake = body.is_awake().unwrap();
                body.set_awake(awake).unwrap();
            }
        });
        assert_one_capability_authentication!({
            let mut shape = world.shape(shape).unwrap();
            for _ in 0..8 {
                let _kind = shape.shape_type().unwrap();
                let friction = shape.friction().unwrap();
                shape.set_friction(friction).unwrap();
            }
        });
        assert_one_capability_authentication!({
            let chain = world.chain(chain).unwrap();
            for _ in 0..8 {
                let _segments = chain.segment_count().unwrap();
                let _materials = chain.surface_material_count().unwrap();
            }
        });
        assert_one_capability_authentication!({
            let mut joint = world.joint(joint).unwrap().into_distance().unwrap();
            for _ in 0..8 {
                let length = joint.length().unwrap();
                joint.set_length(length).unwrap();
            }
        });
    }

    #[test]
    fn world_call_uses_one_owner_preflight_and_postflight() {
        let preflights = Rc::new(Cell::new(0));
        let postflights = Rc::new(Cell::new(0));
        let owner = CountingOwner {
            world: crate::Foundation::initialize_default()
                .unwrap()
                .create_world(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a WorldDef")
                        .world_def(),
                )
                .unwrap(),
            preflights: Rc::clone(&preflights),
            postflights: Rc::clone(&postflights),
        };

        let raw = run_owner_call(&owner, |world| Ok(world.raw_world())).unwrap();

        assert_eq!(raw.index1, owner.world.raw().index1);
        assert_eq!(raw.generation, owner.world.raw().generation);
        assert_eq!(preflights.get(), 1);
        assert_eq!(postflights.get(), 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rejected_owned_shape_input_cleanup_during_outer_unwind_does_not_abort() {
        const CHILD: &str = "BOXDD_OUTER_UNWIND_REJECTED_OWNED_SHAPE_INPUT";
        const TEST_NAME: &str = "world::capability::tests::rejected_owned_shape_input_cleanup_during_outer_unwind_does_not_abort";
        const PRIMARY_PANIC: &str = "outer rejected owned shape-input unwind remains primary";

        struct RejectCreationOwner {
            world: crate::World,
            preflights: Cell<usize>,
        }

        impl OwnerAdapter for RejectCreationOwner {
            fn capability_core(&self) -> &WorldCore {
                self.world.core()
            }

            fn capability_completed_step(&self) -> &crate::events::CompletedStepState {
                self.world.completed_step_state()
            }

            fn capability_preflight(&self) -> Result<()> {
                let preflight = self.preflights.get();
                self.preflights.set(preflight + 1);
                if preflight == 0 {
                    super::super::check_world_available(&self.world)
                } else {
                    Err(crate::Error::WorldPoisoned)
                }
            }

            fn capability_postflight(&self) -> Result<()> {
                Ok(())
            }
        }

        struct PanickingPoint {
            converted: std::sync::Arc<std::sync::atomic::AtomicBool>,
            dropped: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        }

        impl From<PanickingPoint> for crate::Vec2 {
            fn from(point: PanickingPoint) -> Self {
                point
                    .converted
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                crate::Vec2::ZERO
            }
        }

        impl Drop for PanickingPoint {
            fn drop(&mut self) {
                if self
                    .dropped
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    == 0
                {
                    panic!("secondary rejected owned shape-input cleanup panic");
                }
            }
        }

        struct InvokeOnDrop<F: FnOnce()>(Option<F>);

        impl<F: FnOnce()> Drop for InvokeOnDrop<F> {
            fn drop(&mut self) {
                if let Some(invoke) = self.0.take() {
                    invoke();
                }
            }
        }

        if std::env::var_os(CHILD).is_some() {
            let converted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let dropped = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let rejected = Rc::new(Cell::new(false));
            let mut owner = RejectCreationOwner {
                world: crate::Foundation::initialize_default()
                    .unwrap()
                    .create_world(
                        crate::Foundation::get()
                            .expect("Foundation must be initialized before constructing a WorldDef")
                            .world_def(),
                    )
                    .unwrap(),
                preflights: Cell::new(0),
            };
            let body_id = owner
                .world
                .create_body(
                    crate::Foundation::get()
                        .expect("Foundation must be initialized before constructing a BodyDef")
                        .body_def(),
                )
                .unwrap();
            let proof = BodyProof::acquire(&mut owner, body_id).unwrap();
            let mut body = crate::Body::new(proof);
            let converted_from_drop = std::sync::Arc::clone(&converted);
            let dropped_from_drop = std::sync::Arc::clone(&dropped);
            let rejected_from_drop = Rc::clone(&rejected);

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _operation = InvokeOnDrop(Some(move || {
                    let point = PanickingPoint {
                        converted: converted_from_drop,
                        dropped: dropped_from_drop,
                    };
                    rejected_from_drop.set(matches!(
                        body.create_segment_between(
                            &crate::ShapeDef::default(),
                            point,
                            crate::Vec2::new(1.0, 0.0),
                        ),
                        Err(crate::Error::WorldPoisoned)
                    ));
                }));
                std::panic::panic_any(PRIMARY_PANIC);
            }));
            let payload = result.expect_err("the outer panic must keep unwinding");
            assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
            assert!(rejected.get());
            assert!(!converted.load(std::sync::atomic::Ordering::SeqCst));
            assert_eq!(dropped.load(std::sync::atomic::Ordering::SeqCst), 1);
            eprintln!("boxdd-outer-unwind-rejected-owned-shape-input: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("outer-unwind rejected owned shape-input child process must start");
        assert!(
            output.status.success(),
            "outer-unwind rejected owned shape-input child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-outer-unwind-rejected-owned-shape-input: completed"),
            "outer-unwind rejected owned shape-input child did not complete\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rejected_owner_operation_cleanup_during_outer_unwind_does_not_abort() {
        const CHILD: &str = "BOXDD_OUTER_UNWIND_REJECTED_OWNER_OPERATION";
        const TEST_NAME: &str = "world::capability::tests::rejected_owner_operation_cleanup_during_outer_unwind_does_not_abort";
        const PRIMARY_PANIC: &str = "outer rejected owner-operation unwind remains primary";

        struct PanicOnDrop(std::sync::Arc<std::sync::atomic::AtomicUsize>);

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                if self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    panic!("secondary rejected owner-operation cleanup panic");
                }
            }
        }

        struct InvokeOnDrop<F: FnOnce()>(Option<F>);

        impl<F: FnOnce()> Drop for InvokeOnDrop<F> {
            fn drop(&mut self) {
                if let Some(invoke) = self.0.take() {
                    invoke();
                }
            }
        }

        if std::env::var_os(CHILD).is_some() {
            let dropped = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let rejected = Rc::new(Cell::new(false));
            let rejected_from_drop = Rc::clone(&rejected);
            let dropped_from_drop = std::sync::Arc::clone(&dropped);
            let owner = CountingOwner {
                world: crate::Foundation::initialize_default()
                    .unwrap()
                    .create_world(
                        crate::Foundation::get()
                            .expect("Foundation must be initialized before constructing a WorldDef")
                            .world_def(),
                    )
                    .unwrap(),
                preflights: Rc::new(Cell::new(0)),
                postflights: Rc::new(Cell::new(0)),
            };
            owner.world.core().poison();

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _operation = InvokeOnDrop(Some(move || {
                    let marker = PanicOnDrop(dropped_from_drop);
                    rejected_from_drop.set(matches!(
                        run_owner_call(&owner, move |_| {
                            let _ = &marker;
                            Ok(())
                        }),
                        Err(crate::Error::WorldPoisoned)
                    ));
                }));
                std::panic::panic_any(PRIMARY_PANIC);
            }));
            let payload = result.expect_err("the outer panic must keep unwinding");
            assert_eq!(payload.downcast_ref::<&'static str>(), Some(&PRIMARY_PANIC));
            assert!(rejected.get());
            assert_eq!(dropped.load(std::sync::atomic::Ordering::SeqCst), 1);
            eprintln!("boxdd-outer-unwind-rejected-owner-operation: completed");
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("test executable path must be available"),
        )
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .expect("outer-unwind rejected owner-operation child process must start");
        assert!(
            output.status.success(),
            "outer-unwind rejected owner-operation child aborted\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("boxdd-outer-unwind-rejected-owner-operation: completed"),
            "outer-unwind rejected owner-operation child did not complete\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
