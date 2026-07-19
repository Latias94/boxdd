use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Error, Result,
    commands::api_coverage::{Classification, RecordingClass, RecordingCoverage},
    recording_ops::{RecordingOp, validate_operations},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RegistryEntry<'a> {
    function: &'a str,
    class: RecordingClass,
    operation: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OperationExemption<'a> {
    operation: &'a str,
    rationale: &'a str,
}

const LOGGED_OPERATIONS: &[(&str, &str)] = &[
    ("b2Body_ApplyAngularImpulse", "BodyApplyAngularImpulse"),
    ("b2Body_ApplyForce", "BodyApplyForce"),
    ("b2Body_ApplyForceToCenter", "BodyApplyForceToCenter"),
    ("b2Body_ApplyLinearImpulse", "BodyApplyLinearImpulse"),
    (
        "b2Body_ApplyLinearImpulseToCenter",
        "BodyApplyLinearImpulseToCenter",
    ),
    ("b2Body_ApplyMassFromShapes", "BodyApplyMassFromShapes"),
    ("b2Body_ApplyTorque", "BodyApplyTorque"),
    ("b2Body_ClearForces", "BodyClearForces"),
    ("b2Body_Disable", "BodyDisable"),
    ("b2Body_Enable", "BodyEnable"),
    ("b2Body_EnableContactEvents", "BodyEnableContactEvents"),
    ("b2Body_EnableHitEvents", "BodyEnableHitEvents"),
    ("b2Body_EnableSleep", "BodyEnableSleep"),
    ("b2Body_SetAngularDamping", "BodySetAngularDamping"),
    ("b2Body_SetAngularVelocity", "BodySetAngularVelocity"),
    ("b2Body_SetAwake", "BodySetAwake"),
    ("b2Body_SetBullet", "BodySetBullet"),
    ("b2Body_SetGravityScale", "BodySetGravityScale"),
    ("b2Body_SetLinearDamping", "BodySetLinearDamping"),
    ("b2Body_SetLinearVelocity", "BodySetLinearVelocity"),
    ("b2Body_SetMassData", "BodySetMassData"),
    ("b2Body_SetMotionLocks", "BodySetMotionLocks"),
    ("b2Body_SetName", "BodySetName"),
    ("b2Body_SetSleepThreshold", "BodySetSleepThreshold"),
    ("b2Body_SetTargetTransform", "BodySetTargetTransform"),
    ("b2Body_SetTransform", "BodySetTransform"),
    ("b2Body_SetType", "BodySetType"),
    ("b2Body_WakeTouching", "BodyWakeTouching"),
    ("b2Chain_SetSurfaceMaterial", "ChainSetSurfaceMaterial"),
    ("b2CreateBody", "CreateBody"),
    ("b2CreateCapsuleShape", "CreateCapsuleShape"),
    ("b2CreateChain", "CreateChain"),
    ("b2CreateCircleShape", "CreateCircleShape"),
    ("b2CreateDistanceJoint", "CreateDistanceJoint"),
    ("b2CreateFilterJoint", "CreateFilterJoint"),
    ("b2CreateMotorJoint", "CreateMotorJoint"),
    ("b2CreatePolygonShape", "CreatePolygonShape"),
    ("b2CreatePrismaticJoint", "CreatePrismaticJoint"),
    ("b2CreateRevoluteJoint", "CreateRevoluteJoint"),
    ("b2CreateSegmentShape", "CreateSegmentShape"),
    ("b2CreateWeldJoint", "CreateWeldJoint"),
    ("b2CreateWheelJoint", "CreateWheelJoint"),
    ("b2DestroyBody", "DestroyBody"),
    ("b2DestroyChain", "DestroyChain"),
    ("b2DestroyJoint", "DestroyJoint"),
    ("b2DestroyShape", "DestroyShape"),
    ("b2DistanceJoint_EnableLimit", "DistanceJointEnableLimit"),
    ("b2DistanceJoint_EnableMotor", "DistanceJointEnableMotor"),
    ("b2DistanceJoint_EnableSpring", "DistanceJointEnableSpring"),
    ("b2DistanceJoint_SetLength", "DistanceJointSetLength"),
    (
        "b2DistanceJoint_SetLengthRange",
        "DistanceJointSetLengthRange",
    ),
    (
        "b2DistanceJoint_SetMaxMotorForce",
        "DistanceJointSetMaxMotorForce",
    ),
    (
        "b2DistanceJoint_SetMotorSpeed",
        "DistanceJointSetMotorSpeed",
    ),
    (
        "b2DistanceJoint_SetSpringDampingRatio",
        "DistanceJointSetSpringDampingRatio",
    ),
    (
        "b2DistanceJoint_SetSpringForceRange",
        "DistanceJointSetSpringForceRange",
    ),
    (
        "b2DistanceJoint_SetSpringHertz",
        "DistanceJointSetSpringHertz",
    ),
    ("b2Joint_SetCollideConnected", "JointSetCollideConnected"),
    ("b2Joint_SetConstraintTuning", "JointSetConstraintTuning"),
    ("b2Joint_SetForceThreshold", "JointSetForceThreshold"),
    ("b2Joint_SetLocalFrameA", "JointSetLocalFrameA"),
    ("b2Joint_SetLocalFrameB", "JointSetLocalFrameB"),
    ("b2Joint_SetTorqueThreshold", "JointSetTorqueThreshold"),
    ("b2Joint_WakeBodies", "JointWakeBodies"),
    (
        "b2MotorJoint_SetAngularDampingRatio",
        "MotorJointSetAngularDampingRatio",
    ),
    ("b2MotorJoint_SetAngularHertz", "MotorJointSetAngularHertz"),
    (
        "b2MotorJoint_SetAngularVelocity",
        "MotorJointSetAngularVelocity",
    ),
    (
        "b2MotorJoint_SetLinearDampingRatio",
        "MotorJointSetLinearDampingRatio",
    ),
    ("b2MotorJoint_SetLinearHertz", "MotorJointSetLinearHertz"),
    (
        "b2MotorJoint_SetLinearVelocity",
        "MotorJointSetLinearVelocity",
    ),
    (
        "b2MotorJoint_SetMaxSpringForce",
        "MotorJointSetMaxSpringForce",
    ),
    (
        "b2MotorJoint_SetMaxSpringTorque",
        "MotorJointSetMaxSpringTorque",
    ),
    (
        "b2MotorJoint_SetMaxVelocityForce",
        "MotorJointSetMaxVelocityForce",
    ),
    (
        "b2MotorJoint_SetMaxVelocityTorque",
        "MotorJointSetMaxVelocityTorque",
    ),
    ("b2PrismaticJoint_EnableLimit", "PrismaticJointEnableLimit"),
    ("b2PrismaticJoint_EnableMotor", "PrismaticJointEnableMotor"),
    (
        "b2PrismaticJoint_EnableSpring",
        "PrismaticJointEnableSpring",
    ),
    ("b2PrismaticJoint_SetLimits", "PrismaticJointSetLimits"),
    (
        "b2PrismaticJoint_SetMaxMotorForce",
        "PrismaticJointSetMaxMotorForce",
    ),
    (
        "b2PrismaticJoint_SetMotorSpeed",
        "PrismaticJointSetMotorSpeed",
    ),
    (
        "b2PrismaticJoint_SetSpringDampingRatio",
        "PrismaticJointSetSpringDampingRatio",
    ),
    (
        "b2PrismaticJoint_SetSpringHertz",
        "PrismaticJointSetSpringHertz",
    ),
    (
        "b2PrismaticJoint_SetTargetTranslation",
        "PrismaticJointSetTargetTranslation",
    ),
    ("b2RevoluteJoint_EnableLimit", "RevoluteJointEnableLimit"),
    ("b2RevoluteJoint_EnableMotor", "RevoluteJointEnableMotor"),
    ("b2RevoluteJoint_EnableSpring", "RevoluteJointEnableSpring"),
    ("b2RevoluteJoint_SetLimits", "RevoluteJointSetLimits"),
    (
        "b2RevoluteJoint_SetMaxMotorTorque",
        "RevoluteJointSetMaxMotorTorque",
    ),
    (
        "b2RevoluteJoint_SetMotorSpeed",
        "RevoluteJointSetMotorSpeed",
    ),
    (
        "b2RevoluteJoint_SetSpringDampingRatio",
        "RevoluteJointSetSpringDampingRatio",
    ),
    (
        "b2RevoluteJoint_SetSpringHertz",
        "RevoluteJointSetSpringHertz",
    ),
    (
        "b2RevoluteJoint_SetTargetAngle",
        "RevoluteJointSetTargetAngle",
    ),
    ("b2Shape_ApplyWind", "ShapeApplyWind"),
    ("b2Shape_EnableContactEvents", "ShapeEnableContactEvents"),
    ("b2Shape_EnableHitEvents", "ShapeEnableHitEvents"),
    ("b2Shape_EnablePreSolveEvents", "ShapeEnablePreSolveEvents"),
    ("b2Shape_EnableSensorEvents", "ShapeEnableSensorEvents"),
    ("b2Shape_RayCast", "ShapeRayCast"),
    ("b2Shape_SetCapsule", "ShapeSetCapsule"),
    ("b2Shape_SetCircle", "ShapeSetCircle"),
    ("b2Shape_SetDensity", "ShapeSetDensity"),
    ("b2Shape_SetFilter", "ShapeSetFilter"),
    ("b2Shape_SetFriction", "ShapeSetFriction"),
    ("b2Shape_SetPolygon", "ShapeSetPolygon"),
    ("b2Shape_SetRestitution", "ShapeSetRestitution"),
    ("b2Shape_SetSegment", "ShapeSetSegment"),
    ("b2Shape_SetSurfaceMaterial", "ShapeSetSurfaceMaterial"),
    ("b2Shape_SetUserMaterial", "ShapeSetUserMaterial"),
    ("b2Shape_TestPoint", "ShapeTestPoint"),
    (
        "b2WeldJoint_SetAngularDampingRatio",
        "WeldJointSetAngularDampingRatio",
    ),
    ("b2WeldJoint_SetAngularHertz", "WeldJointSetAngularHertz"),
    (
        "b2WeldJoint_SetLinearDampingRatio",
        "WeldJointSetLinearDampingRatio",
    ),
    ("b2WeldJoint_SetLinearHertz", "WeldJointSetLinearHertz"),
    ("b2WheelJoint_EnableLimit", "WheelJointEnableLimit"),
    ("b2WheelJoint_EnableMotor", "WheelJointEnableMotor"),
    ("b2WheelJoint_EnableSpring", "WheelJointEnableSpring"),
    ("b2WheelJoint_SetLimits", "WheelJointSetLimits"),
    (
        "b2WheelJoint_SetMaxMotorTorque",
        "WheelJointSetMaxMotorTorque",
    ),
    ("b2WheelJoint_SetMotorSpeed", "WheelJointSetMotorSpeed"),
    (
        "b2WheelJoint_SetSpringDampingRatio",
        "WheelJointSetSpringDampingRatio",
    ),
    ("b2WheelJoint_SetSpringHertz", "WheelJointSetSpringHertz"),
    ("b2World_CastMover", "QueryCastMover"),
    ("b2World_CastRay", "QueryCastRay"),
    ("b2World_CastRayClosest", "QueryCastRayClosest"),
    ("b2World_CastShape", "QueryCastShape"),
    ("b2World_CollideMover", "QueryCollideMover"),
    ("b2World_EnableContinuous", "WorldEnableContinuous"),
    ("b2World_EnableSleeping", "WorldEnableSleeping"),
    ("b2World_EnableSpeculative", "WorldEnableSpeculative"),
    ("b2World_EnableWarmStarting", "WorldEnableWarmStarting"),
    ("b2World_Explode", "WorldExplode"),
    ("b2World_OverlapAABB", "QueryOverlapAABB"),
    ("b2World_OverlapShape", "QueryOverlapShape"),
    ("b2World_SetContactTuning", "WorldSetContactTuning"),
    ("b2World_SetGravity", "WorldSetGravity"),
    ("b2World_SetHitEventThreshold", "WorldSetHitEventThreshold"),
    (
        "b2World_SetMaximumLinearSpeed",
        "WorldSetMaximumLinearSpeed",
    ),
    (
        "b2World_SetRestitutionThreshold",
        "WorldSetRestitutionThreshold",
    ),
    ("b2World_Step", "Step"),
];

const LOGGED_QUERY_FUNCTIONS: &[&str] = &[
    "b2Shape_RayCast",
    "b2Shape_TestPoint",
    "b2World_CastMover",
    "b2World_CastRay",
    "b2World_CastRayClosest",
    "b2World_CastShape",
    "b2World_CollideMover",
    "b2World_OverlapAABB",
    "b2World_OverlapShape",
];

const PURE_WORLDLESS: &[&str] = &[
    "b2Atan2",
    "b2ClipVector",
    "b2CollideCapsuleAndCircle",
    "b2CollideCapsules",
    "b2CollideCircles",
    "b2CollidePolygonAndCapsule",
    "b2CollidePolygonAndCircle",
    "b2CollidePolygons",
    "b2CollideSegmentAndCapsule",
    "b2CollideSegmentAndCircle",
    "b2CollideSegmentAndPolygon",
    "b2ComputeCapsuleAABB",
    "b2ComputeCapsuleMass",
    "b2ComputeCircleAABB",
    "b2ComputeCircleMass",
    "b2ComputeCosSin",
    "b2ComputeHull",
    "b2ComputePolygonAABB",
    "b2ComputePolygonMass",
    "b2ComputeRotationBetweenUnitVectors",
    "b2ComputeSegmentAABB",
    "b2Contact_GetData",
    "b2Contact_IsValid",
    "b2DefaultBodyDef",
    "b2DefaultDebugDraw",
    "b2DefaultExplosionDef",
    "b2DefaultFilter",
    "b2DefaultQueryFilter",
    "b2DefaultSurfaceMaterial",
    "b2DefaultWorldDef",
    "b2DynamicTree_Create",
    "b2DynamicTree_CreateProxy",
    "b2DynamicTree_Destroy",
    "b2DynamicTree_DestroyProxy",
    "b2DynamicTree_EnlargeProxy",
    "b2DynamicTree_GetAABB",
    "b2DynamicTree_GetAreaRatio",
    "b2DynamicTree_GetByteCount",
    "b2DynamicTree_GetCategoryBits",
    "b2DynamicTree_GetHeight",
    "b2DynamicTree_GetProxyCount",
    "b2DynamicTree_GetRootBounds",
    "b2DynamicTree_GetUserData",
    "b2DynamicTree_MoveProxy",
    "b2DynamicTree_Query",
    "b2DynamicTree_QueryAll",
    "b2DynamicTree_RayCast",
    "b2DynamicTree_Rebuild",
    "b2DynamicTree_SetCategoryBits",
    "b2DynamicTree_Validate",
    "b2DynamicTree_ValidateNoEnlarged",
    "b2GetByteCount",
    "b2GetLengthUnitsPerMeter",
    "b2GetMilliseconds",
    "b2GetMillisecondsAndReset",
    "b2GetSweepTransform",
    "b2GetTicks",
    "b2GetVersion",
    "b2Hash",
    "b2IsValidAABB",
    "b2IsValidFloat",
    "b2IsValidPlane",
    "b2IsValidRay",
    "b2IsValidRotation",
    "b2IsValidTransform",
    "b2IsValidVec2",
    "b2MakeBox",
    "b2MakeOffsetBox",
    "b2MakeOffsetPolygon",
    "b2MakeOffsetProxy",
    "b2MakeOffsetRoundedBox",
    "b2MakeOffsetRoundedPolygon",
    "b2MakePolygon",
    "b2MakeProxy",
    "b2MakeRoundedBox",
    "b2MakeSquare",
    "b2PointInCapsule",
    "b2PointInCircle",
    "b2PointInPolygon",
    "b2RayCastCapsule",
    "b2RayCastCircle",
    "b2RayCastPolygon",
    "b2RayCastSegment",
    "b2SegmentDistance",
    "b2SolvePlanes",
    "b2TimeOfImpact",
    "b2TransformPolygon",
    "b2ValidateHull",
    "b2Yield",
];

const READ_ONLY: &[&str] = &[
    "b2Body_ComputeAABB",
    "b2Body_GetAngularDamping",
    "b2Body_GetAngularVelocity",
    "b2Body_GetContactCapacity",
    "b2Body_GetContactData",
    "b2Body_GetGravityScale",
    "b2Body_GetJointCount",
    "b2Body_GetJoints",
    "b2Body_GetLinearDamping",
    "b2Body_GetLinearVelocity",
    "b2Body_GetLocalCenter",
    "b2Body_GetLocalCenterOfMass",
    "b2Body_GetLocalPoint",
    "b2Body_GetLocalPointVelocity",
    "b2Body_GetLocalVector",
    "b2Body_GetMass",
    "b2Body_GetMassData",
    "b2Body_GetMotionLocks",
    "b2Body_GetName",
    "b2Body_GetPosition",
    "b2Body_GetRotation",
    "b2Body_GetRotationalInertia",
    "b2Body_GetShapeCount",
    "b2Body_GetShapes",
    "b2Body_GetSleepThreshold",
    "b2Body_GetTransform",
    "b2Body_GetType",
    "b2Body_GetUserData",
    "b2Body_GetWorld",
    "b2Body_GetWorldCenter",
    "b2Body_GetWorldCenterOfMass",
    "b2Body_GetWorldPoint",
    "b2Body_GetWorldPointVelocity",
    "b2Body_GetWorldVector",
    "b2Body_IsAwake",
    "b2Body_IsBullet",
    "b2Body_IsEnabled",
    "b2Body_IsSleepEnabled",
    "b2Body_IsValid",
    "b2Chain_GetSegmentCount",
    "b2Chain_GetSegments",
    "b2Chain_GetSurfaceMaterial",
    "b2Chain_GetSurfaceMaterialCount",
    "b2Chain_GetWorld",
    "b2Chain_IsValid",
    "b2CollideChainSegmentAndCapsule",
    "b2CollideChainSegmentAndCircle",
    "b2CollideChainSegmentAndPolygon",
    "b2DefaultChainDef",
    "b2DefaultDistanceJointDef",
    "b2DefaultFilterJointDef",
    "b2DefaultMotorJointDef",
    "b2DefaultPrismaticJointDef",
    "b2DefaultRevoluteJointDef",
    "b2DefaultShapeDef",
    "b2DefaultWeldJointDef",
    "b2DefaultWheelJointDef",
    "b2DistanceJoint_GetCurrentLength",
    "b2DistanceJoint_GetLength",
    "b2DistanceJoint_GetMaxLength",
    "b2DistanceJoint_GetMaxMotorForce",
    "b2DistanceJoint_GetMinLength",
    "b2DistanceJoint_GetMotorForce",
    "b2DistanceJoint_GetMotorSpeed",
    "b2DistanceJoint_GetSpringDampingRatio",
    "b2DistanceJoint_GetSpringForceRange",
    "b2DistanceJoint_GetSpringHertz",
    "b2DistanceJoint_IsLimitEnabled",
    "b2DistanceJoint_IsMotorEnabled",
    "b2DistanceJoint_IsSpringEnabled",
    "b2DynamicTree_BoxCast",
    "b2DynamicTree_ShapeCast",
    "b2Joint_GetAngularSeparation",
    "b2Joint_GetBodyA",
    "b2Joint_GetBodyB",
    "b2Joint_GetCollideConnected",
    "b2Joint_GetConstraintForce",
    "b2Joint_GetConstraintTorque",
    "b2Joint_GetConstraintTuning",
    "b2Joint_GetForceThreshold",
    "b2Joint_GetLinearSeparation",
    "b2Joint_GetLocalFrameA",
    "b2Joint_GetLocalFrameB",
    "b2Joint_GetTorqueThreshold",
    "b2Joint_GetType",
    "b2Joint_GetUserData",
    "b2Joint_GetWorld",
    "b2Joint_IsValid",
    "b2MotorJoint_GetAngularDampingRatio",
    "b2MotorJoint_GetAngularHertz",
    "b2MotorJoint_GetAngularVelocity",
    "b2MotorJoint_GetLinearDampingRatio",
    "b2MotorJoint_GetLinearHertz",
    "b2MotorJoint_GetLinearVelocity",
    "b2MotorJoint_GetMaxSpringForce",
    "b2MotorJoint_GetMaxSpringTorque",
    "b2MotorJoint_GetMaxVelocityForce",
    "b2MotorJoint_GetMaxVelocityTorque",
    "b2PrismaticJoint_GetLowerLimit",
    "b2PrismaticJoint_GetMaxMotorForce",
    "b2PrismaticJoint_GetMotorForce",
    "b2PrismaticJoint_GetMotorSpeed",
    "b2PrismaticJoint_GetSpeed",
    "b2PrismaticJoint_GetSpringDampingRatio",
    "b2PrismaticJoint_GetSpringHertz",
    "b2PrismaticJoint_GetTargetTranslation",
    "b2PrismaticJoint_GetTranslation",
    "b2PrismaticJoint_GetUpperLimit",
    "b2PrismaticJoint_IsLimitEnabled",
    "b2PrismaticJoint_IsMotorEnabled",
    "b2PrismaticJoint_IsSpringEnabled",
    "b2RevoluteJoint_GetAngle",
    "b2RevoluteJoint_GetLowerLimit",
    "b2RevoluteJoint_GetMaxMotorTorque",
    "b2RevoluteJoint_GetMotorSpeed",
    "b2RevoluteJoint_GetMotorTorque",
    "b2RevoluteJoint_GetSpringDampingRatio",
    "b2RevoluteJoint_GetSpringHertz",
    "b2RevoluteJoint_GetTargetAngle",
    "b2RevoluteJoint_GetUpperLimit",
    "b2RevoluteJoint_IsLimitEnabled",
    "b2RevoluteJoint_IsMotorEnabled",
    "b2RevoluteJoint_IsSpringEnabled",
    "b2ShapeCast",
    "b2ShapeCastCapsule",
    "b2ShapeCastCircle",
    "b2ShapeCastPolygon",
    "b2ShapeCastSegment",
    "b2ShapeDistance",
    "b2Shape_AreContactEventsEnabled",
    "b2Shape_AreHitEventsEnabled",
    "b2Shape_ArePreSolveEventsEnabled",
    "b2Shape_AreSensorEventsEnabled",
    "b2Shape_ComputeMassData",
    "b2Shape_GetAABB",
    "b2Shape_GetBody",
    "b2Shape_GetCapsule",
    "b2Shape_GetChainSegment",
    "b2Shape_GetCircle",
    "b2Shape_GetClosestPoint",
    "b2Shape_GetContactCapacity",
    "b2Shape_GetContactData",
    "b2Shape_GetDensity",
    "b2Shape_GetFilter",
    "b2Shape_GetFriction",
    "b2Shape_GetParentChain",
    "b2Shape_GetPolygon",
    "b2Shape_GetRestitution",
    "b2Shape_GetSegment",
    "b2Shape_GetSensorCapacity",
    "b2Shape_GetSensorData",
    "b2Shape_GetSurfaceMaterial",
    "b2Shape_GetType",
    "b2Shape_GetUserData",
    "b2Shape_GetUserMaterial",
    "b2Shape_GetWorld",
    "b2Shape_IsSensor",
    "b2Shape_IsValid",
    "b2WeldJoint_GetAngularDampingRatio",
    "b2WeldJoint_GetAngularHertz",
    "b2WeldJoint_GetLinearDampingRatio",
    "b2WeldJoint_GetLinearHertz",
    "b2WheelJoint_GetLowerLimit",
    "b2WheelJoint_GetMaxMotorTorque",
    "b2WheelJoint_GetMotorSpeed",
    "b2WheelJoint_GetMotorTorque",
    "b2WheelJoint_GetSpringDampingRatio",
    "b2WheelJoint_GetSpringHertz",
    "b2WheelJoint_GetUpperLimit",
    "b2WheelJoint_IsLimitEnabled",
    "b2WheelJoint_IsMotorEnabled",
    "b2WheelJoint_IsSpringEnabled",
    "b2World_Draw",
    "b2World_GetAwakeBodyCount",
    "b2World_GetBodyEvents",
    "b2World_GetContactEvents",
    "b2World_GetCounters",
    "b2World_GetGravity",
    "b2World_GetHitEventThreshold",
    "b2World_GetJointEvents",
    "b2World_GetMaximumLinearSpeed",
    "b2World_GetProfile",
    "b2World_GetRestitutionThreshold",
    "b2World_GetSensorEvents",
    "b2World_GetUserData",
    "b2World_IsContinuousEnabled",
    "b2World_IsSleepingEnabled",
    "b2World_IsValid",
    "b2World_IsWarmStartingEnabled",
];

const UNLOGGED_MUTATION_FORBIDDEN: &[&str] = &[
    "b2Body_SetUserData",
    "b2CreateWorld",
    "b2Joint_SetUserData",
    "b2Shape_SetUserData",
    "b2World_SetUserData",
];

const CALLBACK_INSTALL_UNSUPPORTED: &[&str] = &[
    "b2World_SetCustomFilterCallback",
    "b2World_SetPreSolveCallback",
];

const REPLAY_MIXER_LIFECYCLE: &[&str] = &[
    "b2World_SetFrictionCallback",
    "b2World_SetRestitutionCallback",
];

const RECORDING_LIFECYCLE: &[&str] = &[];

const SNAPSHOT_LIFECYCLE: &[&str] = &[];

const VERSIONED_FUNCTION_ALIASES: &[&[&str]] = &[
    &["b2Body_GetLocalCenterOfMass", "b2Body_GetLocalCenter"],
    &["b2Body_GetWorldCenterOfMass", "b2Body_GetWorldCenter"],
    &["b2DynamicTree_ShapeCast", "b2DynamicTree_BoxCast"],
];

const OPERATION_EXEMPTIONS: &[OperationExemption<'static>] = &[
    OperationExemption {
        operation: "WorldSetContactRecycleDistance",
        rationale: "The target operation has no public function in the active header revision.",
    },
    OperationExemption {
        operation: "WorldRebuildStaticTree",
        rationale: "The target operation has no public function in the active header revision.",
    },
    OperationExemption {
        operation: "BodyEnableContactRecycling",
        rationale: "The target operation has no public function in the active header revision.",
    },
    OperationExemption {
        operation: "CreateChainSegmentShape",
        rationale: "The target operation has no public function in the active header revision.",
    },
    OperationExemption {
        operation: "ShapeSetChainSegment",
        rationale: "The target operation has no public function in the active header revision.",
    },
    OperationExemption {
        operation: "StateHash",
        rationale: "The producer emits this protocol metadata operation without a public C entry point.",
    },
    OperationExemption {
        operation: "RecordingBounds",
        rationale: "The producer emits this protocol metadata operation without a public C entry point.",
    },
];

fn production_entries() -> Vec<RegistryEntry<'static>> {
    let mut entries = LOGGED_OPERATIONS
        .iter()
        .map(|(function, operation)| RegistryEntry {
            function,
            class: if LOGGED_QUERY_FUNCTIONS.contains(function) {
                RecordingClass::LoggedQuery
            } else {
                RecordingClass::LoggedMutation
            },
            operation: Some(operation),
        })
        .collect::<Vec<_>>();
    entries.push(RegistryEntry {
        function: "b2DestroyWorld",
        class: RecordingClass::WorldDestroyTerminal,
        operation: Some("DestroyWorld"),
    });
    for (functions, class) in [
        (PURE_WORLDLESS, RecordingClass::PureWorldless),
        (READ_ONLY, RecordingClass::ReadOnly),
        (
            CALLBACK_INSTALL_UNSUPPORTED,
            RecordingClass::CallbackInstallUnsupported,
        ),
        (REPLAY_MIXER_LIFECYCLE, RecordingClass::ReplayMixerLifecycle),
        (RECORDING_LIFECYCLE, RecordingClass::RecordingLifecycle),
        (SNAPSHOT_LIFECYCLE, RecordingClass::SnapshotLifecycle),
        (
            UNLOGGED_MUTATION_FORBIDDEN,
            RecordingClass::UnloggedMutationForbidden,
        ),
    ] {
        entries.extend(functions.iter().map(|function| RegistryEntry {
            function,
            class,
            operation: None,
        }));
    }
    entries
}

pub(super) fn validate_registry(
    safe_functions: &BTreeSet<&str>,
    known_functions: &BTreeSet<&str>,
    operations: &[RecordingOp],
) -> Result<()> {
    let entries = production_entries_for(known_functions)?;
    validate_registry_entries(
        safe_functions,
        known_functions,
        operations,
        &entries,
        OPERATION_EXEMPTIONS,
    )
}

fn production_entries_for(known_functions: &BTreeSet<&str>) -> Result<Vec<RegistryEntry<'static>>> {
    for aliases in VERSIONED_FUNCTION_ALIASES {
        let present = aliases
            .iter()
            .filter(|function| known_functions.contains(**function))
            .copied()
            .collect::<Vec<_>>();
        if present.len() != 1 {
            return Err(Error::message(format!(
                "recording registry expected exactly one versioned function from {aliases:?}, observed {present:?}"
            )));
        }
    }
    Ok(production_entries()
        .into_iter()
        .filter(|entry| {
            !VERSIONED_FUNCTION_ALIASES
                .iter()
                .any(|aliases| aliases.contains(&entry.function))
                || known_functions.contains(entry.function)
        })
        .collect())
}

fn validate_registry_entries(
    safe_functions: &BTreeSet<&str>,
    known_functions: &BTreeSet<&str>,
    operations: &[RecordingOp],
    entries: &[RegistryEntry<'_>],
    exemptions: &[OperationExemption<'_>],
) -> Result<()> {
    validate_operations(operations)?;
    let mut errors = Vec::new();
    let operations_by_name = operations
        .iter()
        .map(|operation| (operation.name.as_str(), operation))
        .collect::<BTreeMap<_, _>>();
    let mut registry_functions = BTreeSet::new();
    let mut mapped_operations = BTreeSet::new();

    for entry in entries {
        if !registry_functions.insert(entry.function) {
            errors.push(format!(
                "recording registry classifies function `{}` more than once",
                entry.function
            ));
        }
        let requires_operation = matches!(
            entry.class,
            RecordingClass::LoggedMutation
                | RecordingClass::LoggedQuery
                | RecordingClass::WorldDestroyTerminal
        );
        if requires_operation != entry.operation.is_some() {
            errors.push(format!(
                "recording registry function `{}` has an invalid operation binding for {:?}",
                entry.function, entry.class
            ));
        }
        if let Some(operation) = entry.operation {
            if !mapped_operations.insert(operation) {
                errors.push(format!(
                    "recording operation `{operation}` is mapped by more than one function"
                ));
            }
            if !operations_by_name.contains_key(operation) {
                errors.push(format!(
                    "recording registry function `{}` references unknown operation `{operation}`",
                    entry.function
                ));
            }
        }
    }

    for function in safe_functions.difference(&registry_functions) {
        errors.push(format!(
            "safe function `{function}` has no explicit recording registry entry"
        ));
    }
    for function in registry_functions.difference(known_functions) {
        errors.push(format!(
            "recording registry contains unknown or stale function `{function}`"
        ));
    }

    let mut exempt_operations = BTreeSet::new();
    for exemption in exemptions {
        if !exempt_operations.insert(exemption.operation) {
            errors.push(format!(
                "recording operation `{}` has duplicate exemptions",
                exemption.operation
            ));
        }
        if exemption.rationale.trim().len() < 24 {
            errors.push(format!(
                "recording operation `{}` exemption needs a specific rationale",
                exemption.operation
            ));
        }
        if !operations_by_name.contains_key(exemption.operation) {
            errors.push(format!(
                "recording registry contains stale operation exemption `{}`",
                exemption.operation
            ));
        }
        if mapped_operations.contains(exemption.operation) {
            errors.push(format!(
                "recording operation `{}` is both mapped and exempt",
                exemption.operation
            ));
        }
    }

    let accounted_operations = mapped_operations
        .union(&exempt_operations)
        .copied()
        .collect::<BTreeSet<_>>();
    for operation in operations_by_name.keys() {
        if !accounted_operations.contains(operation) {
            errors.push(format!(
                "recording operation `{operation}` is neither mapped nor explicitly exempt"
            ));
        }
    }
    for operation in accounted_operations {
        if !operations_by_name.contains_key(operation) {
            errors.push(format!(
                "recording registry references stale operation `{operation}`"
            ));
        }
    }

    let query_functions = LOGGED_QUERY_FUNCTIONS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if query_functions.len() != LOGGED_QUERY_FUNCTIONS.len() {
        errors.push("logged query function registry contains duplicates".to_owned());
    }
    for function in query_functions {
        if registry_functions.contains(function)
            && !entries.iter().any(|entry| {
                entry.function == function && entry.class == RecordingClass::LoggedQuery
            })
        {
            errors.push(format!(
                "logged query function `{function}` is absent from the operation registry"
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

pub(super) fn expected(
    logical_name: &str,
    classification: Classification,
    operations: &[RecordingOp],
) -> Option<RecordingCoverage> {
    if classification != Classification::Safe {
        return None;
    }
    let entry = production_entries()
        .into_iter()
        .find(|entry| entry.function == logical_name)?;
    Some(RecordingCoverage {
        class: entry.class,
        opcode: entry
            .operation
            .and_then(|operation| opcode_for(operation, operations)),
    })
}

pub(super) fn is_explicitly_classified(logical_name: &str, classification: Classification) -> bool {
    classification != Classification::Safe
        || production_entries()
            .iter()
            .any(|entry| entry.function == logical_name)
}

fn opcode_for(name: &str, operations: &[RecordingOp]) -> Option<u8> {
    operations
        .iter()
        .find(|operation| operation.name == name)
        .map(|operation| operation.opcode)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operations() -> Vec<RecordingOp> {
        vec![RecordingOp {
            opcode: 0x20,
            name: "BodySetTransform".to_owned(),
            return_tag: "RET_NONE".to_owned(),
            arguments: Vec::new(),
        }]
    }

    #[test]
    fn registry_is_independent_of_editable_area_or_name_heuristics() {
        let logged = expected("b2Body_SetTransform", Classification::Safe, &operations()).unwrap();
        assert_eq!(logged.class, RecordingClass::LoggedMutation);
        assert_eq!(logged.opcode, Some(0x20));

        let read = expected("b2Body_GetTransform", Classification::Safe, &operations()).unwrap();
        assert_eq!(read.class, RecordingClass::ReadOnly);

        let unknown = expected("b2Body_LooksReadOnly", Classification::Safe, &operations());
        assert_eq!(unknown, None);
        assert!(!is_explicitly_classified(
            "b2Body_LooksReadOnly",
            Classification::Safe
        ));
    }

    #[test]
    fn versioned_function_aliases_select_exactly_one_header_generation() {
        let active = BTreeSet::from([
            "b2Body_GetLocalCenterOfMass",
            "b2Body_GetWorldCenterOfMass",
            "b2DynamicTree_ShapeCast",
        ]);
        let active_entries = production_entries_for(&active).expect("active aliases");
        assert!(
            active_entries
                .iter()
                .any(|entry| entry.function == "b2Body_GetLocalCenterOfMass")
        );
        assert!(
            !active_entries
                .iter()
                .any(|entry| entry.function == "b2Body_GetLocalCenter")
        );

        let target = BTreeSet::from([
            "b2Body_GetLocalCenter",
            "b2Body_GetWorldCenter",
            "b2DynamicTree_BoxCast",
        ]);
        let target_entries = production_entries_for(&target).expect("target aliases");
        assert!(
            target_entries
                .iter()
                .any(|entry| entry.function == "b2Body_GetLocalCenter")
        );
        assert!(
            !target_entries
                .iter()
                .any(|entry| entry.function == "b2Body_GetLocalCenterOfMass")
        );

        let mut ambiguous = target;
        ambiguous.insert("b2Body_GetLocalCenterOfMass");
        let error = production_entries_for(&ambiguous)
            .expect_err("two aliases in one header must fail closed");
        assert!(error.to_string().contains("exactly one versioned function"));
    }

    #[test]
    fn registry_validation_rejects_missing_duplicate_stale_and_unclassified_entries() {
        let operations = vec![
            RecordingOp {
                opcode: 0x20,
                name: "BodySetTransform".to_owned(),
                return_tag: "RET_NONE".to_owned(),
                arguments: Vec::new(),
            },
            RecordingOp {
                opcode: 0xF1,
                name: "StateHash".to_owned(),
                return_tag: "RET_NONE".to_owned(),
                arguments: Vec::new(),
            },
        ];
        let safe = BTreeSet::from(["b2Body_SetTransform", "b2Body_GetTransform"]);
        let known = safe.clone();
        let entries = [
            RegistryEntry {
                function: "b2Body_SetTransform",
                class: RecordingClass::LoggedMutation,
                operation: Some("BodySetTranform"),
            },
            RegistryEntry {
                function: "b2Body_SetTransform",
                class: RecordingClass::ReadOnly,
                operation: None,
            },
            RegistryEntry {
                function: "b2Stale",
                class: RecordingClass::ReadOnly,
                operation: None,
            },
        ];
        let exemptions = [OperationExemption {
            operation: "MissingMetadata",
            rationale: "This deliberately stale operation has a test-only rationale.",
        }];

        let error = validate_registry_entries(&safe, &known, &operations, &entries, &exemptions)
            .expect_err("invalid registry must fail closed")
            .to_string();

        assert!(error.contains("more than once"));
        assert!(error.contains("unknown operation `BodySetTranform`"));
        assert!(error.contains("safe function `b2Body_GetTransform`"));
        assert!(error.contains("unknown or stale function `b2Stale`"));
        assert!(error.contains("stale operation exemption `MissingMetadata`"));
        assert!(error.contains("operation `StateHash` is neither mapped nor explicitly exempt"));
    }

    #[test]
    fn registry_validation_rejects_duplicate_opcode_and_mapped_exemption() {
        let duplicate_operations = vec![
            RecordingOp {
                opcode: 0x20,
                name: "BodySetTransform".to_owned(),
                return_tag: "RET_NONE".to_owned(),
                arguments: Vec::new(),
            },
            RecordingOp {
                opcode: 0x20,
                name: "BodySetVelocity".to_owned(),
                return_tag: "RET_NONE".to_owned(),
                arguments: Vec::new(),
            },
        ];
        let error = validate_registry_entries(
            &BTreeSet::new(),
            &BTreeSet::new(),
            &duplicate_operations,
            &[],
            &[],
        )
        .expect_err("duplicate opcode must fail");
        assert!(error.to_string().contains("duplicate recording opcode"));

        let operations = operations();
        let entries = [RegistryEntry {
            function: "b2Body_SetTransform",
            class: RecordingClass::LoggedMutation,
            operation: Some("BodySetTransform"),
        }];
        let exemptions = [OperationExemption {
            operation: "BodySetTransform",
            rationale: "This deliberately conflicting exemption is only for validation testing.",
        }];
        let error = validate_registry_entries(
            &BTreeSet::from(["b2Body_SetTransform"]),
            &BTreeSet::from(["b2Body_SetTransform"]),
            &operations,
            &entries,
            &exemptions,
        )
        .expect_err("mapped operation exemption must fail");
        assert!(error.to_string().contains("both mapped and exempt"));
    }

    #[test]
    fn registry_accepts_a_known_non_safe_function() {
        let entries = [RegistryEntry {
            function: "b2RawOnly",
            class: RecordingClass::LoggedMutation,
            operation: Some("BodySetTransform"),
        }];
        let operations = operations();

        validate_registry_entries(
            &BTreeSet::new(),
            &BTreeSet::from(["b2RawOnly"]),
            &operations,
            &entries,
            &[],
        )
        .expect("a known Raw function may still require a recording classification");
    }
}
