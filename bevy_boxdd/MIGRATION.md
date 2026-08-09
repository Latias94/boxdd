# Migrating from boxdd 0.5 to 0.6

This guide ships with `bevy_boxdd` so rustdoc can compile both the core `boxdd` and Bevy examples
against the released crates.

`boxdd` 0.6 is an intentionally breaking soundness and ABI release. It targets the pinned Box2D
3.2.0 development snapshot at commit
`56edae79f2949d86142b03450d5d60f63bcf5a6f`; it does not support arbitrary Box2D 3.2 builds.
There are no compatibility shims for ownership or raw-interop APIs that could not be made sound.

Upgrade the workspace crates together:

```toml
[dependencies]
boxdd = "0.6"
boxdd-sys = "0.6"       # only when raw FFI is required
bevy_boxdd = "0.6"      # only for Bevy integration
```

The minimum supported Rust version is 1.95. Select `double-precision` through the top-level crate
that owns the integration so Cargo forwards one consistent precision choice to `boxdd-sys`.

The public error surface is also unified:

| 0.5 | 0.6 |
| --- | --- |
| `ApiError` | `Error` |
| `ApiResult<T>` | `Result<T>` |
| `ApiError::InvalidJointType` | `Error::WrongJointType { expected, actual }` |
| `ApiError::InvalidArgument` | `Error::InvalidArgument { operation, argument, constraint }` |
| `ApiError::IndexOutOfRange` | `Error::IndexOutOfRange { operation, index, bound }` |

Remove `try_*`/panic-style branching and propagate the canonical `Result` returned by each
operation. Other retained variants keep their names under `Error`; `BoxddPluginError::Api` keeps
its name and now contains `Error`.

## Migration Checklist

1. Replace `WorldHandle` and `Owned*` handles with one `World`, stored IDs, and borrow-scoped
   capabilities.
2. Replace both panic-style and `try_*` calls with the canonical `Result` operation.
3. Remove `Raw*Id`, bind/unbind, and raw live-ID storage. Introduce application-owned stable keys
   where identity must outlive a world.
4. Separate absolute `Position`/`WorldTransform` values from local `Vec2`/`Transform` values.
5. Acquire `Query` from the owner and add an explicit absolute origin to every world query.
6. Consume events from the `CompletedStep` returned by `World::step`.
7. Initialize an explicit `Foundation`, derive scale-aware defaults from it, create worlds through
   it, and replace integer workers with `WorkerCount`.
8. Remove world and typed-user-data access from worker callbacks.
9. Replace the old `serialize` feature with same-world snapshots or an application-owned durable
   schema.
10. Rebuild joint definitions through checked same-world builders.
11. Select and attest one explicit native or WASM provider.
12. Add `BoxddWorldOrigin` to Bevy applications and convert absolute boundaries explicitly.

## One World and Borrow-Scoped Capabilities

0.5 exposed several overlapping ownership surfaces: `World`, a copyable/read-only `WorldHandle`,
RAII `OwnedBody`/`OwnedShape`/`OwnedJoint`/`OwnedChain`, scoped handles, and direct ID operations.
Their destruction, aliasing, and failure behavior was difficult to compose consistently.

0.6 has one owner and two forms of access:

- `World` owns the native simulation and every Rust-side registry or callback resource.
- `BodyId`, `ShapeId`, `JointId`, and `ChainId` are copyable world-bound identifiers for
  application storage.
- `Body<'_>`, `Shape<'_>`, `Joint<'_>`, and `Chain<'_>` are capabilities tied to a mutable borrow of
  the owner. Dropping one only releases the borrow. Call `destroy` explicitly when destruction is
  intended.
- `Query<'_>` is a read-only capability tied to a world or recording-session borrow.

Creation now returns an ID directly. Acquire a capability for object operations:

```rust
use boxdd::{BodyBuilder, BodyType, Foundation, ShapeDef, shapes};

let foundation = Foundation::initialize_default()?;
let mut world = foundation.create_world(foundation.world_def())?;
let body_id = world.create_body(
    BodyBuilder::from(foundation.body_def())
        .body_type(BodyType::Dynamic)
        .position([0.0, 2.0])
        .build()?,
)?;

let shape_id = world.body(body_id)?.create_polygon(
    &ShapeDef::builder().density(1.0).build()?,
    &shapes::box_polygon(0.5, 0.5)?,
)?;

world.shape(shape_id)?.set_friction(0.4)?;
world.body(body_id)?.destroy()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

A capability validates the ID at acquisition and excludes overlapping mutable owner access for
its lifetime. Reacquire it after returning to application code rather than storing a self-
referential world/handle graph.

## One Fallible API

The panic-style/`try_*` split was removed. Every public operation that can reject invalid input,
stale identity, callback reentry, unavailable owner state, or native failure returns `Result` under
its direct name:

| 0.5 pattern | 0.6 replacement |
| --- | --- |
| `world.step(...)` / `world.try_step(...)` | `world.step(...)?` |
| `world.create_body_id(...)` / `try_create_body_id(...)` | `world.create_body(...)?` |
| `world.try_overlap_aabb(...)` | `world.query()?.overlap_aabb(...)?` |
| `body.try_set_awake(...)` | `body.set_awake(...)?` |
| `world.try_create_distance_joint_id(...)` | `world.create_distance_joint(...)?` |

Definitions and geometry values still expose explicit `validate` helpers when preflight is useful,
but calling an operation always performs its required validation. Do not restore a local
panic-wrapper API at untrusted, editor, plugin, or data-loading boundaries.

## IDs and Application Identity

Safe live IDs are opaque capabilities bound to a world token, native world generation, object
registration nonce, and, for contacts, the current contact epoch. They cannot be constructed from
raw Box2D structs, detached with `unbind`, rebound to a world, or serialized.

Replace `RawBodyId`, `RawShapeId`, `RawJointId`, `RawChainId`, `RawContactId`, and bind/unbind tables
with application-owned identity:

```text
save/entity key  ->  current BodyId for this World
```

The application key is durable; the `BodyId` is only the live registration currently implementing
that key. Destroying an object, destroying the world, or restoring a snapshot can invalidate the
live ID.

After a same-world restore, use
`SnapshotRestore::{body_id, shape_id, joint_id, chain_id}` to translate IDs captured at snapshot
time. Post-snapshot objects have no restored mapping.

Inspect a `ContactId` through `World::contact_is_valid` or `World::contact_data`. Contact IDs are
step-epoch capabilities and should not be retained as application identity.

Use `boxdd-sys` directly if an integration genuinely needs an FFI `b2*Id`. Crossing that boundary
means accepting Box2D's complete lifetime, world, generation, callback, and thread contract.

## Absolute and Local Coordinates

| 0.5 use | 0.6 type | Meaning |
| --- | --- | --- |
| `Vec2` used as a body/world position | `Position` | Absolute coordinate using `WorldScalar`. |
| `Transform` returned for a body | `WorldTransform` | Absolute translation plus `f32` rotation. |
| Offset, direction, normal, velocity, extent | `Vec2` | Local/relative `f32` quantity. |
| Transform between local shapes/frames | `Transform` | Local `f32` rigid transform. |

`WorldScalar` is `f32` by default and `f64` with `double-precision`. Narrow large coordinates only
after subtracting an explicit origin:

```rust
use boxdd::{Position, Vec2};

let origin = Position::new(10_000_000.0, -2_000_000.0);
let absolute = origin.offset(Vec2::new(4.0, -3.0));
let local = absolute.checked_relative_to(origin)?;
assert_eq!(local, Vec2::new(4.0, -3.0));
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Position::relative_to_lossy` is the named escape hatch for intentional narrowing. The removed
`cgmath` feature has no compatibility layer; use `glam`, `nalgebra`, or `mint` at the application
boundary.

## Queries and Reusable Buffers

Acquire one query capability and express geometry relative to an explicit absolute origin:

```rust
use boxdd::{Aabb, Foundation, Position, QueryFilter, ShapeQueryBuffer, Vec2};

let foundation = Foundation::initialize_default()?;
let world = foundation.create_world(foundation.world_def())?;
let query = world.query()?;
let origin = Position::new(1_000_000.0, 0.0);
let local_box = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(10.0, 10.0))?;

let mut hits = ShapeQueryBuffer::new();
query.overlap_aabb_into(origin, local_box, QueryFilter::default(), &mut hits)?;
let ray = query.cast_ray_closest(
    origin,
    Vec2::new(20.0, 0.0),
    QueryFilter::default(),
)?;
# let _ = (hits, ray);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`cast_ray_closest_with_stats` retains broad-phase node and leaf visits even when no hit is found.
The same capability owns overlap, all-hit ray, shape-cast, mover-cast, and mover-plane operations;
their local geometry stays `f32` while returned world points use `Position`.

The standalone `DynamicTree` remains a local-coordinate broad phase. Its `TreeProxyId` is likewise
opaque and registration-bound. Upstream's old tree shape cast is now an AABB box cast through
`TreeBoxCastInput` and `box_cast`.

## Completed Steps and Events

`World::step` no longer leaves event retrieval as unrelated world calls. It returns a
`CompletedStep<'_>` capability:

```rust
use boxdd::{Foundation, StepEventsSnapshot};

let foundation = Foundation::initialize_default()?;
let mut world = foundation.create_world(foundation.world_def())?;
let completed = world.step(1.0 / 60.0, 4)?;
let contacts = completed.contact_events()?;
for event in contacts.begin() {
    let _ = (event.shape_a, event.shape_b, event.contact_id);
}
let retained: StepEventsSnapshot = completed.to_owned()?;
# let _ = retained;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Each event family is fetched, copied, and mapped only when requested and at most once per completed
step. Borrowed family views keep reusable owner storage borrowed; `to_owned` creates data that may
outlive the next world mutation.

## World Definitions and Scheduling

`WorldDef` is now a Rust-owned Safe configuration value. `WorldDef::from_raw`, raw task callbacks,
raw material callback pointers, and `WorldBuilder::task_system_raw` were removed. Configure the
qualified built-in scheduler with `WorkerCount`:

```rust
use boxdd::{Foundation, WorkerCount, WorldBuilder, WorldCapacity};

let foundation = Foundation::initialize_default()?;
let workers = WorkerCount::new(4)?;
let capacity = WorldCapacity::new(128, 1024, 32, 256, 2048)?;
let mut world = foundation.create_world(
    WorldBuilder::from(foundation.world_def())
        .worker_count(workers)
        .capacity(capacity)
        .build()?,
)?;
world.set_worker_count(WorkerCount::new(2)?)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Native builds use the pinned built-in scheduler. Current WASM providers accept exactly one worker.
Runtime worker changes happen at an owner-thread step boundary and do not make the world sendable.
Applications that need a custom native task system must own it through `boxdd-sys`, outside the
Safe wrapper.

Initialize process-global foundation configuration before other safe Box2D use:

```rust
use boxdd::{Foundation, FoundationConfig};

let foundation = Foundation::initialize(FoundationConfig::new(100.0))?;
# let _ = foundation;
# Ok::<(), boxdd::Error>(())
```

The first successful configuration freezes the process contract. Repeating the same configuration
is idempotent; a conflict is an error. Obtain scale-aware world and body defaults from that same
root through `world_def` and `body_def`. Obtain a joint base from the active `World` or
`RecordingSession` so the owner authenticates both body IDs. Worldless native helpers and
`DynamicTree` also require the Foundation to have been initialized first.

## Callbacks and Owner Threads

`CallbackWorld`, callback context APIs, and callback-time access to owner-thread world/user-data
state were removed. Capture immutable thread-safe application data or publish a message for the
owner thread:

```rust
use boxdd::{Foundation, ShapeId};
use std::sync::Arc;

let foundation = Foundation::initialize_default()?;
let mut world = foundation.create_world(foundation.world_def())?;
let blocked = Arc::new(std::collections::HashSet::<ShapeId>::new());
world.set_custom_filter({
    let blocked = Arc::clone(&blocked);
    move |a, b| !blocked.contains(&a) && !blocked.contains(&b)
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

On native targets, custom-filter, pre-solve, friction, and restitution callbacks may run
concurrently on Box2D worker threads and require `Send + Sync + 'static`. They receive branded IDs
and copyable values, never a `World`. Query, dynamic-tree, event-view, and debug-draw closures are
synchronous and cannot escape their borrow.

Material mixers now require an application-defined stable behavior identity:

| 0.5 | 0.6 |
| --- | --- |
| `set_friction_callback(f)` / `try_set_friction_callback(f)` | `set_friction_callback(MixerId::from_bytes(id), f)?` |
| `set_restitution_callback(f)` / `try_set_restitution_callback(f)` | `set_restitution_callback(MixerId::from_bytes(id), f)?` |
| `clear_friction_callback()` / `try_clear_friction_callback()` | `clear_friction_callback()?` |
| `clear_restitution_callback()` / `try_clear_restitution_callback()` | `clear_restitution_callback()?` |

Change the identity whenever the callback behavior or its input data changes. Recordings capture
these identities and replay configuration must provide the same values. Equality authenticates the
caller-declared behavior version; it does not hash or inspect closure code.

`World`, borrow-scoped capabilities, snapshots, recording sessions, and replay players remain
owner-thread-only. Put the complete world on a dedicated thread and communicate through channels
when an application otherwise needs multi-threaded or async access.

## Snapshots, Recording, and Durable State

The 0.5 `serialize` feature and wrapper-maintained scene registry were removed. Choose the 0.6
mechanism by intent:

| Intent | 0.6 mechanism |
| --- | --- |
| Roll back the same live world | `World::snapshot`, then `World::restore`. |
| Capture a controlled operation stream | `World::start_recording`, then `RecordingSession::finish`. |
| Inspect/replay that process-local recording | `ReplayPlayer::open(foundation, &recording, config)`. |
| Long-lived or cross-build saves | Application-owned versioned schema; rebuild a world. |

An in-process `Snapshot` is an unforgeable capability bound to its exact origin world. Its private
native payload cannot be imported, exported, or used for fresh-world loading through Safe Rust.
Restore preflights host wiring and identity state before native mutation; a failure after native
restore begins terminalizes the owner rather than exposing partial state.

`RecordingSession` exclusively borrows its world and exposes only recordable operations. It cannot
install custom-filter or pre-solve callbacks. `finish` validates the writer output and produces an
opaque `Recording` carrying the required material-mixer identities.

The new `RecordingLimits` value is a hard total-stream limit in `1..=256 MiB`; its default is the
256 MiB repository safety ceiling. `MixerIdentities` carries exact `MixerId` values captured in the
opaque recording. External recording bytes and sidecars have no Safe Rust import or export route.

`ReplayPlayer::open(foundation, &recording, config)` accepts only an opaque `Recording`, copies its
private stream, and acquires exclusive process-global access through the explicit Foundation root.
`step`, `seek`, `restart`, and keyframe-policy changes invalidate prior epoch-bound views. Safe Rust
exposes no snapshot/recording bytes, native stream lengths, public native parser errors, or
replay-from-bytes entry point.

## Joint Definitions and Typed Access

Raw-backed joint definitions and `JointBaseBuilder` were removed. Build a checked definition from
two live same-world body IDs:

```rust
use boxdd::{BodyBuilder, DistanceJointDef, Foundation};

let foundation = Foundation::initialize_default()?;
let mut world = foundation.create_world(foundation.world_def())?;
let a = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
let b = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
let def = DistanceJointDef::new(
    world
        .joint_base(a, b)?
        .with_collide_connected(false),
);
let joint_id = world.create_distance_joint(&def)?;

let mut joint = world.joint(joint_id)?.into_distance()?;
joint.set_length(2.0)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The fluent `world.distance`, `revolute`, `prismatic`, `wheel`, and `weld` builders accept absolute
`Position` anchors and convert them to local frames after validating world identity and
representable offsets. Joint acquisition authenticates and caches the family once; typed
conversion reports `Error::WrongJointType` without another native type query.

## Raw Definition Interop Was Removed

Safe definitions are pure Rust values. Public `from_raw`, `into_raw`, `into_raw_guard`, and
`RawBodyDef`-style lowering APIs were removed because pointer-bearing native definitions cannot be
carried through later Safe Rust calls without coupling their validity to hidden backing storage.
Construct and validate definitions through the explicit Foundation instead:

```rust
use boxdd::{BodyBuilder, BodyType, Foundation};

let foundation = Foundation::initialize_default()?;
let definition = BodyBuilder::from(foundation.body_def())
    .name("player")?
    .body_type(BodyType::Dynamic)
    .build()?;
definition.validate()?;
# let _ = definition;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The Safe wrapper lowers these values privately for the duration of its native call. An integration
that must retain arbitrary native pointers must construct the corresponding `boxdd-sys` definition
and own its complete unsafe lifetime contract; there is no Safe-definition-to-raw bridge.

## Providers, Precision, and WASM

Vendored source remains the default. The old `pkg-config`, dynamic/name-only linking,
`BOX2D_LIB_DIR`-only selection, and silent fallback behavior were removed.

System and prebuilt providers are static, exact-manifest adapters. Their manifests bind the pinned
source SHA, target, precision, CRT/SIMD/validation flags, generated bindings, private ABI, snapshot
layout, recording contract, and exact archive/header digests. Official prebuilt packages
add signed whole-package provenance. See [`boxdd-sys/README.md`](../boxdd-sys/README.md) for the
current environment variables and qualification commands.

For WASM, distinguish compile-only targets from runtime support:

- `wasm32-wasip1` is compile-only.
- The versioned Emscripten package is the qualified runtime adapter.
- Current providers support one worker and do not prove cross-module Rust callback transport.
- Callback-backed query, dynamic-tree, foundation-hook, replay-draw, and debug-draw entry points
  are absent on `wasm32`; callback-free closest-ray and mover casts remain available through
  `Query`.

The Emscripten SDK is repository release/qualification tooling. `boxdd-sys` consumes a checked-in
provider ABI contract and never downloads, discovers, or executes an SDK during its build.

## Bevy Migration

Bevy `Transform` translation is local to `BoxddWorldOrigin`; Box2D positions are absolute. Insert
the origin before the plugin when the simulation starts far from zero:

```rust
# use bevy::prelude::*;
# use bevy_boxdd::prelude::*;
# fn configure(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
let foundation = boxdd::Foundation::initialize_default()?;
let origin = BoxddWorldOrigin::new(boxdd::Position::new(10_000_000.0, 0.0))?;
app.insert_resource(origin)
    .add_plugins(BoxddPhysicsPlugin::new(
        foundation,
        BoxddPhysicsSettings::default(),
    ));
# Ok(())
# }
```

Use `checked_local_to_absolute` for authored query points and world joint anchors. Use
`checked_absolute_to_local` for query hits, event points, and debug-draw positions. Both
`TransformSyncMode` directions pass through this bridge.

`BoxddWorldOrigin::request_rebase` stages all affected transforms and commits atomically. An
unrepresentable request leaves the active origin unchanged and pauses physics until the request is
replaced, cancelled, or becomes representable. Removing or replacing the resource after plugin
initialization also pauses the physics pipeline; restore the committed resource state instead of
bypassing the rebase transaction.

The 0.5 public `bevy_boxdd::systems` module and its individual system functions are private in 0.6.
Order application systems around the stable `BoxddPhysicsSet` variants instead of invoking or
composing plugin internals. This preserves the plugin's validate, rebase, restore, reconcile,
cleanup, creation, step, and writeback invariants across future internal refactors.

| 0.5 public systems | 0.6 ordering set |
| --- | --- |
| `cleanup_removed_*` | `BoxddPhysicsSet::Cleanup` |
| `create_missing_bodies` | `BoxddPhysicsSet::CreateBodies` |
| `apply_body_settings`, `sync_bevy_transforms_to_boxdd` | `BoxddPhysicsSet::PrepareBodies` |
| `create_missing_shapes`, `create_missing_joints`, `apply_body_controls` | `BoxddPhysicsSet::PrepareConstraints` |
| `step_world`, `publish_physics_messages` | `BoxddPhysicsSet::Step` |
| `sync_boxdd_transforms_to_bevy` | `BoxddPhysicsSet::Writeback` |

`Validate`, `Rebase`, `Restore`, and `Reconcile` are new internal pipeline stages exposed only as
stable ordering points.

`BoxddBody`, `BoxddShape`, and `BoxddJoint` are now read-only projections of the context's private
identity graph. Their tuple fields are private and the components are neither `Copy` nor `Clone`.
Read the authenticated native identifier through `.id()`; do not copy a marker to represent
ownership on another entity. Moving one through ordinary ECS operations does not transfer native
ownership: the reconcile phase removes the stale projection and restores the authoritative one.

`BoxddPhysicsContext::world_mut` was removed. Use the context's checked operations for gravity,
queries, snapshots, and other supported native changes, and express body, collider, joint, and
control changes through ECS authoring components. Code that genuinely needs unrestricted Box2D
mutation must own a separate `boxdd::World`; it cannot bypass the plugin's identity graph while the
plugin owns the native world.

## Removed Without a Safe Replacement

- `WorldHandle`, `OwnedBody`, `OwnedShape`, `OwnedJoint`, and `OwnedChain`.
- The parallel panic and `try_*` API families.
- `Raw*Id`, bind/unbind, and raw live-object getters.
- Raw `WorldDef` construction and raw task-system callback installation.
- The public unchecked wrapper feature/module.
- `CallbackWorld` and callback-time owner-world/user-data access.
- The `serialize` feature's wrapper-owned scene format and registry reconstruction.
- Native snapshot/recording byte import, export, fresh-world load, and replay-from-bytes.
- Dynamic/name-only provider linking and implicit `pkg-config` discovery.
- Dynamic-tree shape casts; the pinned operation is an AABB box cast.
- Runtime execution on `wasm32-wasip1`.
- The `cgmath` feature and conversion/error APIs.

When no replacement is listed, keep the behavior in application-owned code or cross the raw FFI
boundary explicitly after documenting its invariants. Do not recreate removed semantics as a Safe
Rust shim.
