# boxdd 0.3 Completeness Matrix

## Legend

- `safe-covered`: the main safe wrapper exposes the normal workflow directly
- `raw-only`: the seam is still public, but explicitly raw or low-level by design
- `intentional omission`: not wrapped in `0.3.0` on purpose
- `candidate after 0.3`: plausible follow-up, but not part of the `0.3.0` release bar

## World

| Slice | Status | Notes |
| --- | --- | --- |
| world creation, destruction, stepping, deferred-destroy flushing | safe-covered | Includes panic-by-default and `try_*` execution entrypoints for callback-sensitive calls. |
| broad-phase queries and reusable-buffer hot paths | safe-covered | `World` and `WorldHandle` now share one internal definition for mirrored query helpers, including owned `Vec`, reusable-buffer `*_into`, and zero-allocation `visit_*` overlap styles. |
| event snapshots, reusable-buffer snapshots, zero-copy views, raw event slices | safe-covered | Intentionally centered on `World`, not `WorldHandle`. |
| debug draw and collected commands | safe-covered | Includes reusable-buffer collection and explicit raw draw path. |
| callback registration and runtime tuning | safe-covered | Includes custom filter, pre-solve, friction, restitution, explosions, counters, profile, and speculative collision control. |
| `b2World_DumpMemoryStats` | intentional omission | Writes a fixed debug file and does not fit the main safe runtime surface. |
| `b2World_RebuildStaticTree` | intentional omission | Upstream labels this as internal testing support. |

## Global Utilities

| Slice | Status | Notes |
| --- | --- | --- |
| deterministic math helpers and value validation | safe-covered | Public helpers cover deterministic `atan2`, cosine/sine, rotation-between-unit-vectors, plus `Vec2` / `Rot` / `Transform` / `Plane` validity checks. |
| runtime version and global length-unit scale | safe-covered | `version()` plus `length_units_per_meter()` / `set_length_units_per_meter(...)` cover the remaining startup-level global Box2D utility surface. |
| byte-count, timing, yield, hash, and scalar validation helpers | safe-covered | `allocated_byte_count()`, `ticks()`, `milliseconds_*`, `yield_now()`, `HASH_INIT`, `hash_bytes(...)`, and `is_valid_float(...)` cover the remaining low-risk foundation helpers without exposing raw `ffi`. |
| global allocator / assert / log callbacks | candidate after 0.3 | These are process-wide hooks with substantially higher design and misuse risk than pure helper functions, so they stay in `boxdd-sys` for now. |

## Body

| Slice | Status | Notes |
| --- | --- | --- |
| creation, builders, value-object inspection, raw conversion symmetry | safe-covered | `BodyDef` is readable and has explicit `from_raw(...)` / `into_raw()`. |
| runtime state, transforms, forces/impulses, sleep/awake/enabled/bullet/name, fast rotation | safe-covered | Owned/scoped/world-id styles are aligned. |
| attached shape/joint enumeration and contact-data extraction | safe-covered | Hot-path enumeration and contact-data reads support reusable caller-owned buffers. |
| raw world-id and raw pointer user-data seams | raw-only | Kept explicitly as `*_raw` escape hatches. |

## Shape

| Slice | Status | Notes |
| --- | --- | --- |
| crate-owned geometry values and shape create/edit/get/set helpers | safe-covered | Main API no longer leaks raw `b2Circle` / `b2Polygon` style values, and polygon construction covers square/box/rounded/offset/hull-based helpers. |
| runtime AABB, point tests, ray casts, mass data, sensor/contact event toggles | safe-covered | Owned/scoped/world-id styles are aligned. |
| sensor overlaps, contact data, and reusable-buffer variants | safe-covered | Covers the hot-path story that originally motivated `0.3.0`. |
| raw geometry conversions, raw world-id, raw pointer user-data seams | raw-only | Explicit `from_raw(...)` / `into_raw()` and `*_raw` APIs stay available. |

## Chain

| Slice | Status | Notes |
| --- | --- | --- |
| creation, materials, filters, points, material-layout inspection | safe-covered | `ChainDef` is now a readable value object. |
| segment extraction and reusable-buffer variants | safe-covered | Owned/scoped handle paths share one internal implementation. |
| raw world-id seam | raw-only | Kept explicitly as `world_id_raw`. |

## Joint Common Surface

| Slice | Status | Notes |
| --- | --- | --- |
| creation builders and readable joint defs/base config | safe-covered | `JointBase` and concrete joint defs are no longer write-only shells. |
| common runtime metadata and controls | safe-covered | Joint type, body ids, frames, tuning, wake helpers, thresholds, and user data are covered across owned/scoped/world-id styles. |
| raw world-id and raw pointer user-data seams | raw-only | Kept explicitly named for low-level interop. |

## Typed Joint Families

| Slice | Status | Notes |
| --- | --- | --- |
| distance, filter, motor, prismatic, revolute, weld, wheel families | safe-covered | Includes type-specific runtime state/control and `ApiError::InvalidJointType` on wrong-family `try_*` calls. |
| broader future joint-family additions if upstream expands | candidate after 0.3 | Current wrapper is complete for the joint families exposed today. |

## Contact

| Slice | Status | Notes |
| --- | --- | --- |
| `ContactId` validity checks and direct contact-data reads | safe-covered | `ContactIdExt` covers `is_valid`, `data`, `data_raw`, and `try_*` variants. |
| first-class `Contact` handle type | candidate after 0.3 | Not necessary for current upstream surface, which only exposes validation and snapshot fetch by id. |

## WorldHandle

| Slice | Status | Notes |
| --- | --- | --- |
| world-level queries and runtime getters | safe-covered | Includes counters/profile/runtime tuning mirrors where lifecycle semantics stay simple. |
| body-by-id read-only runtime mirrors | safe-covered | Includes transforms, velocities, point/vector conversion helpers, mass data, damping/flags, motion locks, and attached shape/joint enumeration without requiring a mutable `World` borrow. |
| shape-by-id read-only runtime mirrors | safe-covered | Includes material/body lookup, AABB/point/raycast/closest-point helpers, mass data, event-flag reads, and reusable-buffer sensor-overlap enumeration for query-produced `ShapeId` values. |
| joint-by-id read-only common runtime mirrors | safe-covered | Includes type/body lookup, collision/tuning metadata, local frames, thresholds, separations, and constraint force/torque reads for ids returned by body/world enumeration paths. |
| owned event snapshots and reusable-buffer snapshot reads | safe-covered | Mirrors `World` for `*_events`, `*_events_into`, and `try_*` without exposing borrowed/raw buffer lifetimes. |
| event views and raw event slices | intentional omission | These stay on `World` because they are tied to step-local buffers and deferred-destroy flushing semantics. |
| mutation, callback registration, stepping | intentional omission | `WorldHandle` remains a cheap stored-query/read-only helper, not a second mutable world API. |

## Unchecked Feature

| Slice | Status | Notes |
| --- | --- | --- |
| body/shape/joint/chain unchecked extension traits | safe-covered | Feature-gated surface kept, with shared implementations to reduce drift. |
| expanding unchecked to more surfaces | candidate after 0.3 | Only worth doing if it unlocks a real advanced-use workflow rather than duplicating the main safe API. |

## Cross-Cutting Raw Seams

| Seam | Status | Notes |
| --- | --- | --- |
| `from_raw(...)` / `into_raw()` on crate-owned value types | raw-only | Intentional low-level escape hatch; safe vocabulary stays crate-owned, including core math values plus opaque ids such as `BodyId`, `ShapeId`, `JointId`, `ChainId`, and `ContactId`. |
| `debug_draw_raw` | raw-only | Kept for render backends that want direct FFI slices and strings. |
| `with_*_events_raw(...)` | raw-only | Explicit unsafe zero-copy access to Box2D event buffers. |
| `*_user_data_ptr_raw` | raw-only | Kept for pointer-based integration with external ownership systems. |
| `world_id_raw` | raw-only | Kept for ID-style integration and external systems already keyed by Box2D ids. |

## Short Post-0.3 Candidate Backlog

- Continue auditing low-value owned/scoped duplication outside the already-refactored hot paths and joint/runtime internals.
- Revisit whether any intentional raw seams can be removed entirely once real engine-integration feedback arrives.
