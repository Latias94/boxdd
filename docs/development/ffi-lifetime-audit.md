# FFI Lifetime Audit

This document records the ownership and callback rules for the 0.6 Safe Rust layer over the pinned
Box2D 3.2.0 development snapshot
`56edae79f2949d86142b03450d5d60f63bcf5a6f`. A new wrapper is not complete until its applicable
row has executable evidence.

## Ownership Domains

- `boxdd-sys` is raw FFI and makes no Safe Rust guarantees.
- `WorldCore` and every public world/handle owner are `!Send` and `!Sync`. Arbitrary typed user
  data, callback owners, deferred destruction, native world destruction, and the ordinary-world
  foundation lease stay on the owner thread.
- `WorkerCallbackState` is a separate `Send + Sync` capability. It contains only a world brand, a
  worker-safe identity registry, panic arbitration, and liveness state. It never owns or upgrades
  to `WorldCore`.
- Live object IDs are world-bound registration capabilities. `Raw*Id` values are authenticated,
  process-local surrogates and must be rebound by their original live world.
- `Snapshot` and `RecordingSession` borrow one live owner world. `SnapshotImage` owns compatible
  bytes. `ReplayPlayer` owns its native player/world and holds the process-exclusive foundation
  lease.
- `DynamicTree` is an independent owner-thread native allocation with a transient foundation lease.
  Its proxy IDs contain a process-unique tree token and a per-registration nonce; native integer
  slot reuse cannot rebind a stale or foreign proxy capability.

## Callback Policy

Every C-to-Rust trampoline uses the same policy:

1. Enter callback depth before user code.
2. Run the closure without holding an owner-state registry lock.
3. Catch unwind-capable panics before returning to C.
4. Return a conservative callback-specific sentinel after a panic.
5. Keep only the first panic payload and never drop losing payloads on a callback stack.
6. Leave C, perform allowed deferred cleanup at an owner boundary, then resume the first panic.

With `panic=abort`, a callback panic aborts as configured; the library does not claim recovery.

| Callback class | Execution and access contract |
| --- | --- |
| Custom filter, pre-solve, friction mix, restitution mix | Native only. May run concurrently on Box2D workers. Closure is `Send + Sync + 'static`; inputs are branded IDs and copyable values; no world or typed user-data access. |
| World query, dynamic-tree query/cast, direct debug draw | Native only. Synchronous and closure-scoped on the calling owner thread. The closure cannot escape and callback depth rejects incompatible native reentry. |
| Event view | Synchronous and closure-scoped on the calling owner thread; it borrows memory returned directly to the Rust caller rather than installing a Rust function pointer in the provider. |
| Foundation assert/log hooks | Native only. Process-global and potentially entered from any native thread. Hooks are `Send + Sync`, recursion-suppressed, and panic-contained. Assertions always end in the configured native trap path. |
| Replay mixers and debug draw | Native only. Owned by the replay player and routed through replay liveness/panic state. Read views remain closure-scoped and epoch-bound. |

There is no `CallbackWorld` and no callback-time path to owner-thread world state.

Current WASM adapters have not proven cross-module Rust function-pointer transport. Safe callback
tables are therefore removed with target `cfg` boundaries instead of relying on an unverified ABI.
Compile-fail probes cover every public callback family, while runtime smoke covers the remaining
callback-free world, query, shape, joint, memory, and identity paths.

## Risk Matrix

| Hazard | Public boundary | Guard | Executable evidence |
| --- | --- | --- | --- |
| Native writes beyond a Rust allocation or exposes uninitialized elements | Event, object, contact, sensor, joint, and query output buffers | Shared `MaybeUninit`/capacity helper proves actual capacity, validates returned counts, and sets length only after initialization | `boxdd/src/core/ffi_vec.rs`, `boxdd/tests/buffer_reuse.rs`, sanitizer gates |
| Wrong-world/tree, recycled, wrong-kind, forged, or stale IDs reach C | All safe ID-taking APIs and event/contact/tree outputs | World or tree token plus a registration nonce; native generations, kind authentication, and contact epochs where applicable | `boxdd/tests/world_ownership.rs`, `boxdd/tests/serde_values.rs`, `boxdd/tests/dynamic_tree.rs`, `boxdd/tests/compile_fail.rs` |
| Owner state moves to another thread | `World`, `WorldHandle`, owned handles, snapshots, recording, replay, Bevy context | `Rc`/owner markers and no broad unsafe `Send`/`Sync`; Bevy stores the world as a non-send resource | `boxdd/src/core/world_core.rs` unit assertions, `boxdd/tests/ffi_lifecycle.rs`, `boxdd/tests/ui/owner_state_send.rs`, `bevy_boxdd/tests/plugin.rs` |
| Worker callback owns or reaches owner state | Custom filter, pre-solve, material mixers | `WorkerCallbackState` has no `Rc<WorldCore>`/weak upgrade and only resolves branded IDs through a worker-safe registry | `boxdd/tests/worker_callbacks_multithread.rs`, `boxdd/tests/ui/callback_world_removed.rs` |
| Rust panic crosses C | Every callback trampoline | Shared catch/arbitrate/fallback/resume policy | `boxdd/tests/panic_across_ffi_is_caught.rs`, `boxdd/tests/panic_abort.rs`, `boxdd/tests/worker_callbacks_multithread.rs`, `boxdd/tests/material_mix_callbacks.rs`, `boxdd/tests/dynamic_tree.rs`, `boxdd/tests/replay.rs` |
| World or native owner is reentered during callback | `try_*` world APIs, queries, tree callbacks, debug draw, views | Callback depth plus owner native-call frame; recoverable methods return `ApiError::InCallback` | `boxdd/tests/try_api.rs`, `boxdd/src/world/tests.rs`, `boxdd/src/query/availability_tests.rs` |
| User closure runs while a registry lock is held | Typed user data and callbacks | Lease/borrow state is acquired under lock, closure runs after unlock, and unwind cleanup restores state | `boxdd/tests/user_data.rs`, `boxdd/tests/foundation_world_activity.rs` |
| Typed user data leaks, aliases a restored object, or double-drops | World/object destroy, replacement, snapshot restore, world teardown | Owner-thread erased boxes, registration nonces, transactional restore manifests, isolated cleanup panic aggregation | `boxdd/tests/user_data.rs`, `boxdd/tests/owned_destruction.rs`, `boxdd/tests/snapshot.rs`, `boxdd/tests/foundation_world_activity.rs` |
| Borrowed native memory escapes | Event buffers, debug-draw vertices/strings, replay views/query hits | Higher-ranked closure APIs; owned snapshot/command alternatives; mutation epochs for replay | `boxdd/tests/events_and_sensors.rs`, `boxdd/tests/buffer_reuse.rs`, `boxdd/tests/ui/event_view_escape.rs`, `boxdd/tests/ui/replay_view_escape.rs` |
| Drop calls C while still on a callback stack | World, owned handles, dynamic tree, recording, replay | Boundary cleanup queue; terminalize/defer when an owner frame exists; retain inert resources when no safe boundary exists | `boxdd/src/core/callback_state.rs` tests, `boxdd/tests/owned_destruction.rs`, `boxdd/tests/dynamic_tree.rs`, `boxdd/tests/recording.rs`, `boxdd/tests/replay.rs`, `boxdd/tests/foundation_world_activity.rs` |
| Global foundation state changes during native activity | World creation, worldless helpers, dynamic tree, replay | Frozen configuration plus counted ordinary/transient leases and one exclusive replay lease | `boxdd/tests/foundation_runtime.rs`, `boxdd/tests/foundation_world_activity.rs`, `boxdd/tests/replay.rs` |
| Foundation-independent byte preflight reaches mutable native state | `SnapshotImage::from_bytes` | The repository-source lexical/preprocessor call-closure audit permits only local identity/parser helpers, constant ABI inputs, byte/math library primitives, and `b2IsDoublePrecision`; every other external, macro-hidden, or indirect call fails closed | `boxdd-sys/tests/adapter.rs`, `boxdd-sys/tests/support/c_call_graph.rs` |
| Snapshot rejection partially mutates a world | `World::try_restore`, `SnapshotImage::load` | Full metadata/native preflight before C; prepared host transaction; any post-native failure terminalizes and destroys the world | `boxdd/tests/snapshot.rs` |
| Recording buffer dies while native world still records | `RecordingSession` | RAII session owns the native allocation, exclusively borrows the world, and stops before copying/destroying the buffer | `boxdd/tests/recording.rs` |
| Malformed replay input reaches native parsing or a stale view observes mutation | `ReplayPlayer` | Complete Rust preflight, copied input, exclusive lease, lifecycle state, monotonically advancing view epoch | `boxdd/tests/replay.rs`, `boxdd/tests/ui/replay_view_escape.rs`, `xtask` recording-wire tests |
| Caller-owned pointer is mistaken for Rust ownership | `*_user_data_ptr_raw`, raw task callbacks, unchecked/raw debug seams | Explicit `unsafe` contract and `_raw` naming; Rust never drops caller memory | `boxdd/tests/ffi_lifecycle.rs`, `boxdd/tests/unchecked_api.rs` |

## Event and View Rules

- Owned event methods and `*_into` variants copy values and may outlive the step.
- `with_*_events_view` borrows native event storage only for the closure. Destructive owned-handle
  drops are deferred until the outermost view exits.
- Raw event slices remain unsafe and closure-scoped.
- On native targets, direct debug draw borrows vertex/string memory only during a callback.
  `DebugDrawCmd` collection copies all data. Both callback entry points are compile-time unavailable
  on current WASM adapters; the command value type remains portable.
- Replay views borrow the player for the closure. Callback/lifecycle availability gates reject
  before changing the epoch. Once those gates pass, every mutation attempt advances the epoch,
  including rejected range/policy requests, so a previously observed native view cannot be reused.

## Snapshot, Recording, and Replay

- An in-process `Snapshot` is an unforgeable origin-world capability. In-place restore requires
  matching host callback/mixer wiring and reconciles ID/user-data manifests.
- `SnapshotImage` validates its envelope, checksum, upstream/precision/private ABI identity,
  snapshot layout, and native object graph. It can only create a fresh world with fresh IDs and
  empty host registries.
- The adapter identity and snapshot validator used by `SnapshotImage::from_bytes` are a deliberate
  foundation-lease exception. The runtime identity handshake must succeed before the native
  validator receives any Rust-owned `SnapshotFacts` or `SnapshotEntry` output pointer. The
  authorized validator inspects caller-owned bytes and compile-time ABI constants and may query
  only `b2IsDoublePrecision`. A lexical/preprocessor dependency-closure test rejects length-scale,
  allocator, world/object, mutation, unknown external, macro-hidden, and indirect
  native calls on this path.
- `RecordingSession` exposes only operations that Box2D records. Ordinary world methods remain
  unavailable through the exclusive mutable borrow; custom-filter/pre-solve installation is
  rejected.
- Raw recording bytes require a separately persisted `MixerRequirements` sidecar. Replay installs
  exactly those deterministic mixers before stepping.
- Replay preflight does not set exclusive state for malformed input. A valid player excludes
  ordinary worlds, transient calls, and other players until shutdown restores the previous length
  scale and releases its lease.

These byte formats are ABI-bound transport artifacts, not a cross-version persistence contract.

## Audit Checklist for New Wrappers

- Does native code retain caller memory or a callback context after return?
- Can the call happen during a callback, restore, recording, replay, borrowed view, or terminal state?
- Does the call require a world-bound ID, active contact epoch, definition cookie, normalized value,
  finite coordinate, bounded capacity, or exact object kind?
- Is each coordinate explicitly absolute (`Position`/`WorldTransform`) or local
  (`Vec2`/`Transform`)?
- Can a returned pointer, count, or union field be absent, negative, too large, or uninitialized?
- Can the operation destroy objects or recycle native identities?
- Can a user closure panic or drop arbitrary state while a lock/C frame is active?
- Which foundation lease protects the call, and can replay exclude it?
- Does failure occur before native mutation, or must the owner become terminal afterward?
- Is the provider/runtime capability represented in the structured API contract and executable
  evidence?

Any affirmative answer requires a focused test or an explicit raw/omitted rationale before the API
is classified as safe.
