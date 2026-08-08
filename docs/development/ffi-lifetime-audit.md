# FFI Lifetime Audit

This document records the ownership and callback rules for the 0.6 Safe Rust layer over the pinned
Box2D 3.2.0 development snapshot
`56edae79f2949d86142b03450d5d60f63bcf5a6f`. A new wrapper is not complete until its applicable
row has executable evidence.

## Ownership Domains

- `boxdd-sys` is raw FFI and makes no Safe Rust guarantees.
- `WorldCore`, `World`, and every borrow-scoped object/query capability are `!Send` and `!Sync`.
  Arbitrary typed user data, callback owners, explicit destruction, native world destruction, and
  the ordinary-world foundation lease stay on the owner thread.
- `WorkerCallbackState` is a separate `Send + Sync` panic latch. Step-local callback contexts carry
  an immutable shape resolver where needed; neither object owns or upgrades to `WorldCore`.
- Live object IDs are opaque world-bound registration capabilities. Safe Rust exposes no raw-ID
  conversion, bind/unbind, or persistence seam.
- `Snapshot` is an opaque capability bound to one live origin world. `RecordingSession` exclusively
  borrows one live owner world, and `Recording` owns a private process-local stream. `ReplayPlayer`
  owns its native player/world and holds the process-exclusive foundation lease.
- `DynamicTree` is an independent owner-thread native allocation with a transient foundation lease.
  Its proxy IDs contain a process-unique tree token and a per-registration nonce; native integer
  slot reuse cannot rebind a stale or foreign proxy capability.

## Callback Policy

Every C-to-Rust trampoline uses the same policy:

1. Enter callback depth before user code.
2. Run the closure without holding an owner-state registry lock.
3. Catch unwind-capable panics before returning to C.
4. Return a conservative callback-specific sentinel after a panic.
5. Never drop an arbitrary panic payload on a callback stack; queue competing worker payloads.
6. Leave C, join workers where applicable, dispose suppressed payloads behind isolated panic
   boundaries, perform allowed deferred cleanup, then resume the primary panic.

With `panic=abort`, a callback panic aborts as configured; the library does not claim recovery.

| Callback class | Execution and access contract |
| --- | --- |
| Custom filter, pre-solve, friction mix, restitution mix | Native only. May run concurrently on Box2D workers. Closure is `Send + Sync + 'static`; inputs are branded IDs and copyable values; no world or typed user-data access. |
| World query, dynamic-tree query/cast, direct debug draw | Native only. Synchronous and closure-scoped on the calling owner thread. The closure cannot escape and callback depth rejects incompatible native reentry. |
| Post-native query visitor | Ordinary Rust iteration over a fully mapped result batch. No callback guard remains active, so independent safe Box2D owners may be used. |
| Completed-step event view | Owner-thread and borrow-scoped; each family view borrows reusable mapped owner storage and installs no Rust function pointer in the provider. |
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
| Wrong-world/tree, recycled, wrong-kind, or stale IDs reach C | All safe ID-taking APIs and event/contact/tree outputs | World or tree token plus a registration nonce; native generations, kind authentication, and contact epochs where applicable | `boxdd/tests/world_ownership.rs`, `boxdd/tests/dynamic_tree.rs`, `boxdd/tests/compile_fail.rs` |
| Owner state moves to another thread | `World`, borrow-scoped capabilities, snapshots, recording, replay, Bevy context | `Rc`/owner markers and no broad unsafe `Send`/`Sync`; Bevy stores the world as a non-send resource | `boxdd/src/core/world_core.rs` unit assertions, `boxdd/tests/ffi_lifecycle.rs`, `boxdd/tests/ui/owner_state_send.rs`, `bevy_boxdd/tests/plugin.rs` |
| Worker callback owns or reaches owner state | Custom filter, pre-solve, material mixers | Worker contexts contain only the panic latch, immutable step-local resolver, and `Send + Sync` closure; publication requires a generic `Send + Sync` proof | `boxdd/src/core/callback_state.rs`, `boxdd/tests/worker_callbacks_multithread.rs`, `boxdd/tests/ui/callback_world_removed.rs` |
| Rust panic crosses C | Every callback trampoline | Shared catch/arbitrate/fallback/resume policy | `boxdd/tests/panic_across_ffi_is_caught.rs`, `boxdd/tests/panic_abort.rs`, `boxdd/tests/worker_callbacks_multithread.rs`, `boxdd/tests/material_mix_callbacks.rs`, `boxdd/tests/dynamic_tree.rs`, `boxdd/tests/replay.rs` |
| World or native owner is reentered during callback | Canonical world operations, queries, tree callbacks, debug draw, views | Callback depth plus owner native-call frame; fallible operations return `Error::InCallback` | `boxdd/src/world/tests.rs`, `boxdd/src/query/availability_tests.rs`, callback-focused integration tests |
| User closure runs while a registry lock is held | Typed user data and callbacks | Lease/borrow state is acquired under lock, closure runs after unlock, and unwind cleanup restores state | `boxdd/tests/user_data.rs`, `boxdd/tests/foundation_world_activity.rs` |
| Typed user data leaks, aliases a restored object, or double-drops | Explicit object/world destruction, replacement, snapshot restore, world teardown | Owner-thread erased boxes, registration nonces, transactional restore manifests, isolated cleanup panic aggregation | `boxdd/tests/user_data.rs`, `boxdd/tests/snapshot.rs`, `boxdd/tests/foundation_world_activity.rs` |
| Borrowed native or mapped memory escapes | Completed-step event families, debug-draw vertices/strings, replay views/query hits | Owner-borrowed capabilities or higher-ranked closures; owned event/command alternatives; mutation epochs for replay | `boxdd/tests/events_and_sensors.rs`, `boxdd/tests/buffer_reuse.rs`, `boxdd/tests/ui/event_view_escape.rs`, `boxdd/tests/ui/replay_view_escape.rs` |
| Drop or explicit destroy calls C while still on a callback stack | World, object capabilities, dynamic tree, recording, replay | Owner native-call frames plus boundary cleanup; terminalize/defer when an owner frame exists and retain inert resources when no safe boundary exists | `boxdd/src/core/callback_state.rs` tests, `boxdd/tests/dynamic_tree.rs`, `boxdd/tests/recording.rs`, `boxdd/tests/replay.rs`, `boxdd/tests/foundation_world_activity.rs` |
| Global foundation state changes during native activity | World creation, worldless helpers, dynamic tree, replay | Frozen configuration plus counted ordinary/transient leases and one exclusive replay lease | `boxdd/tests/foundation_runtime.rs`, `boxdd/tests/foundation_world_activity.rs`, `boxdd/tests/replay.rs` |
| Private snapshot validation reaches mutable native state | `World::snapshot`, `World::prepare_restore` | The dedicated adapter accepts only immutable bytes and limits, returns caller-owned facts and entries, and has no world handle in its interface | `boxdd-sys/native/boxdd_snapshot_validate.c`, `boxdd-sys/tests/adapter.rs` |
| Snapshot rejection partially mutates a world | `World::prepare_restore`, `World::restore` | Full metadata/native preflight before C; prepared host transaction; any post-native failure terminalizes and destroys the world | `boxdd/tests/snapshot.rs` |
| Recording buffer dies while native world still records | `RecordingSession` | RAII session owns the native allocation, exclusively borrows the world, and stops before copying/destroying the buffer | `boxdd/tests/recording.rs` |
| Invalid native writer output reaches replay parsing or a stale view observes mutation | `RecordingSession::finish`, `ReplayPlayer` | Complete Rust preflight before publishing an opaque recording, copied private input, exclusive lease, lifecycle state, monotonically advancing view epoch | `boxdd/src/replay/preflight.rs`, `boxdd/tests/replay.rs`, `boxdd/tests/ui/replay_view_escape.rs`, `xtask` recording-wire tests |
| Caller-owned pointer is mistaken for Rust ownership | `*_user_data_ptr_raw` | Explicit `_raw` naming; Rust never drops caller memory | `boxdd/tests/ffi_lifecycle.rs` |

## Event and View Rules

- `World::step` returns a `CompletedStep` capability that keeps the owner borrowed until event
  inspection is complete.
- Each requested event family is fetched and mapped into reusable owner storage at most once.
  Family views borrow that storage; `to_owned` detaches data that may outlive the step.
- On native targets, direct debug draw borrows vertex/string memory only during a callback.
  `DebugDrawCmd` collection copies all data. Both callback entry points are compile-time unavailable
  on current WASM adapters; the command value type remains portable.
- Replay views borrow the player for the closure. Argument, callback, lifecycle, and native-health
  gates reject without changing the epoch. Once those gates pass, the epoch advances immediately
  before native mutation, so a post-FFI failure cannot make a previous observation current again.

## Snapshot, Recording, and Replay

- An in-process `Snapshot` is an unforgeable origin-world capability. In-place restore requires
  matching host callback/mixer wiring and reconciles ID/user-data manifests. Its native payload is
  private, immutable, and cannot create a fresh world through Safe Rust.
- The runtime identity handshake must succeed before the native snapshot validator receives any
  Rust-owned `SnapshotFacts` or `SnapshotEntry` output pointer. The validator may inspect only the
  private captured payload and compile-time ABI constants and may query only
  `b2IsDoublePrecision`. Its narrow C interface and focused corruption tests are the review seam;
  repository tooling does not implement a second C parser or call-graph engine.
- `RecordingSession` exposes only operations that Box2D records. Ordinary world methods remain
  unavailable through the exclusive mutable borrow; custom-filter/pre-solve installation is not
  part of the session API.
- `RecordingSession::finish` validates and privately owns the native writer output together with
  material mixer identities. Safe Rust exposes neither byte import nor byte export. Replay installs
  exactly the matching deterministic mixers before stepping.
- Recording validation does not set exclusive replay state. A valid player excludes ordinary
  worlds, transient calls, and other players until shutdown restores the previous length scale and
  releases its lease.

Native snapshot and recording bytes are deliberately outside the Safe Rust contract.

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
- Is the C disposition recorded in `xtask/api-inventory.toml`, and is the corresponding behavior
  covered by an ordinary compiler or runtime test gate?

Any affirmative answer requires a focused test or an explicit raw/omitted rationale before the API
is classified as safe.
