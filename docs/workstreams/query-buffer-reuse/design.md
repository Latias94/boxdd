# Allocation Hotpaths Refactor (0.3.0)

## Context

`boxdd` exposes several hot-path APIs that return freshly allocated `Vec`s:

- `Query::overlap_aabb`
- `Query::cast_ray_all`
- `Query::overlap_shape`
- `Query::cast_shape`
- `Body::contact_data`
- `Shape::contact_data`
- `Shape::sensor_overlaps`
- `Chain::segments`

These APIs are ergonomic for one-off queries, but they force a fresh allocation strategy on hot paths that typically run every frame.

## Problem Statement

For gameplay code that performs repeated broad-phase or cast queries, returning a new `Vec` on every call creates avoidable allocation churn:

- extra allocator traffic in per-frame loops
- unnecessary pressure on frame-time stability
- no direct way for the caller to retain and reuse capacity

The FFI callback layer already collects results incrementally, so the Rust API can expose a reusable-buffer path without changing the underlying Box2D behavior.

## Goals

- Keep convenience APIs that return owned `Vec`s.
- Add allocation-friendly APIs that reuse caller-owned buffers.
- Keep one capability-based query surface.
- Reduce duplicated callback collection logic inside `query`.

## Non-Goals

- Remove the existing `Vec`-returning APIs.
- Redesign the closest-hit query APIs that do not allocate.
- Convert every Vec-returning API in the crate during the first milestone.

## API Design

Each hot-path API has one owned and one reusable-buffer method:

- `foo(...) -> Vec<T>`
- `foo_into(..., buffer: &mut Buffer<T>) -> Result<()>`

`*_into` semantics:

- `out` is always cleared before results are appended.
- existing capacity is preserved and reused.
- panic forwarding and callback safety rules remain identical to the existing APIs.

## Internal Design

Phase 1 refactors `boxdd/src/query` around shared collection helpers:

- a shared callback collection context for `ShapeId` and `RayResult`
- dedicated buffers used by both owned-returning and reusable-buffer APIs
- one validated `ShapeProxy` input for overlap and cast geometry

Phase 2 extends the same strategy to body/shape/chain/world extraction APIs:

- a shared `core::ffi_vec` helper for FFI-backed `Vec` filling
- `*_into` APIs for contact data, sensor overlaps, and chain segments
- in-place valid filtering for sensor overlap buffers

This keeps behavior aligned while reducing duplication across capability methods.

## Compatibility

The 0.6 capability refactor is intentionally breaking. It keeps the owned-returning convenience
methods while removing duplicate panic/`try_*`, handle, point-list, and offset-query matrices.
Callers construct translated or rotated query geometry with `ShapeProxy::offset_from_points`.

## Follow-Up Candidates

The same pattern likely belongs on other allocation-heavy APIs:

- sensor overlap reads
- contact data extraction
- chain segment collection
- debug draw command collection

Remaining follow-up candidates after 0.3:

- debug draw command collection
- serialize helpers that still repeat raw `Vec` fill patterns
- additional buffer-backed capability methods where profiling shows allocation pressure
