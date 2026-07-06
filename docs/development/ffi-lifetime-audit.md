# FFI Lifetime Audit

This document records the lifetime rules that keep the safe wrapper from turning Box2D's C API into unsound Rust APIs. Treat it as a release checklist for new wrappers: if a wrapper touches a row in the matrix, add or extend executable evidence before exposing a convenience API.

## Stable Principles

- `boxdd-sys` exposes raw bindings and does not promise safety.
- `boxdd` owns safe ids and value wrappers. Raw conversion methods are explicit: `from_raw`, `into_raw`, and `*_raw`.
- `World`, `WorldHandle`, owned handles, and `bevy_boxdd::BoxddPhysicsContext` are public `!Send`/`!Sync` surfaces unless a future audit proves otherwise.
- `WorldCore` is an internal `Send`/`Sync` lifetime anchor guarded by mutexes and atomics. That does not make Box2D's public world API thread-safe.
- Definition objects are copied into Box2D at creation time. Raw pointer-bearing definition constructors stay explicit and unsafe where needed.
- Event buffers produced by Box2D are transient after `World::step`. Safe snapshot methods copy; zero-copy view methods borrow only within a closure and defer destructive operations until borrowed views exit.
- Query, dynamic-tree, custom filter, pre-solve, material-mix, and debug-draw callbacks catch panics before crossing C callback boundaries and resume unwinding after Box2D returns to Rust.

## Risk Matrix

| Hazard | Public Boundary | Guard | Executable Evidence | Rule For New Wrappers |
|---|---|---|---|---|
| Rust panic crosses an `extern "C"` callback | World callbacks, material mix callbacks, query visitors, dynamic tree visitors, debug draw | Callback trampolines wrap Rust closures with panic capture and resume unwinding after the FFI call returns | `boxdd/tests/panic_across_ffi_is_caught.rs`, `boxdd/tests/material_mix_callbacks.rs`, `boxdd/tests/world_and_queries.rs`, `boxdd/tests/dynamic_tree.rs` | Any new C-to-Rust callback must capture panics before returning to C and prove post-panic cleanup or reuse behavior. |
| Box2D world is reentered while locked by a callback | `try_*` APIs, convenience APIs, event views, debug draw, query callbacks | `CallbackGuard` tracks callback depth; `try_*` returns `ApiError::InCallback`, convenience paths panic on misuse | `boxdd/tests/try_api.rs`, `boxdd/tests/panic_across_ffi_is_caught.rs`, `boxdd/src/world/tests.rs` | Decide whether the API is callback-safe. If not, add `check_not_in_callback` and a focused `try_*` test. |
| Stale ids reach native code after destroy | Body, shape, joint, chain, and contact id APIs | Safe paths validate ids before native calls; `try_*` returns `ApiError::Invalid*Id`; convenience paths panic instead of permitting UB | `boxdd/tests/handle_validity_panics.rs`, `boxdd/tests/try_api.rs`, `boxdd/tests/world_destroy_and_recycle.rs`, `boxdd/tests/ffi_lifecycle.rs` | Any id-taking API must validate the id before crossing FFI unless it is explicitly under `unchecked`. |
| Typed user data leaks or double-drops | World, body, shape, and joint typed user data | `UserDataStore` owns typed boxes; replacement, clear, explicit destroy, and owned-handle drop remove and drop stored values | `boxdd/tests/user_data.rs`, `boxdd/tests/ffi_lifecycle.rs` | Every new owner/destroy path must clear typed user data before invalidating the native object. |
| Raw user-data pointers imply Rust ownership | `*_user_data_ptr_raw` setters and getters | Raw pointer APIs are unsafe, explicitly named, and clear any previously owned typed data before installing caller memory | `boxdd/tests/user_data.rs`, `boxdd/tests/ffi_lifecycle.rs` | Keep raw pointer APIs unsafe and named with `_raw`; never drop caller-owned memory. |
| Borrowed event buffers outlive Box2D's event storage | `with_*_events_view`, `with_*_events_raw` | Owned snapshots copy; borrowed/raw views are closure-scoped; `borrowed_event_buffers` defers destroys until the outermost view exits | `boxdd/tests/events_and_sensors.rs`, `boxdd/tests/user_data.rs`, `boxdd/tests/buffer_reuse.rs` | APIs returning or exposing native slices must be closure-scoped or copied into owned Rust values. |
| Debug draw callbacks borrow temporary vertex/string memory | `DebugDraw`, `RawDebugDraw`, `DebugDrawCmd` collection | Direct callbacks are closure-scoped and panic-contained; collected commands copy vertices and strings into owned Rust data | `boxdd/tests/panic_across_ffi_is_caught.rs`, `boxdd/tests/buffer_reuse.rs`, `boxdd/tests/debug_draw_colors.rs` | Direct draw callbacks must not store borrowed slices; collected commands must own copied data. |
| Public world/handle types become sendable by accident | `World`, `WorldHandle`, owned handles, Bevy physics context | Public types carry non-send markers or are stored as Bevy non-send resources | `boxdd/tests/ffi_lifecycle.rs`, `bevy_boxdd/tests/plugin.rs` | Do not remove non-send markers without a dedicated Box2D thread-safety audit and replacement tests. |
| Raw unchecked APIs bypass validation | `unchecked` feature | APIs are gated behind `unchecked` and unsafe contracts | `boxdd/tests/unchecked_api.rs`, API coverage fixture rationale | Any unchecked addition must be unsafe, feature-gated, and documented as caller-validated. |

## Callback-Sensitive APIs

Box2D locks world mutation during callbacks. Keep callback guards on:

- world events and borrowed raw event views
- overlap, ray-cast, shape-cast, and mover query callbacks
- custom filter and pre-solve callbacks
- material mixing callbacks
- dynamic tree query, ray-cast, and shape-cast callbacks
- debug draw callbacks

## Executable Coverage

Default-running lifecycle coverage includes:

- `panic_across_ffi_is_caught.rs` for custom filter, pre-solve, debug draw, raw debug draw, reentry errors, and post-callback-panic world reuse.
- `try_api.rs` for callback-lock behavior across body, query, shape, chain, and debug draw APIs.
- `material_mix_callbacks.rs` for material mixing callback behavior and panic capture.
- `world_and_queries.rs` for world and handle overlap-query callback panic capture and post-panic world reuse.
- `dynamic_tree.rs` for query, ray-cast, and shape-cast callback panic capture with post-panic tree reuse.
- `events_and_sensors.rs` for contact/sensor event views matching owned snapshots, owned snapshots surviving later steps, and owned-handle drops being deferred until event-view closures exit.
- `user_data.rs` for typed user data through callback contexts plus nested raw event views and deferred destruction.
- `ffi_lifecycle.rs` for public non-send/non-sync assertions, explicit destroy user-data cleanup, and raw pointer replacement semantics.
- `buffer_reuse.rs` for reusable owned snapshot buffers for body, contact, sensor, joint events, and debug draw commands.
- `world_destroy_and_recycle.rs` for repeated explicit destruction and world recycling as normal, non-ignored tests.

Do not add a lifecycle exemption by marking a test ignored. If a callback or event-buffer test is unstable, replace it with a deterministic setup or document why the behavior must remain manual-only.

## Bevy Adapter Boundary

`bevy_boxdd` stores `boxdd::World` in `BoxddPhysicsContext` as a non-send Bevy resource. Systems publish plain ids, values, and Bevy entities through messages; they do not move the native world across threads.

## Audit Checklist For New Wrappers

- Does the upstream function borrow caller memory after return?
- Can it be called while Box2D is inside a callback?
- Does it require a valid id, valid definition cookie, finite numeric input, or non-empty geometry?
- Does it return pointers into Box2D-owned memory?
- Can it destroy objects that invalidate ids stored elsewhere?
- Does it invoke a Rust closure from C?
- Does it install caller-owned raw memory into Box2D?

If any answer is yes, add a focused test before exposing a convenience API.
