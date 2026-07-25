# Rustdoc Alignment

Public documentation describes the 0.6 Safe Rust contract for the pinned Box2D 3.2.0 development
snapshot `56edae79f2949d86142b03450d5d60f63bcf5a6f`. It must not generalize behavior from another
3.2 revision or imply compatibility from the semantic version alone.

## Source Hierarchy

1. `boxdd-sys/upstream.toml` for the authoritative revision and generated artifact inventory.
2. `boxdd-sys/third-party/box2d/include/box2d/*.h` for the pinned public C contract.
3. `boxdd-sys/third-party/box2d/src/*.h` and the adapter shim for private snapshot/recording/replay
   invariants that the wrapper intentionally exposes.
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
- **Raw ID**: authenticated process-local surrogate issued by `unbind`; not an arbitrary FFI ID or
  persistence key.
- **Owner thread**: the thread that owns a world/tree/snapshot/recording/replay allocation and its
  arbitrary Rust state.
- **Worker callback**: `Send + Sync` callback with IDs/values only; never a world context.
- **Snapshot image/recording stream**: compatibility-bound ABI artifact, not a stable save format.
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
- allocation behavior for `Vec`, `*_into`, visitor, and view variants;
- exact provider/precision/platform limits for ABI-dependent operations;
- the nearest focused example or executable test.

Use `try_*` examples at runtime, editor, plugin, persistence, and untrusted-input boundaries. A
panic-style example is acceptable for compact setup code only when the documented preconditions are
obvious.

## High-Priority Modules

- `core::foundation`: freeze-on-first-use configuration, hook panic containment, activity leases,
  and replay exclusivity.
- `world`: owner-thread lifecycle, zero-time-step behavior, built-in scheduler, capacity/bounds,
  runtime worker changes, contact recycling, callback lock, and terminal states.
- `id` and object handles: live versus raw IDs, world binding, registration nonce, contact epoch,
  explicit destruction, and deferred drop.
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
- `snapshot`, `recording`, and `replay`: origin authority, image/stream compatibility, preflight,
  host wiring, transactional/terminal failure, mixer sidecars, exclusive leases, epochs, and close.
- `bevy_boxdd`: `BoxddWorldOrigin`, checked conversion/rebase behavior, both transform sync modes,
  absolute query/event/joint data, non-send context, and recoverable plugin errors.

## Removed Semantics

Docs must not reintroduce or recommend:

- arbitrary live-ID `from_raw` construction;
- `CallbackWorld` or callback access to owner-thread typed user data;
- the `serialize` feature or registry-built scene format;
- process-global length changes after first safe use;
- implicit query origin, world-space `Vec2`, or absolute `Transform` conversion;
- standalone world-manifold collision helpers where the pinned API returns `LocalManifold`;
- dynamic/name-only/pkg-config provider selection or silent fallback;
- multi-worker WASM, WASI runtime, or WASM Rust-callback support;
- a claim that `b2GetVersion()` alone proves ABI compatibility.

## Verification

Run rustdoc for each supported public surface with warnings denied, without combining mutually
exclusive provider/precision features:

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p boxdd --no-deps --features double-precision
RUSTDOCFLAGS="-D warnings" cargo doc -p bevy_boxdd --no-deps --features double-precision
```

Also run `cargo run -p xtask -- api-coverage --check` and Pages validation. The generated coverage
document determines whether an upstream capability is safe, raw, or omitted; rustdoc must match
that classification and name its executable evidence.
