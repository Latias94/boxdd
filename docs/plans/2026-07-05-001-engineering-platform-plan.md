---
title: Boxdd Engineering Platform Refactor
type: refactor
date: 2026-07-05
plan_id: 2026-07-05-001
product_contract_source: ce-plan-bootstrap
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: code
scope: deep
---

# Boxdd Engineering Platform Refactor

## Intent

`boxdd` already has a broad safe wrapper around the Box2D v3 C API, many runtime
tests, and a rich example catalog. The missing layer is the engineering
platform around the binding: upstream API coverage, ABI/layout checks, official
sample parity, workspace metadata, release hygiene, and a clear path for Bevy
and GitHub Pages without weakening the core crate.

This plan intentionally starts with verifiable boundaries before larger API or
integration work. The first pass should make future fearless refactors safer by
turning upstream drift, layout drift, and sample parity drift into test failures.

## Sources

- `F:\SourceCodes\Rust\boxddd`
  - `docs/api-coverage.md`
  - `boxddd/tests/api_coverage.rs`
  - `boxddd/tests/fixtures/api_coverage_symbols.txt`
  - `boxddd-sys/tests/layout.rs`
  - `docs/upstream-parity/box3d-sample-matrix.md`
  - `xtask/src/main.rs`
  - `bevy_boxddd/`
  - `.github/workflows/*.yml`
- Vendored Box2D headers under `boxdd-sys/third-party/box2d/include/box2d`.
- Vendored Box2D samples under `boxdd-sys/third-party/box2d/samples`.
- Official Box2D documentation: https://box2d.org/documentation/
- External ecosystem references: `bevy_rapier2d`, `avian2d`, `cargo-nextest`,
  `cargo-semver-checks`, and the `cargo xtask` pattern.

## Decisions

1. Keep `boxdd-sys` focused on FFI/build concerns and keep `boxdd` focused on
   safe Rust ergonomics.
2. Add a machine-checked upstream API coverage fixture before changing broad API
   surfaces.
3. Add ABI/layout smoke tests in `boxdd-sys` before relying on generated C
   bindings across more CI targets.
4. Add an `xtask` crate for repository checks that need project knowledge.
5. Treat `bevy_boxdd` and browser Pages as second-stage work gated by the
   coverage/layout/parity baseline.
6. Preserve the existing prebuilt-binaries workflow until release policy is
   explicitly changed.
7. Use breaking changes where they remove stale design or make the wrapper more
   honest, but do not break users just to mirror `boxddd` mechanically.

## Implementation Units

### Unit 1: Workspace And Publishing Baseline

Bring the workspace shape closer to `boxddd` while keeping existing `boxdd`
features intact.

Tasks:

- Move shared manifest values into root `[workspace.package]`.
- Move common dependency versions into `[workspace.dependencies]`.
- Use workspace inheritance in `boxdd/Cargo.toml` and `boxdd-sys/Cargo.toml`.
- Upgrade the workspace resolver to `3`.
- Add missing root `LICENSE-MIT` and `LICENSE-APACHE` files to match the
  published license expression.
- Keep `repo-ref/` ignored.

Verification:

- `cargo metadata --no-deps`
- `cargo check --workspace --all-targets`

### Unit 2: Upstream API Coverage Fixture

Port the `boxddd` coverage model to Box2D's `B2_API` symbols.

Tasks:

- Add `boxdd/tests/api_coverage.rs`.
- Add `boxdd/tests/fixtures/api_coverage_symbols.txt`.
- Add `docs/api-coverage.md`.
- Require every `B2_API` function in vendored headers to appear in the fixture.
- Classify coverage as `safe`, `raw`, `omitted`, or `deferred`.
- Document why process-global hooks, allocator/assert/log callbacks, raw user
  data, and other unsafe seams are not part of the safe API.

Verification:

- `cargo nextest run -p boxdd --test api_coverage`
- Fallback if `cargo-nextest` is unavailable: `cargo test -p boxdd --test api_coverage`

### Unit 3: Sys ABI/Layout Audit

Add a small but meaningful ABI regression guard for the C bindings.

Tasks:

- Add `boxdd-sys/tests/layout.rs`.
- Assert sizes, alignments, and representative field offsets for common public
  C types such as ids, `b2Vec2`, `b2Rot`, `b2Transform`, `b2AABB`,
  `b2WorldDef`, and `b2ShapeDef`.
- Add compile-time symbol smoke checks for representative upstream functions.

Verification:

- `cargo nextest run -p boxdd-sys --test layout`
- Fallback: `cargo test -p boxdd-sys --test layout`

### Unit 4: Xtask Sample Parity

Create the repository automation needed to keep official sample coverage honest.

Tasks:

- Add `xtask/` as a workspace member.
- Implement `cargo run -p xtask -- sample-parity --check`.
- Add `docs/upstream-parity/box2d-sample-matrix.md`.
- Parse official `RegisterSample(...)` calls from vendored Box2D samples.
- Validate that each official sample is represented in the matrix as
  `FaithfulPort`, `TeachingAdaptation`, `TestOnly`, `Deferred`, or
  `UpstreamReference`.

Verification:

- `cargo run -p xtask -- sample-parity --check`

### Unit 5: Development Documentation

Make the new engineering boundaries discoverable for maintainers.

Tasks:

- Add `docs/development/ci.md`.
- Add `docs/development/rustdoc-alignment.md`.
- Add `docs/development/ffi-lifetime-audit.md`.
- Update `README.md` with API coverage, sample parity, and development command
  pointers without turning the README into a second API reference.

Verification:

- `cargo doc -p boxdd --no-deps`

### Unit 6: CI Modernization

Wire the new checks into CI without removing existing prebuilt binary behavior.

Tasks:

- Add API coverage, sys layout, and xtask sample parity checks to CI.
- Prefer `cargo nextest` on the main test path.
- Keep existing feature matrix and wasm checks.
- Keep `.github/workflows/prebuilt-binaries.yml` unless a later release policy
  decision removes prebuilt artifacts.

Verification:

- Local dry-run equivalent commands from Units 1-5.

## Deferred Second Stage

### Bevy Integration

Add a dedicated `bevy_boxdd` crate after the baseline checks pass. It should be a
2D ECS-native integration, not a direct copy of `bevy_boxddd`.

Expected shape:

- `BoxddPhysicsPlugin`
- `BoxddDebugDrawPlugin` behind a feature
- `RigidBody`, `Collider`, `PhysicsMaterial`, velocities, external force and
  impulse components
- XY translation plus Z-rotation transform sync
- Contact/sensor/joint event messages
- 2D examples for falling bodies, sensors, ray casts, character mover, chain
  terrain, joints, and debug draw

Entry gate:

- Units 1-6 pass locally.

### GitHub Pages

Add a static example hub first, then consider browser-hosted demos after the
Bevy crate and wasm story are stable.

Expected shape:

- `docs/pages/index.html`
- example catalog generated or validated by `xtask`
- optional `pages.yml` workflow after the content is generated deterministically

Entry gate:

- `xtask` exists and can validate repository docs.

## Risks

- API coverage fixture generation can be noisy if upstream headers include
  helper APIs that should stay raw-only. The mitigation is explicit fixture
  statuses and documented rationale.
- Bevy integration is large enough to deserve a separate checkpoint; starting it
  before baseline checks would make regressions harder to attribute.
- CI changes can fail on platform-specific tool availability. The first CI pass
  should keep commands simple and avoid deleting existing release workflows.
- Workspace metadata inheritance may affect packaging. `cargo package --list`
  should be run before release work.

## Done Criteria

- Workspace metadata is centralized and the license files match Cargo metadata.
- Every vendored `B2_API` function is accounted for by a checked fixture.
- `boxdd-sys` has a layout smoke test for common ABI-sensitive types.
- `xtask sample-parity --check` validates the official Box2D sample matrix.
- Development docs explain the new checks and how to update them.
- CI invokes the new checks while preserving existing build and prebuilt binary
  behavior.
- Changes are formatted, tested, and committed in reviewable conventional
  commits.
