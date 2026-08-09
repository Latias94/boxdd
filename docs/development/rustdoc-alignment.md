# Rustdoc Alignment

Public documentation describes the 0.6 Safe Rust contract for the pinned Box2D 3.2.0 development
snapshot `56edae79f2949d86142b03450d5d60f63bcf5a6f`. It must not generalize behavior from another
3.2 revision or imply compatibility from the semantic version alone.

## Source Hierarchy

1. `boxdd-sys/upstream.toml` for the authoritative revision and generated artifact inventory.
2. `boxdd-sys/third-party/box2d/include/box2d/*.h` for the pinned public C contract.
3. `boxdd-sys/third-party/box2d/src/*.h` and the adapter shim for private snapshot/recording/replay
   invariants that the wrapper intentionally enforces behind opaque capabilities.
4. `boxdd-sys/third-party/box2d/docs/*.md` for concepts and units.
5. Local integration/compile-fail tests for Rust ownership, failure, and callback behavior.
6. The official online manual only when it agrees with the pinned checkout.

Header comments are the primary per-function source. Do not copy long upstream prose; summarize the
constraint in Rust terms and link to a local type/module/example.

## Required Vocabulary

- **Absolute world**: `Position`, `WorldTransform`, and `WorldScalar` (`f32` or `f64`).
- **Local/relative**: `Vec2`, `Transform`, rotations, directions, extents, velocities, and offsets;
  always `f32`.
- **Live ID**: opaque, world-bound, registration-bound capability.
- **Object capability**: a `Body`, `Shape`, `Joint`, or `Chain` tied to one mutable owner borrow;
  dropping it releases the borrow and destruction is explicit.
- **Query capability**: read-only access tied to a world or recording-session borrow, with reusable
  buffers for allocation-sensitive paths.
- **Owner thread**: the thread that owns a world, tree, snapshot, recording session, replay player,
  or arbitrary Rust state. A finished opaque `Recording` is a `Send + Sync` value.
- **Worker callback**: `Send + Sync` callback with IDs/values only; never a world context.
- **Snapshot/recording capability**: opaque process-local authority. A snapshot restores only its
  originating world; a recording can cross threads but exposes no native stream bytes.
- **Provider**: explicit vendored/system/prebuilt/WASM adapter whose complete identity is verified;
  never merely a Box2D semantic version.

## Public Item Checklist

Document each applicable item with:

- coordinate frame and scalar width for every geometric input/output;
- owner, borrow duration, invalidation event, and whether the type is `!Send`/`!Sync`;
- callback thread, concurrency, reentry, panic, return-sentinel, and closure-escape rules;
- all validated preconditions that would otherwise reach a Box2D assertion;
- whether failure happens before native mutation or makes the owner terminal;
- ID world/kind/epoch requirements and what destroy/restore/step invalidates;
- allocation behavior for convenience collections, reusable buffers, visitors, and views;
- exact provider/precision/platform limits for ABI-dependent operations;
- the nearest focused example or executable test.

Document the canonical `Result` operation. Do not imply that a parallel panic or `try_*` variant
exists.

## High-Priority Modules

- `core::foundation`: freeze-on-first-use configuration, hook panic containment, activity leases,
  and replay exclusivity.
- `world`: owner-thread lifecycle, zero-time-step behavior, built-in scheduler, capacity/bounds,
  runtime worker changes, contact recycling, callback lock, and terminal states.
- `id` and object capabilities: opaque live IDs, world binding, registration nonce, contact epoch,
  one-time acquisition validation, and explicit destruction.
- `types`: absolute/local coordinate split, checked/lossy narrowing, runtime manifold anchor
  reconstruction, and precision-aware interop.
- `query` and `debug_draw`: explicit origins, absolute outputs, visitor/view duration, reusable
  buffers, and callback panic behavior.
- `collision` and `shapes::geometry`: shape-A local frames, `LocalManifold`, validation, and the
  distinction from runtime solver `Manifold`.
- `dynamic_tree`: independent owner, local broad-phase coordinates, proxy lifecycle, AABB box cast,
  callback control, and replay exclusion.
- `joints`: same-world bodies, absolute builder anchors, local runtime frames, normalized axes,
  ordered limits, family-specific APIs, and joint-event identity.
- `snapshot`, `recording`, and `replay`: origin authority, opaque payload ownership, private
  preflight, host wiring, transactional/terminal failure, mixer identities, exclusive leases,
  epochs, and close.
- `bevy_boxdd`: `BoxddWorldOrigin`, checked conversion/rebase behavior, both transform sync modes,
  absolute query/event/joint data, non-send context, and recoverable plugin errors.

## Removed Semantics

Docs must not reintroduce or recommend:

- arbitrary live-ID `from_raw` construction;
- raw live-ID conversion, bind/unbind, or live-ID serialization;
- `WorldHandle`, owning object handles, or implicit drop-based object destruction;
- parallel panic and `try_*` API families;
- raw `WorldDef` construction or raw task-system callback installation;
- `CallbackWorld` or callback access to owner-thread typed user data;
- the `serialize` feature or registry-built scene format;
- process-global length changes after first safe use;
- implicit query origin, world-space `Vec2`, or absolute `Transform` conversion;
- standalone world-manifold collision helpers where the pinned API returns `LocalManifold`;
- dynamic/name-only/pkg-config provider selection or silent fallback;
- multi-worker WASM, WASI runtime, or WASM Rust-callback support;
- Safe Rust import or export of native snapshot/recording bytes, fresh-world snapshot loading, or
  replay from a bare byte slice;
- a claim that `b2GetVersion()` alone proves ABI compatibility.

## Verification

Run rustdoc for each supported public surface with warnings denied, without combining mutually
exclusive provider/precision features:

```bash
RUSTDOCFLAGS="-D warnings" cargo test -p bevy_boxdd --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p boxdd --no-deps --features double-precision
RUSTDOCFLAGS="-D warnings" cargo doc -p bevy_boxdd --no-deps --features double-precision
```

The Bevy doctest target includes the packaged `bevy_boxdd/MIGRATION.md` guide so its core and Bevy
public API examples compile as part of the documentation gate and on docs.rs.

Also run `cargo run -p xtask -- api-inventory --check` and Pages validation. The API inventory is
an explicit human-reviewed classification of every vendored C function as safe, raw, or omitted.
It is an accounting boundary, not a static proof of Rust behavior; rustc, trybuild, runtime tests,
Miri, sanitizers, and the ABI probe provide the executable evidence.
