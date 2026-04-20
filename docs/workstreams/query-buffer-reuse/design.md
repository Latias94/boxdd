# Allocation Hotpaths Refactor (0.3.0)

## Context

`boxdd` exposes several hot-path APIs that return freshly allocated `Vec`s:

- `World::overlap_aabb`
- `World::cast_ray_all`
- `World::overlap_polygon_points`
- `World::cast_shape_points`
- Offset query variants on `World` and `WorldHandle`
- `Body::contact_data` / `OwnedBody::contact_data`
- `Shape::contact_data` / `OwnedShape::contact_data`
- `Shape::sensor_overlaps` / `OwnedShape::sensor_overlaps`
- `World::shape_sensor_overlaps`
- `Chain::segments` / `OwnedChain::segments`

These APIs are ergonomic for one-off queries, but they force a fresh allocation strategy on hot paths that typically run every frame.

## Problem Statement

For gameplay code that performs repeated broad-phase or cast queries, returning a new `Vec` on every call creates avoidable allocation churn:

- extra allocator traffic in per-frame loops
- unnecessary pressure on frame-time stability
- no direct way for the caller to retain and reuse capacity

The FFI callback layer already collects results incrementally, so the Rust API can expose a reusable-buffer path without changing the underlying Box2D behavior.

## Goals

- Keep the existing convenience APIs that return owned `Vec`s.
- Add allocation-friendly APIs that reuse caller-owned buffers.
- Make the new API shape consistent across `World` and `WorldHandle`.
- Reduce duplicated callback collection logic inside `query.rs`.
- Keep the refactor source-compatible for existing users.

## Non-Goals

- Remove the existing `Vec`-returning APIs.
- Redesign the closest-hit query APIs that do not allocate.
- Convert every Vec-returning API in the crate during the first milestone.

## API Design

Each hot-path API gains a pair of additive methods:

- `foo(...) -> Vec<T>`
- `foo_into(..., out: &mut Vec<T>)`
- `try_foo(...) -> ApiResult<Vec<T>>`
- `try_foo_into(..., out: &mut Vec<T>) -> ApiResult<()>`

`*_into` semantics:

- `out` is always cleared before results are appended.
- existing capacity is preserved and reused.
- panic forwarding and callback safety rules remain identical to the existing APIs.

## Internal Design

Phase 1 refactors `boxdd/src/query.rs` around shared collection helpers:

- a shared callback collection context for `ShapeId` and `RayResult`
- dedicated `*_into_impl` helpers used by both owned-returning and reusable-buffer APIs
- `SmallVec<[b2Vec2; 8]>` for temporary proxy points because Box2D proxy vertices are capped at `B2_MAX_POLYGON_VERTICES`

Phase 2 extends the same strategy to body/shape/chain/world extraction APIs:

- a shared `core::ffi_vec` helper for FFI-backed `Vec` filling
- `*_into` APIs for contact data, sensor overlaps, and chain segments
- in-place valid filtering for sensor overlap buffers

This keeps behavior aligned while reducing duplication across owned/scoped/id-style handles.

## Compatibility

This is an additive API change. Existing code using `Vec`-returning query APIs continues to compile unchanged.

## Follow-Up Candidates

The same pattern likely belongs on other allocation-heavy APIs:

- sensor overlap reads
- contact data extraction
- chain segment collection
- debug draw command collection

Remaining follow-up candidates after 0.3:

- debug draw command collection
- serialize helpers that still repeat raw `Vec` fill patterns
- broader `World` / `WorldHandle` duplication cleanup
