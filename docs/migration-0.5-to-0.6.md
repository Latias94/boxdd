# Migrating from boxdd 0.5 to 0.6

`boxdd` 0.6 is an intentionally breaking soundness and ABI release. It targets the pinned Box2D
3.2.0 development snapshot at commit
`56edae79f2949d86142b03450d5d60f63bcf5a6f`; it does not support arbitrary Box2D 3.2 builds.
There are no compatibility shims for APIs whose old semantics cannot be made sound.

Upgrade all three workspace crates together:

```toml
[dependencies]
boxdd = "0.6"
boxdd-sys = "0.6"       # only when raw FFI is required
bevy_boxdd = "0.6"      # only for Bevy integration
```

The minimum supported Rust version is 1.95. Select `double-precision` through the top-level crate
that owns the integration so Cargo forwards one consistent precision choice to `boxdd-sys`.

## Migration Checklist

1. Replace raw-shaped live IDs with world-bound IDs and audit persisted IDs.
2. Separate absolute `Position`/`WorldTransform` values from local `Vec2`/`Transform` values.
3. Add an explicit origin to every world query and update hit/debug-draw types.
4. Update standalone collision calls to the shape-A local manifold model.
5. Initialize foundation configuration before any safe Box2D call.
6. Remove world access and typed user-data access from worker callbacks.
7. Replace integer worker configuration with `WorkerCount` and audit target support.
8. Replace the old `serialize` feature with snapshot, recording, or application-owned persistence.
9. Rebuild joint definitions through checked same-world builders.
10. Select and attest one explicit native or WASM provider.
11. Add `BoxddWorldOrigin` to Bevy applications and convert all absolute boundaries explicitly.
12. Replace the removed `cgmath` feature with `glam`, `nalgebra`, or `mint` interop.

## IDs and World Ownership

### 0.5

`BodyId`, `ShapeId`, `JointId`, `ChainId`, and `ContactId` mirrored native fields. Safe code could
construct them from raw structs, persist them, or accidentally pass an ID to another world.

### 0.6

Live IDs are opaque capabilities bound to a Rust world token, native world generation, object
registration nonce, and, for contacts, the current step epoch. They are created by world/handle
APIs and cannot be deserialized or forged in Safe Rust.

For temporary process-local storage:

```rust
# use boxdd::{BodyBuilder, World, WorldDef};
let world = &mut World::new(WorldDef::default())?;
let live = world.create_body_id(BodyBuilder::new().build());
let unbound = live.unbind();
let rebound = world.bind_body_id(unbound)?;
assert_eq!(live, rebound);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Use the corresponding `RawBodyId`, `RawShapeId`, `RawJointId`, `RawChainId`, or `RawContactId`.
`Raw*Id::into_ffi` is an explicit low-level export, but there is no public constructor from an
arbitrary FFI ID. Authenticated raw IDs are process-local and become invalid when their original
registration or world expires; they are not save-file identifiers.

Replace cross-world ID tables with application-owned stable keys mapped to live IDs per world.
After an in-place snapshot restore, use `SnapshotRestore::{body_id,shape_id,joint_id,chain_id}` to
translate IDs captured at snapshot time. A fresh `SnapshotLoad` publishes its newly minted IDs.

## Absolute and Local Coordinates

### Type mapping

| 0.5 use | 0.6 type | Meaning |
| --- | --- | --- |
| `Vec2` used as a body/world position | `Position` | Absolute world coordinate using `WorldScalar`. |
| `Transform` returned for a body | `WorldTransform` | Absolute translation plus `f32` rotation. |
| `Vec2` offset, direction, normal, velocity, extent | `Vec2` | Local/relative `f32` quantity in both precision modes. |
| `Transform` between local shapes/frames | `Transform` | Local `f32` rigid transform. |

`WorldScalar` is `f32` by default and `f64` with `double-precision`. Do not cast a double-precision
world position directly to an engine-local `f32` vector. Use an explicit origin:

```rust
# use boxdd::{Position, Vec2};
let origin = Position::new(10_000_000.0, -2_000_000.0);
let absolute = origin.offset(Vec2::new(4.0, -3.0));
let local = absolute.checked_relative_to(origin)?;
assert_eq!(local, Vec2::new(4.0, -3.0));
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Position::relative_to_lossy` is the named escape hatch for intentional narrowing. Math interop
uses the scalar appropriate to each domain: points/world transforms use `WorldScalar`, while
vectors/local transforms remain `f32`.

The `cgmath` feature and its conversion APIs, including `TransformFromCgmathError`, were removed.
The direct `cgmath 0.18` dependency is unmaintained and affected by RustSec advisories
RUSTSEC-2026-0196 and RUSTSEC-2026-0197. Use the supported `glam`, `nalgebra`, or `mint` feature and
convert at the application boundary when migrating existing `cgmath` values.

## Queries, Casts, and Debug Draw

Every world query now separates a high-precision absolute origin from local `f32` geometry.

```rust
# use boxdd::{Aabb, Position, QueryFilter, Vec2, World, WorldDef};
# let world = World::new(WorldDef::default())?;
let origin = Position::new(1_000_000.0, 0.0);
let local_box = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::splat(10.0));
let hits = world.try_overlap_aabb(origin, local_box, QueryFilter::default())?;
let ray = world.try_cast_ray_closest(origin, Vec2::new(20.0, 0.0), QueryFilter::default())?;
# let _ = (hits, ray);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`cast_ray_closest` and `try_cast_ray_closest` still return `Option<RayResult>`. Use the new
`cast_ray_closest_with_stats` or `try_cast_ray_closest_with_stats` when broad-phase diagnostics
matter. Their `ClosestRayCastResult` retains `node_visits` and `leaf_visits` even when `hit` is
`None`. Bevy users can obtain the same data, with entity mapping, through
`BoxddPhysicsContext::try_cast_ray_closest_entity_with_stats` and
`BoxddClosestRayCastResult`.

Apply the same leading `Position` argument to polygon overlap, shape cast, and mover APIs. Offset
arguments inside those calls remain local `Vec2` values. `RayResult::point`,
`WorldCastOutput::point`, shape closest points, pre-solve points, and debug-draw points are now
absolute `Position` values. Debug-draw polygon transforms are `WorldTransform`.

The standalone dynamic tree remains a local broad phase. Upstream replaced its shape cast with an
AABB box cast: migrate `TreeShapeCastInput`/`shape_cast` to `TreeBoxCastInput`/`box_cast`.

`TreeProxyId` is now an opaque live capability. Obtain it from `DynamicTree::create_proxy` or a
tree traversal callback; the safe `from_raw` and `into_raw` escape hatches were removed. IDs are
bound to one tree and one proxy registration, so passing an ID to another tree returns
`ApiError::WrongTree`, while use after destroy, replacement, or native slot reuse returns
`ApiError::InvalidTreeProxyId` before Box2D is called.

## Body Definitions and Raw Interop

`BodyDef` now owns its optional name instead of retaining a caller-owned C pointer. Configure and
inspect creation-time names and sleep thresholds through `BodyBuilder::{name,try_name,clear_name}`,
`BodyBuilder::sleep_threshold`, `BodyDef::name`, and `BodyDef::sleep_threshold`.

The safe `BodyDef::into_raw() -> boxdd_sys::ffi::b2BodyDef` conversion was removed because an owned
name would dangle as soon as the wrapper was consumed. Keep the replacement guard alive while the
raw definition is borrowed:

```rust
# use boxdd::{BodyBuilder, RawBodyDef};
let definition = BodyBuilder::new()
    .name("player")
    .sleep_threshold(0.05)
    .build();
let raw: RawBodyDef = definition.into_raw_guard();
let ffi_definition: &boxdd_sys::ffi::b2BodyDef = raw.as_raw();
# let _ = ffi_definition;
```

Prefer `RawBodyDef::as_raw`. If an unsafe integration uses `as_ptr`, it must not move or drop the
guard before the pointer is no longer used.

`BodyType::from_raw` and `BodyDef::body_type` now return `Option<BodyType>`. An unsafe raw
definition can carry a discriminant from a newer or incompatible native ABI, so Safe Rust no
longer silently treats every unknown value as `BodyType::Dynamic`. Use `BodyType::try_from(raw)`
when the rejected raw discriminant is useful to the caller.

Runtime enum getters also validate native output instead of assuming that the linked provider is
compatible. `try_body_type`, `try_shape_type`, and `try_joint_type` return the corresponding
`ApiError::InvalidNative*Type { raw }` and poison the world when Box2D returns an unknown
discriminant. Their convenience variants still panic. The raw shape and joint type getters remain
available for diagnostics and do not attempt closed-enum decoding. Replay inspection follows the
same rule through `ReplayBodyView::try_body_type`; an unknown discriminant terminalizes the player,
and later native replay operations return `ReplayError::NativeFailure`.

`ShapeDef::from_raw` is now `unsafe`. A raw shape definition can contain an opaque `userData`
pointer that Box2D retains after creation. Prefer `ShapeDef::builder`; use `unsafe` import only
when the raw definition was initialized by the matching Box2D ABI and its pointer remains valid
for every created shape's complete lifetime.

Low-level snapshot validation now distinguishes adapter authorization from snapshot-content
rejection:

```rust
use boxdd_sys::adapter::{
    SnapshotLimits, SnapshotValidationError, SNAPSHOT_BAD_HEADER, validate_snapshot,
};

# fn use_validation(_: boxdd_sys::adapter::SnapshotValidation) {}
# fn reject_input() {}
# fn reject_provider(_: boxdd_sys::adapter::AdapterIdentityError) {}
# let bytes: &[u8] = &[];
match validate_snapshot(bytes, &SnapshotLimits::default()) {
    Ok(validation) => use_validation(validation),
    Err(SnapshotValidationError::Status(SNAPSHOT_BAD_HEADER)) => reject_input(),
    Err(SnapshotValidationError::AdapterIdentity(error)) => reject_provider(error),
    Err(_) => reject_input(),
}
```

Code that previously compared `validate_snapshot(...).unwrap_err()` directly with a `SNAPSHOT_*`
constant must match `SnapshotValidationError::Status`. The identity gate runs before the native
validator receives any Rust-owned output pointer, including when no `Foundation` exists yet.

## Collision and Manifolds

Standalone collision helpers no longer take two independent world transforms. Supply shape B's
transform relative to shape A:

```rust
# use boxdd::{Transform, collide_polygon_and_circle, shapes};
let manifold = collide_polygon_and_circle(
    shapes::box_polygon(1.0, 0.5),
    shapes::circle([0.0_f32, 0.0], 0.25),
    Transform::from_pos_angle([0.8_f32, 0.0], 0.0),
);
```

These helpers return `LocalManifold`; its normal and points are in shape A's local frame. Convert a
point to the world only when needed with shape A's `WorldTransform::transform_point`.

`Sweep::transform_at` now validates every sweep vector and rotation plus the finite `[0, 1]` time
interval before entering Box2D, and panics on invalid input. Use `Sweep::try_transform_at` when
invalid or externally supplied sweep data must return `ApiError::InvalidArgument` instead.

Runtime contact `Manifold` has different semantics. Its normal is a world direction, while each
`ManifoldPoint` contains `anchor_a` and `anchor_b`, which are `f32` offsets from the corresponding
body center. Reconstruct absolute points with `world_point_a(body_a_center)` or
`world_point_b(body_b_center)`. Code that read an absolute `ManifoldPoint::point` must choose the
body whose anchor it intends to use.

## Foundation Configuration

The safe global `set_length_units_per_meter` function was removed because it could race native
state and invalidate active worlds. Initialize the process once, before any other safe Box2D call:

```rust
use boxdd::{FoundationConfig, initialize_foundation};

initialize_foundation(FoundationConfig::new(100.0))?;
# Ok::<(), boxdd::FoundationInitError>(())
```

The first successful configuration freezes length units and optional assert/log hooks. Repeating
the identical configuration returns the same `Foundation`; a different configuration returns
`FoundationInitError::ConfigurationConflict`. Calling another safe native helper first lazily
freezes the default configuration.

Replay temporarily owns the foundation exclusively. It cannot open while ordinary worlds,
dynamic trees, or worldless native-call leases are active, and those operations cannot begin while
a player is live.

## Callbacks and Lifecycle

`CallbackWorld`, `set_custom_filter_with_ctx`, and `set_pre_solve_with_ctx` were deleted. There is
no replacement that permits a worker callback to access a `World`, `WorldHandle`, owner-thread
typed user data, or destruction queue. Capture immutable/thread-safe application data directly in
the closure, or publish a small message for the owner thread to process after `World::step`.

```rust
# use boxdd::{World, WorldDef};
# use std::sync::Arc;
let mut world = World::new(WorldDef::default())?;
let blocked = Arc::new(std::collections::HashSet::new());
world.try_set_custom_filter({
    let blocked = Arc::clone(&blocked);
    move |a, b| !blocked.contains(&a) && !blocked.contains(&b)
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

On native targets, custom-filter, pre-solve, friction, and restitution callbacks may run
concurrently on Box2D worker threads and require `Send + Sync + 'static`. They receive world-bound
IDs and copyable values. Query, dynamic-tree, event-view, and debug-draw closures are synchronous
and cannot escape their call. All callbacks catch Rust panics before returning to C; the first
panic resumes after native control reaches the owning Rust call.

`World`, `WorldHandle`, owned handles, snapshots, recording sessions, replay players, and arbitrary
typed user data remain owner-thread-only. Remove any unsafe `Send`/`Sync` assumptions in downstream
code. Put the complete world on a dedicated thread and communicate through channels when needed.

## Scheduler, Capacity, and Step Boundaries

Replace integer worker counts with validated values:

```rust
use boxdd::{WorkerCount, World, WorldCapacity, WorldDef};

let workers = WorkerCount::new(4)?;
let capacity = WorldCapacity::new(128, 1024, 32, 256, 2048)?;
let mut world = World::new(
    WorldDef::builder()
        .worker_count(workers)
        .capacity(capacity)
        .build(),
)?;
world.try_set_worker_count(2)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Native builds use the pinned Box2D built-in scheduler when more than one worker is selected.
Current WASM providers accept exactly one worker. A native world created with unsafe raw task
callbacks has a fixed external scheduler contract and rejects safe worker-count changes. Runtime
changes are step-boundary operations; they do not make the owner-thread world sendable. WASM safe
construction rejects raw task callbacks supplied through `WorldDef::from_raw`.

Use `World::bounds`, `maximum_capacity`, and `contact_recycle_distance` for the new runtime
capabilities. Zero-duration `try_step(0.0, sub_steps)` follows the pinned upstream maintenance
semantics; do not assume it is a complete no-op.

## Snapshots, Recording, and Replay

The 0.5 `serialize` feature and wrapper-maintained scene registry were removed.

Choose the replacement by intent:

| Intent | 0.6 API |
| --- | --- |
| Roll back the same live world | `World::snapshot` then `World::try_restore`. |
| Load compatible bytes into a fresh world | `SnapshotImage::from_bytes` then `load`. |
| Capture the operation stream of a controlled simulation | `World::try_start_recording`. |
| Inspect/replay a compatible recording | `ReplayPlayer::open_recording` or `open_bytes`. |
| Long-lived/cross-version game saves | Application-owned schema; rebuild a new world. |

An in-process `Snapshot` is bound to its origin world and can restore typed user data only there.
`SnapshotImage` validates integrity and the complete adapter/snapshot ABI and always creates a new
world with fresh Safe IDs and empty host registries. Images requiring host callbacks or mixers are
rejected for fresh-world loading.

`RecordingSession` borrows the world mutably for its complete lifetime. Use the session's explicit
operations, including queries, and call `finish` to copy an owned stream. Custom filter and
pre-solve wiring is intentionally unavailable during recording. Persist `MixerRequirements`
alongside raw recording bytes and reinstall the same deterministic mixers in `ReplayConfig`.

`ReplayPlayer` copies and preflights bytes before creating native state, then acquires exclusive
foundation access. Its read views are closure-scoped and carry a mutation epoch; `step`, `seek`,
`restart`, and keyframe policy changes invalidate previous views. Treat malformed input,
divergence, end-of-stream, and native failure as distinct outcomes.

Snapshot images and recording streams are not guaranteed to work across upstream commits,
precision modes, private ABI/layout changes, provider identities, or wrapper versions.

## Joint Definitions and Invariants

`JointBaseBuilder` and raw-backed definition constructors were removed. Construct a checked base
from two live IDs in the same world:

```rust
# use boxdd::{BodyBuilder, DistanceJointDef, JointBase, World, WorldDef};
# let mut world = World::new(WorldDef::default())?;
let a = world.create_body_id(BodyBuilder::new().build());
let b = world.create_body_id(BodyBuilder::new().build());
let base = JointBase::new(a, b).with_collide_connected(false);
let def = DistanceJointDef::new(base);
world.try_create_distance_joint_id(&def)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The fluent `world.distance`, `revolute`, `prismatic`, `wheel`, and `weld` builders accept absolute
`Position` anchors and convert them to local frames after validating world identity and finite,
representable offsets. Axes remain normalized local `Vec2` directions. New validation rejects
non-finite tuning, reversed limits, invalid force ranges, negative capacities, and wrong-family
runtime calls before reaching a native assertion.

Degree-only convenience methods removed from prismatic APIs have no replacement because linear
translation and speed are not angular quantities. Supply native units explicitly.

## Provider and Precision Migration

The default remains vendored source. The old `pkg-config`, `BOX2D_LIB_DIR`-only, dynamic-link, and
silent fallback behavior was removed.

For a local system archive, provide all inputs explicitly:

```text
BOXDD_SYS_PROVIDER=system
BOX2D_LIB_DIR=/provider/lib
BOXDD_SYS_SYSTEM_MANIFEST=/provider/manifest.toml
```

The manifest binds the exact static archive, public header, pregenerated binding, upstream SHA,
target, precision, CRT, SIMD, validation flags, private ABI, snapshot layout, and recording
contract. Generate a caller-trusted local manifest with the `boxdd-sys` package helper described in
[`boxdd-sys/README.md`](../boxdd-sys/README.md).

Official prebuilt selection additionally requires `BOXDD_SYS_PREBUILT_MANIFEST` and
`BOXDD_SYS_PREBUILT_BUNDLE`. `boxdd-sys` uses its packaged, digest-pinned Sigstore trusted root by
default. `BOXDD_SYS_PREBUILT_TRUSTED_ROOT` is an optional path override and is accepted only when
its digest matches that crate-owned root exactly. Publisher provenance is a signature over the
canonical provider manifest and must match this repository, workflow name, push trigger, source
commit, release tag, target coordinates, and archive digest. Neither adapter performs network
access or archive extraction.

For WASM, distinguish compile-only checks from runtime support. `wasm32-unknown-unknown` and
`wasm32-wasip1` can be compile-only; only the versioned Emscripten provider is a runtime adapter.
Node qualification covers single and double precision. GitHub Pages is single precision. Current
WASM providers do not prove cross-module Rust function-pointer transport and support one worker.
Consequently, callback-backed world hooks, foundation hooks, world and recording-session query
collectors, dynamic-tree queries/casts, replay mixers/drawing, and direct debug drawing are absent
at compile time on `wasm32`. Callback-free closest-ray and mover casts remain available.

## Bevy Migration

Bevy `Transform` translation is always local to `BoxddWorldOrigin`; Box2D positions are absolute.
Insert the origin before the plugin when it is not zero:

```rust
# use bevy::prelude::*;
# use bevy_boxdd::prelude::*;
# fn configure(app: &mut App) -> Result<(), BoxddWorldOriginError> {
let origin = BoxddWorldOrigin::try_new(boxdd::Position::new(10_000_000.0, 0.0))?;
app.insert_resource(origin)
    .add_plugins(BoxddPhysicsPlugin::default());
# Ok(())
# }
```

Removed implicit `Transform <-> boxdd::Transform` helpers have no absolute-world replacement.
Use `checked_local_transform_to_world` and `checked_apply_world_transform` on the origin resource.
Use `checked_local_to_absolute` for authored query points and world joint anchors, and
`checked_absolute_to_local` for query hits, event points, and debug-draw positions.

`TransformSyncMode::BevyToPhysics` and `PhysicsToBevy` both pass through this bridge. Request a
frame move with `BoxddWorldOrigin::request_rebase`; the plugin stages every affected transform and
commits atomically. An unrepresentable rebase remains pending and pauses physics until it is
replaced, cancelled, or becomes representable.

`JointDescriptor` world anchors are `boxdd::Position`. In double precision, do not derive them by
casting a large Bevy-local `f32` translation; convert through the active origin.

## Removed Without a Safe Replacement

- Arbitrary construction of live IDs from native structs.
- `CallbackWorld` and callback-time access to owner-thread world/user-data state.
- The `serialize` feature's wrapper-owned scene format and registry reconstruction.
- Dynamic/name-only system linking and implicit `pkg-config` discovery.
- `World::enable_speculative` / `try_enable_speculative`, whose upstream meaning no longer exists.
- `RawDebugDraw` convenience entry points; use precision-aware `DebugDraw`, collected owned
  `DebugDrawCmd` values, or accept the full `boxdd-sys` unsafe contract.
- Dynamic-tree shape casts; the pinned upstream operation is an AABB box cast.
- Runtime execution on `wasm32-wasip1`; that target is compile-only.
- The `cgmath` feature and conversion/error APIs; migrate to `glam`, `nalgebra`, or `mint`.

When no replacement is listed, keep the behavior in application-owned code or cross the raw FFI
boundary explicitly after documenting its invariants. Do not recreate the removed behavior as a
Safe Rust shim.
