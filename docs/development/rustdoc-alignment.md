# Rustdoc Alignment

The public safe API should explain Box2D concepts in Rust terms while keeping the upstream source of truth traceable.

## Source hierarchy

1. Vendored headers in `boxdd-sys/third-party/box2d/include/box2d/*.h`
2. Vendored manual pages in `boxdd-sys/third-party/box2d/docs/*.md`
3. Official online manual at <https://box2d.org/documentation/>
4. Local examples and tests that demonstrate Rust-specific ownership or error handling

Header comments are the best source for per-function semantics. Manual pages are the best source for concepts such as units, fixed timesteps, ids, events, sensors, and callbacks.

## Comment policy

- Code comments stay short and explain Rust-side safety or ownership decisions.
- Public rustdoc should call out upstream constraints that would otherwise be hidden behind Box2D asserts.
- Do not copy long upstream prose. Summarize behavior and point to the relevant module or example.
- Prefer `try_*` examples at engine/runtime boundaries and panic-style examples for small quickstarts.

## High-priority modules

- `world`: fixed timestep, callback lock, world ownership, worker-count caveat.
- `body`: body type semantics, stale-id behavior, sleep/awake controls.
- `shapes`: definition copy semantics, sensor flags, shape geometry validation, chain restrictions.
- `events`: event buffer lifetime and snapshot vs zero-copy tradeoffs.
- `query`: filter semantics, callback early exit, buffer reuse.
- `dynamic_tree`: standalone broad phase ownership, proxy lifecycle, callback panic containment.
- `collision` and `shapes::geometry`: standalone distance/cast/manifold helpers, shape-specific casts, and recoverable validation for malformed inputs.
- `joints`: local-frame runtime getters/setters, world id escape hatches, and typed-joint error behavior.
- `bevy_boxdd`: XY/Z-rotation transform mapping, non-send world context, distance/revolute joint descriptor lifecycle, entity query mapping, debug draw collection, recoverable error messages, and contact/sensor message emission requirements.

## Drift checks

Use `docs/api-coverage.md` to decide whether a new upstream symbol needs safe docs, raw-only notes, omission rationale, or deferred tracking.
