# Box2D API Coverage

<!-- api-coverage: total=430 safe=422 raw=6 omitted=2 deferred=0 -->

This file is generated from the API artifact named by `boxdd-sys/upstream.toml`. The contract is validated against the exact vendored headers, canonical public Rust paths, real `#[test]` evidence, provider modes, precision-specific link symbols, explicit recording capability classes, and ABI struct/callback fingerprints.

Pinned active upstream: `c05c48738fbe5c27625e36c5f0cfbdaddfc8359a`.

## Summary

| Status | Count |
|---|---:|
| `safe` | 422 |
| `raw` | 6 |
| `omitted` | 2 |
| `deferred` | 0 |
| Total | 430 |

## By Area

| Area | Safe | Raw | Omitted | Deferred | Total |
|---|---:|---:|---:|---:|---:|
| Body | 68 | 0 | 0 | 0 | 68 |
| Chain | 13 | 0 | 0 | 0 | 13 |
| Collision | 21 | 0 | 0 | 0 | 21 |
| DynamicTree | 20 | 1 | 0 | 0 | 21 |
| Foundation | 38 | 4 | 0 | 0 | 42 |
| Joint | 151 | 0 | 0 | 0 | 151 |
| Math | 9 | 1 | 0 | 0 | 10 |
| Shape | 60 | 0 | 0 | 0 | 60 |
| World | 42 | 0 | 2 | 0 | 44 |

## ABI Safe Rust Exposure

| Capability | Safe | Raw | Omitted | Deferred | Total |
|---|---:|---:|---:|---:|---:|
| Structs | 66 | 1 | 2 | 6 | 75 |
| Fields | 290 | 17 | 61 | 53 | 421 |
| Callbacks | 8 | 7 | 0 | 2 | 17 |

### Non-Safe ABI Capabilities

| Capability | Status | Rationale |
|---|---|---|
| `b2BodyDef::sleepThreshold` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyDef::sleepThreshold; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyDef::name` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyDef::name; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyDef::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyDef::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2BodyDef::motionLocks` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyDef::motionLocks; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyDef::internalValue` | `omitted` | boxdd_sys::ffi::b2BodyDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2BodyEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-type witness for boxdd_sys::ffi::b2BodyEvents; the reviewed semantic adapter cannot satisfy a strict Safe classification until that proof exists. |
| `b2BodyEvents::moveEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyEvents::moveEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyEvents::moveCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyEvents::moveCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyId::index1` | `omitted` | boxdd_sys::ffi::b2BodyId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2BodyId::world0` | `omitted` | boxdd_sys::ffi::b2BodyId::world0 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2BodyId::generation` | `omitted` | boxdd_sys::ffi::b2BodyId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2BodyMoveEvent::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyMoveEvent::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2BodyMoveEvent::transform` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyMoveEvent::transform; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyMoveEvent::bodyId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyMoveEvent::bodyId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2BodyMoveEvent::fellAsleep` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2BodyMoveEvent::fellAsleep; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ChainDef::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ChainDef::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2ChainDef::points` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ChainDef::points; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ChainDef::count` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ChainDef::count; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ChainDef::materialCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ChainDef::materialCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ChainDef::internalValue` | `omitted` | boxdd_sys::ffi::b2ChainDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ChainId::index1` | `omitted` | boxdd_sys::ffi::b2ChainId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ChainId::world0` | `omitted` | boxdd_sys::ffi::b2ChainId::world0 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ChainId::generation` | `omitted` | boxdd_sys::ffi::b2ChainId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ContactBeginTouchEvent::shapeIdA` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactBeginTouchEvent::shapeIdA; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactBeginTouchEvent::shapeIdB` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactBeginTouchEvent::shapeIdB; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactBeginTouchEvent::contactId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactBeginTouchEvent::contactId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
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
| `b2ContactHitEvent::shapeIdA` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactHitEvent::shapeIdA; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactHitEvent::shapeIdB` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactHitEvent::shapeIdB; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactHitEvent::contactId` | `deferred` | boxdd_sys::ffi::b2ContactHitEvent::contactId is intentionally classified as deferred; raw layout reachability is not a Safe Rust witness. |
| `b2ContactHitEvent::point` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactHitEvent::point; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactHitEvent::normal` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactHitEvent::normal; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactHitEvent::approachSpeed` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ContactHitEvent::approachSpeed; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ContactId::index1` | `omitted` | boxdd_sys::ffi::b2ContactId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ContactId::world0` | `omitted` | boxdd_sys::ffi::b2ContactId::world0 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ContactId::padding` | `omitted` | boxdd_sys::ffi::b2ContactId::padding is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2ContactId::generation` | `omitted` | boxdd_sys::ffi::b2ContactId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2CosSin` | `deferred` | No public Safe Rust path currently has an exact AST raw-type witness for boxdd_sys::ffi::b2CosSin; the reviewed semantic adapter cannot satisfy a strict Safe classification until that proof exists. |
| `b2CosSin::cosine` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2CosSin::cosine; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2CosSin::sine` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2CosSin::sine; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2DebugDraw::DrawPolygonFcn` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DebugDraw::DrawPolygonFcn; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2DebugDraw::DrawSolidPolygonFcn` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DebugDraw::DrawSolidPolygonFcn; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2DebugDraw::DrawCircleFcn` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DebugDraw::DrawCircleFcn; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2DebugDraw::DrawSolidCircleFcn` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DebugDraw::DrawSolidCircleFcn; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2DebugDraw::DrawSolidCapsuleFcn` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DebugDraw::DrawSolidCapsuleFcn; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2DebugDraw::DrawLineFcn` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DebugDraw::DrawLineFcn; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2DebugDraw::DrawTransformFcn` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DebugDraw::DrawTransformFcn; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2DebugDraw::DrawPointFcn` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DebugDraw::DrawPointFcn; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2DebugDraw::DrawStringFcn` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DebugDraw::DrawStringFcn; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2DebugDraw::drawingBounds` | `raw` | boxdd_sys::ffi::b2DebugDraw::drawingBounds is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
| `b2DistanceJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2DistanceJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2DynamicTree::nodes` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::nodes; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::root` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::root; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::nodeCount` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::nodeCount; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::nodeCapacity` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::nodeCapacity; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::freeList` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::freeList; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::proxyCount` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::proxyCount; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::leafIndices` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::leafIndices; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::leafBoxes` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::leafBoxes; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::leafCenters` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::leafCenters; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::binIndices` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::binIndices; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2DynamicTree::rebuildCapacity` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2DynamicTree::rebuildCapacity; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2FilterJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2FilterJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2Hull::points` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2Hull::points; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2Hull::count` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2Hull::count; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2JointDef::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2JointDef::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2JointEvent::jointId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2JointEvent::jointId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2JointEvent::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2JointEvent::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `struct b2JointEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-type witness for boxdd_sys::ffi::b2JointEvents; the reviewed semantic adapter cannot satisfy a strict Safe classification until that proof exists. |
| `b2JointEvents::jointEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2JointEvents::jointEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2JointEvents::count` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2JointEvents::count; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2JointId::index1` | `omitted` | boxdd_sys::ffi::b2JointId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2JointId::world0` | `omitted` | boxdd_sys::ffi::b2JointId::world0 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2JointId::generation` | `omitted` | boxdd_sys::ffi::b2JointId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `struct b2Mat22` | `raw` | boxdd_sys::ffi::b2Mat22 is intentionally classified as raw; raw ABI presence is not a Safe Rust witness. |
| `b2Mat22::cx` | `raw` | boxdd_sys::ffi::b2Mat22::cx is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
| `b2Mat22::cy` | `raw` | boxdd_sys::ffi::b2Mat22::cy is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
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
| `b2PrismaticJointDef::targetTranslation` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2PrismaticJointDef::targetTranslation; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2PrismaticJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2PrismaticJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2RayResult::nodeVisits` | `deferred` | boxdd_sys::ffi::b2RayResult::nodeVisits is intentionally classified as deferred; raw layout reachability is not a Safe Rust witness. |
| `b2RayResult::leafVisits` | `deferred` | boxdd_sys::ffi::b2RayResult::leafVisits is intentionally classified as deferred; raw layout reachability is not a Safe Rust witness. |
| `b2RevoluteJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2RevoluteJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2SensorBeginTouchEvent::sensorShapeId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorBeginTouchEvent::sensorShapeId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorBeginTouchEvent::visitorShapeId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorBeginTouchEvent::visitorShapeId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorEndTouchEvent::sensorShapeId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEndTouchEvent::sensorShapeId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorEndTouchEvent::visitorShapeId` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEndTouchEvent::visitorShapeId; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `struct b2SensorEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-type witness for boxdd_sys::ffi::b2SensorEvents; the reviewed semantic adapter cannot satisfy a strict Safe classification until that proof exists. |
| `b2SensorEvents::beginEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEvents::beginEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorEvents::endEvents` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEvents::endEvents; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorEvents::beginCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEvents::beginCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2SensorEvents::endCount` | `deferred` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2SensorEvents::endCount; fail-closed validation recommends deferred rather than accepting the reviewed semantic path as proof. |
| `b2ShapeDef::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2ShapeDef::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2ShapeDef::internalValue` | `omitted` | boxdd_sys::ffi::b2ShapeDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
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
| `b2WeldJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2WeldJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2WheelJointDef::internalValue` | `omitted` | boxdd_sys::ffi::b2WheelJointDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2WorldDef::frictionCallback` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2WorldDef::frictionCallback; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2WorldDef::restitutionCallback` | `raw` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2WorldDef::restitutionCallback; fail-closed validation recommends raw rather than accepting the reviewed semantic path as proof. |
| `b2WorldDef::enqueueTask` | `raw` | boxdd_sys::ffi::b2WorldDef::enqueueTask is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
| `b2WorldDef::finishTask` | `raw` | boxdd_sys::ffi::b2WorldDef::finishTask is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
| `b2WorldDef::userTaskContext` | `raw` | boxdd_sys::ffi::b2WorldDef::userTaskContext is intentionally classified as raw; raw layout reachability is not a Safe Rust witness. |
| `b2WorldDef::userData` | `omitted` | No public Safe Rust path currently has an exact AST raw-field witness for boxdd_sys::ffi::b2WorldDef::userData; fail-closed validation recommends omitted rather than accepting the reviewed semantic path as proof. |
| `b2WorldDef::internalValue` | `omitted` | boxdd_sys::ffi::b2WorldDef::internalValue is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2WorldId::index1` | `omitted` | boxdd_sys::ffi::b2WorldId::index1 is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `b2WorldId::generation` | `omitted` | boxdd_sys::ffi::b2WorldId::generation is intentionally classified as omitted; raw layout reachability is not a Safe Rust witness. |
| `callback b2AllocFcn` | `raw` | boxdd_sys::ffi::b2AllocFcn remains raw because its process-global or scheduler safety contract is not exposed as an ordinary Safe Rust callback adapter. |
| `callback b2AssertFcn` | `raw` | boxdd_sys::ffi::b2AssertFcn remains raw because its process-global or scheduler safety contract is not exposed as an ordinary Safe Rust callback adapter. |
| `callback b2EnqueueTaskCallback` | `raw` | boxdd_sys::ffi::b2EnqueueTaskCallback remains raw because its process-global or scheduler safety contract is not exposed as an ordinary Safe Rust callback adapter. |
| `callback b2FinishTaskCallback` | `raw` | boxdd_sys::ffi::b2FinishTaskCallback remains raw because its process-global or scheduler safety contract is not exposed as an ordinary Safe Rust callback adapter. |
| `callback b2FreeFcn` | `raw` | boxdd_sys::ffi::b2FreeFcn remains raw because its process-global or scheduler safety contract is not exposed as an ordinary Safe Rust callback adapter. |
| `callback b2FrictionCallback` | `deferred` | No reviewed public path proves a callable value in the exact native argument slot for b2FrictionCallback; reaching a callback-taking symbol, including passing None, is insufficient. |
| `callback b2LogFcn` | `raw` | boxdd_sys::ffi::b2LogFcn remains raw because its process-global or scheduler safety contract is not exposed as an ordinary Safe Rust callback adapter. |
| `callback b2RestitutionCallback` | `deferred` | No reviewed public path proves a callable value in the exact native argument slot for b2RestitutionCallback; reaching a callback-taking symbol, including passing None, is insufficient. |
| `callback b2TaskCallback` | `raw` | boxdd_sys::ffi::b2TaskCallback remains raw because its process-global or scheduler safety contract is not exposed as an ordinary Safe Rust callback adapter. |

## Non-Safe Capabilities

| Logical API | Status | Area | Rationale |
|---|---|---|---|
| `b2DynamicTree_SetCategoryBits` | `raw` | DynamicTree | The pinned upstream assertion aliases `b2TreeNode` leaf user data as `children.child1` and can trap for valid non-sentinel user data. The Safe `DynamicTree::replace_category_bits` capability creates an equivalent proxy, destroys the old proxy, and returns the new identity without calling this defective native function. |
| `b2InternalAssertFcn` | `raw` | Foundation | Raw only: upstream internal assert implementation, not a stable safe API surface. |
| `b2SetAllocator` | `raw` | Foundation | Raw only: process-global allocator hook needs a scoped startup guard before safe exposure. |
| `b2SetAssertFcn` | `raw` | Foundation | Raw only: process-global assert callback has panic/callback unwinding semantics that need a dedicated design. |
| `b2SetLengthUnitsPerMeter` | `raw` | Math | Raw during U1: process-global length scale mutation is not synchronized with safe Box2D activity; U6 will replace it with foundation initialization. |
| `b2SetLogFcn` | `raw` | Foundation | Raw only: process-global log callback needs a scoped callback guard before safe exposure. |
| `b2World_DumpMemoryStats` | `omitted` | World | Intentionally omitted: upstream writes fixed diagnostic output, so callers should use upstream diagnostics tooling explicitly. |
| `b2World_RebuildStaticTree` | `omitted` | World | Intentionally omitted: upstream labels this as internal testing support, not stable runtime API. |

## Maintenance

- Run `cargo run -p xtask -- api-coverage --check` to reject header, Rust path, evidence, recording-capability, wire-schema, ABI, or generated-document drift.
- Run `cargo run -p xtask -- api-coverage --write` only to regenerate this report after the structured contract has been reviewed.
- Run `cargo run -p xtask -- upstream-sync --check` to verify the manifest, gitlink, checkout, exact source-path inventory, reviewed recording-source Git objects, and all named artifacts.
