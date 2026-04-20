# boxdd 0.3 Fearless Refactor

## Context

`boxdd 0.3.0` is intentionally larger than a narrow bug-fix release.

The first delivered slice already improved one of the crate's biggest practical gaps:
allocation-heavy hot-path APIs now expose reusable-buffer `*_into` variants.

That work solved a real gameplay problem reported by users, but it also highlighted a
broader issue: the crate is already a strong safe wrapper, yet some parts of the
official Box2D v3 surface are still exposed unevenly.

In particular:

- hot-path query APIs were previously ergonomic but allocation-hostile
- character mover / geometric controller APIs were only partially wrapped
- some callback and query surfaces are already strongly typed, while adjacent ones still
  require raw FFI knowledge
- `World` / `WorldHandle` and owned/scoped handle layers still contain repeated logic

`0.3.0` is the release where we deliberately clean this up instead of preserving every
legacy shape out of inertia.

## Problem Statement

The current crate is good enough for many projects, but it is not yet as complete or as
coherent as it should be for a flagship safe Box2D binding.

The main gaps are:

- gameplay hot paths still need auditing beyond the initial query buffer reuse work
- debug draw command collection still needs the same reusable-buffer treatment as the main query APIs
- the character mover flow is incomplete unless users drop to `boxdd_sys::ffi`
- some safe APIs still duplicate the same implementation patterns across handle styles
- the crate lacks a clearer release-level refactor plan that ties these efforts together

If we do not address these now, the likely outcome is a sequence of small additive
patches that preserve avoidable duplication and keep advanced users half inside the safe
API and half inside raw FFI.

## Goals

- Make `0.3.0` a coherent safe-wrapper upgrade, not only an allocation patch.
- Productize the Box2D character mover flow as an ergonomic safe API.
- Keep hot-path APIs friendly to per-frame reuse patterns.
- Identify and remove redundant implementation patterns where the maintenance cost is not justified.
- Track larger follow-up refactors explicitly so the crate does not drift back toward ad-hoc growth.

## Non-Goals

- Preserve every existing API shape if a clearer breaking refactor is materially better.
- Wrap every last upstream helper in `0.3.0`.
- Introduce abstraction layers that reduce clarity just to avoid some local duplication.

## Design Principles

### 1. Safe First, Not Thin First

The crate should expose upstream concepts faithfully, but not at the expense of ergonomic
or allocation-hostile APIs. A good safe wrapper should encode the common usage pattern,
not merely rename C functions.

### 2. Hot Paths Must Be Reusable

If an API is plausibly called every frame, the safe surface should not force fresh heap
allocation. Convenience methods can remain, but reusable-buffer variants should exist when
the call pattern justifies them.

### 3. Complete Flows Beat Isolated Entry Points

Wrapping `b2World_CastMover` alone is not enough. The useful product surface is the
character-mover flow:

1. cast the mover
2. collect contact planes
3. solve planes
4. clip velocity

Users should not need raw FFI to finish the workflow.

### 4. Duplication Must Earn Its Keep

Some duplication is acceptable for API clarity. Repeated callback plumbing, raw `Vec`
fill patterns, or near-identical handle implementations should be consolidated when the
result is simpler and easier to audit.

## Release Scope

### Delivered in the first `0.3.0` slice

- reusable-buffer query APIs
- reusable-buffer contact / sensor / chain extraction APIs
- reusable-buffer debug draw command collection
- shared FFI `Vec` fill helpers
- hot-path docs, tests, examples, version bump, changelog updates

### Targeted next in this workstream

- character mover collision-plane collection APIs
- safe plane / collision-plane / solver result types
- `solve_planes` and `clip_vector` safe entry points
- docs and examples that show the intended mover flow
- typed friction / restitution callbacks
- standalone collision geometry helpers for distance, shape cast, TOI, and AABB validation/ray cast
- crate-owned wrapper cleanup for remaining leaked Box2D value types (`ShapeType`, `MassData`, contact data, and manifolds)

### Planned follow-up audit items

- unify standalone collision geometry helpers with shape-construction helper types
- `World` / `WorldHandle` duplication review
- owned / scoped handle duplication review outside the hottest paths
- continue value-type cleanup for remaining raw Box2D structs that still leak through public APIs

## Release Strategy

`0.3.0` may include breaking API cleanup where it materially improves the crate.

The bar is:

- the new surface is more coherent than the old one
- the change removes legacy shape or duplication instead of adding another layer beside it
- the result is easier to explain, test, and maintain

This workstream is the umbrella record for that effort.
