//! Rust-side object identity registration for one Box2D world.
//!
//! Native generations are not a sufficient safe identity boundary because snapshot restore may
//! deliberately reintroduce an older native tuple. Every active object therefore receives a
//! monotonic registration nonce which is never restored or rolled back.

use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use boxdd_sys::ffi;

use crate::error::{Error, Result};
#[cfg(test)]
use crate::id::WorldToken;
use crate::id::{ContactEpoch, IdBrand, RegistrationNonce};
use crate::joints::JointType;
use crate::types::{BodyId, ChainId, ContactId, JointId, ShapeId};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RawKey {
    index1: i32,
    generation: u16,
}

impl RawKey {
    const fn new(index1: i32, generation: u16) -> Self {
        Self { index1, generation }
    }

    const fn body(self, world0: u16) -> ffi::b2BodyId {
        ffi::b2BodyId {
            index1: self.index1,
            world0,
            generation: self.generation,
        }
    }

    const fn shape(self, world0: u16) -> ffi::b2ShapeId {
        ffi::b2ShapeId {
            index1: self.index1,
            world0,
            generation: self.generation,
        }
    }

    const fn joint(self, world0: u16) -> ffi::b2JointId {
        ffi::b2JointId {
            index1: self.index1,
            world0,
            generation: self.generation,
        }
    }

    const fn chain(self, world0: u16) -> ffi::b2ChainId {
        ffi::b2ChainId {
            index1: self.index1,
            world0,
            generation: self.generation,
        }
    }
}

impl From<ffi::b2BodyId> for RawKey {
    fn from(raw: ffi::b2BodyId) -> Self {
        Self::new(raw.index1, raw.generation)
    }
}

impl From<ffi::b2ShapeId> for RawKey {
    fn from(raw: ffi::b2ShapeId) -> Self {
        Self::new(raw.index1, raw.generation)
    }
}

impl From<ffi::b2JointId> for RawKey {
    fn from(raw: ffi::b2JointId) -> Self {
        Self::new(raw.index1, raw.generation)
    }
}

impl From<ffi::b2ChainId> for RawKey {
    fn from(raw: ffi::b2ChainId) -> Self {
        Self::new(raw.index1, raw.generation)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum RetiredNonce {
    Unique(RegistrationNonce),
    Ambiguous,
}

#[derive(Clone)]
struct BodyRegistration {
    nonce: RegistrationNonce,
    shapes: Vec<RawKey>,
    joints: Vec<RawKey>,
    chains: Vec<RawKey>,
}

impl BodyRegistration {
    fn new(nonce: RegistrationNonce) -> Self {
        Self {
            nonce,
            shapes: Vec::new(),
            joints: Vec::new(),
            chains: Vec::new(),
        }
    }
}

#[derive(Clone)]
struct ShapeRegistration {
    nonce: RegistrationNonce,
    body: RawKey,
    chain: Option<RawKey>,
}

#[derive(Clone)]
struct JointRegistration {
    nonce: RegistrationNonce,
    bodies: [RawKey; 2],
    kind: JointType,
}

#[derive(Clone)]
struct ChainRegistration {
    nonce: RegistrationNonce,
    body: RawKey,
    segments: Vec<RawKey>,
}

#[derive(Default)]
struct IdentityState {
    revision: u128,
    #[cfg(not(target_arch = "wasm32"))]
    step_shapes: Option<Arc<StepShapeResolver>>,
    bodies: HashMap<RawKey, BodyRegistration>,
    shapes: HashMap<RawKey, ShapeRegistration>,
    joints: HashMap<RawKey, JointRegistration>,
    chains: HashMap<RawKey, ChainRegistration>,
    retired_bodies: HashMap<RawKey, RetiredNonce>,
    retired_shapes: HashMap<RawKey, RetiredNonce>,
    retired_joints: HashMap<RawKey, RetiredNonce>,
    retired_chains: HashMap<RawKey, RetiredNonce>,
}

impl IdentityState {
    fn advance_revision(&mut self) {
        // World activity exclusion is the transaction invariant. This wide revision is an extra
        // misuse detector for crate-internal callers, not an identity capability.
        self.revision = self.revision.wrapping_add(1);
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.step_shapes = None;
        }
    }

    fn try_reserve_bodies(&mut self, additional: usize) -> Result<()> {
        let active_after = self
            .bodies
            .len()
            .checked_add(additional)
            .ok_or(Error::ObjectIdentityExhausted)?;
        try_reserve_map(&mut self.bodies, additional)?;
        try_reserve_map(&mut self.retired_bodies, active_after)
    }

    fn try_reserve_shapes(&mut self, additional: usize) -> Result<()> {
        let active_after = self
            .shapes
            .len()
            .checked_add(additional)
            .ok_or(Error::ObjectIdentityExhausted)?;
        try_reserve_map(&mut self.shapes, additional)?;
        try_reserve_map(&mut self.retired_shapes, active_after)
    }

    fn try_reserve_joints(&mut self, additional: usize) -> Result<()> {
        let active_after = self
            .joints
            .len()
            .checked_add(additional)
            .ok_or(Error::ObjectIdentityExhausted)?;
        try_reserve_map(&mut self.joints, additional)?;
        try_reserve_map(&mut self.retired_joints, active_after)
    }

    fn try_reserve_chains(&mut self, additional: usize) -> Result<()> {
        let active_after = self
            .chains
            .len()
            .checked_add(additional)
            .ok_or(Error::ObjectIdentityExhausted)?;
        try_reserve_map(&mut self.chains, additional)?;
        try_reserve_map(&mut self.retired_chains, active_after)
    }
}

/// Owner-side identity registry for one world.
///
/// Native worker callbacks never retain this registry. They receive an immutable
/// [`StepShapeResolver`] selected before the native step begins.
pub(crate) struct ActiveIdentityRegistry {
    brand: IdBrand,
    last_nonce: AtomicU64,
    state: Mutex<IdentityState>,
    #[cfg(test)]
    state_locks: AtomicUsize,
    #[cfg(test)]
    fail_next_creation_reservation: AtomicBool,
    #[cfg(test)]
    fail_next_step_shape_snapshot: AtomicBool,
}

/// Identity capacity and provenance reserved before a native body is created.
///
/// Dropping a reservation burns its nonce but does not publish any identity. This is deliberate:
/// a nonce is evidence for one attempted registration and is never reused.
pub(crate) struct PendingBody {
    registry: Arc<ActiveIdentityRegistry>,
    nonce: RegistrationNonce,
    base_revision: u128,
}

/// Identity capacity and provenance reserved before a native shape is created.
pub(crate) struct PendingShape {
    registry: Arc<ActiveIdentityRegistry>,
    nonce: RegistrationNonce,
    body: BodyId,
    body_key: RawKey,
    base_revision: u128,
}

/// Identity capacity and provenance reserved before a native joint is created.
pub(crate) struct PendingJoint {
    registry: Arc<ActiveIdentityRegistry>,
    nonce: RegistrationNonce,
    bodies: [BodyId; 2],
    body_keys: [RawKey; 2],
    kind: JointType,
    base_revision: u128,
}

/// Identity capacity and provenance reserved before a native chain and its segments are created.
pub(crate) struct PendingChain {
    registry: Arc<ActiveIdentityRegistry>,
    chain_nonce: RegistrationNonce,
    segment_nonces: Vec<RegistrationNonce>,
    raw_segments: Vec<ffi::b2ShapeId>,
    segment_keys: Vec<RawKey>,
    segment_ids: Vec<ShapeId>,
    seen_segment_keys: HashSet<RawKey>,
    body: BodyId,
    body_key: RawKey,
    base_revision: u128,
}

/// A native body output validated against a pre-FFI reservation.
#[must_use = "a bound body identity must be published or discarded with its native object"]
pub(crate) struct BoundBody {
    registry: Arc<ActiveIdentityRegistry>,
    raw: ffi::b2BodyId,
    nonce: RegistrationNonce,
    base_revision: u128,
}

/// A native shape output validated against a pre-FFI reservation.
#[must_use = "a bound shape identity must be published or discarded with its native object"]
pub(crate) struct BoundShape {
    registry: Arc<ActiveIdentityRegistry>,
    raw: ffi::b2ShapeId,
    nonce: RegistrationNonce,
    body: BodyId,
    body_key: RawKey,
    base_revision: u128,
}

/// A native joint output validated against a pre-FFI reservation.
#[must_use = "a bound joint identity must be published or discarded with its native object"]
pub(crate) struct BoundJoint {
    registry: Arc<ActiveIdentityRegistry>,
    raw: ffi::b2JointId,
    nonce: RegistrationNonce,
    bodies: [BodyId; 2],
    body_keys: [RawKey; 2],
    kind: JointType,
    base_revision: u128,
}

/// A native chain output and all segment outputs validated against one pre-FFI reservation.
#[must_use = "a bound chain identity must be published or discarded with its native objects"]
pub(crate) struct BoundChain {
    registry: Arc<ActiveIdentityRegistry>,
    raw: ffi::b2ChainId,
    chain_nonce: RegistrationNonce,
    segment_nonces: Vec<RegistrationNonce>,
    segment_keys: Vec<RawKey>,
    segment_ids: Vec<ShapeId>,
    body: BodyId,
    body_key: RawKey,
    base_revision: u128,
}

/// Immutable shape identities published to one native step.
///
/// The compact sorted table avoids retaining the owner registry lock on Box2D worker threads.
/// Safe world access excludes identity mutation for the complete native step, while this owned
/// copy also remains valid until Box2D has joined every task spawned by the step.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct StepShapeResolver {
    brand: IdBrand,
    entries: Vec<StepShapeIdentity>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone)]
struct StepShapeIdentity {
    key: RawKey,
    nonce: RegistrationNonce,
}

#[cfg(not(target_arch = "wasm32"))]
impl StepShapeResolver {
    #[inline]
    pub(crate) fn shape(&self, raw: ffi::b2ShapeId) -> Result<ShapeId> {
        self.brand.check_shape_raw(raw)?;
        let key = RawKey::from(raw);
        let index = self
            .entries
            .binary_search_by_key(&key, |entry| entry.key)
            .map_err(|_| Error::InvalidShapeId)?;
        Ok(self.brand.shape(raw, self.entries[index].nonce))
    }
}

/// One owner-local, batch resolver for transient native outputs.
///
/// Holding the identity lock for the complete output family avoids both the process-global
/// registry lookup and one lock/unlock pair per event. The resolver accepts retired entries
/// because Box2D end/break events may name objects destroyed during the completed step.
pub(crate) struct OutputIdentityResolver<'a> {
    brand: IdBrand,
    state: MutexGuard<'a, IdentityState>,
}

impl OutputIdentityResolver<'_> {
    pub(crate) fn contact(&self, raw: ffi::b2ContactId, epoch: ContactEpoch) -> Result<ContactId> {
        IdBrand::try_contact(self.brand, raw, epoch)
    }

    pub(crate) fn active_body(&self, raw: ffi::b2BodyId) -> Result<BodyId> {
        self.brand.check_body_raw(raw)?;
        let nonce = self
            .state
            .bodies
            .get(&RawKey::from(raw))
            .map(|entry| entry.nonce)
            .ok_or(Error::InvalidBodyId)?;
        Ok(self.brand.body(raw, nonce))
    }

    pub(crate) fn active_shape(&self, raw: ffi::b2ShapeId) -> Result<ShapeId> {
        self.brand.check_shape_raw(raw)?;
        let nonce = self
            .state
            .shapes
            .get(&RawKey::from(raw))
            .map(|entry| entry.nonce)
            .ok_or(Error::InvalidShapeId)?;
        Ok(self.brand.shape(raw, nonce))
    }

    pub(crate) fn active_chain(&self, raw: ffi::b2ChainId) -> Result<ChainId> {
        self.brand.check_chain_raw(raw)?;
        let nonce = self
            .state
            .chains
            .get(&RawKey::from(raw))
            .map(|entry| entry.nonce)
            .ok_or(Error::InvalidChainId)?;
        Ok(self.brand.chain(raw, nonce))
    }

    pub(crate) fn active_joint(&self, raw: ffi::b2JointId) -> Result<JointId> {
        self.brand.check_joint_raw(raw)?;
        let nonce = self
            .state
            .joints
            .get(&RawKey::from(raw))
            .map(|entry| entry.nonce)
            .ok_or(Error::InvalidJointId)?;
        Ok(self.brand.joint(raw, nonce))
    }

    pub(crate) fn body(&self, raw: ffi::b2BodyId) -> Result<BodyId> {
        self.brand.check_body_raw(raw)?;
        let key = RawKey::from(raw);
        let nonce = resolve_observed_nonce(
            self.state.bodies.get(&key).map(|entry| entry.nonce),
            self.state.retired_bodies.get(&key).copied(),
            Error::InvalidBodyId,
        )?;
        Ok(self.brand.body(raw, nonce))
    }

    pub(crate) fn shape(&self, raw: ffi::b2ShapeId) -> Result<ShapeId> {
        self.brand.check_shape_raw(raw)?;
        let key = RawKey::from(raw);
        let nonce = resolve_observed_nonce(
            self.state.shapes.get(&key).map(|entry| entry.nonce),
            self.state.retired_shapes.get(&key).copied(),
            Error::InvalidShapeId,
        )?;
        Ok(self.brand.shape(raw, nonce))
    }

    pub(crate) fn joint(&self, raw: ffi::b2JointId) -> Result<JointId> {
        self.brand.check_joint_raw(raw)?;
        let key = RawKey::from(raw);
        let nonce = resolve_observed_nonce(
            self.state.joints.get(&key).map(|entry| entry.nonce),
            self.state.retired_joints.get(&key).copied(),
            Error::InvalidJointId,
        )?;
        Ok(self.brand.joint(raw, nonce))
    }
}

impl ActiveIdentityRegistry {
    pub(crate) fn new(brand: IdBrand) -> Arc<Self> {
        Arc::new(Self {
            brand,
            last_nonce: AtomicU64::new(0),
            state: Mutex::new(IdentityState::default()),
            #[cfg(test)]
            state_locks: AtomicUsize::new(0),
            #[cfg(test)]
            fail_next_creation_reservation: AtomicBool::new(false),
            #[cfg(test)]
            fail_next_step_shape_snapshot: AtomicBool::new(false),
        })
    }

    /// Copy active shape identities into worker-safe storage before native mutation begins.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn step_shape_resolver(&self) -> Result<Arc<StepShapeResolver>> {
        #[cfg(test)]
        if self
            .fail_next_step_shape_snapshot
            .swap(false, Ordering::AcqRel)
        {
            return Err(Error::IdentityTrackingAllocationFailed);
        }

        let mut state = self.lock_state();
        if let Some(resolver) = &state.step_shapes {
            return Ok(Arc::clone(resolver));
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(state.shapes.len())
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        entries.extend(state.shapes.iter().map(|(&key, entry)| StepShapeIdentity {
            key,
            nonce: entry.nonce,
        }));
        entries.sort_unstable_by_key(|entry| entry.key);
        let resolver = Arc::new(StepShapeResolver {
            brand: self.brand,
            entries,
        });
        state.step_shapes = Some(Arc::clone(&resolver));
        Ok(resolver)
    }

    pub(crate) fn with_output_resolver<T>(
        &self,
        resolve: impl FnOnce(&OutputIdentityResolver<'_>) -> Result<T>,
    ) -> Result<T> {
        let resolver = OutputIdentityResolver {
            brand: self.brand,
            state: self.lock_state(),
        };
        resolve(&resolver)
    }

    fn reserve_nonce_range(&self, count: u64) -> Result<u64> {
        debug_assert!(count > 0);
        self.last_nonce
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |last| {
                last.checked_add(count)
            })
            .map_err(|_| Error::ObjectIdentityExhausted)
    }

    fn allocate_nonce(&self) -> Result<RegistrationNonce> {
        let previous = self.reserve_nonce_range(1)?;
        RegistrationNonce::new(previous + 1)
    }

    fn allocate_nonces(&self, count: usize) -> Result<Vec<RegistrationNonce>> {
        let count = u64::try_from(count).map_err(|_| Error::ObjectIdentityExhausted)?;
        if count == 0 {
            return Ok(Vec::new());
        }
        let previous = self.reserve_nonce_range(count)?;
        let mut nonces = Vec::new();
        nonces
            .try_reserve_exact(count as usize)
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        for offset in 1..=count {
            nonces.push(RegistrationNonce::new(previous + offset)?);
        }
        Ok(nonces)
    }

    pub(crate) fn reserve_body(self: &Arc<Self>) -> Result<PendingBody> {
        #[cfg(test)]
        self.check_creation_reservation_hook()?;
        let mut state = self.lock_state();
        state.try_reserve_bodies(1)?;
        let nonce = self.allocate_nonce()?;
        Ok(PendingBody {
            registry: Arc::clone(self),
            nonce,
            base_revision: state.revision,
        })
    }

    pub(crate) fn reserve_shape(self: &Arc<Self>, body: BodyId) -> Result<PendingShape> {
        self.check_body_brand(body)?;
        let body_key = RawKey::from(body.into_raw());
        let mut state = self.lock_state();
        require_body(&state, body_key, body)?;
        #[cfg(test)]
        self.check_creation_reservation_hook()?;
        state.try_reserve_shapes(1)?;
        state
            .bodies
            .get_mut(&body_key)
            .expect("body registration checked")
            .shapes
            .try_reserve(1)
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        let nonce = self.allocate_nonce()?;
        Ok(PendingShape {
            registry: Arc::clone(self),
            nonce,
            body,
            body_key,
            base_revision: state.revision,
        })
    }

    pub(crate) fn reserve_joint(
        self: &Arc<Self>,
        body_a: BodyId,
        body_b: BodyId,
        kind: JointType,
    ) -> Result<PendingJoint> {
        self.check_body_brand(body_a)?;
        self.check_body_brand(body_b)?;
        let body_keys = [
            RawKey::from(body_a.into_raw()),
            RawKey::from(body_b.into_raw()),
        ];
        let mut state = self.lock_state();
        require_body(&state, body_keys[0], body_a)?;
        require_body(&state, body_keys[1], body_b)?;
        #[cfg(test)]
        self.check_creation_reservation_hook()?;
        state.try_reserve_joints(1)?;
        state
            .bodies
            .get_mut(&body_keys[0])
            .expect("body registration checked")
            .joints
            .try_reserve(1)
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        if body_keys[1] != body_keys[0] {
            state
                .bodies
                .get_mut(&body_keys[1])
                .expect("body registration checked")
                .joints
                .try_reserve(1)
                .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        }
        let nonce = self.allocate_nonce()?;
        Ok(PendingJoint {
            registry: Arc::clone(self),
            nonce,
            bodies: [body_a, body_b],
            body_keys,
            kind,
            base_revision: state.revision,
        })
    }

    pub(crate) fn reserve_chain(
        self: &Arc<Self>,
        body: BodyId,
        segment_count: usize,
    ) -> Result<PendingChain> {
        self.check_body_brand(body)?;
        let body_key = RawKey::from(body.into_raw());
        let mut state = self.lock_state();
        require_body(&state, body_key, body)?;
        #[cfg(test)]
        self.check_creation_reservation_hook()?;
        state.try_reserve_chains(1)?;
        state.try_reserve_shapes(segment_count)?;
        let body_registration = state
            .bodies
            .get_mut(&body_key)
            .expect("body registration checked");
        body_registration
            .chains
            .try_reserve(1)
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        body_registration
            .shapes
            .try_reserve(segment_count)
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;

        let mut segment_keys = Vec::new();
        segment_keys
            .try_reserve_exact(segment_count)
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        let mut raw_segments = Vec::new();
        raw_segments
            .try_reserve_exact(segment_count)
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        let mut seen_segment_keys = HashSet::new();
        seen_segment_keys
            .try_reserve(segment_count)
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        let mut segment_ids = Vec::new();
        segment_ids
            .try_reserve_exact(segment_count)
            .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
        let mut nonces = self
            .allocate_nonces(
                segment_count
                    .checked_add(1)
                    .ok_or(Error::ObjectIdentityExhausted)?,
            )?
            .into_iter();
        let chain_nonce = nonces.next().expect("one chain nonce allocated");
        let segment_nonces = nonces.collect();
        Ok(PendingChain {
            registry: Arc::clone(self),
            chain_nonce,
            segment_nonces,
            raw_segments,
            segment_keys,
            segment_ids,
            seen_segment_keys,
            body,
            body_key,
            base_revision: state.revision,
        })
    }

    #[cfg(test)]
    pub(crate) fn register_body(self: &Arc<Self>, raw: ffi::b2BodyId) -> Result<BodyId> {
        self.reserve_body()?.bind(raw).map(BoundBody::publish)
    }

    #[cfg(test)]
    pub(crate) fn register_shape(
        self: &Arc<Self>,
        raw: ffi::b2ShapeId,
        body: BodyId,
    ) -> Result<ShapeId> {
        self.reserve_shape(body)?.bind(raw).map(BoundShape::publish)
    }

    #[cfg(test)]
    pub(crate) fn register_joint(
        self: &Arc<Self>,
        raw: ffi::b2JointId,
        body_a: BodyId,
        body_b: BodyId,
        kind: JointType,
    ) -> Result<JointId> {
        self.reserve_joint(body_a, body_b, kind)?
            .bind(raw)
            .map(BoundJoint::publish)
    }

    /// Register a chain and every native segment as one host-side transaction.
    #[cfg(test)]
    pub(crate) fn register_chain(
        self: &Arc<Self>,
        raw: ffi::b2ChainId,
        body: BodyId,
        raw_segments: &[ffi::b2ShapeId],
    ) -> Result<(ChainId, Vec<ShapeId>)> {
        self.reserve_chain(body, raw_segments.len())?
            .bind(raw, raw_segments)
            .map(BoundChain::publish_with_segments)
    }

    #[cfg(test)]
    pub(crate) fn resolve_body(&self, raw: ffi::b2BodyId) -> Result<BodyId> {
        self.brand.check_body_raw(raw)?;
        let state = self.lock_state();
        let nonce = state
            .bodies
            .get(&RawKey::from(raw))
            .map(|entry| entry.nonce)
            .ok_or(Error::InvalidBodyId)?;
        std::mem::drop(state);
        Ok(self.brand.body(raw, nonce))
    }

    #[cfg(test)]
    fn resolve_shape(&self, raw: ffi::b2ShapeId) -> Result<ShapeId> {
        self.brand.check_shape_raw(raw)?;
        let state = self.lock_state();
        let nonce = state
            .shapes
            .get(&RawKey::from(raw))
            .map(|entry| entry.nonce)
            .ok_or(Error::InvalidShapeId)?;
        std::mem::drop(state);
        Ok(self.brand.shape(raw, nonce))
    }

    #[cfg(test)]
    fn resolve_shape_output(&self, raw: ffi::b2ShapeId) -> Result<ShapeId> {
        self.brand.check_shape_raw(raw)?;
        let state = self.lock_state();
        let key = RawKey::from(raw);
        let nonce = resolve_observed_nonce(
            state.shapes.get(&key).map(|entry| entry.nonce),
            state.retired_shapes.get(&key).copied(),
            Error::InvalidShapeId,
        )?;
        std::mem::drop(state);
        Ok(self.brand.shape(raw, nonce))
    }

    #[cfg(test)]
    fn resolve_chain_output(&self, raw: ffi::b2ChainId) -> Result<ChainId> {
        self.brand.check_chain_raw(raw)?;
        let state = self.lock_state();
        let key = RawKey::from(raw);
        let nonce = resolve_observed_nonce(
            state.chains.get(&key).map(|entry| entry.nonce),
            state.retired_chains.get(&key).copied(),
            Error::InvalidChainId,
        )?;
        std::mem::drop(state);
        Ok(self.brand.chain(raw, nonce))
    }

    pub(crate) fn contains_body_raw(&self, raw: ffi::b2BodyId) -> bool {
        self.lock_state().bodies.contains_key(&RawKey::from(raw))
    }

    pub(crate) fn contains_shape_raw(&self, raw: ffi::b2ShapeId) -> bool {
        self.lock_state().shapes.contains_key(&RawKey::from(raw))
    }

    pub(crate) fn contains_joint_raw(&self, raw: ffi::b2JointId) -> bool {
        self.lock_state().joints.contains_key(&RawKey::from(raw))
    }

    pub(crate) fn contains_chain_raw(&self, raw: ffi::b2ChainId) -> bool {
        self.lock_state().chains.contains_key(&RawKey::from(raw))
    }

    pub(crate) fn contains_body(&self, id: BodyId) -> bool {
        self.check_body_brand(id).is_ok()
            && self
                .lock_state()
                .bodies
                .get(&RawKey::from(id.into_raw()))
                .is_some_and(|entry| entry.nonce == id.registration_nonce())
    }

    pub(crate) fn contains_shape(&self, id: ShapeId) -> bool {
        self.check_shape_brand(id).is_ok()
            && self
                .lock_state()
                .shapes
                .get(&RawKey::from(id.into_raw()))
                .is_some_and(|entry| entry.nonce == id.registration_nonce())
    }

    #[cfg(test)]
    pub(crate) fn contains_joint(&self, id: JointId) -> bool {
        self.check_joint_brand(id).is_ok()
            && self
                .lock_state()
                .joints
                .get(&RawKey::from(id.into_raw()))
                .is_some_and(|entry| entry.nonce == id.registration_nonce())
    }

    pub(crate) fn joint_type(&self, id: JointId) -> Result<JointType> {
        self.check_joint_brand(id)?;
        let state = self.lock_state();
        let registration = state
            .joints
            .get(&RawKey::from(id.into_raw()))
            .ok_or(Error::InvalidJointId)?;
        if registration.nonce != id.registration_nonce() {
            return Err(Error::InvalidJointId);
        }
        Ok(registration.kind)
    }

    pub(crate) fn contains_chain(&self, id: ChainId) -> bool {
        self.check_chain_brand(id).is_ok()
            && self
                .lock_state()
                .chains
                .get(&RawKey::from(id.into_raw()))
                .is_some_and(|entry| entry.nonce == id.registration_nonce())
    }

    pub(crate) fn unregister_body(&self, id: BodyId) -> bool {
        if self.check_body_brand(id).is_err() {
            return false;
        }
        let key = RawKey::from(id.into_raw());
        let mut state = self.lock_state();
        let Some(body) = state.bodies.get(&key) else {
            return false;
        };
        if body.nonce != id.registration_nonce() {
            return false;
        }
        let body = state
            .bodies
            .remove(&key)
            .expect("body registration checked");
        for chain in body.chains {
            remove_chain(&mut state, chain);
        }
        for joint in body.joints {
            remove_joint(&mut state, joint);
        }
        for shape in body.shapes {
            remove_shape(&mut state, shape);
        }
        retain_retired(&mut state.retired_bodies, key, body.nonce);
        state.advance_revision();
        true
    }

    pub(crate) fn unregister_shape(&self, id: ShapeId) -> bool {
        if self.check_shape_brand(id).is_err() {
            return false;
        }
        let key = RawKey::from(id.into_raw());
        let mut state = self.lock_state();
        if !state
            .shapes
            .get(&key)
            .is_some_and(|entry| entry.nonce == id.registration_nonce())
        {
            return false;
        }
        remove_shape(&mut state, key);
        state.advance_revision();
        true
    }

    pub(crate) fn unregister_joint(&self, id: JointId) -> bool {
        if self.check_joint_brand(id).is_err() {
            return false;
        }
        let key = RawKey::from(id.into_raw());
        let mut state = self.lock_state();
        if !state
            .joints
            .get(&key)
            .is_some_and(|entry| entry.nonce == id.registration_nonce())
        {
            return false;
        }
        remove_joint(&mut state, key);
        state.advance_revision();
        true
    }

    pub(crate) fn unregister_chain(&self, id: ChainId) -> bool {
        if self.check_chain_brand(id).is_err() {
            return false;
        }
        let key = RawKey::from(id.into_raw());
        let mut state = self.lock_state();
        if !state
            .chains
            .get(&key)
            .is_some_and(|entry| entry.nonce == id.registration_nonce())
        {
            return false;
        }
        remove_chain(&mut state, key);
        state.advance_revision();
        true
    }

    pub(crate) fn clear_retired_outputs(&self) {
        let mut state = self.lock_state();
        state.retired_bodies.clear();
        state.retired_shapes.clear();
        state.retired_joints.clear();
        state.retired_chains.clear();
    }

    pub(crate) fn clear(&self) {
        let mut state = self.lock_state();
        state.bodies.clear();
        state.shapes.clear();
        state.joints.clear();
        state.chains.clear();
        state.retired_bodies.clear();
        state.retired_shapes.clear();
        state.retired_joints.clear();
        state.retired_chains.clear();
        state.advance_revision();
    }

    pub(crate) fn snapshot_manifest(&self) -> Result<IdentityManifest> {
        let state = self.lock_state();
        let mut bodies = HashMap::new();
        let mut shapes = HashMap::new();
        let mut joints = HashMap::new();
        let mut chains = HashMap::new();
        try_reserve_map(&mut bodies, state.bodies.len())?;
        try_reserve_map(&mut shapes, state.shapes.len())?;
        try_reserve_map(&mut joints, state.joints.len())?;
        try_reserve_map(&mut chains, state.chains.len())?;
        for (&key, entry) in &state.bodies {
            bodies.insert(
                key,
                BodyRegistration {
                    nonce: entry.nonce,
                    shapes: try_clone_vec(&entry.shapes)?,
                    joints: try_clone_vec(&entry.joints)?,
                    chains: try_clone_vec(&entry.chains)?,
                },
            );
        }
        shapes.extend(
            state
                .shapes
                .iter()
                .map(|(&key, entry)| (key, entry.clone())),
        );
        joints.extend(
            state
                .joints
                .iter()
                .map(|(&key, entry)| (key, entry.clone())),
        );
        for (&key, entry) in &state.chains {
            chains.insert(
                key,
                ChainRegistration {
                    nonce: entry.nonce,
                    body: entry.body,
                    segments: try_clone_vec(&entry.segments)?,
                },
            );
        }
        Ok(IdentityManifest {
            brand: self.brand,
            bodies,
            shapes,
            joints,
            chains,
        })
    }

    /// Preallocate a complete replacement identity table without changing the active table.
    ///
    /// Nonces are preserved only for the intersection of snapshot membership and the exact current
    /// registration. Restored objects whose original registration has disappeared receive fresh
    /// nonces, so old safe ids cannot become valid again after native slot reuse.
    pub(crate) fn prepare_restore(
        &self,
        manifest: &IdentityManifest,
    ) -> Result<PreparedIdentityRestore> {
        if manifest.brand != self.brand {
            return Err(Error::WrongWorld);
        }
        let current = self.lock_state();
        let fresh_count = count_changed(&current.bodies, &manifest.bodies)
            .checked_add(count_changed(&current.shapes, &manifest.shapes))
            .and_then(|count| count.checked_add(count_changed(&current.joints, &manifest.joints)))
            .and_then(|count| count.checked_add(count_changed(&current.chains, &manifest.chains)))
            .ok_or(Error::ObjectIdentityExhausted)?;
        let mut fresh = self.allocate_nonces(fresh_count)?.into_iter();
        let mut target = IdentityState::default();
        target.try_reserve_bodies(manifest.bodies.len())?;
        target.try_reserve_shapes(manifest.shapes.len())?;
        target.try_reserve_joints(manifest.joints.len())?;
        target.try_reserve_chains(manifest.chains.len())?;

        for (&key, snapshot) in &manifest.bodies {
            let nonce = preserved_or_fresh(
                current.bodies.get(&key).map(|entry| entry.nonce),
                snapshot.nonce,
                &mut fresh,
            );
            target.bodies.insert(
                key,
                BodyRegistration {
                    nonce,
                    shapes: try_clone_vec(&snapshot.shapes)?,
                    joints: try_clone_vec(&snapshot.joints)?,
                    chains: try_clone_vec(&snapshot.chains)?,
                },
            );
        }
        for (&key, snapshot) in &manifest.shapes {
            let nonce = preserved_or_fresh(
                current.shapes.get(&key).map(|entry| entry.nonce),
                snapshot.nonce,
                &mut fresh,
            );
            target.shapes.insert(
                key,
                ShapeRegistration {
                    nonce,
                    body: snapshot.body,
                    chain: snapshot.chain,
                },
            );
        }
        for (&key, snapshot) in &manifest.joints {
            let nonce = preserved_or_fresh(
                current.joints.get(&key).map(|entry| entry.nonce),
                snapshot.nonce,
                &mut fresh,
            );
            target.joints.insert(
                key,
                JointRegistration {
                    nonce,
                    bodies: snapshot.bodies,
                    kind: snapshot.kind,
                },
            );
        }
        for (&key, snapshot) in &manifest.chains {
            let nonce = preserved_or_fresh(
                current.chains.get(&key).map(|entry| entry.nonce),
                snapshot.nonce,
                &mut fresh,
            );
            target.chains.insert(
                key,
                ChainRegistration {
                    nonce,
                    body: snapshot.body,
                    segments: try_clone_vec(&snapshot.segments)?,
                },
            );
        }
        debug_assert!(fresh.next().is_none());
        Ok(PreparedIdentityRestore {
            brand: self.brand,
            base_revision: current.revision,
            target,
        })
    }

    /// Atomically publish a previously prepared restore table.
    pub(crate) fn commit_restore(&self, mut prepared: PreparedIdentityRestore) -> Result<()> {
        if prepared.brand != self.brand {
            return Err(Error::WrongWorld);
        }
        let mut state = self.lock_state();
        if state.revision != prepared.base_revision {
            return Err(Error::WorldBusy);
        }
        prepared.target.revision = state.revision.wrapping_add(1);
        *state = prepared.target;
        Ok(())
    }

    fn check_body_brand(&self, id: BodyId) -> Result<()> {
        check_brand(self.brand, id.brand())
    }

    fn check_shape_brand(&self, id: ShapeId) -> Result<()> {
        check_brand(self.brand, id.brand())
    }

    fn check_joint_brand(&self, id: JointId) -> Result<()> {
        check_brand(self.brand, id.brand())
    }

    fn check_chain_brand(&self, id: ChainId) -> Result<()> {
        check_brand(self.brand, id.brand())
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, IdentityState> {
        #[cfg(test)]
        self.state_locks.fetch_add(1, Ordering::Relaxed);
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[cfg(test)]
    pub(crate) fn state_lock_count_for_test(&self) -> usize {
        self.state_locks.load(Ordering::Relaxed)
    }

    #[cfg(test)]
    fn set_last_nonce_for_test(&self, value: u64) {
        self.last_nonce.store(value, Ordering::Relaxed);
    }

    #[cfg(test)]
    fn check_creation_reservation_hook(&self) -> Result<()> {
        if self
            .fail_next_creation_reservation
            .swap(false, Ordering::AcqRel)
        {
            Err(Error::IdentityTrackingAllocationFailed)
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    pub(crate) fn fail_next_creation_reservation_for_test(&self) {
        self.fail_next_creation_reservation
            .store(true, Ordering::Release);
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn fail_next_step_shape_snapshot_for_test(&self) {
        self.fail_next_step_shape_snapshot
            .store(true, Ordering::Release);
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn hold_state_lock_for_test(&self) -> HeldIdentityLock<'_> {
        HeldIdentityLock {
            _state: self.lock_state(),
        }
    }
}

impl PendingBody {
    pub(crate) fn bind(self, raw: ffi::b2BodyId) -> Result<BoundBody> {
        self.registry.brand.check_body_raw(raw)?;
        let key = RawKey::from(raw);
        let state = self.registry.lock_state();
        ensure_revision(&state, self.base_revision)?;
        if state.bodies.contains_key(&key) || state.retired_bodies.contains_key(&key) {
            return Err(Error::ObjectIdentityExhausted);
        }
        drop(state);
        Ok(BoundBody {
            registry: self.registry,
            raw,
            nonce: self.nonce,
            base_revision: self.base_revision,
        })
    }
}

impl PendingShape {
    pub(crate) const fn body(&self) -> BodyId {
        self.body
    }

    pub(crate) fn bind(self, raw: ffi::b2ShapeId) -> Result<BoundShape> {
        self.registry.brand.check_shape_raw(raw)?;
        let key = RawKey::from(raw);
        let state = self.registry.lock_state();
        ensure_revision(&state, self.base_revision)?;
        require_body(&state, self.body_key, self.body)?;
        if state.shapes.contains_key(&key) || state.retired_shapes.contains_key(&key) {
            return Err(Error::ObjectIdentityExhausted);
        }
        drop(state);
        Ok(BoundShape {
            registry: self.registry,
            raw,
            nonce: self.nonce,
            body: self.body,
            body_key: self.body_key,
            base_revision: self.base_revision,
        })
    }
}

impl PendingJoint {
    pub(crate) const fn bodies(&self) -> [BodyId; 2] {
        self.bodies
    }

    pub(crate) fn bind(self, raw: ffi::b2JointId) -> Result<BoundJoint> {
        self.registry.brand.check_joint_raw(raw)?;
        let key = RawKey::from(raw);
        let state = self.registry.lock_state();
        ensure_revision(&state, self.base_revision)?;
        require_body(&state, self.body_keys[0], self.bodies[0])?;
        require_body(&state, self.body_keys[1], self.bodies[1])?;
        if state.joints.contains_key(&key) || state.retired_joints.contains_key(&key) {
            return Err(Error::ObjectIdentityExhausted);
        }
        drop(state);
        Ok(BoundJoint {
            registry: self.registry,
            raw,
            nonce: self.nonce,
            bodies: self.bodies,
            body_keys: self.body_keys,
            kind: self.kind,
            base_revision: self.base_revision,
        })
    }
}

impl PendingChain {
    pub(crate) const fn body(&self) -> BodyId {
        self.body
    }

    /// Read the segment outputs into storage reserved before native creation, then bind them.
    ///
    /// # Safety
    ///
    /// `fill` must initialize exactly the number of elements it reports, up to `capacity`.
    pub(crate) unsafe fn bind_native(
        mut self,
        raw: ffi::b2ChainId,
        reported_count: i32,
        fill: impl FnOnce(*mut ffi::b2ShapeId, i32) -> i32,
        mut validate: impl FnMut(ffi::b2ShapeId) -> Result<()>,
    ) -> Result<BoundChain> {
        let expected =
            i32::try_from(self.segment_nonces.len()).map_err(|_| Error::ObjectIdentityExhausted)?;
        if reported_count < 0 {
            return Err(Error::NegativeFfiOutputCapacity {
                capacity: reported_count,
            });
        }
        if reported_count != expected {
            return Err(Error::InvalidChainId);
        }

        let initialized = fill(self.raw_segments.as_mut_ptr(), expected);
        let initialized_usize = usize::try_from(initialized)
            .map_err(|_| Error::NegativeFfiOutputCount { count: initialized })?;
        if initialized > expected {
            return Err(Error::FfiOutputCountExceedsCapacity {
                count: initialized,
                capacity: expected,
            });
        }
        // SAFETY: guaranteed by the caller's `fill` contract and bounded above by capacity.
        unsafe { self.raw_segments.set_len(initialized_usize) };
        if initialized != expected {
            return Err(Error::InvalidChainId);
        }
        for &segment in &self.raw_segments {
            validate(segment)?;
        }
        let raw_segments = core::mem::take(&mut self.raw_segments);
        self.bind(raw, &raw_segments)
    }

    pub(crate) fn bind(
        mut self,
        raw: ffi::b2ChainId,
        raw_segments: &[ffi::b2ShapeId],
    ) -> Result<BoundChain> {
        self.registry.brand.check_chain_raw(raw)?;
        if raw_segments.len() != self.segment_nonces.len() {
            return Err(Error::InvalidChainId);
        }

        for (&segment, &nonce) in raw_segments.iter().zip(&self.segment_nonces) {
            self.registry.brand.check_shape_raw(segment)?;
            let key = RawKey::from(segment);
            if !self.seen_segment_keys.insert(key) {
                return Err(Error::ObjectIdentityExhausted);
            }
            self.segment_keys.push(key);
            self.segment_ids
                .push(self.registry.brand.shape(segment, nonce));
        }

        let chain_key = RawKey::from(raw);
        let state = self.registry.lock_state();
        ensure_revision(&state, self.base_revision)?;
        require_body(&state, self.body_key, self.body)?;
        if state.chains.contains_key(&chain_key)
            || state.retired_chains.contains_key(&chain_key)
            || self
                .segment_keys
                .iter()
                .any(|key| state.shapes.contains_key(key) || state.retired_shapes.contains_key(key))
        {
            return Err(Error::ObjectIdentityExhausted);
        }
        drop(state);
        Ok(BoundChain {
            registry: self.registry,
            raw,
            chain_nonce: self.chain_nonce,
            segment_nonces: self.segment_nonces,
            segment_keys: self.segment_keys,
            segment_ids: self.segment_ids,
            body: self.body,
            body_key: self.body_key,
            base_revision: self.base_revision,
        })
    }
}

impl BoundBody {
    pub(crate) fn publish(self) -> BodyId {
        let Self {
            registry,
            raw,
            nonce,
            base_revision,
        } = self;
        let key = RawKey::from(raw);
        let mut state = registry.lock_state();
        debug_assert_eq!(state.revision, base_revision);
        let previous = state.bodies.insert(key, BodyRegistration::new(nonce));
        debug_assert!(previous.is_none(), "reserved body identity remained unique");
        state.advance_revision();
        drop(state);
        registry.brand.body(raw, nonce)
    }
}

impl BoundShape {
    pub(crate) fn publish(self) -> ShapeId {
        let Self {
            registry,
            raw,
            nonce,
            body,
            body_key,
            base_revision,
        } = self;
        let key = RawKey::from(raw);
        let mut state = registry.lock_state();
        debug_assert_eq!(state.revision, base_revision);
        debug_assert!(require_body(&state, body_key, body).is_ok());
        let previous = state.shapes.insert(
            key,
            ShapeRegistration {
                nonce,
                body: body_key,
                chain: None,
            },
        );
        debug_assert!(
            previous.is_none(),
            "reserved shape identity remained unique"
        );
        state
            .bodies
            .get_mut(&body_key)
            .expect("shape parent body remained registered")
            .shapes
            .push(key);
        state.advance_revision();
        drop(state);
        registry.brand.shape(raw, nonce)
    }
}

impl BoundJoint {
    pub(crate) fn publish(self) -> JointId {
        let Self {
            registry,
            raw,
            nonce,
            bodies,
            body_keys,
            kind,
            base_revision,
        } = self;
        let key = RawKey::from(raw);
        let mut state = registry.lock_state();
        debug_assert_eq!(state.revision, base_revision);
        debug_assert!(require_body(&state, body_keys[0], bodies[0]).is_ok());
        debug_assert!(require_body(&state, body_keys[1], bodies[1]).is_ok());
        let previous = state.joints.insert(
            key,
            JointRegistration {
                nonce,
                bodies: body_keys,
                kind,
            },
        );
        debug_assert!(
            previous.is_none(),
            "reserved joint identity remained unique"
        );
        push_unique(
            &mut state
                .bodies
                .get_mut(&body_keys[0])
                .expect("joint body A remained registered")
                .joints,
            key,
        );
        push_unique(
            &mut state
                .bodies
                .get_mut(&body_keys[1])
                .expect("joint body B remained registered")
                .joints,
            key,
        );
        state.advance_revision();
        drop(state);
        registry.brand.joint(raw, nonce)
    }
}

impl BoundChain {
    pub(crate) fn publish(self) -> ChainId {
        self.publish_with_segments().0
    }

    pub(crate) fn publish_with_segments(self) -> (ChainId, Vec<ShapeId>) {
        let Self {
            registry,
            raw,
            chain_nonce,
            segment_nonces,
            segment_keys,
            segment_ids,
            body,
            body_key,
            base_revision,
        } = self;
        let chain_key = RawKey::from(raw);
        let mut state = registry.lock_state();
        debug_assert_eq!(state.revision, base_revision);
        debug_assert!(require_body(&state, body_key, body).is_ok());
        debug_assert_eq!(segment_keys.len(), segment_nonces.len());

        for (&key, &nonce) in segment_keys.iter().zip(&segment_nonces) {
            let previous = state.shapes.insert(
                key,
                ShapeRegistration {
                    nonce,
                    body: body_key,
                    chain: Some(chain_key),
                },
            );
            debug_assert!(previous.is_none(), "reserved chain segment remained unique");
        }
        let body_registration = state
            .bodies
            .get_mut(&body_key)
            .expect("chain parent body remained registered");
        body_registration.chains.push(chain_key);
        body_registration
            .shapes
            .extend(segment_keys.iter().copied());
        let previous = state.chains.insert(
            chain_key,
            ChainRegistration {
                nonce: chain_nonce,
                body: body_key,
                segments: segment_keys,
            },
        );
        debug_assert!(
            previous.is_none(),
            "reserved chain identity remained unique"
        );
        state.advance_revision();
        drop(state);
        (registry.brand.chain(raw, chain_nonce), segment_ids)
    }
}

fn ensure_revision(state: &IdentityState, expected: u128) -> Result<()> {
    if state.revision == expected {
        Ok(())
    } else {
        Err(Error::WorldBusy)
    }
}

fn resolve_observed_nonce(
    active: Option<RegistrationNonce>,
    retired: Option<RetiredNonce>,
    invalid: Error,
) -> Result<RegistrationNonce> {
    match (active, retired) {
        (Some(active), None) => Ok(active),
        (Some(active), Some(RetiredNonce::Unique(retired))) if active == retired => Ok(active),
        (None, Some(RetiredNonce::Unique(retired))) => Ok(retired),
        (Some(_), Some(RetiredNonce::Unique(_) | RetiredNonce::Ambiguous))
        | (None, Some(RetiredNonce::Ambiguous))
        | (None, None) => Err(invalid),
    }
}

fn retain_retired(
    retired: &mut HashMap<RawKey, RetiredNonce>,
    key: RawKey,
    nonce: RegistrationNonce,
) {
    use std::collections::hash_map::Entry;

    match retired.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(RetiredNonce::Unique(nonce));
        }
        Entry::Occupied(mut entry) => {
            if !matches!(entry.get(), RetiredNonce::Unique(current) if *current == nonce) {
                entry.insert(RetiredNonce::Ambiguous);
            }
        }
    }
}

fn require_body(state: &IdentityState, key: RawKey, id: BodyId) -> Result<()> {
    let registration = state.bodies.get(&key).ok_or(Error::InvalidBodyId)?;
    if registration.nonce == id.registration_nonce() {
        Ok(())
    } else {
        Err(Error::InvalidBodyId)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) struct HeldIdentityLock<'a> {
    _state: MutexGuard<'a, IdentityState>,
}

/// In-process snapshot capability for the Rust identity table.
#[derive(Clone)]
pub(crate) struct IdentityManifest {
    brand: IdBrand,
    bodies: HashMap<RawKey, BodyRegistration>,
    shapes: HashMap<RawKey, ShapeRegistration>,
    joints: HashMap<RawKey, JointRegistration>,
    chains: HashMap<RawKey, ChainRegistration>,
}

impl IdentityManifest {
    pub(crate) fn body_ids(&self) -> impl ExactSizeIterator<Item = BodyId> + '_ {
        self.bodies
            .iter()
            .map(|(&key, entry)| self.brand.body(key.body(self.brand.world0()), entry.nonce))
    }

    pub(crate) fn shape_ids(&self) -> impl ExactSizeIterator<Item = ShapeId> + '_ {
        self.shapes.iter().map(|(&key, entry)| {
            self.brand
                .shape(key.shape(self.brand.world0()), entry.nonce)
        })
    }

    pub(crate) fn joint_ids(&self) -> impl ExactSizeIterator<Item = JointId> + '_ {
        self.joints.iter().map(|(&key, entry)| {
            self.brand
                .joint(key.joint(self.brand.world0()), entry.nonce)
        })
    }

    pub(crate) fn chain_ids(&self) -> impl ExactSizeIterator<Item = ChainId> + '_ {
        self.chains.iter().map(|(&key, entry)| {
            self.brand
                .chain(key.chain(self.brand.world0()), entry.nonce)
        })
    }

    /// Prove that this host manifest describes exactly the live object identity graph in a
    /// deeply validated native snapshot.
    pub(crate) fn validate_snapshot_entries(
        &self,
        entries: &[boxdd_sys::adapter::SnapshotEntry],
    ) -> Result<()> {
        NativeIdentityEntries::new(entries)?.validate_manifest(self)
    }
}

struct NativeIdentityEntries<'a> {
    by_slot: HashMap<(u32, i32), &'a boxdd_sys::adapter::SnapshotEntry>,
    live_bodies: usize,
    live_shapes: usize,
    live_joints: usize,
    live_chains: usize,
}

impl<'a> NativeIdentityEntries<'a> {
    fn new(entries: &'a [boxdd_sys::adapter::SnapshotEntry]) -> Result<Self> {
        use boxdd_sys::adapter::{
            SNAPSHOT_ENTRY_BODY, SNAPSHOT_ENTRY_CHAIN, SNAPSHOT_ENTRY_JOINT, SNAPSHOT_ENTRY_LIVE,
            SNAPSHOT_ENTRY_SHAPE, SNAPSHOT_ENTRY_VERSION,
        };

        let mut by_slot = HashMap::new();
        by_slot
            .try_reserve(entries.len())
            .map_err(|_| Error::SnapshotAllocationFailed)?;
        let mut live_bodies = 0usize;
        let mut live_shapes = 0usize;
        let mut live_joints = 0usize;
        let mut live_chains = 0usize;

        for entry in entries {
            if entry.struct_size as usize
                != core::mem::size_of::<boxdd_sys::adapter::SnapshotEntry>()
                || entry.version != SNAPSHOT_ENTRY_VERSION
                || entry.index < 0
            {
                return Err(Error::SnapshotManifestMismatch);
            }
            if by_slot.insert((entry.kind, entry.index), entry).is_some() {
                return Err(Error::SnapshotManifestMismatch);
            }
            if entry.flags & SNAPSHOT_ENTRY_LIVE == 0 {
                continue;
            }
            match entry.kind {
                SNAPSHOT_ENTRY_BODY => live_bodies += 1,
                SNAPSHOT_ENTRY_SHAPE => live_shapes += 1,
                SNAPSHOT_ENTRY_JOINT => live_joints += 1,
                SNAPSHOT_ENTRY_CHAIN => live_chains += 1,
                _ => {}
            }
        }

        Ok(Self {
            by_slot,
            live_bodies,
            live_shapes,
            live_joints,
            live_chains,
        })
    }

    fn validate_manifest(&self, manifest: &IdentityManifest) -> Result<()> {
        use boxdd_sys::adapter::{
            SNAPSHOT_ENTRY_BODY, SNAPSHOT_ENTRY_CHAIN, SNAPSHOT_ENTRY_JOINT, SNAPSHOT_ENTRY_SHAPE,
        };

        if self.live_bodies != manifest.bodies.len()
            || self.live_shapes != manifest.shapes.len()
            || self.live_joints != manifest.joints.len()
            || self.live_chains != manifest.chains.len()
        {
            return Err(Error::SnapshotManifestMismatch);
        }

        let mut body_relations = HashMap::<RawKey, [usize; 3]>::new();
        body_relations
            .try_reserve(manifest.bodies.len())
            .map_err(|_| Error::SnapshotAllocationFailed)?;
        body_relations.extend(manifest.bodies.keys().copied().map(|key| (key, [0; 3])));

        let mut chain_segments = HashMap::<RawKey, usize>::new();
        chain_segments
            .try_reserve(manifest.chains.len())
            .map_err(|_| Error::SnapshotAllocationFailed)?;
        chain_segments.extend(manifest.chains.keys().copied().map(|key| (key, 0)));

        for &key in manifest.bodies.keys() {
            self.require_key(SNAPSHOT_ENTRY_BODY, key)?;
        }

        for (&key, shape) in &manifest.shapes {
            let native = self.require_key(SNAPSHOT_ENTRY_SHAPE, key)?;
            if self.key_for_slot(SNAPSHOT_ENTRY_BODY, native.owner_a)? != shape.body {
                return Err(Error::SnapshotManifestMismatch);
            }
            let native_chain = if native.owner_b < 0 {
                None
            } else {
                Some(self.key_for_slot(SNAPSHOT_ENTRY_CHAIN, native.owner_b)?)
            };
            if native_chain != shape.chain {
                return Err(Error::SnapshotManifestMismatch);
            }
            body_relations
                .get_mut(&shape.body)
                .ok_or(Error::SnapshotManifestMismatch)?[0] += 1;
            if let Some(chain) = shape.chain {
                *chain_segments
                    .get_mut(&chain)
                    .ok_or(Error::SnapshotManifestMismatch)? += 1;
            }
        }

        for (&key, joint) in &manifest.joints {
            let native = self.require_key(SNAPSHOT_ENTRY_JOINT, key)?;
            let native_bodies = [
                self.key_for_slot(SNAPSHOT_ENTRY_BODY, native.owner_a)?,
                self.key_for_slot(SNAPSHOT_ENTRY_BODY, native.owner_b)?,
            ];
            if native_bodies != joint.bodies
                || JointType::from_raw(native.subtype) != Some(joint.kind)
            {
                return Err(Error::SnapshotManifestMismatch);
            }
            body_relations
                .get_mut(&joint.bodies[0])
                .ok_or(Error::SnapshotManifestMismatch)?[1] += 1;
            if joint.bodies[1] != joint.bodies[0] {
                body_relations
                    .get_mut(&joint.bodies[1])
                    .ok_or(Error::SnapshotManifestMismatch)?[1] += 1;
            }
        }

        for (&key, chain) in &manifest.chains {
            let native = self.require_key(SNAPSHOT_ENTRY_CHAIN, key)?;
            if self.key_for_slot(SNAPSHOT_ENTRY_BODY, native.owner_a)? != chain.body {
                return Err(Error::SnapshotManifestMismatch);
            }
            body_relations
                .get_mut(&chain.body)
                .ok_or(Error::SnapshotManifestMismatch)?[2] += 1;
        }

        let largest_relation = manifest
            .bodies
            .values()
            .map(|body| {
                body.shapes
                    .len()
                    .max(body.joints.len())
                    .max(body.chains.len())
            })
            .chain(manifest.chains.values().map(|chain| chain.segments.len()))
            .max()
            .unwrap_or(0);
        let mut seen = HashSet::new();
        seen.try_reserve(largest_relation)
            .map_err(|_| Error::SnapshotAllocationFailed)?;

        for (&body_key, body) in &manifest.bodies {
            let counts = body_relations
                .get(&body_key)
                .ok_or(Error::SnapshotManifestMismatch)?;
            if [body.shapes.len(), body.joints.len(), body.chains.len()] != *counts
                || !relations_are_exact(&mut seen, &body.shapes, |shape| {
                    manifest
                        .shapes
                        .get(shape)
                        .is_some_and(|entry| entry.body == body_key)
                })
                || !relations_are_exact(&mut seen, &body.joints, |joint| {
                    manifest
                        .joints
                        .get(joint)
                        .is_some_and(|entry| entry.bodies.contains(&body_key))
                })
                || !relations_are_exact(&mut seen, &body.chains, |chain| {
                    manifest
                        .chains
                        .get(chain)
                        .is_some_and(|entry| entry.body == body_key)
                })
            {
                return Err(Error::SnapshotManifestMismatch);
            }
        }

        for (&chain_key, chain) in &manifest.chains {
            if chain.segments.len()
                != *chain_segments
                    .get(&chain_key)
                    .ok_or(Error::SnapshotManifestMismatch)?
                || !relations_are_exact(&mut seen, &chain.segments, |shape| {
                    manifest
                        .shapes
                        .get(shape)
                        .is_some_and(|entry| entry.chain == Some(chain_key))
                })
            {
                return Err(Error::SnapshotManifestMismatch);
            }
            for (expected_order, &segment) in chain.segments.iter().enumerate() {
                let native = self.require_key(SNAPSHOT_ENTRY_SHAPE, segment)?;
                if native.owner_b_order
                    != i32::try_from(expected_order).map_err(|_| Error::SnapshotManifestMismatch)?
                {
                    return Err(Error::SnapshotManifestMismatch);
                }
            }
        }

        Ok(())
    }

    fn require_key(&self, kind: u32, key: RawKey) -> Result<&'a boxdd_sys::adapter::SnapshotEntry> {
        let index = key
            .index1
            .checked_sub(1)
            .ok_or(Error::SnapshotManifestMismatch)?;
        let entry = self
            .by_slot
            .get(&(kind, index))
            .copied()
            .ok_or(Error::SnapshotManifestMismatch)?;
        if entry.flags & boxdd_sys::adapter::SNAPSHOT_ENTRY_LIVE == 0
            || snapshot_raw_key(entry)? != key
        {
            return Err(Error::SnapshotManifestMismatch);
        }
        Ok(entry)
    }

    fn key_for_slot(&self, kind: u32, index: i32) -> Result<RawKey> {
        let entry = self
            .by_slot
            .get(&(kind, index))
            .copied()
            .ok_or(Error::SnapshotManifestMismatch)?;
        if entry.flags & boxdd_sys::adapter::SNAPSHOT_ENTRY_LIVE == 0 {
            return Err(Error::SnapshotManifestMismatch);
        }
        snapshot_raw_key(entry)
    }
}

fn snapshot_raw_key(entry: &boxdd_sys::adapter::SnapshotEntry) -> Result<RawKey> {
    let index1 = entry
        .index
        .checked_add(1)
        .ok_or(Error::SnapshotManifestMismatch)?;
    let generation =
        u16::try_from(entry.generation).map_err(|_| Error::SnapshotManifestMismatch)?;
    if index1 <= 0 {
        return Err(Error::SnapshotManifestMismatch);
    }
    Ok(RawKey::new(index1, generation))
}

fn relations_are_exact(
    seen: &mut HashSet<RawKey>,
    relations: &[RawKey],
    mut valid: impl FnMut(&RawKey) -> bool,
) -> bool {
    seen.clear();
    relations
        .iter()
        .all(|relation| seen.insert(*relation) && valid(relation))
}

/// A fully allocated identity replacement which is inert until committed.
pub(crate) struct PreparedIdentityRestore {
    brand: IdBrand,
    base_revision: u128,
    target: IdentityState,
}

impl PreparedIdentityRestore {
    pub(crate) fn body_after_restore(
        &self,
        manifest: &IdentityManifest,
        snapshot_id: BodyId,
    ) -> Option<BodyId> {
        check_brand(self.brand, snapshot_id.brand()).ok()?;
        let key = RawKey::from(snapshot_id.into_raw());
        if manifest.brand != self.brand
            || manifest.bodies.get(&key)?.nonce != snapshot_id.registration_nonce()
        {
            return None;
        }
        let nonce = self.target.bodies.get(&key)?.nonce;
        Some(self.brand.body(snapshot_id.into_raw(), nonce))
    }

    pub(crate) fn shape_after_restore(
        &self,
        manifest: &IdentityManifest,
        snapshot_id: ShapeId,
    ) -> Option<ShapeId> {
        check_brand(self.brand, snapshot_id.brand()).ok()?;
        let key = RawKey::from(snapshot_id.into_raw());
        if manifest.brand != self.brand
            || manifest.shapes.get(&key)?.nonce != snapshot_id.registration_nonce()
        {
            return None;
        }
        let nonce = self.target.shapes.get(&key)?.nonce;
        Some(self.brand.shape(snapshot_id.into_raw(), nonce))
    }

    pub(crate) fn joint_after_restore(
        &self,
        manifest: &IdentityManifest,
        snapshot_id: JointId,
    ) -> Option<JointId> {
        check_brand(self.brand, snapshot_id.brand()).ok()?;
        let key = RawKey::from(snapshot_id.into_raw());
        if manifest.brand != self.brand
            || manifest.joints.get(&key)?.nonce != snapshot_id.registration_nonce()
        {
            return None;
        }
        let nonce = self.target.joints.get(&key)?.nonce;
        Some(self.brand.joint(snapshot_id.into_raw(), nonce))
    }

    pub(crate) fn chain_after_restore(
        &self,
        manifest: &IdentityManifest,
        snapshot_id: ChainId,
    ) -> Option<ChainId> {
        check_brand(self.brand, snapshot_id.brand()).ok()?;
        let key = RawKey::from(snapshot_id.into_raw());
        if manifest.brand != self.brand
            || manifest.chains.get(&key)?.nonce != snapshot_id.registration_nonce()
        {
            return None;
        }
        let nonce = self.target.chains.get(&key)?.nonce;
        Some(self.brand.chain(snapshot_id.into_raw(), nonce))
    }
}

fn check_brand(expected: IdBrand, actual: IdBrand) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(Error::WrongWorld)
    }
}

fn push_unique(values: &mut Vec<RawKey>, key: RawKey) {
    if !values.contains(&key) {
        values.push(key);
    }
}

fn remove_shape(state: &mut IdentityState, key: RawKey) {
    let Some(shape) = state.shapes.remove(&key) else {
        return;
    };
    if let Some(body) = state.bodies.get_mut(&shape.body) {
        body.shapes.retain(|candidate| *candidate != key);
    }
    if let Some(chain_key) = shape.chain
        && let Some(chain) = state.chains.get_mut(&chain_key)
    {
        chain.segments.retain(|candidate| *candidate != key);
    }
    retain_retired(&mut state.retired_shapes, key, shape.nonce);
}

fn remove_joint(state: &mut IdentityState, key: RawKey) {
    let Some(joint) = state.joints.remove(&key) else {
        return;
    };
    for body_key in joint.bodies {
        if let Some(body) = state.bodies.get_mut(&body_key) {
            body.joints.retain(|candidate| *candidate != key);
        }
    }
    retain_retired(&mut state.retired_joints, key, joint.nonce);
}

fn remove_chain(state: &mut IdentityState, key: RawKey) {
    let Some(chain) = state.chains.remove(&key) else {
        return;
    };
    {
        let shapes = &state.shapes;
        if let Some(body) = state.bodies.get_mut(&chain.body) {
            body.chains.retain(|candidate| *candidate != key);
            body.shapes.retain(|candidate| {
                shapes
                    .get(candidate)
                    .is_none_or(|shape| shape.chain != Some(key))
            });
        }
    }
    for segment in chain.segments {
        let Some(shape) = state.shapes.remove(&segment) else {
            continue;
        };
        debug_assert_eq!(shape.body, chain.body);
        debug_assert_eq!(shape.chain, Some(key));
        retain_retired(&mut state.retired_shapes, segment, shape.nonce);
    }
    retain_retired(&mut state.retired_chains, key, chain.nonce);
}

fn try_clone_vec<T: Copy>(source: &[T]) -> Result<Vec<T>> {
    let mut cloned = Vec::new();
    cloned
        .try_reserve_exact(source.len())
        .map_err(|_| Error::IdentityTrackingAllocationFailed)?;
    cloned.extend_from_slice(source);
    Ok(cloned)
}

fn try_reserve_map<K, V>(map: &mut HashMap<K, V>, additional: usize) -> Result<()>
where
    K: Eq + std::hash::Hash,
{
    map.try_reserve(additional)
        .map_err(|_| Error::IdentityTrackingAllocationFailed)
}

trait HasNonce {
    fn nonce(&self) -> RegistrationNonce;
}

impl HasNonce for BodyRegistration {
    fn nonce(&self) -> RegistrationNonce {
        self.nonce
    }
}

impl HasNonce for ShapeRegistration {
    fn nonce(&self) -> RegistrationNonce {
        self.nonce
    }
}

impl HasNonce for JointRegistration {
    fn nonce(&self) -> RegistrationNonce {
        self.nonce
    }
}

impl HasNonce for ChainRegistration {
    fn nonce(&self) -> RegistrationNonce {
        self.nonce
    }
}

fn count_changed<T: HasNonce>(
    current: &HashMap<RawKey, T>,
    snapshot: &HashMap<RawKey, T>,
) -> usize {
    snapshot
        .iter()
        .filter(|(key, entry)| {
            current
                .get(key)
                .is_none_or(|current| current.nonce() != entry.nonce())
        })
        .count()
}

fn preserved_or_fresh(
    current: Option<RegistrationNonce>,
    snapshot: RegistrationNonce,
    fresh: &mut impl Iterator<Item = RegistrationNonce>,
) -> RegistrationNonce {
    if current == Some(snapshot) {
        snapshot
    } else {
        fresh.next().expect("fresh nonce count precomputed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_impl_all;

    assert_impl_all!(ActiveIdentityRegistry: Send, Sync);
    #[cfg(not(target_arch = "wasm32"))]
    assert_impl_all!(StepShapeResolver: Send, Sync);

    fn fixture() -> (IdBrand, Arc<ActiveIdentityRegistry>) {
        let brand = IdBrand::new(
            ffi::b2WorldId {
                index1: 1,
                generation: 7,
            },
            WorldToken::allocate().unwrap(),
        )
        .unwrap();
        (brand, ActiveIdentityRegistry::new(brand))
    }

    fn raw_body(brand: IdBrand, index1: i32, generation: u16) -> ffi::b2BodyId {
        ffi::b2BodyId {
            index1,
            world0: brand.world0(),
            generation,
        }
    }

    fn raw_shape(brand: IdBrand, index1: i32, generation: u16) -> ffi::b2ShapeId {
        ffi::b2ShapeId {
            index1,
            world0: brand.world0(),
            generation,
        }
    }

    fn raw_chain(brand: IdBrand, index1: i32, generation: u16) -> ffi::b2ChainId {
        ffi::b2ChainId {
            index1,
            world0: brand.world0(),
            generation,
        }
    }

    #[test]
    fn raw_key_keeps_generations_distinct_in_hash_maps() {
        let first = RawKey::new(7, 1);
        let second = RawKey::new(7, 2);
        let mut keys = std::collections::HashMap::new();

        keys.insert(first, "first");
        keys.insert(second, "second");

        assert_eq!(keys.len(), 2);
        assert_eq!(keys.get(&first), Some(&"first"));
        assert_eq!(keys.get(&second), Some(&"second"));
    }

    #[test]
    fn scalar_and_batch_nonce_allocations_share_one_monotonic_sequence() {
        let (_, registry) = fixture();

        assert_eq!(registry.allocate_nonce(), RegistrationNonce::new(1));
        assert_eq!(
            registry.allocate_nonces(3),
            Ok(vec![
                RegistrationNonce::new(2).unwrap(),
                RegistrationNonce::new(3).unwrap(),
                RegistrationNonce::new(4).unwrap(),
            ])
        );
        assert_eq!(registry.allocate_nonce(), RegistrationNonce::new(5));
    }

    #[test]
    fn dropped_creation_reservation_burns_its_nonce() {
        let (_, registry) = fixture();
        let first = registry.reserve_body().unwrap();
        let first_nonce = first.nonce;
        drop(first);

        let second = registry.reserve_body().unwrap();

        assert!(second.nonce > first_nonce);
    }

    #[test]
    fn slot_reuse_gets_a_new_registration_nonce_after_retired_outputs_are_released() {
        let (brand, registry) = fixture();
        let raw = raw_body(brand, 1, 0);
        let first = registry.register_body(raw).unwrap();
        assert!(registry.unregister_body(first));
        registry.clear_retired_outputs();
        let second = registry.register_body(raw).unwrap();

        assert_ne!(first, second);
        assert!(!registry.contains_body(first));
        assert!(registry.contains_body(second));
        assert_eq!(registry.resolve_body(raw), Ok(second));
    }

    #[test]
    fn retained_retired_keys_block_registration_until_outputs_are_released() {
        let (brand, registry) = fixture();
        let body = registry.register_body(raw_body(brand, 1, 0)).unwrap();
        let raw = raw_shape(brand, 1, 7);
        let retired = registry.register_shape(raw, body).unwrap();
        assert!(registry.unregister_shape(retired));

        assert_eq!(
            registry.register_shape(raw, body),
            Err(Error::ObjectIdentityExhausted)
        );
        assert_eq!(registry.resolve_shape_output(raw), Ok(retired));

        let adjacent_raw = raw_shape(brand, 1, 8);
        let adjacent = registry.register_shape(adjacent_raw, body).unwrap();
        assert!(registry.contains_shape(adjacent));

        registry.clear_retired_outputs();
        let replacement = registry.register_shape(raw, body).unwrap();
        assert_ne!(replacement, retired);
        assert!(registry.contains_shape(replacement));
    }

    #[test]
    fn every_object_kind_rejects_an_exact_retired_raw_key() {
        let (brand, registry) = fixture();
        let body_raw = raw_body(brand, 1, 0);
        let retired_body = registry.register_body(body_raw).unwrap();
        assert!(registry.unregister_body(retired_body));
        assert_eq!(
            registry.register_body(body_raw),
            Err(Error::ObjectIdentityExhausted)
        );

        let owner = registry.register_body(raw_body(brand, 2, 0)).unwrap();
        let joint_raw = ffi::b2JointId {
            index1: 1,
            world0: brand.world0(),
            generation: 3,
        };
        let retired_joint = registry
            .register_joint(joint_raw, owner, owner, JointType::Distance)
            .unwrap();
        assert!(registry.unregister_joint(retired_joint));
        assert_eq!(
            registry.register_joint(joint_raw, owner, owner, JointType::Distance),
            Err(Error::ObjectIdentityExhausted)
        );

        let chain_raw = raw_chain(brand, 1, 5);
        let segment_raw = raw_shape(brand, 1, 8);
        let (retired_chain, _) = registry
            .register_chain(chain_raw, owner, &[segment_raw])
            .unwrap();
        assert!(registry.unregister_chain(retired_chain));
        assert_eq!(
            registry.register_chain(chain_raw, owner, &[raw_shape(brand, 2, 0)]),
            Err(Error::ObjectIdentityExhausted)
        );
    }

    #[test]
    fn owner_lookup_uses_only_the_active_registration() {
        let (brand, registry) = fixture();
        let body = registry.register_body(raw_body(brand, 1, 0)).unwrap();
        let raw = raw_shape(brand, 1, 0);
        let shape = registry.register_shape(raw, body).unwrap();

        assert_eq!(registry.resolve_shape(raw), Ok(shape));
        assert!(registry.unregister_shape(shape));
        assert_eq!(registry.resolve_shape(raw), Err(Error::InvalidShapeId));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn step_shape_resolver_is_an_independent_concurrent_snapshot() {
        let (brand, registry) = fixture();
        let body = registry.register_body(raw_body(brand, 1, 0)).unwrap();
        let raw_shapes = [
            raw_shape(brand, 90, 4),
            raw_shape(brand, 2, 9),
            raw_shape(brand, 2, 1),
            raw_shape(brand, 500, 0),
        ];
        let shapes = raw_shapes.map(|raw| registry.register_shape(raw, body).unwrap());
        let resolver = registry.step_shape_resolver().unwrap();

        assert!(registry.unregister_shape(shapes[0]));
        assert_eq!(
            registry.resolve_shape(raw_shapes[0]),
            Err(Error::InvalidShapeId)
        );
        assert_eq!(resolver.shape(raw_shapes[0]), Ok(shapes[0]));
        let refreshed = registry.step_shape_resolver().unwrap();
        assert!(!Arc::ptr_eq(&resolver, &refreshed));
        assert_eq!(refreshed.shape(raw_shapes[0]), Err(Error::InvalidShapeId));

        let owner_lock = registry.hold_state_lock_for_test();
        std::thread::scope(|scope| {
            for _ in 0..16 {
                scope.spawn(|| {
                    for (&raw, &shape) in raw_shapes.iter().zip(&shapes) {
                        assert_eq!(resolver.shape(raw), Ok(shape));
                    }
                });
            }
        });
        drop(owner_lock);

        assert_eq!(
            resolver.shape(raw_shape(brand, 2, 8)),
            Err(Error::InvalidShapeId)
        );
    }

    #[test]
    fn nonce_exhaustion_does_not_mutate_the_active_table() {
        let (brand, registry) = fixture();
        registry.set_last_nonce_for_test(u64::MAX);
        let raw = raw_body(brand, 1, 0);

        assert_eq!(
            registry.register_body(raw),
            Err(Error::ObjectIdentityExhausted)
        );
        assert_eq!(registry.resolve_body(raw), Err(Error::InvalidBodyId));
    }

    #[test]
    fn chain_registration_is_atomic_on_duplicate_segments() {
        let (brand, registry) = fixture();
        let body = registry.register_body(raw_body(brand, 1, 0)).unwrap();
        let raw_chain = raw_chain(brand, 1, 0);
        let segment = raw_shape(brand, 1, 0);

        assert_eq!(
            registry.register_chain(raw_chain, body, &[segment, segment]),
            Err(Error::ObjectIdentityExhausted)
        );
        assert_eq!(
            registry.resolve_chain_output(raw_chain),
            Err(Error::InvalidChainId)
        );
        assert_eq!(registry.resolve_shape(segment), Err(Error::InvalidShapeId));
        assert!(registry.contains_body(body));
    }

    #[test]
    fn chain_registration_is_atomic_on_retired_chain_or_segment_conflicts() {
        let (brand, registry) = fixture();
        let body = registry.register_body(raw_body(brand, 1, 0)).unwrap();
        let retired_chain_raw = raw_chain(brand, 1, 4);
        let retired_segment_raw = raw_shape(brand, 1, 9);
        let (retired_chain, _) = registry
            .register_chain(retired_chain_raw, body, &[retired_segment_raw])
            .unwrap();
        assert!(registry.unregister_chain(retired_chain));

        let fresh_chain_raw = raw_chain(brand, 2, 0);
        let fresh_segment_raw = raw_shape(brand, 2, 0);
        assert_eq!(
            registry.register_chain(fresh_chain_raw, body, &[retired_segment_raw]),
            Err(Error::ObjectIdentityExhausted)
        );
        assert_eq!(
            registry.resolve_chain_output(fresh_chain_raw),
            Err(Error::InvalidChainId)
        );
        assert_eq!(
            registry.resolve_shape(fresh_segment_raw),
            Err(Error::InvalidShapeId)
        );

        assert_eq!(
            registry.register_chain(retired_chain_raw, body, &[fresh_segment_raw]),
            Err(Error::ObjectIdentityExhausted)
        );
        assert_eq!(
            registry.resolve_shape(fresh_segment_raw),
            Err(Error::InvalidShapeId)
        );
        assert!(registry.contains_body(body));
    }

    #[test]
    fn chain_registration_preserves_order_and_batch_removal_keeps_unrelated_shapes() {
        let (brand, registry) = fixture();
        let body = registry.register_body(raw_body(brand, 1, 0)).unwrap();
        let unrelated_raw = raw_shape(brand, 99, 0);
        let unrelated = registry.register_shape(unrelated_raw, body).unwrap();
        let raw_chain = raw_chain(brand, 1, 0);
        let raw_segments = [
            raw_shape(brand, 3, 1),
            raw_shape(brand, 1, 0),
            raw_shape(brand, 3, 2),
            raw_shape(brand, 2, 0),
        ];
        let expected_segment_keys = raw_segments
            .iter()
            .copied()
            .map(RawKey::from)
            .collect::<Vec<_>>();

        let (chain, segments) = registry
            .register_chain(raw_chain, body, &raw_segments)
            .unwrap();
        assert_eq!(
            segments
                .iter()
                .copied()
                .map(ShapeId::into_raw)
                .map(RawKey::from)
                .collect::<Vec<_>>(),
            expected_segment_keys
        );
        {
            let state = registry.lock_state();
            assert_eq!(
                state.chains.get(&RawKey::from(raw_chain)).unwrap().segments,
                expected_segment_keys
            );
            assert_eq!(
                state
                    .bodies
                    .get(&RawKey::from(body.into_raw()))
                    .unwrap()
                    .shapes,
                std::iter::once(RawKey::from(unrelated_raw))
                    .chain(expected_segment_keys.iter().copied())
                    .collect::<Vec<_>>()
            );
        }

        assert!(registry.unregister_chain(chain));
        assert!(registry.contains_body(body));
        assert!(registry.contains_shape(unrelated));
        for (&raw_segment, &segment) in raw_segments.iter().zip(&segments) {
            assert!(!registry.contains_shape(segment));
            assert_eq!(
                registry.resolve_shape(raw_segment),
                Err(Error::InvalidShapeId)
            );
            assert_eq!(registry.resolve_shape_output(raw_segment), Ok(segment));
        }
        {
            let state = registry.lock_state();
            assert!(!state.chains.contains_key(&RawKey::from(raw_chain)));
            assert_eq!(
                state
                    .bodies
                    .get(&RawKey::from(body.into_raw()))
                    .unwrap()
                    .shapes,
                [RawKey::from(unrelated_raw)]
            );
        }

        registry.clear_retired_outputs();
        let (replacement_chain, replacement_segments) = registry
            .register_chain(raw_chain, body, &raw_segments)
            .unwrap();
        assert_ne!(replacement_chain, chain);
        for (&stale, &replacement) in segments.iter().zip(&replacement_segments) {
            assert_ne!(replacement, stale);
            assert!(!registry.contains_shape(stale));
            assert!(registry.contains_shape(replacement));
        }
    }

    #[test]
    fn dropped_restore_plan_leaves_active_state_but_consumes_nonces() {
        let (brand, registry) = fixture();
        let raw = raw_body(brand, 1, 0);
        let snapshot_id = registry.register_body(raw).unwrap();
        let manifest = registry.snapshot_manifest().unwrap();
        assert!(registry.unregister_body(snapshot_id));
        registry.clear_retired_outputs();
        let replacement = registry.register_body(raw).unwrap();

        let prepared = registry.prepare_restore(&manifest).unwrap();
        let restored = prepared.body_after_restore(&manifest, snapshot_id).unwrap();
        assert_ne!(restored, snapshot_id);
        assert_ne!(restored, replacement);
        drop(prepared);

        assert!(registry.contains_body(replacement));
        assert!(!registry.contains_body(snapshot_id));
        assert!(!registry.contains_body(restored));
        let later_raw = raw_body(brand, 2, 0);
        let later = registry.register_body(later_raw).unwrap();
        assert!(later.registration_nonce() > restored.registration_nonce());
    }

    #[test]
    fn restore_preserves_only_the_exact_registration_intersection() {
        let (brand, registry) = fixture();
        let raw_a = raw_body(brand, 1, 0);
        let raw_b = raw_body(brand, 2, 0);
        let a = registry.register_body(raw_a).unwrap();
        let b = registry.register_body(raw_b).unwrap();
        let manifest = registry.snapshot_manifest().unwrap();

        assert!(registry.unregister_body(b));
        registry.clear_retired_outputs();
        let replacement_b = registry.register_body(raw_b).unwrap();
        let post_snapshot = registry.register_body(raw_body(brand, 3, 0)).unwrap();
        let prepared = registry.prepare_restore(&manifest).unwrap();
        let restored_a = prepared.body_after_restore(&manifest, a).unwrap();
        let restored_b = prepared.body_after_restore(&manifest, b).unwrap();
        assert_eq!(restored_a, a);
        assert_ne!(restored_b, b);
        assert_ne!(restored_b, replacement_b);

        registry.commit_restore(prepared).unwrap();
        assert!(registry.contains_body(a));
        assert!(registry.contains_body(restored_b));
        assert!(!registry.contains_body(b));
        assert!(!registry.contains_body(replacement_b));
        assert!(!registry.contains_body(post_snapshot));
    }

    #[test]
    fn registration_property_holds_across_many_reuses() {
        let (brand, registry) = fixture();
        let raw = raw_body(brand, 1, 0);
        let mut previous = None;
        for _ in 0..1_024 {
            let current = registry.register_body(raw).unwrap();
            if let Some(previous) = previous {
                assert_ne!(current, previous);
                assert!(!registry.contains_body(previous));
            }
            assert!(registry.contains_body(current));
            assert!(registry.unregister_body(current));
            registry.clear_retired_outputs();
            previous = Some(current);
        }
    }
}
