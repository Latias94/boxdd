# FFI Lifetime Audit

This document records the lifetime rules that keep the safe wrapper from turning Box2D's C API into unsound Rust APIs.

## Stable principles

- `boxdd-sys` exposes raw bindings and does not promise safety.
- `boxdd` owns safe ids and value wrappers. Raw conversion methods are explicit (`from_raw`, `into_raw`, and `*_raw`).
- `World`, owned handles, and Bevy physics context are `!Send`/`!Sync` unless a future audit proves otherwise.
- Definition objects are copied into Box2D at creation time. Raw pointer-bearing definition constructors stay explicit and unsafe where needed.
- Event buffers produced by Box2D are transient after `World::step`. Safe snapshot methods copy; zero-copy view methods borrow only within a closure.
- Query, dynamic-tree, custom filter, pre-solve, and debug-draw callbacks catch panics before crossing the FFI boundary and resume unwinding after Box2D returns.

## Callback-sensitive APIs

Box2D locks world mutation during callbacks. Safe APIs use callback guards so misuse returns `ApiError::InCallback` on `try_*` paths or panics on convenience paths.

Keep this model for:

- world events and borrowed raw event views
- overlap, ray-cast, shape-cast, and mover query callbacks
- custom filter and pre-solve callbacks
- material mixing callbacks
- dynamic tree query/ray/shape callbacks

## Executable coverage

The lifecycle rules above are covered by default-running tests:

- `world_callbacks.rs` exercises custom filter and pre-solve callbacks without ignored tests.
- `panic_across_ffi_is_caught.rs` covers custom filter, pre-solve, debug draw, raw debug draw, and `try_*` reentry returning `ApiError::InCallback`.
- `material_mix_callbacks.rs` covers material-mixing callback behavior and panic capture.
- `world_and_queries.rs` covers world and handle overlap-query callback panic capture and post-panic world reuse.
- `dynamic_tree.rs` covers query, ray-cast, and shape-cast callback panic capture with post-panic tree reuse.
- `events_and_sensors.rs` covers contact/sensor event views matching owned snapshots, owned snapshots surviving later steps, and owned-handle drops being deferred until event-view closures exit.
- `user_data.rs` covers typed user data through callback contexts plus nested raw event views and deferred destruction.
- `buffer_reuse.rs` covers reusable owned snapshot buffers for body, contact, sensor, and joint events.
- `world_destroy_and_recycle.rs` covers repeated explicit destruction and world recycling as normal, non-ignored tests.

Do not add a lifecycle exemption by marking a test ignored. If a callback or event-buffer test is unstable, replace it with a deterministic setup or document why the behavior must remain manual-only.

## Bevy adapter boundary

`bevy_boxdd` stores `boxdd::World` in `BoxddPhysicsContext` as a non-send Bevy resource. Systems publish plain ids, values, and Bevy entities through messages; they do not move the native world across threads.

## Audit checklist for new wrappers

- Does the upstream function borrow caller memory after return?
- Can it be called while Box2D is inside a callback?
- Does it require a valid id, valid definition cookie, finite numeric input, or non-empty geometry?
- Does it return pointers into Box2D-owned memory?
- Can it destroy objects that invalidate ids stored elsewhere?
- Does it invoke a Rust closure from C?

If any answer is yes, add a focused test before exposing a convenience API.
