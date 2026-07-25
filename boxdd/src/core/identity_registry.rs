//! Rust-side object identity registration for one Box2D world.
//!
//! Native generations are not a sufficient safe identity boundary because snapshot restore may
//! deliberately reintroduce an older native tuple. Every active object therefore receives a
//! monotonic registration nonce which is never restored or rolled back.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use boxdd_sys::ffi;

use crate::error::{ApiError, ApiResult};
use crate::id::{IdBrand, RegistrationNonce, WorldToken};
use crate::joints::JointType;
use crate::types::{BodyId, ChainId, JointId, ShapeId};

static REGISTRIES: OnceLock<Mutex<HashMap<WorldToken, Weak<ActiveIdentityRegistry>>>> =
    OnceLock::new();

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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
    bodies: HashMap<RawKey, BodyRegistration>,
    shapes: HashMap<RawKey, ShapeRegistration>,
    joints: HashMap<RawKey, JointRegistration>,
    chains: HashMap<RawKey, ChainRegistration>,
    retired_bodies: HashMap<RawKey, RegistrationNonce>,
    retired_shapes: HashMap<RawKey, RegistrationNonce>,
    retired_joints: HashMap<RawKey, RegistrationNonce>,
    retired_chains: HashMap<RawKey, RegistrationNonce>,
}

impl IdentityState {
    fn advance_revision(&mut self) {
        // World activity exclusion is the transaction invariant. This wide revision is an extra
        // misuse detector for crate-internal callers, not an identity capability.
        self.revision = self.revision.wrapping_add(1);
    }

    fn try_reserve_bodies(&mut self, additional: usize) -> ApiResult<()> {
        let active_after = self
            .bodies
            .len()
            .checked_add(additional)
            .ok_or(ApiError::ObjectIdentityExhausted)?;
        self.bodies
            .try_reserve(additional)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        self.retired_bodies
            .try_reserve(active_after)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)
    }

    fn try_reserve_shapes(&mut self, additional: usize) -> ApiResult<()> {
        let active_after = self
            .shapes
            .len()
            .checked_add(additional)
            .ok_or(ApiError::ObjectIdentityExhausted)?;
        self.shapes
            .try_reserve(additional)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        self.retired_shapes
            .try_reserve(active_after)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)
    }

    fn try_reserve_joints(&mut self, additional: usize) -> ApiResult<()> {
        let active_after = self
            .joints
            .len()
            .checked_add(additional)
            .ok_or(ApiError::ObjectIdentityExhausted)?;
        self.joints
            .try_reserve(additional)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        self.retired_joints
            .try_reserve(active_after)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)
    }

    fn try_reserve_chains(&mut self, additional: usize) -> ApiResult<()> {
        let active_after = self
            .chains
            .len()
            .checked_add(additional)
            .ok_or(ApiError::ObjectIdentityExhausted)?;
        self.chains
            .try_reserve(additional)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        self.retired_chains
            .try_reserve(active_after)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)
    }
}

/// A worker-safe registry which does not own the owner-thread `WorldCore`.
pub(crate) struct ActiveIdentityRegistry {
    brand: IdBrand,
    last_nonce: AtomicU64,
    state: Mutex<IdentityState>,
}

impl ActiveIdentityRegistry {
    pub(crate) fn new(brand: IdBrand) -> Arc<Self> {
        let registry = Arc::new(Self {
            brand,
            last_nonce: AtomicU64::new(0),
            state: Mutex::new(IdentityState::default()),
        });
        install(&registry);
        registry
    }

    pub(crate) fn from_snapshot_entries(
        brand: IdBrand,
        entries: &[boxdd_sys::adapter::SnapshotEntry],
    ) -> ApiResult<Arc<Self>> {
        use boxdd_sys::adapter::{
            SNAPSHOT_ENTRY_BODY, SNAPSHOT_ENTRY_CHAIN, SNAPSHOT_ENTRY_JOINT, SNAPSHOT_ENTRY_LIVE,
            SNAPSHOT_ENTRY_SHAPE,
        };

        let mut body_slots = allocate_entry_slots(entries, SNAPSHOT_ENTRY_BODY)?;
        let mut shape_slots = allocate_entry_slots(entries, SNAPSHOT_ENTRY_SHAPE)?;
        let mut joint_slots = allocate_entry_slots(entries, SNAPSHOT_ENTRY_JOINT)?;
        let mut chain_slots = allocate_entry_slots(entries, SNAPSHOT_ENTRY_CHAIN)?;
        let live_count = entries
            .iter()
            .filter(|entry| {
                entry.flags & SNAPSHOT_ENTRY_LIVE != 0
                    && matches!(
                        entry.kind,
                        SNAPSHOT_ENTRY_BODY
                            | SNAPSHOT_ENTRY_SHAPE
                            | SNAPSHOT_ENTRY_JOINT
                            | SNAPSHOT_ENTRY_CHAIN
                    )
            })
            .count();
        let last_nonce =
            u64::try_from(live_count).map_err(|_| ApiError::ObjectIdentityExhausted)?;

        let mut state = IdentityState::default();
        state.try_reserve_bodies(count_live(entries, SNAPSHOT_ENTRY_BODY))?;
        state.try_reserve_shapes(count_live(entries, SNAPSHOT_ENTRY_SHAPE))?;
        state.try_reserve_joints(count_live(entries, SNAPSHOT_ENTRY_JOINT))?;
        state.try_reserve_chains(count_live(entries, SNAPSHOT_ENTRY_CHAIN))?;
        let mut chain_segment_orders = HashMap::<RawKey, usize>::new();
        chain_segment_orders
            .try_reserve(count_live(entries, SNAPSHOT_ENTRY_SHAPE))
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        let mut chain_segment_counts = HashMap::<RawKey, usize>::new();
        chain_segment_counts
            .try_reserve(count_live(entries, SNAPSHOT_ENTRY_CHAIN))
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;

        let mut next_nonce = 0_u64;
        for entry in entries.iter().filter(|entry| {
            entry.kind == SNAPSHOT_ENTRY_BODY && entry.flags & SNAPSHOT_ENTRY_LIVE != 0
        }) {
            let key = snapshot_raw_key(entry)?;
            let nonce = next_import_nonce(&mut next_nonce)?;
            insert_slot(&mut body_slots, entry.index, key)?;
            if state
                .bodies
                .insert(key, BodyRegistration::new(nonce))
                .is_some()
            {
                return Err(ApiError::SnapshotManifestMismatch);
            }
        }
        for entry in entries.iter().filter(|entry| {
            entry.kind == SNAPSHOT_ENTRY_CHAIN && entry.flags & SNAPSHOT_ENTRY_LIVE != 0
        }) {
            let key = snapshot_raw_key(entry)?;
            let body = lookup_slot(&body_slots, entry.owner_a)?;
            let nonce = next_import_nonce(&mut next_nonce)?;
            let segment_count = usize::try_from(entry.color_index)
                .map_err(|_| ApiError::SnapshotManifestMismatch)?;
            insert_slot(&mut chain_slots, entry.index, key)?;
            let mut segments = Vec::new();
            segments
                .try_reserve_exact(segment_count)
                .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
            if state
                .chains
                .insert(
                    key,
                    ChainRegistration {
                        nonce,
                        body,
                        segments,
                    },
                )
                .is_some()
                || chain_segment_counts.insert(key, segment_count).is_some()
            {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            push_fallible(&mut body_registration_mut(&mut state, body)?.chains, key)?;
        }
        for entry in entries.iter().filter(|entry| {
            entry.kind == SNAPSHOT_ENTRY_SHAPE && entry.flags & SNAPSHOT_ENTRY_LIVE != 0
        }) {
            let key = snapshot_raw_key(entry)?;
            let body = lookup_slot(&body_slots, entry.owner_a)?;
            let chain = if entry.owner_b < 0 {
                None
            } else {
                Some(lookup_slot(&chain_slots, entry.owner_b)?)
            };
            let nonce = next_import_nonce(&mut next_nonce)?;
            insert_slot(&mut shape_slots, entry.index, key)?;
            if state
                .shapes
                .insert(key, ShapeRegistration { nonce, body, chain })
                .is_some()
            {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            push_fallible(&mut body_registration_mut(&mut state, body)?.shapes, key)?;
            if let Some(chain) = chain {
                let order = usize::try_from(entry.owner_b_order)
                    .map_err(|_| ApiError::SnapshotManifestMismatch)?;
                if chain_segment_orders.insert(key, order).is_some() {
                    return Err(ApiError::SnapshotManifestMismatch);
                }
                push_fallible(
                    &mut chain_registration_mut(&mut state, chain)?.segments,
                    key,
                )?;
            }
        }
        for (&chain_key, chain) in &mut state.chains {
            let expected = *chain_segment_counts
                .get(&chain_key)
                .ok_or(ApiError::SnapshotManifestMismatch)?;
            if chain.segments.len() != expected {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            chain.segments.sort_unstable_by_key(|segment| {
                chain_segment_orders
                    .get(segment)
                    .copied()
                    .unwrap_or(usize::MAX)
            });
            if chain
                .segments
                .iter()
                .enumerate()
                .any(|(expected_order, segment)| {
                    chain_segment_orders.get(segment).copied() != Some(expected_order)
                })
            {
                return Err(ApiError::SnapshotManifestMismatch);
            }
        }
        for entry in entries.iter().filter(|entry| {
            entry.kind == SNAPSHOT_ENTRY_JOINT && entry.flags & SNAPSHOT_ENTRY_LIVE != 0
        }) {
            let key = snapshot_raw_key(entry)?;
            let body_a = lookup_slot(&body_slots, entry.owner_a)?;
            let body_b = lookup_slot(&body_slots, entry.owner_b)?;
            let kind =
                JointType::from_raw(entry.subtype).ok_or(ApiError::SnapshotManifestMismatch)?;
            let nonce = next_import_nonce(&mut next_nonce)?;
            insert_slot(&mut joint_slots, entry.index, key)?;
            if state
                .joints
                .insert(
                    key,
                    JointRegistration {
                        nonce,
                        bodies: [body_a, body_b],
                        kind,
                    },
                )
                .is_some()
            {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            push_fallible(&mut body_registration_mut(&mut state, body_a)?.joints, key)?;
            if body_b != body_a {
                push_fallible(&mut body_registration_mut(&mut state, body_b)?.joints, key)?;
            }
        }
        debug_assert_eq!(next_nonce, last_nonce);

        let registry = Arc::new(Self {
            brand,
            last_nonce: AtomicU64::new(last_nonce),
            state: Mutex::new(state),
        });
        install(&registry);
        Ok(registry)
    }

    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) const fn brand(&self) -> IdBrand {
        self.brand
    }

    fn reserve_nonce_range(&self, count: u64) -> ApiResult<u64> {
        debug_assert!(count > 0);
        self.last_nonce
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |last| {
                last.checked_add(count)
            })
            .map_err(|_| ApiError::ObjectIdentityExhausted)
    }

    fn allocate_nonce(&self) -> ApiResult<RegistrationNonce> {
        let previous = self.reserve_nonce_range(1)?;
        RegistrationNonce::new(previous + 1)
    }

    fn allocate_nonces(&self, count: usize) -> ApiResult<Vec<RegistrationNonce>> {
        let count = u64::try_from(count).map_err(|_| ApiError::ObjectIdentityExhausted)?;
        if count == 0 {
            return Ok(Vec::new());
        }
        let previous = self.reserve_nonce_range(count)?;
        let mut nonces = Vec::new();
        nonces
            .try_reserve_exact(count as usize)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        for offset in 1..=count {
            nonces.push(RegistrationNonce::new(previous + offset)?);
        }
        Ok(nonces)
    }

    pub(crate) fn register_body(&self, raw: ffi::b2BodyId) -> ApiResult<BodyId> {
        self.brand.check_body_raw(raw)?;
        let key = RawKey::from(raw);
        let mut state = self.lock_state();
        if state.bodies.contains_key(&key) || state.retired_bodies.contains_key(&key) {
            return Err(ApiError::ObjectIdentityExhausted);
        }
        state.try_reserve_bodies(1)?;
        let nonce = self.allocate_nonce()?;
        state.bodies.insert(key, BodyRegistration::new(nonce));
        state.advance_revision();
        Ok(self.brand.body(raw, nonce))
    }

    pub(crate) fn register_shape(&self, raw: ffi::b2ShapeId, body: BodyId) -> ApiResult<ShapeId> {
        self.brand.check_shape_raw(raw)?;
        self.check_body_brand(body)?;
        let key = RawKey::from(raw);
        let body_key = RawKey::from(body.into_raw());
        let mut state = self.lock_state();
        if state.shapes.contains_key(&key) || state.retired_shapes.contains_key(&key) {
            return Err(ApiError::ObjectIdentityExhausted);
        }
        let body_registration = state.bodies.get(&body_key).ok_or(ApiError::InvalidBodyId)?;
        if body_registration.nonce != body.registration_nonce() {
            return Err(ApiError::InvalidBodyId);
        }
        state.try_reserve_shapes(1)?;
        state
            .bodies
            .get_mut(&body_key)
            .expect("body registration checked")
            .shapes
            .try_reserve(1)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        let nonce = self.allocate_nonce()?;
        state.shapes.insert(
            key,
            ShapeRegistration {
                nonce,
                body: body_key,
                chain: None,
            },
        );
        state
            .bodies
            .get_mut(&body_key)
            .expect("body registration checked")
            .shapes
            .push(key);
        state.advance_revision();
        Ok(self.brand.shape(raw, nonce))
    }

    pub(crate) fn register_joint(
        &self,
        raw: ffi::b2JointId,
        body_a: BodyId,
        body_b: BodyId,
        kind: JointType,
    ) -> ApiResult<JointId> {
        self.brand.check_joint_raw(raw)?;
        self.check_body_brand(body_a)?;
        self.check_body_brand(body_b)?;
        let key = RawKey::from(raw);
        let body_keys = [
            RawKey::from(body_a.into_raw()),
            RawKey::from(body_b.into_raw()),
        ];
        let mut state = self.lock_state();
        for (body_key, body) in [(body_keys[0], body_a), (body_keys[1], body_b)] {
            let registration = state.bodies.get(&body_key).ok_or(ApiError::InvalidBodyId)?;
            if registration.nonce != body.registration_nonce() {
                return Err(ApiError::InvalidBodyId);
            }
        }
        if state.joints.contains_key(&key) || state.retired_joints.contains_key(&key) {
            return Err(ApiError::ObjectIdentityExhausted);
        }
        state.try_reserve_joints(1)?;
        state
            .bodies
            .get_mut(&body_keys[0])
            .expect("body registration checked")
            .joints
            .try_reserve(1)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        if body_keys[1] != body_keys[0] {
            state
                .bodies
                .get_mut(&body_keys[1])
                .expect("body registration checked")
                .joints
                .try_reserve(1)
                .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        }
        let nonce = self.allocate_nonce()?;
        state.joints.insert(
            key,
            JointRegistration {
                nonce,
                bodies: body_keys,
                kind,
            },
        );
        push_unique(
            &mut state
                .bodies
                .get_mut(&body_keys[0])
                .expect("body registration checked")
                .joints,
            key,
        );
        push_unique(
            &mut state
                .bodies
                .get_mut(&body_keys[1])
                .expect("body registration checked")
                .joints,
            key,
        );
        state.advance_revision();
        Ok(self.brand.joint(raw, nonce))
    }

    /// Register a chain and every native segment as one host-side transaction.
    pub(crate) fn register_chain(
        &self,
        raw: ffi::b2ChainId,
        body: BodyId,
        raw_segments: &[ffi::b2ShapeId],
    ) -> ApiResult<(ChainId, Vec<ShapeId>)> {
        self.brand.check_chain_raw(raw)?;
        self.check_body_brand(body)?;
        let chain_key = RawKey::from(raw);
        let body_key = RawKey::from(body.into_raw());
        let mut segment_keys = Vec::new();
        segment_keys
            .try_reserve_exact(raw_segments.len())
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        let mut seen_segment_keys = HashSet::new();
        seen_segment_keys
            .try_reserve(raw_segments.len())
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        for &segment in raw_segments {
            self.brand.check_shape_raw(segment)?;
            let key = RawKey::from(segment);
            if !seen_segment_keys.insert(key) {
                return Err(ApiError::ObjectIdentityExhausted);
            }
            segment_keys.push(key);
        }

        let mut state = self.lock_state();
        let body_registration = state.bodies.get(&body_key).ok_or(ApiError::InvalidBodyId)?;
        if body_registration.nonce != body.registration_nonce() {
            return Err(ApiError::InvalidBodyId);
        }
        if state.chains.contains_key(&chain_key)
            || state.retired_chains.contains_key(&chain_key)
            || segment_keys
                .iter()
                .any(|key| state.shapes.contains_key(key) || state.retired_shapes.contains_key(key))
        {
            return Err(ApiError::ObjectIdentityExhausted);
        }

        state.try_reserve_chains(1)?;
        state.try_reserve_shapes(segment_keys.len())?;
        let body_registration = state
            .bodies
            .get_mut(&body_key)
            .expect("body registration checked");
        body_registration
            .chains
            .try_reserve(1)
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        body_registration
            .shapes
            .try_reserve(segment_keys.len())
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;

        let mut nonces = self
            .allocate_nonces(
                raw_segments
                    .len()
                    .checked_add(1)
                    .ok_or(ApiError::ObjectIdentityExhausted)?,
            )?
            .into_iter();
        let chain_nonce = nonces.next().expect("one chain nonce allocated");
        let mut segments = Vec::new();
        segments
            .try_reserve_exact(raw_segments.len())
            .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
        for ((&segment, &key), nonce) in raw_segments.iter().zip(&segment_keys).zip(&mut nonces) {
            state.shapes.insert(
                key,
                ShapeRegistration {
                    nonce,
                    body: body_key,
                    chain: Some(chain_key),
                },
            );
            segments.push(self.brand.shape(segment, nonce));
        }
        let body_registration = state
            .bodies
            .get_mut(&body_key)
            .expect("body registration checked");
        body_registration.chains.push(chain_key);
        body_registration
            .shapes
            .extend(segment_keys.iter().copied());
        state.chains.insert(
            chain_key,
            ChainRegistration {
                nonce: chain_nonce,
                body: body_key,
                segments: segment_keys,
            },
        );
        state.advance_revision();
        Ok((self.brand.chain(raw, chain_nonce), segments))
    }

    pub(crate) fn resolve_body(&self, raw: ffi::b2BodyId) -> ApiResult<BodyId> {
        self.brand.check_body_raw(raw)?;
        let state = self.lock_state();
        let nonce = state
            .bodies
            .get(&RawKey::from(raw))
            .map(|entry| entry.nonce)
            .ok_or(ApiError::InvalidBodyId)?;
        std::mem::drop(state);
        Ok(self.brand.body(raw, nonce))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn resolve_shape(&self, raw: ffi::b2ShapeId) -> ApiResult<ShapeId> {
        self.brand.check_shape_raw(raw)?;
        let state = self.lock_state();
        let nonce = state
            .shapes
            .get(&RawKey::from(raw))
            .map(|entry| entry.nonce)
            .ok_or(ApiError::InvalidShapeId)?;
        std::mem::drop(state);
        Ok(self.brand.shape(raw, nonce))
    }

    fn resolve_body_output(&self, raw: ffi::b2BodyId) -> ApiResult<BodyId> {
        self.brand.check_body_raw(raw)?;
        let state = self.lock_state();
        let key = RawKey::from(raw);
        let nonce = state
            .bodies
            .get(&key)
            .map(|entry| entry.nonce)
            .or_else(|| state.retired_bodies.get(&key).copied())
            .ok_or(ApiError::InvalidBodyId)?;
        std::mem::drop(state);
        Ok(self.brand.body(raw, nonce))
    }

    fn resolve_shape_output(&self, raw: ffi::b2ShapeId) -> ApiResult<ShapeId> {
        self.brand.check_shape_raw(raw)?;
        let state = self.lock_state();
        let key = RawKey::from(raw);
        let nonce = state
            .shapes
            .get(&key)
            .map(|entry| entry.nonce)
            .or_else(|| state.retired_shapes.get(&key).copied())
            .ok_or(ApiError::InvalidShapeId)?;
        std::mem::drop(state);
        Ok(self.brand.shape(raw, nonce))
    }

    fn resolve_joint_output(&self, raw: ffi::b2JointId) -> ApiResult<JointId> {
        self.brand.check_joint_raw(raw)?;
        let state = self.lock_state();
        let key = RawKey::from(raw);
        let nonce = state
            .joints
            .get(&key)
            .map(|entry| entry.nonce)
            .or_else(|| state.retired_joints.get(&key).copied())
            .ok_or(ApiError::InvalidJointId)?;
        std::mem::drop(state);
        Ok(self.brand.joint(raw, nonce))
    }

    fn resolve_chain_output(&self, raw: ffi::b2ChainId) -> ApiResult<ChainId> {
        self.brand.check_chain_raw(raw)?;
        let state = self.lock_state();
        let key = RawKey::from(raw);
        let nonce = state
            .chains
            .get(&key)
            .map(|entry| entry.nonce)
            .or_else(|| state.retired_chains.get(&key).copied())
            .ok_or(ApiError::InvalidChainId)?;
        std::mem::drop(state);
        Ok(self.brand.chain(raw, nonce))
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

    pub(crate) fn contains_joint(&self, id: JointId) -> bool {
        self.check_joint_brand(id).is_ok()
            && self
                .lock_state()
                .joints
                .get(&RawKey::from(id.into_raw()))
                .is_some_and(|entry| entry.nonce == id.registration_nonce())
    }

    pub(crate) fn joint_type(&self, id: JointId) -> ApiResult<JointType> {
        self.check_joint_brand(id)?;
        let state = self.lock_state();
        let registration = state
            .joints
            .get(&RawKey::from(id.into_raw()))
            .ok_or(ApiError::InvalidJointId)?;
        if registration.nonce != id.registration_nonce() {
            return Err(ApiError::InvalidJointId);
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
        state.retired_bodies.insert(key, body.nonce);
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

    pub(crate) fn clear_and_uninstall(self: &Arc<Self>) {
        {
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
        uninstall(self);
    }

    pub(crate) fn snapshot_manifest(&self) -> ApiResult<IdentityManifest> {
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
    ) -> ApiResult<PreparedIdentityRestore> {
        if manifest.brand != self.brand {
            return Err(ApiError::WrongWorld);
        }
        let current = self.lock_state();
        let fresh_count = count_changed(&current.bodies, &manifest.bodies)
            .checked_add(count_changed(&current.shapes, &manifest.shapes))
            .and_then(|count| count.checked_add(count_changed(&current.joints, &manifest.joints)))
            .and_then(|count| count.checked_add(count_changed(&current.chains, &manifest.chains)))
            .ok_or(ApiError::ObjectIdentityExhausted)?;
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
    pub(crate) fn commit_restore(&self, mut prepared: PreparedIdentityRestore) -> ApiResult<()> {
        if prepared.brand != self.brand {
            return Err(ApiError::WrongWorld);
        }
        let mut state = self.lock_state();
        if state.revision != prepared.base_revision {
            return Err(ApiError::WorldBusy);
        }
        prepared.target.revision = state.revision.wrapping_add(1);
        *state = prepared.target;
        Ok(())
    }

    fn check_body_brand(&self, id: BodyId) -> ApiResult<()> {
        check_brand(self.brand, id.brand())
    }

    fn check_shape_brand(&self, id: ShapeId) -> ApiResult<()> {
        check_brand(self.brand, id.brand())
    }

    fn check_joint_brand(&self, id: JointId) -> ApiResult<()> {
        check_brand(self.brand, id.brand())
    }

    fn check_chain_brand(&self, id: ChainId) -> ApiResult<()> {
        check_brand(self.brand, id.brand())
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, IdentityState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[cfg(test)]
    fn set_last_nonce_for_test(&self, value: u64) {
        self.last_nonce.store(value, Ordering::Relaxed);
    }
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
    ) -> ApiResult<()> {
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
    fn new(entries: &'a [boxdd_sys::adapter::SnapshotEntry]) -> ApiResult<Self> {
        use boxdd_sys::adapter::{
            SNAPSHOT_ENTRY_BODY, SNAPSHOT_ENTRY_CHAIN, SNAPSHOT_ENTRY_JOINT, SNAPSHOT_ENTRY_LIVE,
            SNAPSHOT_ENTRY_SHAPE, SNAPSHOT_ENTRY_VERSION,
        };

        let mut by_slot = HashMap::new();
        by_slot
            .try_reserve(entries.len())
            .map_err(|_| ApiError::SnapshotAllocationFailed)?;
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
                return Err(ApiError::SnapshotManifestMismatch);
            }
            if by_slot.insert((entry.kind, entry.index), entry).is_some() {
                return Err(ApiError::SnapshotManifestMismatch);
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

    fn validate_manifest(&self, manifest: &IdentityManifest) -> ApiResult<()> {
        use boxdd_sys::adapter::{
            SNAPSHOT_ENTRY_BODY, SNAPSHOT_ENTRY_CHAIN, SNAPSHOT_ENTRY_JOINT, SNAPSHOT_ENTRY_SHAPE,
        };

        if self.live_bodies != manifest.bodies.len()
            || self.live_shapes != manifest.shapes.len()
            || self.live_joints != manifest.joints.len()
            || self.live_chains != manifest.chains.len()
        {
            return Err(ApiError::SnapshotManifestMismatch);
        }

        let mut body_relations = HashMap::<RawKey, [usize; 3]>::new();
        body_relations
            .try_reserve(manifest.bodies.len())
            .map_err(|_| ApiError::SnapshotAllocationFailed)?;
        body_relations.extend(manifest.bodies.keys().copied().map(|key| (key, [0; 3])));

        let mut chain_segments = HashMap::<RawKey, usize>::new();
        chain_segments
            .try_reserve(manifest.chains.len())
            .map_err(|_| ApiError::SnapshotAllocationFailed)?;
        chain_segments.extend(manifest.chains.keys().copied().map(|key| (key, 0)));

        for &key in manifest.bodies.keys() {
            self.require_key(SNAPSHOT_ENTRY_BODY, key)?;
        }

        for (&key, shape) in &manifest.shapes {
            let native = self.require_key(SNAPSHOT_ENTRY_SHAPE, key)?;
            if self.key_for_slot(SNAPSHOT_ENTRY_BODY, native.owner_a)? != shape.body {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            let native_chain = if native.owner_b < 0 {
                None
            } else {
                Some(self.key_for_slot(SNAPSHOT_ENTRY_CHAIN, native.owner_b)?)
            };
            if native_chain != shape.chain {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            body_relations
                .get_mut(&shape.body)
                .ok_or(ApiError::SnapshotManifestMismatch)?[0] += 1;
            if let Some(chain) = shape.chain {
                *chain_segments
                    .get_mut(&chain)
                    .ok_or(ApiError::SnapshotManifestMismatch)? += 1;
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
                return Err(ApiError::SnapshotManifestMismatch);
            }
            body_relations
                .get_mut(&joint.bodies[0])
                .ok_or(ApiError::SnapshotManifestMismatch)?[1] += 1;
            if joint.bodies[1] != joint.bodies[0] {
                body_relations
                    .get_mut(&joint.bodies[1])
                    .ok_or(ApiError::SnapshotManifestMismatch)?[1] += 1;
            }
        }

        for (&key, chain) in &manifest.chains {
            let native = self.require_key(SNAPSHOT_ENTRY_CHAIN, key)?;
            if self.key_for_slot(SNAPSHOT_ENTRY_BODY, native.owner_a)? != chain.body {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            body_relations
                .get_mut(&chain.body)
                .ok_or(ApiError::SnapshotManifestMismatch)?[2] += 1;
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
            .map_err(|_| ApiError::SnapshotAllocationFailed)?;

        for (&body_key, body) in &manifest.bodies {
            let counts = body_relations
                .get(&body_key)
                .ok_or(ApiError::SnapshotManifestMismatch)?;
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
                return Err(ApiError::SnapshotManifestMismatch);
            }
        }

        for (&chain_key, chain) in &manifest.chains {
            if chain.segments.len()
                != *chain_segments
                    .get(&chain_key)
                    .ok_or(ApiError::SnapshotManifestMismatch)?
                || !relations_are_exact(&mut seen, &chain.segments, |shape| {
                    manifest
                        .shapes
                        .get(shape)
                        .is_some_and(|entry| entry.chain == Some(chain_key))
                })
            {
                return Err(ApiError::SnapshotManifestMismatch);
            }
            for (expected_order, &segment) in chain.segments.iter().enumerate() {
                let native = self.require_key(SNAPSHOT_ENTRY_SHAPE, segment)?;
                if native.owner_b_order
                    != i32::try_from(expected_order)
                        .map_err(|_| ApiError::SnapshotManifestMismatch)?
                {
                    return Err(ApiError::SnapshotManifestMismatch);
                }
            }
        }

        Ok(())
    }

    fn require_key(
        &self,
        kind: u32,
        key: RawKey,
    ) -> ApiResult<&'a boxdd_sys::adapter::SnapshotEntry> {
        let index = key
            .index1
            .checked_sub(1)
            .ok_or(ApiError::SnapshotManifestMismatch)?;
        let entry = self
            .by_slot
            .get(&(kind, index))
            .copied()
            .ok_or(ApiError::SnapshotManifestMismatch)?;
        if entry.flags & boxdd_sys::adapter::SNAPSHOT_ENTRY_LIVE == 0
            || snapshot_raw_key(entry)? != key
        {
            return Err(ApiError::SnapshotManifestMismatch);
        }
        Ok(entry)
    }

    fn key_for_slot(&self, kind: u32, index: i32) -> ApiResult<RawKey> {
        let entry = self
            .by_slot
            .get(&(kind, index))
            .copied()
            .ok_or(ApiError::SnapshotManifestMismatch)?;
        if entry.flags & boxdd_sys::adapter::SNAPSHOT_ENTRY_LIVE == 0 {
            return Err(ApiError::SnapshotManifestMismatch);
        }
        snapshot_raw_key(entry)
    }
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

fn count_live(entries: &[boxdd_sys::adapter::SnapshotEntry], kind: u32) -> usize {
    entries
        .iter()
        .filter(|entry| {
            entry.kind == kind && entry.flags & boxdd_sys::adapter::SNAPSHOT_ENTRY_LIVE != 0
        })
        .count()
}

fn allocate_entry_slots(
    entries: &[boxdd_sys::adapter::SnapshotEntry],
    kind: u32,
) -> ApiResult<Vec<Option<RawKey>>> {
    let length =
        entries
            .iter()
            .filter(|entry| entry.kind == kind)
            .try_fold(0usize, |length, entry| {
                let index =
                    usize::try_from(entry.index).map_err(|_| ApiError::SnapshotManifestMismatch)?;
                Ok::<_, ApiError>(length.max(index.saturating_add(1)))
            })?;
    let mut slots = Vec::new();
    slots
        .try_reserve_exact(length)
        .map_err(|_| ApiError::SnapshotAllocationFailed)?;
    slots.resize(length, None);
    Ok(slots)
}

fn snapshot_raw_key(entry: &boxdd_sys::adapter::SnapshotEntry) -> ApiResult<RawKey> {
    let index1 = entry
        .index
        .checked_add(1)
        .ok_or(ApiError::SnapshotManifestMismatch)?;
    let generation =
        u16::try_from(entry.generation).map_err(|_| ApiError::SnapshotManifestMismatch)?;
    if index1 <= 0 {
        return Err(ApiError::SnapshotManifestMismatch);
    }
    Ok(RawKey::new(index1, generation))
}

fn insert_slot(slots: &mut [Option<RawKey>], index: i32, key: RawKey) -> ApiResult<()> {
    let slot = slots
        .get_mut(usize::try_from(index).map_err(|_| ApiError::SnapshotManifestMismatch)?)
        .ok_or(ApiError::SnapshotManifestMismatch)?;
    if slot.replace(key).is_some() {
        return Err(ApiError::SnapshotManifestMismatch);
    }
    Ok(())
}

fn lookup_slot(slots: &[Option<RawKey>], index: i32) -> ApiResult<RawKey> {
    slots
        .get(usize::try_from(index).map_err(|_| ApiError::SnapshotManifestMismatch)?)
        .copied()
        .flatten()
        .ok_or(ApiError::SnapshotManifestMismatch)
}

fn next_import_nonce(last: &mut u64) -> ApiResult<RegistrationNonce> {
    *last = last
        .checked_add(1)
        .ok_or(ApiError::ObjectIdentityExhausted)?;
    RegistrationNonce::new(*last)
}

fn push_fallible<T>(target: &mut Vec<T>, value: T) -> ApiResult<()> {
    target
        .try_reserve(1)
        .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
    target.push(value);
    Ok(())
}

fn body_registration_mut(
    state: &mut IdentityState,
    key: RawKey,
) -> ApiResult<&mut BodyRegistration> {
    state
        .bodies
        .get_mut(&key)
        .ok_or(ApiError::SnapshotManifestMismatch)
}

fn chain_registration_mut(
    state: &mut IdentityState,
    key: RawKey,
) -> ApiResult<&mut ChainRegistration> {
    state
        .chains
        .get_mut(&key)
        .ok_or(ApiError::SnapshotManifestMismatch)
}

fn install(registry: &Arc<ActiveIdentityRegistry>) {
    let registries = REGISTRIES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut registries = registries
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let previous = registries.insert(registry.brand.token(), Arc::downgrade(registry));
    debug_assert!(previous.and_then(|weak| weak.upgrade()).is_none());
}

fn uninstall(registry: &Arc<ActiveIdentityRegistry>) {
    let Some(registries) = REGISTRIES.get() else {
        return;
    };
    let mut registries = registries
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let should_remove = registries
        .get(&registry.brand.token())
        .and_then(Weak::upgrade)
        .is_none_or(|installed| Arc::ptr_eq(&installed, registry));
    if should_remove {
        registries.remove(&registry.brand.token());
    }
}

fn lookup(brand: IdBrand) -> Option<Arc<ActiveIdentityRegistry>> {
    let registries = REGISTRIES.get()?;
    let registry = registries
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&brand.token())
        .and_then(std::sync::Weak::upgrade)?;
    (registry.brand == brand).then_some(registry)
}

pub(crate) fn resolve_body_for_brand(brand: IdBrand, raw: ffi::b2BodyId) -> ApiResult<BodyId> {
    brand.check_body_raw(raw)?;
    let registry = lookup(brand).ok_or(ApiError::InvalidBodyId)?;
    ActiveIdentityRegistry::resolve_body_output(&registry, raw)
}

pub(crate) fn resolve_shape_for_brand(brand: IdBrand, raw: ffi::b2ShapeId) -> ApiResult<ShapeId> {
    brand.check_shape_raw(raw)?;
    let registry = lookup(brand).ok_or(ApiError::InvalidShapeId)?;
    ActiveIdentityRegistry::resolve_shape_output(&registry, raw)
}

pub(crate) fn resolve_joint_for_brand(brand: IdBrand, raw: ffi::b2JointId) -> ApiResult<JointId> {
    brand.check_joint_raw(raw)?;
    let registry = lookup(brand).ok_or(ApiError::InvalidJointId)?;
    ActiveIdentityRegistry::resolve_joint_output(&registry, raw)
}

pub(crate) fn resolve_chain_for_brand(brand: IdBrand, raw: ffi::b2ChainId) -> ApiResult<ChainId> {
    brand.check_chain_raw(raw)?;
    let registry = lookup(brand).ok_or(ApiError::InvalidChainId)?;
    ActiveIdentityRegistry::resolve_chain_output(&registry, raw)
}

pub(crate) fn body_is_active(id: BodyId) -> bool {
    lookup(id.brand()).is_some_and(|registry| registry.contains_body(id))
}

pub(crate) fn shape_is_active(id: ShapeId) -> bool {
    lookup(id.brand()).is_some_and(|registry| registry.contains_shape(id))
}

fn check_brand(expected: IdBrand, actual: IdBrand) -> ApiResult<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(ApiError::WrongWorld)
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
    state.retired_shapes.insert(key, shape.nonce);
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
    state.retired_joints.insert(key, joint.nonce);
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
        state.retired_shapes.insert(segment, shape.nonce);
    }
    state.retired_chains.insert(key, chain.nonce);
}

fn try_clone_vec<T: Copy>(source: &[T]) -> ApiResult<Vec<T>> {
    let mut cloned = Vec::new();
    cloned
        .try_reserve_exact(source.len())
        .map_err(|_| ApiError::IdentityTrackingAllocationFailed)?;
    cloned.extend_from_slice(source);
    Ok(cloned)
}

fn try_reserve_map<K, V>(map: &mut HashMap<K, V>, additional: usize) -> ApiResult<()>
where
    K: Eq + std::hash::Hash,
{
    map.try_reserve(additional)
        .map_err(|_| ApiError::IdentityTrackingAllocationFailed)
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
            Err(ApiError::ObjectIdentityExhausted)
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
            Err(ApiError::ObjectIdentityExhausted)
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
            Err(ApiError::ObjectIdentityExhausted)
        );

        let chain_raw = raw_chain(brand, 1, 5);
        let segment_raw = raw_shape(brand, 1, 8);
        let (retired_chain, _) = registry
            .register_chain(chain_raw, owner, &[segment_raw])
            .unwrap();
        assert!(registry.unregister_chain(retired_chain));
        assert_eq!(
            registry.register_chain(chain_raw, owner, &[raw_shape(brand, 2, 0)]),
            Err(ApiError::ObjectIdentityExhausted)
        );
    }

    #[test]
    fn worker_lookup_uses_only_the_active_registration() {
        let (brand, registry) = fixture();
        let body = registry.register_body(raw_body(brand, 1, 0)).unwrap();
        let raw = raw_shape(brand, 1, 0);
        let shape = registry.register_shape(raw, body).unwrap();

        assert_eq!(registry.resolve_shape(raw), Ok(shape));
        assert!(registry.unregister_shape(shape));
        assert_eq!(registry.resolve_shape(raw), Err(ApiError::InvalidShapeId));
    }

    #[test]
    fn nonce_exhaustion_does_not_mutate_the_active_table() {
        let (brand, registry) = fixture();
        registry.set_last_nonce_for_test(u64::MAX);
        let raw = raw_body(brand, 1, 0);

        assert_eq!(
            registry.register_body(raw),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(registry.resolve_body(raw), Err(ApiError::InvalidBodyId));
    }

    #[test]
    fn chain_registration_is_atomic_on_duplicate_segments() {
        let (brand, registry) = fixture();
        let body = registry.register_body(raw_body(brand, 1, 0)).unwrap();
        let raw_chain = raw_chain(brand, 1, 0);
        let segment = raw_shape(brand, 1, 0);

        assert_eq!(
            registry.register_chain(raw_chain, body, &[segment, segment]),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(
            registry.resolve_chain_output(raw_chain),
            Err(ApiError::InvalidChainId)
        );
        assert_eq!(
            registry.resolve_shape(segment),
            Err(ApiError::InvalidShapeId)
        );
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
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(
            registry.resolve_chain_output(fresh_chain_raw),
            Err(ApiError::InvalidChainId)
        );
        assert_eq!(
            registry.resolve_shape(fresh_segment_raw),
            Err(ApiError::InvalidShapeId)
        );

        assert_eq!(
            registry.register_chain(retired_chain_raw, body, &[fresh_segment_raw]),
            Err(ApiError::ObjectIdentityExhausted)
        );
        assert_eq!(
            registry.resolve_shape(fresh_segment_raw),
            Err(ApiError::InvalidShapeId)
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
                Err(ApiError::InvalidShapeId)
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
