# FFI Lifetime Audit

This document records the lifetime rules that keep the safe wrapper from turning Box2D's C API into unsound Rust APIs.

## Stable principles

- `boxdd-sys` exposes raw bindings and does not promise safety.
- `boxdd` owns safe ids and value wrappers. Raw conversion methods are explicit (`from_raw`, `into_raw`, and `*_raw`).
- `World`, owned handles, and Bevy physics context are `!Send`/`!Sync` unless a future audit proves otherwise.
- Definition objects are copied into Box2D at creation time. Raw pointer-bearing definition constructors stay explicit and unsafe where needed.
- Event buffers produced by Box2D are transient after `World::step`. Safe snapshot methods copy; zero-copy view methods borrow only within a closure.
- Query and dynamic-tree callbacks catch panics before crossing the FFI boundary and resume unwinding after Box2D returns.

## Callback-sensitive APIs

Box2D locks world mutation during callbacks. Safe APIs use callback guards so misuse returns `ApiError::InCallback` on `try_*` paths or panics on convenience paths.

Keep this model for:

- world events and borrowed raw event views
- overlap, ray-cast, shape-cast, and mover query callbacks
- custom filter and pre-solve callbacks
- material mixing callbacks
- dynamic tree query/ray/shape callbacks

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
