pub use crate::{
    Body, BodyBuilder, BodyDef, BodyType, Error, Filter, Foundation, FoundationActivity,
    FoundationActivityError, FoundationAdapterIdentityField, FoundationConfig,
    FoundationDiagnostics, FoundationInitError, LocalManifold, LocalManifoldPoint,
    MAX_BODY_NAME_BYTES, MaterialMixInput, MixerId, MixerIdentities, PreparedSnapshotRestore,
    Recording, RecordingLimits, RecordingSession, ReplayBodyView, ReplayConfig, ReplayEpoch,
    ReplayInfo, ReplayKeyframePolicy, ReplayKeyframeState, ReplayPlayer, ReplayQueryHitView,
    ReplayQueryKind, ReplayQueryView, ReplayStatus, ReplayView, Result, ShapeCastInput, ShapeProxy,
    Snapshot, SnapshotRestore, World, WorldBuilder, WorldDef,
    debug_draw::{DebugDrawCmd, DebugDrawOptions, HexColor},
    dynamic_tree::{
        DynamicTree, TreeBoxCastInput, TreeCastControl, TreeProxyId, TreeRayCastInput, TreeStats,
    },
    events::{
        BodyEvents, BodyMoveEvent, CompletedStep, ContactBeginTouchEvent, ContactEndTouchEvent,
        ContactEvents, ContactEventsView, ContactHitEvent, JointEvent, JointEvents,
        SensorBeginTouchEvent, SensorEndTouchEvent, SensorEvents, SensorEventsView,
        StepEventsSnapshot,
    },
    joints::{
        ConstraintTuning, DistanceJoint, DistanceJointDef, FilterJoint, FilterJointDef, Joint,
        JointBase, JointType, MotorJoint, MotorJointDef, PrismaticJoint, PrismaticJointDef,
        RevoluteJoint, RevoluteJointDef, WeldJoint, WeldJointDef, WheelJoint, WheelJointDef,
    },
    query::{
        Aabb, ClosestRayCastResult, CollisionPlane, MoverPlaneResult, Plane, PlaneSolverResult,
        Query, QueryFilter, RayResult, clip_vector, solve_planes,
    },
    shapes::{
        self, Capsule, ChainSegment, Circle, MAX_POLYGON_VERTICES, Polygon, Segment, Shape,
        ShapeDef, ShapeDefBuilder, ShapeType, SurfaceMaterial,
        chain::{Chain, ChainDef, ChainDefBuilder, ChainDefMaterialLayout},
    },
    types::{
        BodyId, ChainId, ContactData, ContactId, JointId, MAX_MANIFOLD_POINTS, Manifold,
        ManifoldPoint, MassData, MotionLocks, Position, PositionToLocalError, ShapeId, Vec2,
        WorldCastOutput, WorldScalar, WorldTransform, WorldTransformFromInteropError,
    },
    world::{B2_MAX_WORKERS, Counters, Profile, WorkerCount, WorldCapacity},
    world_extras::ExplosionDef,
    {Rot, Transform},
};

#[cfg(not(target_arch = "wasm32"))]
pub use crate::query::{MoverQueryBuffer, RayQueryBuffer, ShapeQueryBuffer};

#[cfg(not(target_arch = "wasm32"))]
pub use crate::{FoundationAssertHook, FoundationLogHook, debug_draw::DebugDraw};

#[cfg(feature = "glam")]
pub use crate::RotFromGlamError;

#[cfg(feature = "glam")]
pub use crate::TransformFromGlamError;

#[cfg(feature = "mint")]
pub use crate::RotFromMintError;

#[cfg(feature = "mint")]
pub use crate::TransformFromMintError;
