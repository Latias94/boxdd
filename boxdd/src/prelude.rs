pub use crate::{
    ApiError, ApiResult, Body, BodyBuilder, BodyDef, BodyType, Filter, Foundation,
    FoundationActivity, FoundationActivityError, FoundationAdapterIdentityField, FoundationConfig,
    FoundationDiagnostics, FoundationInitError, LocalManifold, LocalManifoldPoint,
    MAX_BODY_NAME_BYTES, MaterialMixInput, MixerRequirements, OwnedBody, OwnedHandleCounts,
    RawBodyDef, Recording, RecordingCapacity, RecordingSession, ReplayBodyView, ReplayConfig,
    ReplayEpoch, ReplayError, ReplayInfo, ReplayKeyframePolicy, ReplayKeyframeState,
    ReplayMalformedError, ReplayPlayer, ReplayQueryHitView, ReplayQueryKind, ReplayQueryView,
    ReplayStatus, ReplayView, ShapeCastInput, Snapshot, SnapshotImage, SnapshotLoad,
    SnapshotRestore, World, WorldBuilder, WorldDef, WorldHandle,
    debug_draw::{DebugDrawCmd, DebugDrawOptions, HexColor},
    dynamic_tree::{
        DynamicTree, TreeBoxCastInput, TreeCastControl, TreeProxyId, TreeRayCastInput, TreeStats,
    },
    events::{
        BodyMoveEvent, ContactBeginTouchEvent, ContactEndTouchEvent, ContactEvents,
        ContactHitEvent, JointEvent, SensorBeginTouchEvent, SensorEndTouchEvent, SensorEvents,
    },
    foundation, initialize_foundation,
    joints::{
        ConstraintTuning, DistanceJointDef, FilterJointDef, Joint, JointBase, JointType,
        MotorJointDef, OwnedJoint, PrismaticJointDef, RevoluteJointDef, WeldJointDef,
        WheelJointDef,
    },
    query::{
        Aabb, ClosestRayCastResult, CollisionPlane, MoverPlaneResult, Plane, PlaneSolverResult,
        QueryFilter, RayResult, clip_vector, solve_planes, try_clip_vector, try_solve_planes,
    },
    shapes::{
        self, Capsule, ChainSegment, Circle, MAX_POLYGON_VERTICES, OwnedShape, Polygon, Segment,
        Shape, ShapeDef, ShapeDefBuilder, ShapeType, SurfaceMaterial,
        chain::{Chain, ChainDef, ChainDefBuilder, ChainDefMaterialLayout, OwnedChain},
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
pub use crate::{FoundationAssertHook, FoundationLogHook, debug_draw::DebugDraw};

#[cfg(feature = "unchecked")]
pub use crate::unchecked::*;

#[cfg(feature = "glam")]
pub use crate::RotFromGlamError;

#[cfg(feature = "glam")]
pub use crate::TransformFromGlamError;

#[cfg(feature = "mint")]
pub use crate::RotFromMintError;

#[cfg(feature = "mint")]
pub use crate::TransformFromMintError;
