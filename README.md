<div align="center">

# boxdd 0.6 - Rust bindings for Box2D v3

[![Crates.io](https://img.shields.io/crates/v/boxdd.svg?style=flat-square)](https://crates.io/crates/boxdd)
[![Docs](https://docs.rs/boxdd/badge.svg)](https://docs.rs/boxdd)
[![Live Examples](https://img.shields.io/badge/examples-Bevy%20WASM-2dd4bf?style=flat-square)](https://frankorz.com/boxdd/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square)](#license)

![boxdd](https://raw.githubusercontent.com/Latias94/boxdd/main/screenshots/boxdd.gif)

</div>

`boxdd` 0.6 is the breaking, soundness-focused binding for the pinned Box2D 3.2.0
development snapshot at commit
`56edae79f2949d86142b03450d5d60f63bcf5a6f`. It is not a binding for an arbitrary
Box2D 3.2 checkout. The source revision, precision, private ABI, snapshot layout, recording
format, target, and provider identity are qualified together.

Read the [0.5 to 0.6 migration guide](docs/migration-0.5-to-0.6.md) before upgrading.

## Crates

- `boxdd-sys`: raw FFI, the pinned source tree, generated bindings, and provider identity checks.
- `boxdd`: owner-thread Safe Rust APIs for worlds, objects, queries, callbacks, snapshots,
  recordings, and replay.
- `bevy_boxdd`: Bevy 0.19 ECS integration with an explicit local-to-world origin bridge.

## 0.6 Highlights

- Live `BodyId`, `ShapeId`, `JointId`, `ChainId`, and `ContactId` values are bound to one Rust
  world registration. Cross-world, stale, recycled, wrong-kind, and forged identifiers are
  rejected before native mutation.
- Absolute coordinates use `Position` and `WorldTransform`; local offsets, directions, extents,
  and rotations use `Vec2`, `Transform`, and `f32`. The `double-precision` feature changes
  `WorldScalar` and the native ABI together.
- World queries take an explicit absolute `Position` origin. Standalone collision helpers return
  `LocalManifold`; runtime contact manifolds retain local `f32` anchors with explicit world-point
  reconstruction.
- `initialize_foundation` freezes process-global length units and hooks before the first safe
  native use. Ordinary worlds, worldless native calls, and exclusive replay share one activity
  protocol.
- Native targets can use Box2D's built-in scheduler through validated `WorkerCount` values.
  `World` and its handles remain `!Send` and `!Sync`; worker callbacks receive only thread-safe
  identifiers and values, never an owning world context.
- `Snapshot`, `RecordingSession`, and `ReplayPlayer` encode distinct ownership models for
  in-place restore, external image loading, operation recording, and exclusive replay.
- The conformance contract accounts for exactly 478 exported functions plus ABI-bearing fields
  and callback capabilities using resolvable Rust and behavioral evidence paths.
- Vendored source is the default provider. System and prebuilt native providers are static-only,
  exact-manifest adapters; WASM runtime support uses a versioned Emscripten provider.

## Quick Start

```rust
use boxdd::{BodyBuilder, BodyType, Position, ShapeDef, Vec2, World, WorldDef, shapes};

let mut world = World::new(
    WorldDef::builder()
        .gravity(Vec2::new(0.0, -9.8))
        .build(),
)?;
let body = world.create_body_id(
    BodyBuilder::new()
        .body_type(BodyType::Dynamic)
        .position(Position::new(0.0, 2.0))
        .build(),
);
world.create_polygon_shape_for(
    body,
    &ShapeDef::builder().density(1.0).build(),
    &shapes::box_polygon(0.5, 0.5),
);
world.try_step(1.0 / 60.0, 4)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The panic-style methods are convenient for controlled setup code. Prefer `try_*` methods at
runtime, editor, plugin, and data-loading boundaries.

## Spatial Model

- `WorldScalar` is `f32` by default and `f64` with `double-precision`.
- `Position` and `WorldTransform` represent absolute world coordinates.
- `Vec2` and `Transform` always remain local `f32` values.
- Convert an absolute point to a local frame with `Position::checked_relative_to`. The explicit
  `relative_to_lossy` method is available only when lossy narrowing is intentional.
- Query AABBs, polygons, and cast geometry remain local to the explicit query origin. Ray hit and
  debug draw positions are absolute `Position` values.
- `bevy_boxdd::BoxddWorldOrigin` maps Bevy-local `f32` transforms to absolute Box2D positions and
  performs origin rebases atomically.

## Ownership and Callbacks

`World`, `WorldHandle`, owned handles, snapshots, recording sessions, and replay players are
owner-thread objects. A dedicated physics thread plus channels is the supported way to integrate
with a multi-threaded or async application.

On native targets, worker-capable callbacks (`set_custom_filter`, `set_pre_solve`, friction mixing,
and restitution mixing) require `Send + Sync + 'static` closures and must not call world APIs.
Query, dynamic-tree, event-view, and debug-draw callbacks are closure-scoped. Every C-to-Rust
callback contains panics; the first panic resumes only after native control returns to a Rust-owned
boundary.

WASM adapters currently do not prove cross-module Rust function-pointer transport. Callback-backed
world, query, dynamic-tree, foundation, replay, and debug-draw APIs are therefore absent at compile
time on `wasm32`; callback-free queries such as closest ray casts and mover casts remain available.

Live IDs may be temporarily converted to authenticated process-local `Raw*Id` values with
`unbind`, then rebound through `World::bind_*_id`. These values are not portable persistence IDs.
Use `boxdd-sys` directly when an application deliberately accepts the raw FFI contract.

## Foundation and Scheduling

Configure global length units before any other safe Box2D call:

```rust
use boxdd::{FoundationConfig, WorkerCount, World, WorldDef, initialize_foundation};

initialize_foundation(FoundationConfig::new(1.0))?;
let workers = WorkerCount::new(4)?;
let world = World::new(WorldDef::builder().worker_count(workers).build())?;
# drop(world);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Initialization is idempotent only for the same configuration. A conflicting configuration is an
error. Native targets qualify Box2D's built-in scheduler; current WASM adapters accept exactly one
worker. Raw task callbacks remain an explicitly unsafe native-only alternative and fix the world's
scheduler contract at creation time. Safe WASM world construction rejects raw task or material
callback pointers supplied through `WorldDef::from_raw`.

## Persistence and Replay

- `World::snapshot` returns an unforgeable capability for restoring the same world. Successful
  restore returns a `SnapshotRestore` mapping. Registrations in the unchanged snapshot/current
  identity intersection preserve their Safe IDs; destroyed, replaced, or post-snapshot objects are
  invalidated and remapped as needed. Rejection before the native restore call leaves the world
  live; a failure after that call makes the world terminal.
- `SnapshotImage` is an integrity-checked byte envelope that can only create a fresh world.
  External images are accepted only when the complete adapter and snapshot ABI identity matches.
- `RecordingSession` owns the native recording allocation and is the only world access surface
  while recording. `finish` copies the stream into an owned `Recording`.
- `ReplayPlayer` preflights the complete stream, copies its input, holds exclusive process-global
  foundation access, and exposes closure-scoped, epoch-bound read views. Drop or `close` restores
  the previous global state.

Snapshot images and native recording streams are compatibility-bound artifacts, not stable save
formats. Persist application-level state separately when long-term or cross-build compatibility is
required. See `boxdd/examples/persistence.rs` for the happy path.

## Providers and Compatibility

| Provider | Selection | Contract |
| --- | --- | --- |
| Vendored source | default | Builds the pinned source inventory with matching generated bindings. |
| Local system | `BOXDD_SYS_PROVIDER=system` | Caller-supplied static archive, header, bindings, and exact local attestation manifest. |
| Official prebuilt | `BOXDD_SYS_PROVIDER=prebuilt` | Exact static manifest plus a signed whole-package provenance statement and Sigstore bundle. |
| WASM compile-only | `BOXDD_SYS_PROVIDER=wasm-compile-only` | Type/build qualification only; no runtime claim. |
| WASM runtime | `BOXDD_SYS_PROVIDER=wasm-provider` | Versioned precision-specific imports backed by the pinned Emscripten 6.0.3 provider. |

System and prebuilt adapters never download, extract, cache, discover by name, dynamically link, or
fall back to vendored source. Official prebuilt qualification authenticates the canonical
provenance statement and exact outer archive before extraction; `boxdd-sys` then re-verifies the
already-local complete member inventory and manifest before linking exact bytes. A provider
reporting only `b2GetVersion() == 3.2.0` is insufficient. Single and double precision artifacts,
manifests, bindings, and dependent crate features cannot be mixed.

See [`boxdd-sys/README.md`](boxdd-sys/README.md) for manifest inputs and
[`docs/platforms/wasm.md`](docs/platforms/wasm.md) for the WASM runtime boundary.

## Cargo Features

- `double-precision`: use `f64` absolute world coordinates and the matching Box2D ABI.
- `serde`: serialize safe value/configuration types and authenticated process-local raw IDs. It
  does not serialize a `World`.
- `mint`, `nalgebra`, `glam`: scalar-correct math interop. World-space conversions use
  `WorldScalar`; local vector conversions remain `f32`.
- `bytemuck`: `Pod`/`Zeroable` for layout-qualified value types.
- `simd-avx2`, `disable-simd`, `validate`: forward an explicit native provider identity choice.
- `unchecked`: additional unsafe APIs for caller-proven hot paths.

There is no `serialize` feature in 0.6. Snapshot, recording, and replay APIs are available through
the normal safe crate surface.

## Development

```bash
git submodule update --init --recursive
cargo fmt --all -- --check
cargo nextest run -p boxdd -p boxdd-sys
cargo nextest run -p boxdd -p boxdd-sys --features boxdd/double-precision
cargo nextest run -p bevy_boxdd
cargo check -p boxdd --examples
cargo check -p boxdd --examples --features double-precision
cargo run -p xtask -- upstream-sync --check
cargo run -p xtask -- api-coverage --check
```

The repository pins Rust 1.95 as MSRV and Rust 1.97 as its development toolchain. Provider,
package, sanitizer, Miri, WASM, Pages, and release gates are exposed through `xtask` and CI.

## Examples

- [`boxdd/examples/README.md`](boxdd/examples/README.md) groups the headless core examples by
  workflow. Start with `world_basics`, `foundation_scheduler`, `queries`, and `persistence`.
- [`bevy_boxdd/README.md`](bevy_boxdd/README.md) documents the ECS adapter and explicit
  `BoxddWorldOrigin` bridge.
- <https://frankorz.com/boxdd/> hosts the generated single-precision Bevy + egui Pages testbed.

## Documentation

- [0.5 to 0.6 migration](docs/migration-0.5-to-0.6.md)
- [FFI lifetime audit](docs/development/ffi-lifetime-audit.md)
- [Rustdoc alignment](docs/development/rustdoc-alignment.md)
- [WASM status](docs/platforms/wasm.md)
- [Changelog](CHANGELOG.md)
- [docs.rs](https://docs.rs/boxdd)

## Acknowledgments

- Thanks to the Rust Box2D bindings project for prior art and inspiration:
  <https://github.com/Bastacyclop/rust_box2d>
- Box2D is maintained by Erin Catto: <https://github.com/erincatto/box2d>

## License

`boxdd` and `boxdd-sys` are licensed under MIT OR Apache-2.0. The pinned upstream Box2D source is
MIT-licensed.
