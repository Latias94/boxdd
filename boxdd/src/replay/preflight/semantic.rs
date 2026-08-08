use core::mem::{offset_of, size_of};
use std::ops::Range;

use boxdd_sys::ffi;

use crate::{
    shapes::geometry::{
        MAX_POLYGON_VERTICES, points_have_minimum_pairwise_separation, polygon_semantics_are_valid,
    },
    types::Vec2,
};

use super::{ArgumentTag, NativeRecordingError, ReturnKind, TailKind, generated::OperationRule};

const NORMALIZED_MIN: f32 = 0.9994;
const NORMALIZED_MAX: f32 = 1.0006;
const REVOLUTE_LIMIT: f32 = 0.99 * core::f32::consts::PI;

pub(super) fn validate_argument(
    tag: ArgumentTag,
    bytes: &[u8],
    double_precision: bool,
    length_scale: f32,
    opcode: u8,
) -> Result<(), NativeRecordingError> {
    let mut cursor = Cursor::new(bytes, double_precision, opcode);
    match tag {
        ArgumentTag::Aabb => cursor.aabb()?,
        ArgumentTag::BodyDef => body_def(&mut cursor)?,
        ArgumentTag::BodyId
        | ArgumentTag::ChainId
        | ArgumentTag::JointId
        | ArgumentTag::ShapeId => cursor.skip(8)?,
        ArgumentTag::Bool => {
            cursor.boolean()?;
        }
        ArgumentTag::Capsule => {
            capsule(bytes, opcode)?;
            cursor.skip(bytes.len())?;
        }
        ArgumentTag::ChainDef => chain_def(&mut cursor, length_scale)?,
        ArgumentTag::ChainSegment => {
            chain_segment(bytes, length_scale, opcode)?;
            cursor.skip(bytes.len())?;
        }
        ArgumentTag::Circle => {
            circle(bytes, opcode)?;
            cursor.skip(bytes.len())?;
        }
        ArgumentTag::DistanceJointDef => distance_joint_def(&mut cursor)?,
        ArgumentTag::ExplosionDef => explosion_def(&mut cursor)?,
        ArgumentTag::F32 => {
            cursor.f32()?;
        }
        ArgumentTag::Filter => cursor.skip(20)?,
        ArgumentTag::FilterJointDef => joint_base(&mut cursor)?,
        ArgumentTag::I32 => {
            cursor.i32()?;
        }
        ArgumentTag::Locks => {
            cursor.boolean()?;
            cursor.boolean()?;
            cursor.boolean()?;
        }
        ArgumentTag::MassData => mass_data(&mut cursor)?,
        ArgumentTag::Material => material(&mut cursor)?,
        ArgumentTag::MotorJointDef => motor_joint_def(&mut cursor)?,
        ArgumentTag::Polygon => {
            polygon(bytes, length_scale, opcode)?;
            cursor.skip(bytes.len())?;
        }
        ArgumentTag::Position => {
            cursor.position()?;
        }
        ArgumentTag::PrismaticJointDef => prismatic_joint_def(&mut cursor)?,
        ArgumentTag::QueryFilter => cursor.skip(16)?,
        ArgumentTag::RevoluteJointDef => revolute_joint_def(&mut cursor)?,
        ArgumentTag::Rot => {
            cursor.rotation()?;
        }
        ArgumentTag::Segment => {
            segment(bytes, length_scale, opcode)?;
            cursor.skip(bytes.len())?;
        }
        ArgumentTag::ShapeDef => shape_def(&mut cursor)?,
        ArgumentTag::ShapeProxy => shape_proxy(&mut cursor)?,
        ArgumentTag::Str => {
            cursor.string()?;
        }
        ArgumentTag::U64 => cursor.skip(8)?,
        ArgumentTag::Vec2 => {
            cursor.vec2()?;
        }
        ArgumentTag::WeldJointDef => weld_joint_def(&mut cursor)?,
        ArgumentTag::WheelJointDef => wheel_joint_def(&mut cursor)?,
        ArgumentTag::WorldId => cursor.skip(4)?,
        ArgumentTag::WorldTransform => {
            cursor.position()?;
            cursor.rotation()?;
        }
        ArgumentTag::Transform => {
            cursor.vec2()?;
            cursor.rotation()?;
        }
    }
    cursor.finish()
}

pub(super) fn validate_operation(
    rule: OperationRule,
    payload: &[u8],
    arguments: &[Range<usize>],
    length_scale: f32,
    opcode: u8,
) -> Result<(), NativeRecordingError> {
    use OperationRule::*;

    let f32_arg = |index| argument_f32(payload, arguments, index, opcode);
    let i32_arg = |index| argument_i32(payload, arguments, index, opcode);
    match rule {
        WorldSetRestitutionThreshold
        | WorldSetHitEventThreshold
        | WorldSetContactRecycleDistance
        | BodySetLinearDamping
        | BodySetAngularDamping
        | BodySetSleepThreshold
        | ShapeSetDensity
        | ShapeSetFriction
        | ShapeSetRestitution
        | JointSetForceThreshold
        | JointSetTorqueThreshold
        | DistanceJointSetSpringHertz
        | DistanceJointSetSpringDampingRatio
        | DistanceJointSetMaxMotorForce
        | MotorJointSetMaxVelocityForce
        | MotorJointSetMaxVelocityTorque
        | MotorJointSetLinearHertz
        | MotorJointSetLinearDampingRatio
        | MotorJointSetAngularHertz
        | MotorJointSetAngularDampingRatio
        | MotorJointSetMaxSpringForce
        | MotorJointSetMaxSpringTorque
        | PrismaticJointSetSpringHertz
        | PrismaticJointSetSpringDampingRatio
        | PrismaticJointSetMaxMotorForce
        | RevoluteJointSetSpringHertz
        | RevoluteJointSetSpringDampingRatio
        | RevoluteJointSetMaxMotorTorque
        | WeldJointSetLinearHertz
        | WeldJointSetLinearDampingRatio
        | WeldJointSetAngularHertz
        | WeldJointSetAngularDampingRatio
        | WheelJointSetSpringHertz
        | WheelJointSetSpringDampingRatio
        | WheelJointSetMaxMotorTorque => non_negative(f32_arg(1)?, opcode)?,
        JointSetConstraintTuning => {
            non_negative(f32_arg(1)?, opcode)?;
            non_negative(f32_arg(2)?, opcode)?;
        }
        WorldSetContactTuning => {
            non_negative(f32_arg(1)?, opcode)?;
            non_negative(f32_arg(2)?, opcode)?;
            non_negative(f32_arg(3)?, opcode)?;
        }
        WorldSetMaximumLinearSpeed => positive_with_finite_square(f32_arg(1)?, opcode)?,
        Step => {
            non_negative(f32_arg(1)?, opcode)?;
            if i32_arg(2)? <= 0 {
                return Err(NativeRecordingError::InvalidRange(opcode));
            }
        }
        BodySetType => {
            if !(0..=2).contains(&i32_arg(1)?) {
                return Err(NativeRecordingError::InvalidEnum(opcode));
            }
        }
        BodySetTargetTransform => positive(f32_arg(2)?, opcode)?,
        ShapeApplyWind => non_negative(f32_arg(2)?, opcode)?,
        CreateChainSegmentShape => {
            let chain_segment = argument(payload, arguments, 2, opcode)?;
            let chain_id = pod_i32::<ffi::b2ChainSegment>(
                chain_segment,
                offset_of!(ffi::b2ChainSegment, chainId),
                opcode,
            )?;
            if chain_id != -1 {
                return Err(NativeRecordingError::InvalidValue(opcode));
            }
        }
        CreateCapsuleShape => {
            validate_capsule_separation(
                argument(payload, arguments, 2, opcode)?,
                length_scale,
                opcode,
            )?;
        }
        ShapeSetCapsule => {
            validate_capsule_separation(
                argument(payload, arguments, 1, opcode)?,
                length_scale,
                opcode,
            )?;
        }
        ChainSetSurfaceMaterial => {
            if i32_arg(2)? < 0 {
                return Err(NativeRecordingError::InvalidRange(opcode));
            }
        }
        DistanceJointSetLength => positive(f32_arg(1)?, opcode)?,
        DistanceJointSetSpringForceRange | PrismaticJointSetLimits | WheelJointSetLimits => {
            ordered(f32_arg(1)?, f32_arg(2)?, opcode)?
        }
        DistanceJointSetLengthRange => {
            non_negative(f32_arg(1)?, opcode)?;
            non_negative(f32_arg(2)?, opcode)?;
            ordered(f32_arg(1)?, f32_arg(2)?, opcode)?;
        }
        RevoluteJointSetLimits => {
            let lower = f32_arg(1)?;
            let upper = f32_arg(2)?;
            ordered(lower, upper, opcode)?;
            if lower < -REVOLUTE_LIMIT || upper > REVOLUTE_LIMIT {
                return Err(NativeRecordingError::InvalidRange(opcode));
            }
        }
        QueryCastMover => {
            let capsule = argument(payload, arguments, 2, opcode)?;
            let radius =
                pod_f32::<ffi::b2Capsule>(capsule, offset_of!(ffi::b2Capsule, radius), opcode)?;
            if radius <= 2.0 * 0.005 * length_scale {
                return Err(NativeRecordingError::InvalidRange(opcode));
            }
        }

        DestroyWorld
        | WorldEnableSleeping
        | WorldEnableContinuous
        | WorldSetGravity
        | WorldExplode
        | WorldEnableWarmStarting
        | WorldRebuildStaticTree
        | WorldEnableSpeculative
        | CreateBody
        | DestroyBody
        | BodySetTransform
        | BodySetLinearVelocity
        | BodySetName
        | BodySetAngularVelocity
        | BodyApplyForce
        | BodyApplyForceToCenter
        | BodyApplyTorque
        | BodyClearForces
        | BodyApplyLinearImpulse
        | BodyApplyLinearImpulseToCenter
        | BodyApplyAngularImpulse
        | BodySetMassData
        | BodyApplyMassFromShapes
        | BodySetGravityScale
        | BodySetAwake
        | BodyWakeTouching
        | BodyEnableSleep
        | BodyDisable
        | BodyEnable
        | BodySetMotionLocks
        | BodySetBullet
        | BodyEnableContactRecycling
        | BodyEnableContactEvents
        | BodyEnableHitEvents
        | CreateCircleShape
        | CreateSegmentShape
        | CreatePolygonShape
        | DestroyShape
        | ShapeSetUserMaterial
        | ShapeSetSurfaceMaterial
        | ShapeSetFilter
        | ShapeEnableSensorEvents
        | ShapeEnableContactEvents
        | ShapeEnablePreSolveEvents
        | ShapeEnableHitEvents
        | ShapeSetCircle
        | ShapeSetSegment
        | ShapeSetPolygon
        | ShapeSetChainSegment
        | CreateChain
        | DestroyChain
        | CreateDistanceJoint
        | CreateMotorJoint
        | CreateFilterJoint
        | CreatePrismaticJoint
        | CreateRevoluteJoint
        | CreateWeldJoint
        | CreateWheelJoint
        | DestroyJoint
        | JointSetLocalFrameA
        | JointSetLocalFrameB
        | JointSetCollideConnected
        | JointWakeBodies
        | DistanceJointEnableSpring
        | DistanceJointEnableLimit
        | DistanceJointEnableMotor
        | DistanceJointSetMotorSpeed
        | MotorJointSetLinearVelocity
        | MotorJointSetAngularVelocity
        | PrismaticJointEnableSpring
        | PrismaticJointSetTargetTranslation
        | PrismaticJointEnableLimit
        | PrismaticJointEnableMotor
        | PrismaticJointSetMotorSpeed
        | RevoluteJointEnableSpring
        | RevoluteJointSetTargetAngle
        | RevoluteJointEnableLimit
        | RevoluteJointEnableMotor
        | RevoluteJointSetMotorSpeed
        | WheelJointEnableSpring
        | WheelJointEnableLimit
        | WheelJointEnableMotor
        | WheelJointSetMotorSpeed
        | QueryOverlapAABB
        | QueryOverlapShape
        | QueryCastRay
        | QueryCastShape
        | QueryCollideMover
        | QueryCastRayClosest
        | ShapeTestPoint
        | ShapeRayCast
        | StateHash
        | RecordingBounds => {}
    }
    Ok(())
}

pub(super) fn validate_tail(
    tail: TailKind,
    return_kind: ReturnKind,
    bytes: &[u8],
    double_precision: bool,
    opcode: u8,
) -> Result<(), NativeRecordingError> {
    let mut cursor = Cursor::new(bytes, double_precision, opcode);
    match tail {
        TailKind::None => {}
        TailKind::ReturnedId => {
            if return_kind == ReturnKind::None {
                return Err(NativeRecordingError::GeneratedContract);
            }
            cursor.skip(8)?;
        }
        TailKind::OverlapHits => {
            let count = cursor.count()?;
            for _ in 0..count {
                cursor.skip(8)?;
                cursor.boolean()?;
            }
            tree_stats(&mut cursor)?;
        }
        TailKind::CastHits => {
            let count = cursor.count()?;
            for _ in 0..count {
                cursor.skip(8)?;
                cursor.position()?;
                let normal = cursor.vec2()?;
                let fraction = cursor.f32()?;
                let response = cursor.f32()?;
                unit_or_zero(normal, fraction == 0.0, opcode)?;
                in_unit_interval(fraction, opcode)?;
                if response != -1.0 && response != 0.0 && !(0.0..=1.0).contains(&response) {
                    return Err(NativeRecordingError::InvalidRange(opcode));
                }
            }
            tree_stats(&mut cursor)?;
        }
        TailKind::PlaneHits => {
            let count = cursor.count()?;
            for _ in 0..count {
                cursor.skip(8)?;
                let normal = cursor.vec2()?;
                cursor.f32()?;
                cursor.vec2()?;
                let hit = cursor.boolean()?;
                if hit {
                    normalized(normal, opcode)?;
                }
                cursor.boolean()?;
            }
        }
        TailKind::ClosestRayResult => {
            cursor.skip(8)?;
            cursor.position()?;
            let normal = cursor.vec2()?;
            let fraction = cursor.f32()?;
            let node_visits = cursor.i32()?;
            let leaf_visits = cursor.i32()?;
            let hit = cursor.boolean()?;
            if node_visits < 0 || leaf_visits < 0 || leaf_visits > node_visits {
                return Err(NativeRecordingError::InvalidRange(opcode));
            }
            if hit {
                normalized(normal, opcode)?;
                if !(0.0..=1.0).contains(&fraction) {
                    return Err(NativeRecordingError::InvalidRange(opcode));
                }
            }
        }
        TailKind::MoverResult => in_unit_interval(cursor.f32()?, opcode)?,
        TailKind::BoolResult => {
            cursor.boolean()?;
        }
        TailKind::ShapeCastResult => {
            let normal = cursor.vec2()?;
            cursor.position()?;
            let fraction = cursor.f32()?;
            let iterations = cursor.i32()?;
            let hit = cursor.boolean()?;
            if iterations < 0 {
                return Err(NativeRecordingError::InvalidRange(opcode));
            }
            if hit {
                unit_or_zero(normal, fraction == 0.0, opcode)?;
                in_unit_interval(fraction, opcode)?;
            }
        }
    }
    cursor.finish()
}

fn body_def(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    if !(0..=2).contains(&cursor.i32()?) {
        return Err(NativeRecordingError::InvalidEnum(cursor.opcode));
    }
    cursor.position()?;
    cursor.rotation()?;
    cursor.vec2()?;
    cursor.f32()?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.f32()?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.string()?;
    cursor.skip(8)?;
    for _ in 0..9 {
        cursor.boolean()?;
    }
    Ok(())
}

fn shape_def(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    cursor.skip(8)?;
    material(cursor)?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.skip(20)?;
    let custom_filter = cursor.boolean()?;
    cursor.boolean()?;
    cursor.boolean()?;
    cursor.boolean()?;
    cursor.boolean()?;
    let pre_solve = cursor.boolean()?;
    cursor.boolean()?;
    cursor.boolean()?;
    if custom_filter || pre_solve {
        return Err(NativeRecordingError::InvalidValue(cursor.opcode));
    }
    Ok(())
}

fn chain_def(cursor: &mut Cursor<'_>, length_scale: f32) -> Result<(), NativeRecordingError> {
    cursor.skip(8)?;
    let point_count = cursor.count()?;
    if point_count < 4 {
        return Err(NativeRecordingError::InvalidCount(cursor.opcode));
    }
    let mut points = Vec::new();
    points
        .try_reserve(point_count)
        .map_err(|_| NativeRecordingError::InvalidCount(cursor.opcode))?;
    for _ in 0..point_count {
        let (x, y) = cursor.vec2()?;
        points.push(Vec2::new(x, y));
    }
    if !points_have_minimum_pairwise_separation(&points, length_scale)
        .map_err(|_| NativeRecordingError::InvalidCount(cursor.opcode))?
    {
        return Err(NativeRecordingError::InvalidRange(cursor.opcode));
    }
    let material_count = cursor.count()?;
    if material_count != 1 && material_count != point_count {
        return Err(NativeRecordingError::InvalidCount(cursor.opcode));
    }
    for _ in 0..material_count {
        material(cursor)?;
    }
    cursor.skip(20)?;
    cursor.boolean()?;
    cursor.boolean()?;
    Ok(())
}

fn explosion_def(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    cursor.skip(8)?;
    let position = cursor.position()?;
    let radius = cursor.f32()?;
    let falloff = cursor.f32()?;
    non_negative(radius, cursor.opcode)?;
    non_negative(falloff, cursor.opcode)?;
    let extent = radius + falloff;
    if !extent.is_finite()
        || !crate::world_extras::explosion_query_axis_is_representable(position.0, extent)
        || !crate::world_extras::explosion_query_axis_is_representable(position.1, extent)
    {
        return Err(NativeRecordingError::InvalidValue(cursor.opcode));
    }
    cursor.f32()?;
    Ok(())
}

fn material(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    for _ in 0..3 {
        non_negative(cursor.f32()?, cursor.opcode)?;
    }
    cursor.f32()?;
    cursor.skip(12)
}

fn mass_data(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.vec2()?;
    non_negative(cursor.f32()?, cursor.opcode)
}

fn joint_base(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    cursor.skip(8)?;
    cursor.skip(8)?;
    cursor.skip(8)?;
    cursor.vec2()?;
    cursor.rotation()?;
    cursor.vec2()?;
    cursor.rotation()?;
    for _ in 0..5 {
        non_negative(cursor.f32()?, cursor.opcode)?;
    }
    cursor.boolean()?;
    Ok(())
}

fn distance_joint_def(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    joint_base(cursor)?;
    positive(cursor.f32()?, cursor.opcode)?;
    cursor.boolean()?;
    let lower_force = cursor.f32()?;
    let upper_force = cursor.f32()?;
    ordered(lower_force, upper_force, cursor.opcode)?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.boolean()?;
    let minimum = cursor.f32()?;
    let maximum = cursor.f32()?;
    non_negative(minimum, cursor.opcode)?;
    non_negative(maximum, cursor.opcode)?;
    ordered(minimum, maximum, cursor.opcode)?;
    cursor.boolean()?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.f32()?;
    Ok(())
}

fn motor_joint_def(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    joint_base(cursor)?;
    cursor.vec2()?;
    for index in 0..9 {
        let value = cursor.f32()?;
        if index != 1 {
            non_negative(value, cursor.opcode)?;
        }
    }
    Ok(())
}

fn prismatic_joint_def(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    joint_base(cursor)?;
    cursor.boolean()?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.f32()?;
    cursor.boolean()?;
    let lower = cursor.f32()?;
    let upper = cursor.f32()?;
    ordered(lower, upper, cursor.opcode)?;
    cursor.boolean()?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.f32()?;
    Ok(())
}

fn revolute_joint_def(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    joint_base(cursor)?;
    cursor.f32()?;
    cursor.boolean()?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.boolean()?;
    let lower = cursor.f32()?;
    let upper = cursor.f32()?;
    ordered(lower, upper, cursor.opcode)?;
    if lower < -REVOLUTE_LIMIT || upper > REVOLUTE_LIMIT {
        return Err(NativeRecordingError::InvalidRange(cursor.opcode));
    }
    cursor.boolean()?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.f32()?;
    Ok(())
}

fn weld_joint_def(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    joint_base(cursor)?;
    for _ in 0..4 {
        non_negative(cursor.f32()?, cursor.opcode)?;
    }
    Ok(())
}

fn wheel_joint_def(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    joint_base(cursor)?;
    cursor.boolean()?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.boolean()?;
    let lower = cursor.f32()?;
    let upper = cursor.f32()?;
    ordered(lower, upper, cursor.opcode)?;
    cursor.boolean()?;
    non_negative(cursor.f32()?, cursor.opcode)?;
    cursor.f32()?;
    Ok(())
}

fn shape_proxy(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    let count = cursor.count()?;
    if !(1..=8).contains(&count) {
        return Err(NativeRecordingError::InvalidCount(cursor.opcode));
    }
    for _ in 0..count {
        cursor.vec2()?;
    }
    non_negative(cursor.f32()?, cursor.opcode)
}

fn circle(bytes: &[u8], opcode: u8) -> Result<(), NativeRecordingError> {
    pod_vec2::<ffi::b2Circle>(bytes, offset_of!(ffi::b2Circle, center), opcode)?;
    non_negative(
        pod_f32::<ffi::b2Circle>(bytes, offset_of!(ffi::b2Circle, radius), opcode)?,
        opcode,
    )
}

fn capsule(bytes: &[u8], opcode: u8) -> Result<(), NativeRecordingError> {
    pod_vec2::<ffi::b2Capsule>(bytes, offset_of!(ffi::b2Capsule, center1), opcode)?;
    pod_vec2::<ffi::b2Capsule>(bytes, offset_of!(ffi::b2Capsule, center2), opcode)?;
    non_negative(
        pod_f32::<ffi::b2Capsule>(bytes, offset_of!(ffi::b2Capsule, radius), opcode)?,
        opcode,
    )
}

fn validate_capsule_separation(
    bytes: &[u8],
    length_scale: f32,
    opcode: u8,
) -> Result<(), NativeRecordingError> {
    let center1 = pod_vec2::<ffi::b2Capsule>(bytes, offset_of!(ffi::b2Capsule, center1), opcode)?;
    let center2 = pod_vec2::<ffi::b2Capsule>(bytes, offset_of!(ffi::b2Capsule, center2), opcode)?;
    separated(center1, center2, length_scale, opcode)
}

fn segment(bytes: &[u8], length_scale: f32, opcode: u8) -> Result<(), NativeRecordingError> {
    let point1 = pod_vec2::<ffi::b2Segment>(bytes, offset_of!(ffi::b2Segment, point1), opcode)?;
    let point2 = pod_vec2::<ffi::b2Segment>(bytes, offset_of!(ffi::b2Segment, point2), opcode)?;
    separated(point1, point2, length_scale, opcode)
}

fn chain_segment(bytes: &[u8], length_scale: f32, opcode: u8) -> Result<(), NativeRecordingError> {
    pod_vec2::<ffi::b2ChainSegment>(bytes, offset_of!(ffi::b2ChainSegment, ghost1), opcode)?;
    let point1 = pod_vec2::<ffi::b2ChainSegment>(
        bytes,
        offset_of!(ffi::b2ChainSegment, segment) + offset_of!(ffi::b2Segment, point1),
        opcode,
    )?;
    let point2 = pod_vec2::<ffi::b2ChainSegment>(
        bytes,
        offset_of!(ffi::b2ChainSegment, segment) + offset_of!(ffi::b2Segment, point2),
        opcode,
    )?;
    pod_vec2::<ffi::b2ChainSegment>(bytes, offset_of!(ffi::b2ChainSegment, ghost2), opcode)?;
    separated(point1, point2, length_scale, opcode)
}

fn polygon(bytes: &[u8], length_scale: f32, opcode: u8) -> Result<(), NativeRecordingError> {
    if bytes.len() != size_of::<ffi::b2Polygon>() {
        return Err(NativeRecordingError::InvalidValue(opcode));
    }
    let count = pod_i32::<ffi::b2Polygon>(bytes, offset_of!(ffi::b2Polygon, count), opcode)?;
    if !(3..=MAX_POLYGON_VERTICES as i32).contains(&count) {
        return Err(NativeRecordingError::InvalidCount(opcode));
    }
    let count = count as usize;
    let mut vertices = [Vec2::ZERO; MAX_POLYGON_VERTICES];
    let mut normals = [Vec2::ZERO; MAX_POLYGON_VERTICES];
    for (index, vertex) in vertices.iter_mut().take(count).enumerate() {
        *vertex = pod_vec2::<ffi::b2Polygon>(
            bytes,
            offset_of!(ffi::b2Polygon, vertices) + index * size_of::<ffi::b2Vec2>(),
            opcode,
        )?
        .into();
        normals[index] = pod_vec2::<ffi::b2Polygon>(
            bytes,
            offset_of!(ffi::b2Polygon, normals) + index * size_of::<ffi::b2Vec2>(),
            opcode,
        )?
        .into();
    }
    let centroid: Vec2 =
        pod_vec2::<ffi::b2Polygon>(bytes, offset_of!(ffi::b2Polygon, centroid), opcode)?.into();
    let radius = pod_f32::<ffi::b2Polygon>(bytes, offset_of!(ffi::b2Polygon, radius), opcode)?;
    let minimum_edge_length = 0.005 * length_scale;
    if !polygon_semantics_are_valid(
        &vertices[..count],
        &normals[..count],
        centroid,
        radius,
        minimum_edge_length * minimum_edge_length,
    ) {
        return Err(NativeRecordingError::InvalidValue(opcode));
    }
    Ok(())
}

fn pod_vec2<T>(
    bytes: &[u8],
    offset: usize,
    opcode: u8,
) -> Result<(f32, f32), NativeRecordingError> {
    if bytes.len() != size_of::<T>() {
        return Err(NativeRecordingError::InvalidValue(opcode));
    }
    let x = raw_f32(bytes, offset, opcode)?;
    let y = raw_f32(bytes, offset + 4, opcode)?;
    if !x.is_finite() || !y.is_finite() {
        return Err(NativeRecordingError::InvalidValue(opcode));
    }
    Ok((x, y))
}

fn pod_f32<T>(bytes: &[u8], offset: usize, opcode: u8) -> Result<f32, NativeRecordingError> {
    if bytes.len() != size_of::<T>() {
        return Err(NativeRecordingError::InvalidValue(opcode));
    }
    let value = raw_f32(bytes, offset, opcode)?;
    if !value.is_finite() {
        return Err(NativeRecordingError::InvalidValue(opcode));
    }
    Ok(value)
}

fn pod_i32<T>(bytes: &[u8], offset: usize, opcode: u8) -> Result<i32, NativeRecordingError> {
    if bytes.len() != size_of::<T>() {
        return Err(NativeRecordingError::InvalidValue(opcode));
    }
    raw_i32(bytes, offset, opcode)
}

fn raw_f32(bytes: &[u8], offset: usize, opcode: u8) -> Result<f32, NativeRecordingError> {
    let raw = bytes
        .get(
            offset
                ..offset
                    .checked_add(4)
                    .ok_or(NativeRecordingError::InvalidValue(opcode))?,
        )
        .ok_or(NativeRecordingError::InvalidValue(opcode))?;
    Ok(f32::from_le_bytes(
        raw.try_into()
            .map_err(|_| NativeRecordingError::InvalidValue(opcode))?,
    ))
}

fn raw_i32(bytes: &[u8], offset: usize, opcode: u8) -> Result<i32, NativeRecordingError> {
    let raw = bytes
        .get(
            offset
                ..offset
                    .checked_add(4)
                    .ok_or(NativeRecordingError::InvalidValue(opcode))?,
        )
        .ok_or(NativeRecordingError::InvalidValue(opcode))?;
    Ok(i32::from_le_bytes(
        raw.try_into()
            .map_err(|_| NativeRecordingError::InvalidValue(opcode))?,
    ))
}

fn argument<'a>(
    payload: &'a [u8],
    arguments: &[Range<usize>],
    index: usize,
    opcode: u8,
) -> Result<&'a [u8], NativeRecordingError> {
    payload
        .get(
            arguments
                .get(index)
                .ok_or(NativeRecordingError::GeneratedContract)?
                .clone(),
        )
        .ok_or(NativeRecordingError::InvalidValue(opcode))
}

fn argument_f32(
    payload: &[u8],
    arguments: &[Range<usize>],
    index: usize,
    opcode: u8,
) -> Result<f32, NativeRecordingError> {
    let bytes = argument(payload, arguments, index, opcode)?;
    if bytes.len() != 4 {
        return Err(NativeRecordingError::GeneratedContract);
    }
    let value = raw_f32(bytes, 0, opcode)?;
    if !value.is_finite() {
        return Err(NativeRecordingError::InvalidValue(opcode));
    }
    Ok(value)
}

fn argument_i32(
    payload: &[u8],
    arguments: &[Range<usize>],
    index: usize,
    opcode: u8,
) -> Result<i32, NativeRecordingError> {
    let bytes = argument(payload, arguments, index, opcode)?;
    if bytes.len() != 4 {
        return Err(NativeRecordingError::GeneratedContract);
    }
    raw_i32(bytes, 0, opcode)
}

fn positive(value: f32, opcode: u8) -> Result<(), NativeRecordingError> {
    if value > 0.0 {
        Ok(())
    } else {
        Err(NativeRecordingError::InvalidRange(opcode))
    }
}

fn positive_with_finite_square(value: f32, opcode: u8) -> Result<(), NativeRecordingError> {
    if value > 0.0 && (value * value).is_finite() {
        Ok(())
    } else {
        Err(NativeRecordingError::InvalidRange(opcode))
    }
}

fn non_negative(value: f32, opcode: u8) -> Result<(), NativeRecordingError> {
    if value >= 0.0 {
        Ok(())
    } else {
        Err(NativeRecordingError::InvalidRange(opcode))
    }
}

fn ordered(lower: f32, upper: f32, opcode: u8) -> Result<(), NativeRecordingError> {
    if lower <= upper {
        Ok(())
    } else {
        Err(NativeRecordingError::InvalidRange(opcode))
    }
}

fn in_unit_interval(value: f32, opcode: u8) -> Result<(), NativeRecordingError> {
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(NativeRecordingError::InvalidRange(opcode))
    }
}

fn separated(
    left: (f32, f32),
    right: (f32, f32),
    length_scale: f32,
    opcode: u8,
) -> Result<(), NativeRecordingError> {
    let dx = right.0 - left.0;
    let dy = right.1 - left.1;
    let distance_squared = dx.mul_add(dx, dy * dy);
    let slop = 0.005 * length_scale;
    if distance_squared.is_finite() && distance_squared > slop * slop {
        Ok(())
    } else {
        Err(NativeRecordingError::InvalidRange(opcode))
    }
}

fn normalized(vector: (f32, f32), opcode: u8) -> Result<(), NativeRecordingError> {
    let length_squared = vector.0.mul_add(vector.0, vector.1 * vector.1);
    if length_squared > NORMALIZED_MIN && length_squared < NORMALIZED_MAX {
        Ok(())
    } else {
        Err(NativeRecordingError::InvalidValue(opcode))
    }
}

fn unit_or_zero(
    vector: (f32, f32),
    allow_zero: bool,
    opcode: u8,
) -> Result<(), NativeRecordingError> {
    if allow_zero && vector == (0.0, 0.0) {
        Ok(())
    } else {
        normalized(vector, opcode)
    }
}

fn tree_stats(cursor: &mut Cursor<'_>) -> Result<(), NativeRecordingError> {
    let node_visits = cursor.i32()?;
    let leaf_visits = cursor.i32()?;
    if node_visits < 0 || leaf_visits < 0 || leaf_visits > node_visits {
        return Err(NativeRecordingError::InvalidRange(cursor.opcode));
    }
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
    double_precision: bool,
    opcode: u8,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8], double_precision: bool, opcode: u8) -> Self {
        Self {
            bytes,
            offset: 0,
            double_precision,
            opcode,
        }
    }

    fn take(&mut self, width: usize) -> Result<&'a [u8], NativeRecordingError> {
        let end = self
            .offset
            .checked_add(width)
            .ok_or(NativeRecordingError::InvalidValue(self.opcode))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(NativeRecordingError::InvalidValue(self.opcode))?;
        self.offset = end;
        Ok(value)
    }

    fn skip(&mut self, width: usize) -> Result<(), NativeRecordingError> {
        self.take(width).map(|_| ())
    }

    fn f32(&mut self) -> Result<f32, NativeRecordingError> {
        let value = self.take(4)?;
        let value = f32::from_le_bytes(
            value
                .try_into()
                .map_err(|_| NativeRecordingError::InvalidValue(self.opcode))?,
        );
        if value.is_finite() {
            Ok(value)
        } else {
            Err(NativeRecordingError::InvalidValue(self.opcode))
        }
    }

    fn i32(&mut self) -> Result<i32, NativeRecordingError> {
        let value = self.take(4)?;
        Ok(i32::from_le_bytes(value.try_into().map_err(|_| {
            NativeRecordingError::InvalidValue(self.opcode)
        })?))
    }

    fn boolean(&mut self) -> Result<bool, NativeRecordingError> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(NativeRecordingError::InvalidBoolean(self.opcode)),
        }
    }

    fn count(&mut self) -> Result<usize, NativeRecordingError> {
        let count = self.i32()?;
        usize::try_from(count).map_err(|_| NativeRecordingError::InvalidCount(self.opcode))
    }

    fn vec2(&mut self) -> Result<(f32, f32), NativeRecordingError> {
        Ok((self.f32()?, self.f32()?))
    }

    fn position(&mut self) -> Result<(f64, f64), NativeRecordingError> {
        if self.double_precision {
            let mut components = [0.0; 2];
            for component in &mut components {
                let value = self.take(8)?;
                let value = f64::from_le_bytes(
                    value
                        .try_into()
                        .map_err(|_| NativeRecordingError::InvalidValue(self.opcode))?,
                );
                if !value.is_finite() {
                    return Err(NativeRecordingError::InvalidValue(self.opcode));
                }
                *component = value;
            }
            Ok((components[0], components[1]))
        } else {
            let (x, y) = self.vec2()?;
            Ok((f64::from(x), f64::from(y)))
        }
    }

    fn rotation(&mut self) -> Result<(), NativeRecordingError> {
        normalized(self.vec2()?, self.opcode)
    }

    fn aabb(&mut self) -> Result<(), NativeRecordingError> {
        let lower = self.vec2()?;
        let upper = self.vec2()?;
        if lower.0 > upper.0 || lower.1 > upper.1 {
            return Err(NativeRecordingError::InvalidRange(self.opcode));
        }
        Ok(())
    }

    fn string(&mut self) -> Result<(), NativeRecordingError> {
        let length = self.take(2)?;
        let length = u16::from_le_bytes([length[0], length[1]]);
        if length != u16::MAX {
            self.skip(usize::from(length))?;
        }
        Ok(())
    }

    fn finish(self) -> Result<(), NativeRecordingError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(NativeRecordingError::InvalidValue(self.opcode))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_f32(bytes: &mut Vec<u8>, value: f32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn chain_def_rejects_close_nonadjacent_points() {
        const OPCODE: u8 = 0x7f;
        let mut bytes = vec![0; 8];
        bytes.extend_from_slice(&4_i32.to_le_bytes());
        for (x, y) in [(0.0, 0.0), (1.0, 0.0), (0.001, 0.0), (2.0, 0.0)] {
            push_f32(&mut bytes, x);
            push_f32(&mut bytes, y);
        }
        bytes.extend_from_slice(&1_i32.to_le_bytes());
        for value in [0.6, 0.0, 0.0, 0.0] {
            push_f32(&mut bytes, value);
        }
        bytes.extend_from_slice(&[0; 12]);
        bytes.extend_from_slice(&[0; 20]);
        bytes.extend_from_slice(&[0, 0]);

        let mut cursor = Cursor::new(&bytes, false, OPCODE);
        assert_eq!(
            chain_def(&mut cursor, 1.0),
            Err(NativeRecordingError::InvalidRange(OPCODE))
        );
    }
}
