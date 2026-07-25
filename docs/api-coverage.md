# Box2D API Coverage

<!-- api-coverage: total=478 safe=456 raw=20 omitted=2 deferred=0 -->

This file is generated from the API artifact named by `boxdd-sys/upstream.toml`. The contract is validated against the exact vendored headers, canonical public Rust paths, real `#[test]` evidence, provider modes, precision-specific link symbols, explicit recording capability classes, and ABI struct/callback fingerprints.

Pinned active upstream: `56edae79f2949d86142b03450d5d60f63bcf5a6f`.

## Runtime Evidence Policy

Policy: `ufcs-straight-line-v1`. Runtime witnesses must be executable `nextest` tests and use straight-line, unambiguous UFCS calls to a unique Safe inherent callable, or an unambiguous Safe free function. Receiver-method syntax does not count as coverage. Standard `Result`/`Option` `unwrap`, `expect`, and continuation through `?` are accepted only when their standard wrapper provenance is proven. Macros, unknown attributes, external modules, ambiguous imports or traits, and non-linear control flow fail closed. Explicit `drop` proves RAII only for a directly owned, unwrapped public value. `ReplayPlayer::with_view` additionally requires a proven must-invoke inline closure and successful result consumption. Route aggregation never substitutes for running every declared Cargo/nextest target.

## Summary

| Status | Count |
|---|---:|
| `safe` | 456 |
| `raw` | 20 |
| `omitted` | 2 |
| `deferred` | 0 |
| Total | 478 |

## By Area

| Area | Safe | Raw | Omitted | Deferred | Total |
|---|---:|---:|---:|---:|---:|
| Body | 56 | 0 | 0 | 0 | 56 |
| Body center of mass | 2 | 0 | 0 | 0 | 2 |
| Body contact recycling | 2 | 0 | 0 | 0 | 2 |
| Body coordinate conversion | 2 | 0 | 0 | 0 | 2 |
| Body force mutation | 1 | 0 | 0 | 0 | 1 |
| Body identity validation | 1 | 0 | 0 | 0 | 1 |
| Body impulse mutation | 1 | 0 | 0 | 0 | 1 |
| Body transform mutation | 2 | 0 | 0 | 0 | 2 |
| Body transform query | 2 | 0 | 0 | 0 | 2 |
| Body velocity query | 1 | 0 | 0 | 0 | 1 |
| Borrowed replay body view | 1 | 0 | 0 | 0 | 1 |
| Borrowed replay query hit | 1 | 0 | 0 | 0 | 1 |
| Borrowed replay query view | 1 | 0 | 0 | 0 | 1 |
| Borrowed replay view | 2 | 0 | 0 | 0 | 2 |
| Chain | 9 | 0 | 0 | 0 | 9 |
| Chain collision | 3 | 0 | 0 | 0 | 3 |
| Chain identity validation | 1 | 0 | 0 | 0 | 1 |
| Chain segment geometry mutation | 1 | 0 | 0 | 0 | 1 |
| Chain segment shape creation | 1 | 0 | 0 | 0 | 1 |
| Collision | 16 | 0 | 0 | 0 | 16 |
| Collision bounds | 4 | 0 | 0 | 0 | 4 |
| Completed-step event capture | 4 | 0 | 0 | 0 | 4 |
| Contact identity validation | 1 | 0 | 0 | 0 | 1 |
| Contact snapshots | 1 | 0 | 0 | 0 | 1 |
| Debug drawing | 1 | 0 | 0 | 0 | 1 |
| Dynamic tree box casting | 1 | 0 | 0 | 0 | 1 |
| Dynamic tree lifecycle | 1 | 0 | 0 | 0 | 1 |
| DynamicTree | 19 | 1 | 0 | 0 | 20 |
| Foundation | 29 | 0 | 0 | 0 | 29 |
| Foundation allocation diagnostics | 1 | 0 | 0 | 0 | 1 |
| Foundation allocator configuration | 0 | 1 | 0 | 0 | 1 |
| Foundation defaults | 2 | 0 | 0 | 0 | 2 |
| Foundation initialization | 3 | 0 | 0 | 0 | 3 |
| Internal solver diagnostics | 0 | 1 | 0 | 0 | 1 |
| Internal world tuning | 0 | 1 | 0 | 0 | 1 |
| Joint | 150 | 0 | 0 | 0 | 150 |
| Joint identity validation | 1 | 0 | 0 | 0 | 1 |
| Math | 5 | 0 | 0 | 0 | 5 |
| Native assertion plumbing | 0 | 1 | 0 | 0 | 1 |
| Native build identity | 0 | 1 | 0 | 0 | 1 |
| Native filesystem recording helper | 0 | 2 | 0 | 0 | 2 |
| Native recording buffer ownership | 0 | 2 | 0 | 0 | 2 |
| Native replay validator | 0 | 1 | 0 | 0 | 1 |
| Native value validation primitive | 0 | 9 | 0 | 0 | 9 |
| Recording ownership | 2 | 0 | 0 | 0 | 2 |
| Replay foundation compatibility | 1 | 0 | 0 | 0 | 1 |
| Replay keyframe policy | 5 | 0 | 0 | 0 | 5 |
| Replay metadata | 1 | 0 | 0 | 0 | 1 |
| Replay navigation | 3 | 0 | 0 | 0 | 3 |
| Replay player ownership | 2 | 0 | 0 | 0 | 2 |
| Replay query drawing | 1 | 0 | 0 | 0 | 1 |
| Replay state | 4 | 0 | 0 | 0 | 4 |
| Replay world ownership | 1 | 0 | 0 | 0 | 1 |
| Shape | 55 | 0 | 0 | 0 | 55 |
| Shape identity validation | 1 | 0 | 0 | 0 | 1 |
| Shape query | 3 | 0 | 0 | 0 | 3 |
| Snapshot world creation | 1 | 0 | 0 | 0 | 1 |
| World | 28 | 0 | 2 | 0 | 30 |
| World contact recycling | 2 | 0 | 0 | 0 | 2 |
| World diagnostics | 2 | 0 | 0 | 0 | 2 |
| World operational metadata | 2 | 0 | 0 | 0 | 2 |
| World recording lifecycle | 2 | 0 | 0 | 0 | 2 |
| World snapshot capture | 1 | 0 | 0 | 0 | 1 |
| World snapshot restore | 1 | 0 | 0 | 0 | 1 |
| World spatial query | 7 | 0 | 0 | 0 | 7 |
| World worker configuration | 2 | 0 | 0 | 0 | 2 |

## ABI Safe Rust Exposure

| Capability | Safe | Raw | Omitted | Deferred | Total |
|---|---:|---:|---:|---:|---:|
| Structs | 33 | 46 | 2 | 6 | 87 |
| Fields | 199 | 173 | 53 | 51 | 476 |
| Callbacks | 3 | 12 | 0 | 2 | 17 |

### Non-Safe ABI Capabilities

| Capability | Status | Rationale |
|---|---|---|
| `struct b2BodyDef` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2BodyDef` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2BodyDef::position` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2BodyDef::position` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2BodyDef::sleepThreshold` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyDef::sleepThreshold; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyDef::name` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyDef::name; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyDef::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyDef::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2BodyDef::motionLocks` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyDef::motionLocks; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyDef::enableContactRecycling` | `raw` | The exact native field `b2BodyDef::enableContactRecycling` remains available through the reviewed raw ABI mapping. |
| `b2BodyDef::internalValue` | `omitted` | boxdd_sys::ffi::b2BodyDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2BodyEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-type witness for boxdd_sys::ffi::b2BodyEvents; the reviewed semantic adapter cannot satisfy a strict Safe classification until that proof exists. |
| `b2BodyEvents::moveEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyEvents::moveEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyEvents::moveCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyEvents::moveCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2BodyId` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2BodyId::index1` | `omitted` | boxdd_sys::ffi::b2BodyId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2BodyId::world0` | `omitted` | boxdd_sys::ffi::b2BodyId::world0 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2BodyId::generation` | `omitted` | boxdd_sys::ffi::b2BodyId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2BodyMoveEvent` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2BodyMoveEvent` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2BodyMoveEvent::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyMoveEvent::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2BodyMoveEvent::transform` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2BodyMoveEvent::transform` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2BodyMoveEvent::bodyId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyMoveEvent::bodyId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyMoveEvent::fellAsleep` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyMoveEvent::fellAsleep; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2BoxCastInput` | `raw` | The exact native structure `b2BoxCastInput` remains available through the reviewed raw ABI mapping. |
| `b2BoxCastInput::box` | `raw` | The exact native field `b2BoxCastInput::box` remains available through the reviewed raw ABI mapping. |
| `b2BoxCastInput::translation` | `raw` | The exact native field `b2BoxCastInput::translation` remains available through the reviewed raw ABI mapping. |
| `b2BoxCastInput::maxFraction` | `raw` | The exact native field `b2BoxCastInput::maxFraction` remains available through the reviewed raw ABI mapping. |
| `struct b2Capacity` | `raw` | The exact native structure `b2Capacity` remains available through the reviewed raw ABI mapping. |
| `b2Capacity::staticShapeCount` | `raw` | The exact native field `b2Capacity::staticShapeCount` remains available through the reviewed raw ABI mapping. |
| `b2Capacity::dynamicShapeCount` | `raw` | The exact native field `b2Capacity::dynamicShapeCount` remains available through the reviewed raw ABI mapping. |
| `b2Capacity::staticBodyCount` | `raw` | The exact native field `b2Capacity::staticBodyCount` remains available through the reviewed raw ABI mapping. |
| `b2Capacity::dynamicBodyCount` | `raw` | The exact native field `b2Capacity::dynamicBodyCount` remains available through the reviewed raw ABI mapping. |
| `b2Capacity::contactCount` | `raw` | The exact native field `b2Capacity::contactCount` remains available through the reviewed raw ABI mapping. |
| `b2ChainDef::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ChainDef::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2ChainDef::points` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ChainDef::points; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ChainDef::count` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ChainDef::count; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ChainDef::materialCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ChainDef::materialCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ChainDef::internalValue` | `omitted` | boxdd_sys::ffi::b2ChainDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2ChainId` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2ChainId::index1` | `omitted` | boxdd_sys::ffi::b2ChainId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ChainId::world0` | `omitted` | boxdd_sys::ffi::b2ChainId::world0 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ChainId::generation` | `omitted` | boxdd_sys::ffi::b2ChainId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2ContactBeginTouchEvent` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2ContactBeginTouchEvent::shapeIdA` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactBeginTouchEvent::shapeIdA; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactBeginTouchEvent::shapeIdB` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactBeginTouchEvent::shapeIdB; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactBeginTouchEvent::contactId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactBeginTouchEvent::contactId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2ContactData` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2ContactData` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2ContactData::manifold` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2ContactData::manifold` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `struct b2ContactEndTouchEvent` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2ContactEndTouchEvent::shapeIdA` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactEndTouchEvent::shapeIdA; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactEndTouchEvent::shapeIdB` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactEndTouchEvent::shapeIdB; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactEndTouchEvent::contactId` | `deferred` | boxdd_sys::ffi::b2ContactEndTouchEvent::contactId is intentionally classified as deferred; raw layout reachability is not a Safe Rust witness. |
| `struct b2ContactEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-type witness for boxdd_sys::ffi::b2ContactEvents; the reviewed semantic adapter cannot satisfy a strict Safe classification until that proof exists. |
| `b2ContactEvents::beginEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactEvents::beginEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactEvents::endEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactEvents::endEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactEvents::hitEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactEvents::hitEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactEvents::beginCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactEvents::beginCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactEvents::endCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactEvents::endCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactEvents::hitCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactEvents::hitCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2ContactHitEvent` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2ContactHitEvent` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2ContactHitEvent::shapeIdA` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactHitEvent::shapeIdA; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactHitEvent::shapeIdB` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactHitEvent::shapeIdB; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactHitEvent::contactId` | `deferred` | boxdd_sys::ffi::b2ContactHitEvent::contactId is intentionally classified as deferred; raw layout reachability is not a Safe Rust witness. |
| `b2ContactHitEvent::point` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2ContactHitEvent::point` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2ContactHitEvent::normal` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactHitEvent::normal; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactHitEvent::approachSpeed` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactHitEvent::approachSpeed; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2ContactId` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2ContactId::index1` | `omitted` | boxdd_sys::ffi::b2ContactId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ContactId::world0` | `omitted` | boxdd_sys::ffi::b2ContactId::world0 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ContactId::padding` | `omitted` | boxdd_sys::ffi::b2ContactId::padding is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ContactId::generation` | `omitted` | boxdd_sys::ffi::b2ContactId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2CosSin` | `deferred` | No public Safe Rust path currently has an exact AST raw-type witness for boxdd_sys::ffi::b2CosSin; the reviewed semantic adapter cannot satisfy a strict Safe classification until that proof exists. |
| `b2CosSin::cosine` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2CosSin::cosine; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2CosSin::sine` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2CosSin::sine; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2Counters` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2Counters` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2Counters::byteCount` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2Counters::byteCount` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2Counters::awakeContactCount` | `raw` | The exact native field `b2Counters::awakeContactCount` remains available through the reviewed raw ABI mapping. |
| `b2Counters::recycledContactCount` | `raw` | The exact native field `b2Counters::recycledContactCount` remains available through the reviewed raw ABI mapping. |
| `struct b2DebugDraw` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2DebugDraw` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2DebugDraw::DrawPolygonFcn` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DebugDraw::DrawPolygonFcn` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DebugDraw::DrawSolidPolygonFcn` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DebugDraw::DrawSolidPolygonFcn` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DebugDraw::DrawCircleFcn` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DebugDraw::DrawCircleFcn` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DebugDraw::DrawSolidCircleFcn` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DebugDraw::DrawSolidCircleFcn` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DebugDraw::DrawSolidCapsuleFcn` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DebugDraw::DrawSolidCapsuleFcn` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DebugDraw::DrawLineFcn` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DebugDraw::DrawLineFcn` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DebugDraw::DrawTransformFcn` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DebugDraw::DrawTransformFcn` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DebugDraw::DrawPointFcn` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DebugDraw::DrawPointFcn` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DebugDraw::DrawStringFcn` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DebugDraw::DrawStringFcn` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DebugDraw::DrawBoundsFcn` | `raw` | The exact native field `b2DebugDraw::DrawBoundsFcn` remains available through the reviewed raw ABI mapping. |
| `b2DebugDraw::drawingBounds` | `raw` | boxdd_sys::ffi::b2DebugDraw::drawingBounds is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
| `b2DebugDraw::drawContacts` | `raw` | The exact native field `b2DebugDraw::drawContacts` remains available through the reviewed raw ABI mapping. |
| `b2DebugDraw::drawAnchorA` | `raw` | The exact native field `b2DebugDraw::drawAnchorA` remains available through the reviewed raw ABI mapping. |
| `b2DebugDraw::drawChainNormals` | `raw` | The exact native field `b2DebugDraw::drawChainNormals` remains available through the reviewed raw ABI mapping. |
| `struct b2DistanceInput` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2DistanceInput` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2DistanceInput::transform` | `raw` | The exact native field `b2DistanceInput::transform` remains available through the reviewed raw ABI mapping. |
| `struct b2DistanceJointDef` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::base` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::length` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::enableSpring` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::lowerSpringForce` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::upperSpringForce` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::hertz` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::dampingRatio` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::enableLimit` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::minLength` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::maxLength` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::enableMotor` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::maxMotorForce` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::motorSpeed` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2DistanceJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2DistanceJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2DynamicTree` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2DynamicTree` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2DynamicTree::nodes` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::nodes; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::root` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DynamicTree::root` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DynamicTree::nodeCount` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DynamicTree::nodeCount` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DynamicTree::nodeCapacity` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DynamicTree::nodeCapacity` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DynamicTree::freeList` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DynamicTree::freeList` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DynamicTree::proxyCount` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DynamicTree::proxyCount` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DynamicTree::leafIndices` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DynamicTree::leafIndices` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DynamicTree::leafBoxes` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::leafBoxes; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::leafCenters` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::leafCenters; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::binIndices` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DynamicTree::binIndices` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2DynamicTree::rebuildCapacity` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2DynamicTree::rebuildCapacity` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `struct b2ExplosionDef` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2ExplosionDef` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2ExplosionDef::position` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2ExplosionDef::position` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2FilterJointDef::base` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2FilterJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2FilterJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2Hull::points` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2Hull::points; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2Hull::count` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2Hull::count; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `struct b2JointDef` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointDef::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2JointDef::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2JointDef::bodyIdA` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointDef::bodyIdB` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointDef::localFrameA` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointDef::localFrameB` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointDef::forceThreshold` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointDef::torqueThreshold` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointDef::constraintHertz` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointDef::constraintDampingRatio` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointDef::drawScale` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointDef::collideConnected` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `struct b2JointEvent` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointEvent::jointId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2JointEvent::jointId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2JointEvent::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2JointEvent::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `struct b2JointEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-type witness for boxdd_sys::ffi::b2JointEvents; the reviewed semantic adapter cannot satisfy a strict Safe classification until that proof exists. |
| `b2JointEvents::jointEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2JointEvents::jointEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2JointEvents::count` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2JointEvents::count; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2JointId` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2JointId::index1` | `omitted` | boxdd_sys::ffi::b2JointId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2JointId::world0` | `omitted` | boxdd_sys::ffi::b2JointId::world0 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2JointId::generation` | `omitted` | boxdd_sys::ffi::b2JointId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2LocalManifold` | `raw` | The exact native structure `b2LocalManifold` remains available through the reviewed raw ABI mapping. |
| `b2LocalManifold::normal` | `raw` | The exact native field `b2LocalManifold::normal` remains available through the reviewed raw ABI mapping. |
| `b2LocalManifold::points` | `raw` | The exact native field `b2LocalManifold::points` remains available through the reviewed raw ABI mapping. |
| `b2LocalManifold::pointCount` | `raw` | The exact native field `b2LocalManifold::pointCount` remains available through the reviewed raw ABI mapping. |
| `struct b2LocalManifoldPoint` | `raw` | The exact native structure `b2LocalManifoldPoint` remains available through the reviewed raw ABI mapping. |
| `b2LocalManifoldPoint::point` | `raw` | The exact native field `b2LocalManifoldPoint::point` remains available through the reviewed raw ABI mapping. |
| `b2LocalManifoldPoint::separation` | `raw` | The exact native field `b2LocalManifoldPoint::separation` remains available through the reviewed raw ABI mapping. |
| `b2LocalManifoldPoint::id` | `raw` | The exact native field `b2LocalManifoldPoint::id` remains available through the reviewed raw ABI mapping. |
| `struct b2Manifold` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2Manifold` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2Manifold::points` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2Manifold::points` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `struct b2ManifoldPoint` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2ManifoldPoint` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2ManifoldPoint::baseSeparation` | `raw` | The exact native field `b2ManifoldPoint::baseSeparation` remains available through the reviewed raw ABI mapping. |
| `struct b2Mat22` | `raw` | boxdd_sys::ffi::b2Mat22 is intentionally classified as raw; raw ABI presence is not a Safe Rust witness. |
| `b2Mat22::cx` | `raw` | boxdd_sys::ffi::b2Mat22::cx is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
| `b2Mat22::cy` | `raw` | boxdd_sys::ffi::b2Mat22::cy is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
| `struct b2MotorJointDef` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::base` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::linearVelocity` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::maxVelocityForce` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::angularVelocity` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::maxVelocityTorque` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::linearHertz` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::linearDampingRatio` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::maxSpringForce` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::angularHertz` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::angularDampingRatio` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::maxSpringTorque` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2MotorJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2MotorJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2PlaneResult` | `deferred` | No public Safe Rust path currently has an exact AST raw-type witness for boxdd_sys::ffi::b2PlaneResult; the reviewed semantic adapter cannot satisfy a strict Safe classification until that proof exists. |
| `b2PlaneResult::plane` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2PlaneResult::plane; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2PlaneResult::point` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2PlaneResult::point; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2PlaneResult::hit` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2PlaneResult::hit; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2Polygon::vertices` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2Polygon::vertices; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2Polygon::normals` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2Polygon::normals; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2Polygon::centroid` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2Polygon::centroid; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2Polygon::radius` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2Polygon::radius; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2Polygon::count` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2Polygon::count; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2Pos` | `raw` | The exact native structure `b2Pos` remains available through the reviewed raw ABI mapping. |
| `b2Pos::x` | `raw` | The exact native field `b2Pos::x` remains available through the reviewed raw ABI mapping. |
| `b2Pos::y` | `raw` | The exact native field `b2Pos::y` remains available through the reviewed raw ABI mapping. |
| `struct b2PrismaticJointDef` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::base` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::enableSpring` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::hertz` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::dampingRatio` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::targetTranslation` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2PrismaticJointDef::targetTranslation; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2PrismaticJointDef::enableLimit` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::lowerTranslation` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::upperTranslation` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::enableMotor` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::maxMotorForce` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::motorSpeed` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2PrismaticJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2PrismaticJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2Profile` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2Profile` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2Profile::solverSetup` | `raw` | The exact native field `b2Profile::solverSetup` remains available through the reviewed raw ABI mapping. |
| `b2Profile::constraints` | `raw` | The exact native field `b2Profile::constraints` remains available through the reviewed raw ABI mapping. |
| `struct b2RayResult` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2RayResult` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2RayResult::shapeId` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RayResult::point` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2RayResult::point` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2RayResult::normal` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RayResult::fraction` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RayResult::nodeVisits` | `deferred` | boxdd_sys::ffi::b2RayResult::nodeVisits is intentionally classified as deferred; raw layout reachability is not a Safe Rust witness. |
| `b2RayResult::leafVisits` | `deferred` | boxdd_sys::ffi::b2RayResult::leafVisits is intentionally classified as deferred; raw layout reachability is not a Safe Rust witness. |
| `b2RayResult::hit` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `struct b2RecPlayerInfo` | `raw` | The exact native structure `b2RecPlayerInfo` remains available through the reviewed raw ABI mapping. |
| `b2RecPlayerInfo::frameCount` | `raw` | The exact native field `b2RecPlayerInfo::frameCount` remains available through the reviewed raw ABI mapping. |
| `b2RecPlayerInfo::workerCount` | `raw` | The exact native field `b2RecPlayerInfo::workerCount` remains available through the reviewed raw ABI mapping. |
| `b2RecPlayerInfo::timeStep` | `raw` | The exact native field `b2RecPlayerInfo::timeStep` remains available through the reviewed raw ABI mapping. |
| `b2RecPlayerInfo::subStepCount` | `raw` | The exact native field `b2RecPlayerInfo::subStepCount` remains available through the reviewed raw ABI mapping. |
| `b2RecPlayerInfo::lengthScale` | `raw` | The exact native field `b2RecPlayerInfo::lengthScale` remains available through the reviewed raw ABI mapping. |
| `b2RecPlayerInfo::bounds` | `raw` | The exact native field `b2RecPlayerInfo::bounds` remains available through the reviewed raw ABI mapping. |
| `struct b2RecQueryHit` | `raw` | The exact native structure `b2RecQueryHit` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryHit::shape` | `raw` | The exact native field `b2RecQueryHit::shape` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryHit::point` | `raw` | The exact native field `b2RecQueryHit::point` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryHit::normal` | `raw` | The exact native field `b2RecQueryHit::normal` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryHit::fraction` | `raw` | The exact native field `b2RecQueryHit::fraction` remains available through the reviewed raw ABI mapping. |
| `struct b2RecQueryInfo` | `raw` | The exact native structure `b2RecQueryInfo` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryInfo::type` | `raw` | The exact native field `b2RecQueryInfo::type` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryInfo::filter` | `raw` | The exact native field `b2RecQueryInfo::filter` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryInfo::aabb` | `raw` | The exact native field `b2RecQueryInfo::aabb` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryInfo::origin` | `raw` | The exact native field `b2RecQueryInfo::origin` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryInfo::translation` | `raw` | The exact native field `b2RecQueryInfo::translation` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryInfo::shape` | `raw` | The exact native field `b2RecQueryInfo::shape` remains available through the reviewed raw ABI mapping. |
| `b2RecQueryInfo::hitCount` | `raw` | The exact native field `b2RecQueryInfo::hitCount` remains available through the reviewed raw ABI mapping. |
| `struct b2RevoluteJointDef` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::base` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::targetAngle` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::enableSpring` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::hertz` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::dampingRatio` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::enableLimit` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::lowerAngle` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::upperAngle` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::enableMotor` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::maxMotorTorque` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::motorSpeed` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2RevoluteJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2RevoluteJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2SensorBeginTouchEvent` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2SensorBeginTouchEvent::sensorShapeId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorBeginTouchEvent::sensorShapeId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorBeginTouchEvent::visitorShapeId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorBeginTouchEvent::visitorShapeId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2SensorEndTouchEvent` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2SensorEndTouchEvent::sensorShapeId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEndTouchEvent::sensorShapeId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorEndTouchEvent::visitorShapeId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEndTouchEvent::visitorShapeId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2SensorEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-type witness for boxdd_sys::ffi::b2SensorEvents; the reviewed semantic adapter cannot satisfy a strict Safe classification until that proof exists. |
| `b2SensorEvents::beginEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEvents::beginEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorEvents::endEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEvents::endEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorEvents::beginCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEvents::beginCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorEvents::endCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEvents::endCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2ShapeCastPairInput` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2ShapeCastPairInput` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2ShapeCastPairInput::transform` | `raw` | The exact native field `b2ShapeCastPairInput::transform` remains available through the reviewed raw ABI mapping. |
| `b2ShapeDef::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ShapeDef::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2ShapeDef::internalValue` | `omitted` | boxdd_sys::ffi::b2ShapeDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2ShapeId` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2ShapeId::index1` | `omitted` | boxdd_sys::ffi::b2ShapeId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ShapeId::world0` | `omitted` | boxdd_sys::ffi::b2ShapeId::world0 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ShapeId::generation` | `omitted` | boxdd_sys::ffi::b2ShapeId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2Simplex` | `omitted` | boxdd_sys::ffi::b2Simplex is intentionally classified as omitted; raw ABI presence is not a Safe Rust witness. |
| `b2Simplex::v1` | `omitted` | boxdd_sys::ffi::b2Simplex::v1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2Simplex::v2` | `omitted` | boxdd_sys::ffi::b2Simplex::v2 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2Simplex::v3` | `omitted` | boxdd_sys::ffi::b2Simplex::v3 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2Simplex::count` | `omitted` | boxdd_sys::ffi::b2Simplex::count is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2SimplexCache::indexA` | `omitted` | boxdd_sys::ffi::b2SimplexCache::indexA is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2SimplexCache::indexB` | `omitted` | boxdd_sys::ffi::b2SimplexCache::indexB is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2SimplexVertex` | `omitted` | boxdd_sys::ffi::b2SimplexVertex is intentionally classified as omitted; raw ABI presence is not a Safe Rust witness. |
| `b2SimplexVertex::wA` | `omitted` | boxdd_sys::ffi::b2SimplexVertex::wA is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2SimplexVertex::wB` | `omitted` | boxdd_sys::ffi::b2SimplexVertex::wB is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2SimplexVertex::w` | `omitted` | boxdd_sys::ffi::b2SimplexVertex::w is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2SimplexVertex::a` | `omitted` | boxdd_sys::ffi::b2SimplexVertex::a is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2SimplexVertex::indexA` | `omitted` | boxdd_sys::ffi::b2SimplexVertex::indexA is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2SimplexVertex::indexB` | `omitted` | boxdd_sys::ffi::b2SimplexVertex::indexB is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2Transform` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2Transform::p` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2Transform::q` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `struct b2TreeNode` | `raw` | The exact native structure `b2TreeNode` remains available through the reviewed raw ABI mapping. |
| `b2TreeNode::aabb` | `raw` | The exact native field `b2TreeNode::aabb` remains available through the reviewed raw ABI mapping. |
| `b2TreeNode::categoryBits` | `raw` | The exact native field `b2TreeNode::categoryBits` remains available through the reviewed raw ABI mapping. |
| `b2TreeNode::children` | `raw` | The exact native field `b2TreeNode::children` remains available through the reviewed raw ABI mapping. |
| `b2TreeNode::userData` | `raw` | The exact native field `b2TreeNode::userData` remains available through the reviewed raw ABI mapping. |
| `b2TreeNode::parent` | `raw` | The exact native field `b2TreeNode::parent` remains available through the reviewed raw ABI mapping. |
| `b2TreeNode::next` | `raw` | The exact native field `b2TreeNode::next` remains available through the reviewed raw ABI mapping. |
| `b2TreeNode::height` | `raw` | The exact native field `b2TreeNode::height` remains available through the reviewed raw ABI mapping. |
| `b2TreeNode::flags` | `raw` | The exact native field `b2TreeNode::flags` remains available through the reviewed raw ABI mapping. |
| `struct b2TreeNodeChildren` | `raw` | The exact native structure `b2TreeNodeChildren` remains available through the reviewed raw ABI mapping. |
| `b2TreeNodeChildren::child1` | `raw` | The exact native field `b2TreeNodeChildren::child1` remains available through the reviewed raw ABI mapping. |
| `b2TreeNodeChildren::child2` | `raw` | The exact native field `b2TreeNodeChildren::child2` remains available through the reviewed raw ABI mapping. |
| `struct b2WeldJointDef` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WeldJointDef::base` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WeldJointDef::linearHertz` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WeldJointDef::angularHertz` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WeldJointDef::linearDampingRatio` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WeldJointDef::angularDampingRatio` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WeldJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2WeldJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2WheelJointDef` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::base` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::enableSpring` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::hertz` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::dampingRatio` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::enableLimit` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::lowerTranslation` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::upperTranslation` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::enableMotor` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::maxMotorTorque` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::motorSpeed` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `b2WheelJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2WheelJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2WorldCastOutput` | `raw` | The exact native structure `b2WorldCastOutput` remains available through the reviewed raw ABI mapping. |
| `b2WorldCastOutput::normal` | `raw` | The exact native field `b2WorldCastOutput::normal` remains available through the reviewed raw ABI mapping. |
| `b2WorldCastOutput::point` | `raw` | The exact native field `b2WorldCastOutput::point` remains available through the reviewed raw ABI mapping. |
| `b2WorldCastOutput::fraction` | `raw` | The exact native field `b2WorldCastOutput::fraction` remains available through the reviewed raw ABI mapping. |
| `b2WorldCastOutput::iterations` | `raw` | The exact native field `b2WorldCastOutput::iterations` remains available through the reviewed raw ABI mapping. |
| `b2WorldCastOutput::hit` | `raw` | The exact native field `b2WorldCastOutput::hit` remains available through the reviewed raw ABI mapping. |
| `struct b2WorldDef` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2WorldDef` changed, so the previous Safe review was not inherited and this refreshed capability is conservatively raw. |
| `b2WorldDef::frictionCallback` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2WorldDef::frictionCallback; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2WorldDef::restitutionCallback` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2WorldDef::restitutionCallback; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2WorldDef::enqueueTask` | `raw` | The declaration, overlay contract, or precision-specific raw ABI proof for `b2WorldDef::enqueueTask` changed, so the previous Safe review was not inherited and this refreshed field is conservatively raw. |
| `b2WorldDef::finishTask` | `raw` | boxdd_sys::ffi::b2WorldDef::finishTask is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
| `b2WorldDef::userTaskContext` | `raw` | boxdd_sys::ffi::b2WorldDef::userTaskContext is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
| `b2WorldDef::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2WorldDef::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2WorldDef::capacity` | `raw` | The exact native field `b2WorldDef::capacity` remains available through the reviewed raw ABI mapping. |
| `b2WorldDef::internalValue` | `omitted` | boxdd_sys::ffi::b2WorldDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2WorldId::index1` | `omitted` | boxdd_sys::ffi::b2WorldId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2WorldId::generation` | `omitted` | boxdd_sys::ffi::b2WorldId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2WorldTransform` | `raw` | The exact native structure `b2WorldTransform` remains available through the reviewed raw ABI mapping. |
| `b2WorldTransform::p` | `raw` | The exact native field `b2WorldTransform::p` remains available through the reviewed raw ABI mapping. |
| `b2WorldTransform::q` | `raw` | The exact native field `b2WorldTransform::q` remains available through the reviewed raw ABI mapping. |
| `callback b2AllocFcn` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2AllocFcn` changed, so the previous Safe review was not inherited and this refreshed callback is conservatively raw. |
| `callback b2AssertFcn` | `raw` | boxdd_sys::ffi::b2AssertFcn remains raw because its process-global or scheduler safety contract is not exposed as an ordinary Safe Rust callback adapter. |
| `callback b2CastResultFcn` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2CastResultFcn` changed, so the previous Safe review was not inherited and this refreshed callback is conservatively raw. |
| `callback b2EnqueueTaskCallback` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2EnqueueTaskCallback` changed, so the previous Safe review was not inherited and this refreshed callback is conservatively raw. |
| `callback b2FinishTaskCallback` | `raw` | boxdd_sys::ffi::b2FinishTaskCallback remains raw because its process-global or scheduler safety contract is not exposed as an ordinary Safe Rust callback adapter. |
| `callback b2FreeFcn` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2FreeFcn` changed, so the previous Safe review was not inherited and this refreshed callback is conservatively raw. |
| `callback b2FrictionCallback` | `deferred` | No reviewed public path proves a callable value in the exact native argument slot for b2FrictionCallback; reaching a callback-taking symbol, including passing None, is insufficient. |
| `callback b2LogFcn` | `raw` | boxdd_sys::ffi::b2LogFcn remains raw because its process-global or scheduler safety contract is not exposed as an ordinary Safe Rust callback adapter. |
| `callback b2OverlapResultFcn` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `callback b2PlaneResultFcn` | `raw` | The native declaration is unchanged, but its previous Safe Rust exposure proof no longer matches the refreshed upstream call graph, so this capability is conservatively raw. |
| `callback b2PreSolveFcn` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2PreSolveFcn` changed, so the previous Safe review was not inherited and this refreshed callback is conservatively raw. |
| `callback b2RestitutionCallback` | `deferred` | No reviewed public path proves a callable value in the exact native argument slot for b2RestitutionCallback; reaching a callback-taking symbol, including passing None, is insufficient. |
| `callback b2TaskCallback` | `raw` | The declaration identity or precision-specific raw ABI proof for `b2TaskCallback` changed, so the previous Safe review was not inherited and this refreshed callback is conservatively raw. |
| `callback b2TreeBoxCastCallbackFcn` | `raw` | The exact native callback `b2TreeBoxCastCallbackFcn` remains available through the reviewed raw ABI mapping. |

## Non-Safe Capabilities

| Logical API | Status | Area | Rationale |
|---|---|---|---|
| `b2DynamicTree_SetCategoryBits` | `raw` | DynamicTree | The pinned upstream assertion aliases `b2TreeNode` leaf user data as `children.child1` and can trap for valid non-sentinel user data. The Safe `DynamicTree::replace_category_bits` capability creates an equivalent proxy, destroys the old proxy, and returns the new identity without calling this defective native function. |
| `b2GetGraphColor` | `raw` | Internal solver diagnostics | Graph colors expose upstream solver-internal scheduling state without a stable Safe Rust semantic contract. |
| `b2InternalAssert` | `raw` | Native assertion plumbing | The native assertion entry point accepts C source metadata and remains an explicit low-level FFI capability. |
| `b2IsDoublePrecision` | `raw` | Native build identity | Precision is selected and authenticated by Cargo features and ABI route metadata; the exact runtime probe remains raw. |
| `b2IsValidAABB` | `raw` | Native value validation primitive | Safe AABB validation is implemented in Rust so malformed values are rejected before FFI; the exact native predicate remains a raw primitive. |
| `b2IsValidFloat` | `raw` | Native value validation primitive | Safe scalar validation is implemented in Rust before FFI; the exact native predicate remains a raw primitive. |
| `b2IsValidPlane` | `raw` | Native value validation primitive | Safe plane validation is implemented in Rust before FFI; the exact native predicate remains a raw primitive. |
| `b2IsValidPosition` | `raw` | Native value validation primitive | Safe position validation is implemented in Rust before FFI; the exact native predicate remains a raw primitive. |
| `b2IsValidRay` | `raw` | Native value validation primitive | Safe ray inputs are validated in Rust before FFI; the exact native predicate remains a raw primitive. |
| `b2IsValidRotation` | `raw` | Native value validation primitive | Safe rotation validation is implemented in Rust before FFI; the exact native predicate remains a raw primitive. |
| `b2IsValidTransform` | `raw` | Native value validation primitive | Safe transform validation is implemented in Rust before FFI; the exact native predicate remains a raw primitive. |
| `b2IsValidVec2` | `raw` | Native value validation primitive | Safe vector validation is implemented in Rust before FFI; the exact native predicate remains a raw primitive. |
| `b2IsValidWorldTransform` | `raw` | Native value validation primitive | Safe world-transform validation is implemented in Rust before FFI; the exact native predicate remains a raw primitive. |
| `b2LoadRecordingFromFile` | `raw` | Native filesystem recording helper | Safe Rust loads owned bytes through std::fs and validates them before replay, avoiding native path encoding and allocator ownership. |
| `b2Recording_GetData` | `raw` | Native recording buffer ownership | Safe Recording owns a copied Rust byte buffer and never exposes the native recording handle or borrowed native pointer. |
| `b2Recording_GetSize` | `raw` | Native recording buffer ownership | Safe Recording derives length from its owned Rust byte buffer rather than retaining a native recording handle. |
| `b2SaveRecordingToFile` | `raw` | Native filesystem recording helper | Safe Rust saves owned recording bytes through std::fs, avoiding native path encoding and error-channel ambiguity. |
| `b2SetAllocator` | `raw` | Foundation allocator configuration | Revalidated against the authenticated Box2D 3.2 declaration; the process-global allocator hook still requires a scoped startup guard before it can be exposed safely. |
| `b2ValidateReplay` | `raw` | Native replay validator | Safe replay performs bounded Rust preflight parsing before native allocation; the separate native validator remains a raw diagnostic primitive. |
| `b2World_DumpMemoryStats` | `omitted` | World | Intentionally omitted: upstream writes fixed diagnostic output, so callers should use upstream diagnostics tooling explicitly. |
| `b2World_EnableSpeculative` | `raw` | Internal world tuning | Upstream documents speculative collision toggling as an internal testing hook, so it is intentionally excluded from the stable Safe Rust surface. |
| `b2World_RebuildStaticTree` | `omitted` | World | Intentionally omitted: upstream labels this as internal testing support, not stable runtime API. |

## Maintenance

- Run `cargo run -p xtask -- api-coverage --check` to reject header, Rust path, evidence, recording-capability, wire-schema, ABI, or generated-document drift.
- Run `cargo run -p xtask -- api-coverage --write` only to regenerate this report after the structured contract has been reviewed.
- Run `cargo run -p xtask -- upstream-sync --check` to verify the manifest, gitlink, checkout, exact source-path inventory, reviewed recording-source Git objects, and all named artifacts.
