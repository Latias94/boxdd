use boxdd_sys::{adapter, ffi};
#[cfg(not(target_arch = "wasm32"))]
use core::cell::Cell;
#[cfg(test)]
use core::cell::RefCell;
#[cfg(not(target_arch = "wasm32"))]
use core::ffi::{c_char, c_int};
use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::CStr;
#[cfg(not(target_arch = "wasm32"))]
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// A process-global Box2D assertion hook.
///
/// The arguments are the failed condition, source file, and source line reported by Box2D.
/// Panics are contained by the foundation trampoline and never unwind through C.
/// A build configured with `panic = "abort"` still terminates immediately on panic.
#[cfg(not(target_arch = "wasm32"))]
pub type FoundationAssertHook = dyn Fn(&str, &str, i32) + Send + Sync + 'static;

/// A process-global Box2D log hook.
///
/// Panics are contained by the foundation trampoline and never unwind through C.
/// A build configured with `panic = "abort"` still terminates immediately on panic.
#[cfg(not(target_arch = "wasm32"))]
pub type FoundationLogHook = dyn Fn(&str) + Send + Sync + 'static;

/// Immutable process-global Box2D configuration.
///
/// The first successful [`Foundation::initialize`] call freezes this configuration. Cloning a
/// configuration preserves the identity of its hook `Arc`s, so initializing with either clone is
/// idempotent.
#[derive(Clone)]
pub struct FoundationConfig {
    length_units_per_meter: f32,
    #[cfg(not(target_arch = "wasm32"))]
    assert_hook: Option<Arc<FoundationAssertHook>>,
    #[cfg(not(target_arch = "wasm32"))]
    log_hook: Option<Arc<FoundationLogHook>>,
}

#[cfg(not(target_arch = "wasm32"))]
fn retire_foundation_hook<T: ?Sized>(hook: Option<Arc<T>>) {
    let mut panic = crate::core::callback_state::PanicSlot::default();
    panic.run_cleanup(|| drop(hook));
    panic.resume_or_forget();
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for FoundationConfig {
    fn drop(&mut self) {
        let mut panic = crate::core::callback_state::PanicSlot::default();
        let assert_hook = self.assert_hook.take();
        let log_hook = self.log_hook.take();
        panic.run_cleanup(|| drop(assert_hook));
        panic.run_cleanup(|| drop(log_hook));
        panic.resume_or_forget();
    }
}

impl FoundationConfig {
    /// Construct a configuration with the requested length scale and default hooks.
    ///
    /// [`Foundation::initialize`] validates the scale and every scale-derived native calculation
    /// used by the safe API before changing Box2D's process-global state.
    #[must_use]
    pub fn new(length_units_per_meter: f32) -> Self {
        Self {
            length_units_per_meter,
            #[cfg(not(target_arch = "wasm32"))]
            assert_hook: None,
            #[cfg(not(target_arch = "wasm32"))]
            log_hook: None,
        }
    }

    /// Set the number of application length units represented by one meter.
    ///
    /// [`Foundation::initialize`] validates the scale and its derived native calculations.
    #[must_use]
    pub fn with_length_units_per_meter(mut self, length_units_per_meter: f32) -> Self {
        self.length_units_per_meter = length_units_per_meter;
        self
    }

    /// Install a permanent process-global Box2D assertion hook.
    ///
    /// Use clones of the same `Arc` when more than one initialization path may race. Two distinct
    /// allocations are treated as conflicting hooks even when their closure bodies are identical.
    #[must_use]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_assert_hook(mut self, hook: Arc<FoundationAssertHook>) -> Self {
        let retired = self.assert_hook.replace(hook);
        retire_foundation_hook(retired);
        self
    }

    /// Install a permanent process-global Box2D log hook.
    ///
    /// Use clones of the same `Arc` when more than one initialization path may race. Two distinct
    /// allocations are treated as conflicting hooks even when their closure bodies are identical.
    #[must_use]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_log_hook(mut self, hook: Arc<FoundationLogHook>) -> Self {
        let retired = self.log_hook.replace(hook);
        retire_foundation_hook(retired);
        self
    }

    /// Return the configured application length units per meter.
    #[must_use]
    pub fn length_units_per_meter(&self) -> f32 {
        self.length_units_per_meter
    }

    /// Return whether a custom assertion hook is configured.
    #[must_use]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_assert_hook(&self) -> bool {
        self.assert_hook.is_some()
    }

    /// Return whether a custom log hook is configured.
    #[must_use]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_log_hook(&self) -> bool {
        self.log_hook.is_some()
    }

    fn validate(&self) -> Result<(), FoundationInitError> {
        if crate::core::length_scale::is_safe_length_units_per_meter(self.length_units_per_meter) {
            Ok(())
        } else {
            Err(FoundationInitError::InvalidLengthUnitsPerMeter)
        }
    }

    fn is_same_configuration(&self, other: &Self) -> bool {
        if self.length_units_per_meter.to_bits() != other.length_units_per_meter.to_bits() {
            return false;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            same_optional_hook(&self.assert_hook, &other.assert_hook)
                && same_optional_hook(&self.log_hook, &other.log_hook)
        }
        #[cfg(target_arch = "wasm32")]
        {
            true
        }
    }
}

impl Default for FoundationConfig {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl fmt::Debug for FoundationConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("FoundationConfig");
        debug.field("length_units_per_meter", &self.length_units_per_meter);
        #[cfg(not(target_arch = "wasm32"))]
        {
            debug
                .field("has_assert_hook", &self.has_assert_hook())
                .field("has_log_hook", &self.has_log_hook());
        }
        debug.finish()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn same_optional_hook<T: ?Sized>(left: &Option<Arc<T>>, right: &Option<Arc<T>>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => Arc::ptr_eq(left, right),
        _ => false,
    }
}

/// Failure to initialize the process-global Box2D foundation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FoundationAdapterIdentityField {
    /// Versioned identity structure size.
    StructSize,
    /// Repository adapter ABI version.
    AbiVersion,
    /// Native snapshot wire version.
    SnapshotVersion,
    /// Recording wire version.
    RecordingVersion,
    /// Target pointer width.
    PointerWidth,
    /// Target byte order.
    Endianness,
    /// Single- or double-precision build mode.
    Precision,
    /// Native validation build mode.
    Validation,
    /// Pinned upstream Box2D revision.
    UpstreamSha,
    /// Full Rust target ABI triple.
    TargetAbi,
    /// Repository adapter source digest.
    AdapterSource,
    /// Canonical effective Box2D source digest.
    EffectiveSource,
    /// Generated recording contract digest.
    RecordingContract,
    /// Private snapshot layout identity.
    SnapshotLayout,
    /// Private C ABI identity.
    PrivateAbi,
    /// Identity field introduced by a newer low-level adapter.
    Unknown,
}

impl From<adapter::AdapterIdentityField> for FoundationAdapterIdentityField {
    fn from(field: adapter::AdapterIdentityField) -> Self {
        match field {
            adapter::AdapterIdentityField::StructSize => Self::StructSize,
            adapter::AdapterIdentityField::AbiVersion => Self::AbiVersion,
            adapter::AdapterIdentityField::SnapshotVersion => Self::SnapshotVersion,
            adapter::AdapterIdentityField::RecordingVersion => Self::RecordingVersion,
            adapter::AdapterIdentityField::PointerWidth => Self::PointerWidth,
            adapter::AdapterIdentityField::Endianness => Self::Endianness,
            adapter::AdapterIdentityField::Precision => Self::Precision,
            adapter::AdapterIdentityField::Validation => Self::Validation,
            adapter::AdapterIdentityField::UpstreamSha => Self::UpstreamSha,
            adapter::AdapterIdentityField::TargetAbi => Self::TargetAbi,
            adapter::AdapterIdentityField::AdapterSource => Self::AdapterSource,
            adapter::AdapterIdentityField::EffectiveSource => Self::EffectiveSource,
            adapter::AdapterIdentityField::RecordingContract => Self::RecordingContract,
            adapter::AdapterIdentityField::SnapshotLayout => Self::SnapshotLayout,
            adapter::AdapterIdentityField::PrivateAbi => Self::PrivateAbi,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for FoundationAdapterIdentityField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// Failure to initialize the process-global Box2D foundation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum FoundationInitError {
    /// Safe Box2D operations require a scale with representable ray and shape tolerances.
    #[error("length units per meter must preserve finite Box2D ray and shape tolerances")]
    InvalidLengthUnitsPerMeter,

    /// The linked native adapter did not provide its required identity handshake.
    #[error("the linked Box2D adapter identity is unavailable")]
    AdapterIdentityUnavailable,

    /// The linked native adapter was built for a different ABI or source contract.
    #[error("the linked Box2D adapter has a mismatched {0}")]
    AdapterIdentityMismatch(FoundationAdapterIdentityField),

    /// Another safe call already froze a different process configuration.
    #[error("Box2D foundation is already initialized with a different configuration")]
    ConfigurationConflict,
}

/// A point-in-time snapshot of process-global Box2D activity.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct FoundationActivity {
    /// Number of Rust-owned Box2D worlds holding shared access.
    pub ordinary_worlds: u32,
    /// Number of worldless native calls or persistent native owners holding shared access.
    pub transient_calls: u32,
    /// Whether a replay player holds exclusive access.
    pub replay_active: bool,
}

/// Failure to acquire a process-global Box2D foundation activity lease.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum FoundationActivityError {
    /// Native activity requires the process-global foundation to be initialized explicitly.
    #[error("the Box2D foundation has not been initialized")]
    NotInitialized,

    /// Shared activity cannot begin while replay has exclusive access.
    #[error("a replay player currently owns exclusive Box2D foundation access")]
    ReplayActive,

    /// The packed ordinary-world counter cannot represent another world.
    #[error("the Box2D ordinary-world activity counter is exhausted")]
    OrdinaryWorldCapacityExhausted,

    /// The packed transient-call counter cannot represent another call.
    #[error("the Box2D transient-call activity counter is exhausted")]
    TransientCallCapacityExhausted,

    /// Replay requires the process to have no other Box2D activity.
    #[error("Box2D foundation is busy and cannot grant exclusive replay access: {activity:?}")]
    ReplayUnavailable { activity: FoundationActivity },
}

/// Process-global diagnostics produced by panic-contained Box2D hooks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct FoundationDiagnostics {
    /// Number of assertion callback entries, including recursive entries.
    pub assertion_calls: u64,
    /// Number of panics contained in the assertion trampoline's user-hook path.
    pub assertion_hook_panics: u64,
    /// Number of recursive assertion-hook entries suppressed by the trampoline.
    pub assertion_hook_recursions: u64,
    /// Number of log callback entries, including recursive entries.
    pub log_calls: u64,
    /// Number of panics contained in the log trampoline's user-hook path.
    pub log_hook_panics: u64,
    /// Number of recursive log-hook entries suppressed by the trampoline.
    pub log_hook_recursions: u64,
}

#[derive(Default)]
struct FoundationDiagnosticState {
    assertion_calls: AtomicU64,
    assertion_hook_panics: AtomicU64,
    assertion_hook_recursions: AtomicU64,
    log_calls: AtomicU64,
    log_hook_panics: AtomicU64,
    log_hook_recursions: AtomicU64,
}

impl FoundationDiagnosticState {
    fn snapshot(&self) -> FoundationDiagnostics {
        FoundationDiagnostics {
            assertion_calls: self.assertion_calls.load(Ordering::Relaxed),
            assertion_hook_panics: self.assertion_hook_panics.load(Ordering::Relaxed),
            assertion_hook_recursions: self.assertion_hook_recursions.load(Ordering::Relaxed),
            log_calls: self.log_calls.load(Ordering::Relaxed),
            log_hook_panics: self.log_hook_panics.load(Ordering::Relaxed),
            log_hook_recursions: self.log_hook_recursions.load(Ordering::Relaxed),
        }
    }
}

const ACTIVITY_COUNT_BITS: u32 = 31;
const ORDINARY_ONE: u64 = 1;
const ORDINARY_MASK: u64 = (1_u64 << ACTIVITY_COUNT_BITS) - 1;
const TRANSIENT_SHIFT: u32 = ACTIVITY_COUNT_BITS;
const TRANSIENT_ONE: u64 = 1_u64 << TRANSIENT_SHIFT;
const TRANSIENT_MASK: u64 = ORDINARY_MASK << TRANSIENT_SHIFT;
const REPLAY_BIT: u64 = 1_u64 << (ACTIVITY_COUNT_BITS * 2);

struct FoundationActivityState {
    packed: AtomicU64,
}

impl FoundationActivityState {
    const fn new() -> Self {
        Self {
            packed: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> FoundationActivity {
        Self::unpack(self.packed.load(Ordering::Acquire))
    }

    fn unpack(packed: u64) -> FoundationActivity {
        FoundationActivity {
            ordinary_worlds: (packed & ORDINARY_MASK) as u32,
            transient_calls: ((packed & TRANSIENT_MASK) >> TRANSIENT_SHIFT) as u32,
            replay_active: packed & REPLAY_BIT != 0,
        }
    }

    fn acquire_ordinary(&'static self) -> Result<OrdinaryWorldLease, FoundationActivityError> {
        self.acquire_shared(SharedActivity::Ordinary)?;
        Ok(OrdinaryWorldLease { activity: self })
    }

    fn acquire_transient(
        &'static self,
    ) -> Result<TransientFoundationLease, FoundationActivityError> {
        self.acquire_shared(SharedActivity::Transient)?;
        Ok(TransientFoundationLease { activity: self })
    }

    fn acquire_shared(&self, activity: SharedActivity) -> Result<(), FoundationActivityError> {
        let mut current = self.packed.load(Ordering::Acquire);
        loop {
            if current & REPLAY_BIT != 0 {
                return Err(FoundationActivityError::ReplayActive);
            }

            let (increment, count, exhausted) = match activity {
                SharedActivity::Ordinary => (
                    ORDINARY_ONE,
                    current & ORDINARY_MASK,
                    FoundationActivityError::OrdinaryWorldCapacityExhausted,
                ),
                SharedActivity::Transient => (
                    TRANSIENT_ONE,
                    (current & TRANSIENT_MASK) >> TRANSIENT_SHIFT,
                    FoundationActivityError::TransientCallCapacityExhausted,
                ),
            };
            if count == ORDINARY_MASK {
                return Err(exhausted);
            }

            match self.packed.compare_exchange_weak(
                current,
                current + increment,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(observed) => current = observed,
            }
        }
    }

    fn acquire_replay(&'static self) -> Result<ReplayLease, FoundationActivityError> {
        match self
            .packed
            .compare_exchange(0, REPLAY_BIT, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => Ok(ReplayLease { activity: self }),
            Err(packed) => Err(FoundationActivityError::ReplayUnavailable {
                activity: Self::unpack(packed),
            }),
        }
    }

    fn release(&self, decrement: u64) {
        let previous = self.packed.fetch_sub(decrement, Ordering::AcqRel);
        debug_assert_eq!(previous & REPLAY_BIT, 0);
        let mask = if decrement == ORDINARY_ONE {
            ORDINARY_MASK
        } else {
            TRANSIENT_MASK
        };
        debug_assert_ne!(previous & mask, 0);
    }
}

#[derive(Clone, Copy)]
enum SharedActivity {
    Ordinary,
    Transient,
}

/// The frozen process-global Box2D foundation.
pub struct Foundation {
    config: FoundationConfig,
    activity: FoundationActivityState,
    diagnostics: FoundationDiagnosticState,
}

impl Foundation {
    fn new(config: FoundationConfig) -> Self {
        Self {
            config,
            activity: FoundationActivityState::new(),
            diagnostics: FoundationDiagnosticState::default(),
        }
    }

    /// Freeze the process-global Box2D configuration and return its explicit root capability.
    ///
    /// Repeating this call with clones of the same configuration is idempotent. A different
    /// configuration is rejected before mutating Box2D globals.
    pub fn initialize(config: FoundationConfig) -> crate::Result<&'static Self> {
        initialize_foundation_impl(config).map_err(Into::into)
    }

    /// Initialize Box2D with the default one-unit-per-meter configuration.
    pub fn initialize_default() -> crate::Result<&'static Self> {
        Self::initialize(FoundationConfig::default())
    }

    /// Return the initialized process-global root without creating one implicitly.
    #[must_use]
    pub fn get() -> Option<&'static Self> {
        FOUNDATION.get()
    }

    /// Create a world whose native activity is owned by this foundation.
    pub fn create_world(&'static self, def: crate::WorldDef) -> crate::error::Result<crate::World> {
        crate::World::create(self, def)
    }

    /// Construct scale-aware Box2D world defaults without invoking native code.
    #[must_use]
    pub fn world_def(&self) -> crate::WorldDef {
        crate::WorldDef::with_length_scale(self.length_scale())
    }

    /// Start a scale-aware world definition builder without invoking native code.
    #[must_use]
    pub fn world_builder(&self) -> crate::WorldBuilder {
        self.world_def().into()
    }

    /// Construct scale-aware Box2D body defaults without invoking native code.
    #[must_use]
    pub fn body_def(&self) -> crate::BodyDef {
        crate::BodyDef::with_length_scale(self.length_scale())
    }

    /// Start a scale-aware body definition builder without invoking native code.
    #[must_use]
    pub fn body_builder(&self) -> crate::BodyBuilder {
        self.body_def().into()
    }

    #[inline]
    pub(crate) fn length_scale(&self) -> crate::core::length_scale::LengthScale {
        crate::core::length_scale::LengthScale::try_new(self.config.length_units_per_meter)
            .expect("Foundation stores a validated length scale")
    }

    pub(crate) fn acquire_ordinary_world_lease(
        &'static self,
    ) -> Result<OrdinaryWorldLease, FoundationActivityError> {
        self.activity.acquire_ordinary()
    }

    pub(crate) fn acquire_replay_lease(
        &'static self,
    ) -> Result<ReplayLease, FoundationActivityError> {
        self.activity.acquire_replay()
    }

    /// Return the immutable configuration frozen by explicit initialization.
    #[must_use]
    pub fn config(&self) -> &FoundationConfig {
        &self.config
    }

    /// Return a coherent snapshot of shared and exclusive foundation activity.
    #[must_use]
    pub fn activity(&self) -> FoundationActivity {
        self.activity.snapshot()
    }

    /// Return the current panic-containment and recursion diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> FoundationDiagnostics {
        self.diagnostics.snapshot()
    }
}

impl fmt::Debug for Foundation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Foundation")
            .field("config", &self.config)
            .field("activity", &self.activity())
            .field("diagnostics", &self.diagnostics())
            .finish()
    }
}

static FOUNDATION: OnceLock<Foundation> = OnceLock::new();
static FOUNDATION_INIT_LOCK: Mutex<()> = Mutex::new(());
static WORLD_SLOT_MUTATION_LOCK: Mutex<()> = Mutex::new(());

/// Freeze the process-global Box2D foundation configuration.
///
/// Calling this function repeatedly with clones of the same configuration is idempotent. After
/// explicit initialization succeeds, a different configuration is rejected before mutating Box2D
/// globals.
fn initialize_foundation_impl(
    config: FoundationConfig,
) -> Result<&'static Foundation, FoundationInitError> {
    config.validate()?;

    if let Some(foundation) = FOUNDATION.get() {
        return if foundation.config.is_same_configuration(&config) {
            Ok(foundation)
        } else {
            Err(FoundationInitError::ConfigurationConflict)
        };
    }

    let init_guard = FOUNDATION_INIT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(foundation) = FOUNDATION.get() {
        let result = if foundation.config.is_same_configuration(&config) {
            Ok(foundation)
        } else {
            Err(FoundationInitError::ConfigurationConflict)
        };
        drop(init_guard);
        return result;
    }

    adapter::verify_runtime_identity().map_err(|error| match error {
        adapter::AdapterIdentityError::Unavailable => {
            FoundationInitError::AdapterIdentityUnavailable
        }
        adapter::AdapterIdentityError::Mismatch(field) => {
            FoundationInitError::AdapterIdentityMismatch(field.into())
        }
        _ => FoundationInitError::AdapterIdentityUnavailable,
    })?;

    // SAFETY: the input was validated above and the initialization mutex makes these process-global
    // mutations happen once, before this configuration becomes visible to any safe native call.
    unsafe {
        ffi::b2SetLengthUnitsPerMeter(config.length_units_per_meter);
        #[cfg(not(target_arch = "wasm32"))]
        {
            if config.assert_hook.is_some() {
                ffi::b2SetAssertFcn(Some(assert_trampoline));
            }
            if config.log_hook.is_some() {
                ffi::b2SetLogFcn(Some(log_trampoline));
            }
        }
    }

    let result = match FOUNDATION.set(Foundation::new(config)) {
        Ok(()) => Ok(FOUNDATION
            .get()
            .expect("a successfully initialized foundation must be visible")),
        Err(rejected) => {
            let existing = FOUNDATION.get();
            if existing
                .is_some_and(|foundation| foundation.config.is_same_configuration(&rejected.config))
            {
                Ok(existing.expect("an existing foundation was just observed"))
            } else {
                Err(FoundationInitError::ConfigurationConflict)
            }
        }
    };
    drop(init_guard);
    result
}

/// Lease shared process-global access for one worldless native operation.
pub(crate) fn acquire_transient_lease() -> Result<TransientFoundationLease, FoundationActivityError>
{
    let foundation = Foundation::get().ok_or(FoundationActivityError::NotInitialized)?;
    let lease = foundation.activity.acquire_transient()?;
    #[cfg(test)]
    notify_transient_lease_test_hook();
    Ok(lease)
}

pub(crate) fn current_length_units_per_meter() -> Result<f32, FoundationActivityError> {
    Foundation::get()
        .map(|foundation| foundation.config.length_units_per_meter)
        .ok_or(FoundationActivityError::NotInitialized)
}

#[cfg(test)]
type TransientLeaseTestHook = dyn Fn() + 'static;

#[cfg(test)]
std::thread_local! {
    static TRANSIENT_LEASE_TEST_HOOK: RefCell<Option<std::rc::Rc<TransientLeaseTestHook>>> =
        RefCell::new(None);
}

#[cfg(test)]
struct TransientLeaseTestHookGuard {
    previous: Option<std::rc::Rc<TransientLeaseTestHook>>,
}

#[cfg(test)]
impl Drop for TransientLeaseTestHookGuard {
    fn drop(&mut self) {
        TRANSIENT_LEASE_TEST_HOOK.with(|slot| {
            slot.replace(self.previous.take());
        });
    }
}

#[cfg(test)]
fn install_transient_lease_test_hook(
    hook: std::rc::Rc<TransientLeaseTestHook>,
) -> TransientLeaseTestHookGuard {
    let previous = TRANSIENT_LEASE_TEST_HOOK.with(|slot| slot.replace(Some(hook)));
    TransientLeaseTestHookGuard { previous }
}

#[cfg(test)]
fn notify_transient_lease_test_hook() {
    let hook = TRANSIENT_LEASE_TEST_HOOK.with(|slot| slot.borrow().clone());
    if let Some(hook) = hook {
        hook();
    }
}

/// Check callback availability and lease one fallible worldless native call.
pub(crate) fn transient_native_lease() -> crate::error::Result<TransientFoundationLease> {
    crate::core::callback_state::check_not_in_callback()?;
    acquire_transient_lease().map_err(crate::error::Error::from)
}

/// Keeps one Rust-owned native world counted as shared foundation activity.
#[must_use = "dropping this lease releases the world foundation activity"]
pub(crate) struct OrdinaryWorldLease {
    activity: &'static FoundationActivityState,
}

impl Drop for OrdinaryWorldLease {
    fn drop(&mut self) {
        self.activity.release(ORDINARY_ONE);
    }
}

/// Keeps one worldless native operation counted as transient foundation activity.
#[must_use = "dropping this lease releases the transient foundation activity"]
pub(crate) struct TransientFoundationLease {
    activity: &'static FoundationActivityState,
}

impl Drop for TransientFoundationLease {
    fn drop(&mut self) {
        self.activity.release(TRANSIENT_ONE);
    }
}

/// Keeps replay's process-global mutation window exclusive until its native player is destroyed.
#[must_use = "dropping this lease releases exclusive replay foundation activity"]
pub(crate) struct ReplayLease {
    activity: &'static FoundationActivityState,
}

impl Drop for ReplayLease {
    fn drop(&mut self) {
        let result = self.activity.packed.compare_exchange(
            REPLAY_BIT,
            0,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        debug_assert!(result.is_ok());
    }
}

/// Guard the upstream native world-slot table during one create or destroy call.
///
/// This mutex must not be held around arbitrary Box2D work or user callbacks.
pub(crate) fn lock_world_slot_mutation() -> WorldSlotMutationGuard {
    WorldSlotMutationGuard {
        _guard: WORLD_SLOT_MUTATION_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    }
}

pub(crate) struct WorldSlotMutationGuard {
    _guard: MutexGuard<'static, ()>,
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static ASSERT_HOOK_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static LOG_HOOK_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

#[cfg(not(target_arch = "wasm32"))]
struct HookRecursionGuard<'a>(&'a Cell<bool>);

#[cfg(not(target_arch = "wasm32"))]
impl Drop for HookRecursionGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn assert_trampoline(
    condition: *const c_char,
    file_name: *const c_char,
    line_number: c_int,
) -> c_int {
    let Some(foundation) = FOUNDATION.get() else {
        return 1;
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        run_assert_trampoline(foundation, || {
            // SAFETY: Box2D invokes this trampoline with null-terminated strings that remain valid
            // for the duration of the callback. Null is accepted defensively.
            let condition = unsafe { copy_c_string(condition) };
            // SAFETY: same callback contract as `condition` above.
            let file_name = unsafe { copy_c_string(file_name) };
            invoke_assert_hook(foundation, &condition, &file_name, line_number);
        })
    }));
    match result {
        Ok(result) => result,
        Err(payload) => {
            foundation
                .diagnostics
                .assertion_hook_panics
                .fetch_add(1, Ordering::Relaxed);
            core::mem::forget(payload);
            1
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_assert_trampoline(foundation: &Foundation, invoke: impl FnOnce()) -> c_int {
    foundation
        .diagnostics
        .assertion_calls
        .fetch_add(1, Ordering::Relaxed);
    ASSERT_HOOK_ACTIVE.with(|active| {
        if active.replace(true) {
            foundation
                .diagnostics
                .assertion_hook_recursions
                .fetch_add(1, Ordering::Relaxed);
            return 1;
        }
        let _recursion_guard = HookRecursionGuard(active);
        invoke();
        1
    })
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn log_trampoline(message: *const c_char) {
    let Some(foundation) = FOUNDATION.get() else {
        return;
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        run_log_trampoline(foundation, || {
            // SAFETY: Box2D invokes this trampoline with a null-terminated string that remains valid
            // for the duration of the callback. Null is accepted defensively.
            let message = unsafe { copy_c_string(message) };
            invoke_log_hook(foundation, &message);
        });
    }));
    if let Err(payload) = result {
        foundation
            .diagnostics
            .log_hook_panics
            .fetch_add(1, Ordering::Relaxed);
        core::mem::forget(payload);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_log_trampoline(foundation: &Foundation, invoke: impl FnOnce()) {
    foundation
        .diagnostics
        .log_calls
        .fetch_add(1, Ordering::Relaxed);

    LOG_HOOK_ACTIVE.with(|active| {
        if active.replace(true) {
            foundation
                .diagnostics
                .log_hook_recursions
                .fetch_add(1, Ordering::Relaxed);
            return;
        }
        let _recursion_guard = HookRecursionGuard(active);
        invoke();
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn invoke_assert_hook(foundation: &Foundation, condition: &str, file_name: &str, line_number: i32) {
    let Some(hook) = foundation.config.assert_hook.as_ref() else {
        return;
    };
    if let Err(payload) = catch_unwind(AssertUnwindSafe(|| {
        let _callback_guard = crate::core::callback_state::CallbackGuard::enter();
        hook(condition, file_name, line_number);
    })) {
        foundation
            .diagnostics
            .assertion_hook_panics
            .fetch_add(1, Ordering::Relaxed);
        // A panic payload has arbitrary user-defined drop behavior. There is no unique Rust resume
        // boundary for a process hook, so never drop it while executing on the C callback stack.
        core::mem::forget(payload);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn invoke_log_hook(foundation: &Foundation, message: &str) {
    let Some(hook) = foundation.config.log_hook.as_ref() else {
        return;
    };
    if let Err(payload) = catch_unwind(AssertUnwindSafe(|| {
        let _callback_guard = crate::core::callback_state::CallbackGuard::enter();
        hook(message);
    })) {
        foundation
            .diagnostics
            .log_hook_panics
            .fetch_add(1, Ordering::Relaxed);
        // See `invoke_assert_hook`: process hooks contain and diagnose instead of resuming.
        core::mem::forget(payload);
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn copy_c_string(value: *const c_char) -> String {
    if value.is_null() {
        return "<null>".to_owned();
    }
    // SAFETY: the caller guarantees a valid null-terminated string for the callback duration.
    unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use std::ffi::CString;
    #[cfg(not(target_arch = "wasm32"))]
    use std::process::Command;
    use std::sync::Barrier;
    use std::thread;

    #[cfg(not(target_arch = "wasm32"))]
    const HOOK_TRAMPOLINE_CHILD_ENV: &str = "BOXDD_FOUNDATION_HOOK_TRAMPOLINE_CHILD";
    #[cfg(not(target_arch = "wasm32"))]
    const NATIVE_ASSERT_TRAP_CHILD_ENV: &str = "BOXDD_FOUNDATION_NATIVE_ASSERT_TRAP_CHILD";

    fn leaked_activity() -> &'static FoundationActivityState {
        Box::leak(Box::new(FoundationActivityState::new()))
    }

    #[test]
    fn cloned_hook_configuration_is_identical() {
        let assert_hook: Arc<FoundationAssertHook> = Arc::new(|_, _, _| {});
        let log_hook: Arc<FoundationLogHook> = Arc::new(|_| {});
        let config = FoundationConfig::new(2.0)
            .with_assert_hook(Arc::clone(&assert_hook))
            .with_log_hook(Arc::clone(&log_hook));

        assert!(config.is_same_configuration(&config.clone()));
        assert!(!config.is_same_configuration(&FoundationConfig::new(2.0)));
        assert!(
            !config.is_same_configuration(
                &FoundationConfig::new(2.0)
                    .with_assert_hook(Arc::new(|_, _, _| {}))
                    .with_log_hook(log_hook),
            )
        );
    }

    #[test]
    fn invalid_scales_are_rejected_before_native_initialization() {
        let below_ray_limit = {
            #[cfg(feature = "double-precision")]
            {
                0.5e-9
            }
            #[cfg(not(feature = "double-precision"))]
            {
                0.5e-5
            }
        };
        for scale in [
            0.0,
            -1.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::from_bits(1),
            below_ray_limit,
            1.0e13_f32,
            1.0e22_f32,
            f32::MAX,
        ] {
            assert_eq!(
                FoundationConfig::new(scale).validate(),
                Err(FoundationInitError::InvalidLengthUnitsPerMeter)
            );
        }
    }

    #[test]
    fn adapter_identity_mapping_preserves_effective_source_and_private_abi() {
        assert_eq!(
            FoundationAdapterIdentityField::from(adapter::AdapterIdentityField::EffectiveSource),
            FoundationAdapterIdentityField::EffectiveSource
        );
        assert_eq!(
            FoundationAdapterIdentityField::from(adapter::AdapterIdentityField::PrivateAbi),
            FoundationAdapterIdentityField::PrivateAbi
        );
    }

    #[test]
    fn shared_activity_blocks_replay_until_all_leases_drop() {
        let activity = leaked_activity();
        let ordinary = activity.acquire_ordinary().unwrap();
        let transient = activity.acquire_transient().unwrap();

        assert_eq!(
            activity.snapshot(),
            FoundationActivity {
                ordinary_worlds: 1,
                transient_calls: 1,
                replay_active: false,
            }
        );
        assert!(matches!(
            activity.acquire_replay(),
            Err(FoundationActivityError::ReplayUnavailable { .. })
        ));

        drop(ordinary);
        drop(transient);
        let replay = activity.acquire_replay().unwrap();
        assert_eq!(
            activity.acquire_ordinary().err(),
            Some(FoundationActivityError::ReplayActive)
        );
        assert_eq!(
            activity.acquire_transient().err(),
            Some(FoundationActivityError::ReplayActive)
        );
        drop(replay);
        assert_eq!(activity.snapshot(), FoundationActivity::default());
    }

    #[test]
    fn shared_acquisition_race_preserves_exact_counts() {
        const THREADS: usize = 8;
        let activity = leaked_activity();
        let barrier = Arc::new(Barrier::new(THREADS + 1));
        let mut workers = Vec::new();

        for index in 0..THREADS {
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                let lease = if index % 2 == 0 {
                    SharedLease::Ordinary(activity.acquire_ordinary().unwrap())
                } else {
                    SharedLease::Transient(activity.acquire_transient().unwrap())
                };
                barrier.wait();
                barrier.wait();
                lease
            }));
        }

        barrier.wait();
        assert_eq!(activity.snapshot().ordinary_worlds, 4);
        assert_eq!(activity.snapshot().transient_calls, 4);
        assert!(activity.acquire_replay().is_err());
        barrier.wait();
        for worker in workers {
            drop(worker.join().unwrap());
        }
        assert_eq!(activity.snapshot(), FoundationActivity::default());
    }

    #[test]
    fn shared_and_replay_race_has_exactly_one_winner() {
        let activity = leaked_activity();
        let start = Arc::new(Barrier::new(3));
        let acquired = Arc::new(Barrier::new(3));
        let release = Arc::new(Barrier::new(3));

        let shared = {
            let start = Arc::clone(&start);
            let acquired = Arc::clone(&acquired);
            let release = Arc::clone(&release);
            thread::spawn(move || {
                start.wait();
                let lease = activity.acquire_transient();
                acquired.wait();
                release.wait();
                lease
            })
        };
        let replay = {
            let start = Arc::clone(&start);
            let acquired = Arc::clone(&acquired);
            let release = Arc::clone(&release);
            thread::spawn(move || {
                start.wait();
                let lease = activity.acquire_replay();
                acquired.wait();
                release.wait();
                lease
            })
        };

        start.wait();
        acquired.wait();
        let snapshot = activity.snapshot();
        assert!(
            (snapshot.transient_calls == 1 && !snapshot.replay_active)
                || (snapshot.transient_calls == 0 && snapshot.replay_active),
            "shared and replay activity overlapped: {snapshot:?}"
        );
        release.wait();

        let shared = shared.join().unwrap();
        let replay = replay.join().unwrap();
        assert_ne!(shared.is_ok(), replay.is_ok());
        drop(shared);
        drop(replay);
        assert_eq!(activity.snapshot(), FoundationActivity::default());
    }

    #[derive(Clone, Copy, Debug)]
    enum SafeWorldlessNativeCall {
        MathHelper,
        GeometryConstructor,
        CollisionHelper,
    }

    impl SafeWorldlessNativeCall {
        fn invoke(self) {
            match self {
                Self::MathHelper => {
                    core::hint::black_box(crate::version().unwrap());
                }
                Self::GeometryConstructor => {
                    core::hint::black_box(crate::shapes::box_polygon(0.5, 0.25).unwrap());
                }
                Self::CollisionHelper => {
                    core::hint::black_box(
                        crate::segment_distance(
                            [0.0_f32, 0.0],
                            [1.0_f32, 0.0],
                            [0.0_f32, 1.0],
                            [1.0_f32, 1.0],
                        )
                        .unwrap(),
                    );
                }
            }
        }
    }

    struct ReleaseBarrier(Arc<Barrier>);

    impl Drop for ReleaseBarrier {
        fn drop(&mut self) {
            self.0.wait();
        }
    }

    fn one_step_recording() -> crate::Recording {
        let mut world = crate::Foundation::initialize_default()
            .unwrap()
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();
        world
            .create_body(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(crate::BodyType::Dynamic)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let mut session = world
            .start_recording(crate::RecordingLimits::default())
            .unwrap();
        drop(session.step(1.0 / 60.0, 1).unwrap());
        let recording = session.finish().unwrap();
        drop(world);
        recording
    }

    fn assert_safe_worldless_call_excludes_replay(
        call: SafeWorldlessNativeCall,
        recording: &crate::Recording,
    ) {
        let acquired = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let worker = {
            let acquired = Arc::clone(&acquired);
            let release = Arc::clone(&release);
            thread::spawn(move || {
                let hook = std::rc::Rc::new(move || {
                    acquired.wait();
                    release.wait();
                });
                let _hook_guard = install_transient_lease_test_hook(hook);
                call.invoke();
            })
        };

        acquired.wait();
        let release = ReleaseBarrier(release);
        let expected_activity = FoundationActivity {
            ordinary_worlds: 0,
            transient_calls: 1,
            replay_active: false,
        };
        assert_eq!(
            Foundation::get().unwrap().activity(),
            expected_activity,
            "{call:?}"
        );

        let error = crate::ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            recording,
            crate::ReplayConfig::default(),
        )
        .expect_err("replay must not overlap a transient safe native call");
        assert_eq!(
            error,
            crate::Error::FoundationActivity(FoundationActivityError::ReplayUnavailable {
                activity: expected_activity,
            }),
            "{call:?}",
        );
        assert_eq!(
            Foundation::get().unwrap().activity(),
            expected_activity,
            "{call:?}"
        );

        drop(release);
        worker.join().unwrap();
        assert_eq!(
            Foundation::get().unwrap().activity(),
            FoundationActivity::default()
        );

        let player = crate::ReplayPlayer::open(
            crate::Foundation::initialize_default().unwrap(),
            recording,
            crate::ReplayConfig::default(),
        )
        .expect("replay must begin after the transient safe native call drains");
        assert!(
            Foundation::get().unwrap().activity().replay_active,
            "{call:?}"
        );
        drop(player);
        assert_eq!(
            Foundation::get().unwrap().activity(),
            FoundationActivity::default()
        );
    }

    #[test]
    fn safe_worldless_native_calls_block_replay_until_transient_leases_drain() {
        let recording = one_step_recording();

        for call in [
            SafeWorldlessNativeCall::MathHelper,
            SafeWorldlessNativeCall::GeometryConstructor,
            SafeWorldlessNativeCall::CollisionHelper,
        ] {
            assert_safe_worldless_call_excludes_replay(call, &recording);
        }
    }

    enum SharedLease {
        Ordinary(OrdinaryWorldLease),
        Transient(TransientFoundationLease),
    }

    impl Drop for SharedLease {
        fn drop(&mut self) {
            match self {
                Self::Ordinary(lease) => {
                    core::hint::black_box(lease);
                }
                Self::Transient(lease) => {
                    core::hint::black_box(lease);
                }
            }
        }
    }

    #[test]
    fn hook_panics_are_contained_and_counted() {
        let foundation = Foundation::new(
            FoundationConfig::default()
                .with_assert_hook(Arc::new(|_, _, _| panic!("assert hook")))
                .with_log_hook(Arc::new(|_| panic!("log hook"))),
        );

        invoke_assert_hook(&foundation, "condition", "file.c", 42);
        invoke_log_hook(&foundation, "message");

        let diagnostics = foundation.diagnostics();
        assert_eq!(diagnostics.assertion_hook_panics, 1);
        assert_eq!(diagnostics.log_hook_panics, 1);
    }

    #[test]
    fn hooks_enter_the_shared_callback_depth() {
        let assertion_was_guarded = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let log_was_guarded = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let assertion_observation = Arc::clone(&assertion_was_guarded);
        let log_observation = Arc::clone(&log_was_guarded);
        let foundation = Foundation::new(
            FoundationConfig::default()
                .with_assert_hook(Arc::new(move |_, _, _| {
                    assertion_observation.store(
                        crate::core::callback_state::check_not_in_callback().is_err(),
                        Ordering::Relaxed,
                    );
                }))
                .with_log_hook(Arc::new(move |_| {
                    log_observation.store(
                        crate::core::callback_state::check_not_in_callback().is_err(),
                        Ordering::Relaxed,
                    );
                })),
        );

        invoke_assert_hook(&foundation, "condition", "file.c", 42);
        invoke_log_hook(&foundation, "message");

        assert!(assertion_was_guarded.load(Ordering::Relaxed));
        assert!(log_was_guarded.load(Ordering::Relaxed));
        assert!(!crate::core::callback_state::in_callback());
    }

    #[test]
    fn recursive_hook_entries_are_suppressed() {
        let foundation = Foundation::new(FoundationConfig::default());
        let nested_assertion_ran = std::cell::Cell::new(false);
        let nested_log_ran = std::cell::Cell::new(false);

        assert_eq!(
            run_assert_trampoline(&foundation, || {
                assert_eq!(
                    run_assert_trampoline(&foundation, || nested_assertion_ran.set(true)),
                    1
                );
            }),
            1
        );
        run_log_trampoline(&foundation, || {
            run_log_trampoline(&foundation, || nested_log_ran.set(true));
        });

        assert!(!nested_assertion_ran.get());
        assert!(!nested_log_ran.get());
        let diagnostics = foundation.diagnostics();
        assert_eq!(diagnostics.assertion_calls, 2);
        assert_eq!(diagnostics.assertion_hook_recursions, 1);
        assert_eq!(diagnostics.log_calls, 2);
        assert_eq!(diagnostics.log_hook_recursions, 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn extern_hook_trampolines_copy_strings_contain_panics_and_remain_reusable() {
        if std::env::var_os(HOOK_TRAMPOLINE_CHILD_ENV).is_some() {
            run_hook_trampoline_child();
            return;
        }

        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "core::foundation::tests::extern_hook_trampolines_copy_strings_contain_panics_and_remain_reusable",
                "--nocapture",
            ])
            .env(HOOK_TRAMPOLINE_CHILD_ENV, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "foundation hook trampoline child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_assert_hook_requests_the_upstream_trap() {
        if std::env::var_os(NATIVE_ASSERT_TRAP_CHILD_ENV).is_some() {
            run_native_assert_trap_child();
            return;
        }

        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "core::foundation::tests::native_assert_hook_requests_the_upstream_trap",
                "--nocapture",
            ])
            .env(NATIVE_ASSERT_TRAP_CHILD_ENV, "1")
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "native assertion unexpectedly returned successfully\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            stderr
        );
        assert!(
            stderr.contains("boxdd-foundation-test: assert-hook-entered"),
            "Box2D did not enter the configured assertion hook\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            stderr
        );
        assert!(
            !stderr.contains("boxdd-foundation-test: after-native-assert"),
            "Box2D continued after the assertion hook requested a trap\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            stderr
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_native_assert_trap_child() {
        let assert_hook: Arc<FoundationAssertHook> = Arc::new(|_, _, _| {
            eprintln!("boxdd-foundation-test: assert-hook-entered");
            panic!("assert-hook panic must be contained before Box2D traps");
        });
        let foundation =
            Foundation::initialize(FoundationConfig::default().with_assert_hook(assert_hook))
                .unwrap();
        let world = foundation
            .create_world(
                crate::Foundation::get()
                    .expect("Foundation must be initialized before constructing a WorldDef")
                    .world_def(),
            )
            .unwrap();

        // SAFETY: the world id is live. The invalid sub-step count intentionally exercises the
        // native B2_ASSERT chain in this process-isolated termination probe.
        unsafe { ffi::b2World_Step(world.raw(), 0.0, 0) };
        eprintln!("boxdd-foundation-test: after-native-assert");
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_hook_trampoline_child() {
        let assertion_calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let assertion_index = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let assert_hook: Arc<FoundationAssertHook> = Arc::new({
            let assertion_calls = Arc::clone(&assertion_calls);
            let assertion_index = Arc::clone(&assertion_index);
            move |condition, file_name, line_number| {
                assertion_calls.lock().unwrap().push((
                    condition.to_owned(),
                    file_name.to_owned(),
                    line_number,
                ));
                if assertion_index.fetch_add(1, Ordering::Relaxed) == 0 {
                    panic!("assert trampoline test panic");
                }
            }
        });

        let log_calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let log_index = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let log_hook: Arc<FoundationLogHook> = Arc::new({
            let log_calls = Arc::clone(&log_calls);
            let log_index = Arc::clone(&log_index);
            move |message| {
                log_calls.lock().unwrap().push(message.to_owned());
                if log_index.fetch_add(1, Ordering::Relaxed) == 0 {
                    panic!("log trampoline test panic");
                }
            }
        });

        let foundation = Foundation::initialize(
            FoundationConfig::default()
                .with_assert_hook(assert_hook)
                .with_log_hook(log_hook),
        )
        .unwrap();

        let condition = CString::new("first assertion").unwrap();
        let file_name = CString::new("foundation-test.c").unwrap();
        // SAFETY: both C strings remain valid for each synchronous trampoline invocation.
        let first = unsafe { assert_trampoline(condition.as_ptr(), file_name.as_ptr(), 73) };
        let second = unsafe { assert_trampoline(condition.as_ptr(), file_name.as_ptr(), 74) };
        assert_eq!(first, 1);
        assert_eq!(second, 1);

        let first_message = CString::new(b"first-\xff".to_vec()).unwrap();
        let second_message = CString::new("second").unwrap();
        // SAFETY: each pointer is null-terminated and valid for its synchronous call.
        unsafe {
            log_trampoline(first_message.as_ptr());
            log_trampoline(second_message.as_ptr());
            log_trampoline(core::ptr::null());
        }

        assert_eq!(
            *assertion_calls.lock().unwrap(),
            [
                (
                    "first assertion".to_owned(),
                    "foundation-test.c".to_owned(),
                    73
                ),
                (
                    "first assertion".to_owned(),
                    "foundation-test.c".to_owned(),
                    74
                ),
            ]
        );
        assert_eq!(
            *log_calls.lock().unwrap(),
            [
                "first-\u{fffd}".to_owned(),
                "second".to_owned(),
                "<null>".to_owned()
            ]
        );

        let diagnostics = foundation.diagnostics();
        assert_eq!(diagnostics.assertion_calls, 2);
        assert_eq!(diagnostics.assertion_hook_panics, 1);
        assert_eq!(diagnostics.log_calls, 3);
        assert_eq!(diagnostics.log_hook_panics, 1);
    }

    #[test]
    fn activity_counter_exhaustion_does_not_wrap() {
        let activity = leaked_activity();
        activity.packed.store(ORDINARY_MASK, Ordering::Release);
        assert_eq!(
            activity.acquire_ordinary().err(),
            Some(FoundationActivityError::OrdinaryWorldCapacityExhausted)
        );

        activity
            .packed
            .store(ORDINARY_MASK << TRANSIENT_SHIFT, Ordering::Release);
        assert_eq!(
            activity.acquire_transient().err(),
            Some(FoundationActivityError::TransientCallCapacityExhausted)
        );
    }
}
