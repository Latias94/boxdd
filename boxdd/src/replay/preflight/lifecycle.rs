use std::ops::Range;

use boxdd_sys::adapter::{
    SNAPSHOT_ENTRY_BODY, SNAPSHOT_ENTRY_CHAIN, SNAPSHOT_ENTRY_JOINT, SNAPSHOT_ENTRY_LIVE,
    SNAPSHOT_ENTRY_SHAPE, SnapshotValidation,
};

use super::{
    Argument, ArgumentTag, NativeRecordingError, ReturnKind, TailKind, generated::OperationRule,
};

const NULL_INDEX: i32 = -1;
const STATIC_BODY: u32 = 0;
const DYNAMIC_BODY: u32 = 2;
const CIRCLE_SHAPE: u32 = 0;
const CAPSULE_SHAPE: u32 = 1;
const SEGMENT_SHAPE: u32 = 2;
const POLYGON_SHAPE: u32 = 3;
const CHAIN_SEGMENT_SHAPE: u32 = 4;
const DISTANCE_JOINT: u32 = 0;
const FILTER_JOINT: u32 = 1;
const MOTOR_JOINT: u32 = 2;
const PRISMATIC_JOINT: u32 = 3;
const REVOLUTE_JOINT: u32 = 4;
const WELD_JOINT: u32 = 5;
const WHEEL_JOINT: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WorldId {
    index1: u16,
    generation: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ObjectId {
    index1: i32,
    world0: u16,
    generation: u16,
}

#[derive(Clone, Copy, Debug, Default)]
struct Slot {
    generation: u16,
    live: bool,
}

#[derive(Debug)]
struct Pool {
    slots: Vec<Slot>,
    free: Vec<usize>,
}

impl Pool {
    fn from_snapshot(
        snapshot: &SnapshotValidation,
        kind: u32,
    ) -> Result<Self, NativeRecordingError> {
        let pool_index =
            usize::try_from(kind - 1).map_err(|_| NativeRecordingError::ContractMismatch)?;
        let next = usize::try_from(snapshot.facts.pool_next[pool_index])
            .map_err(|_| NativeRecordingError::ContractMismatch)?;
        let free_count = usize::try_from(snapshot.facts.pool_free[pool_index])
            .map_err(|_| NativeRecordingError::ContractMismatch)?;
        let base = snapshot.facts.pool_next[..pool_index]
            .iter()
            .try_fold(0usize, |sum, count| {
                sum.checked_add(usize::try_from(*count).ok()?)
            })
            .ok_or(NativeRecordingError::ContractMismatch)?;
        let entries = snapshot
            .entries
            .get(
                base..base
                    .checked_add(next)
                    .ok_or(NativeRecordingError::ContractMismatch)?,
            )
            .ok_or(NativeRecordingError::ContractMismatch)?;

        let mut slots = Vec::new();
        slots
            .try_reserve_exact(next)
            .map_err(|_| NativeRecordingError::AllocationFailed)?;
        let mut free = filled_vec(free_count, usize::MAX)?;
        for (index, entry) in entries.iter().enumerate() {
            if entry.kind != kind
                || entry.index
                    != i32::try_from(index).map_err(|_| NativeRecordingError::ContractMismatch)?
                || entry.generation > u32::from(u16::MAX)
            {
                return Err(NativeRecordingError::ContractMismatch);
            }
            let live = entry.flags & SNAPSHOT_ENTRY_LIVE != 0;
            slots.push(Slot {
                generation: entry.generation as u16,
                live,
            });
            if live {
                if entry.free_order != NULL_INDEX {
                    return Err(NativeRecordingError::ContractMismatch);
                }
            } else {
                let order = usize::try_from(entry.free_order)
                    .map_err(|_| NativeRecordingError::ContractMismatch)?;
                let target = free
                    .get_mut(order)
                    .ok_or(NativeRecordingError::ContractMismatch)?;
                if *target != usize::MAX {
                    return Err(NativeRecordingError::ContractMismatch);
                }
                *target = index;
            }
        }
        if free.contains(&usize::MAX) {
            return Err(NativeRecordingError::ContractMismatch);
        }
        Ok(Self { slots, free })
    }

    fn allocate(&mut self, opcode: u8) -> Result<(usize, u16), NativeRecordingError> {
        let index = if let Some(index) = self.free.pop() {
            index
        } else {
            let index = self.slots.len();
            self.slots
                .try_reserve(1)
                .map_err(|_| NativeRecordingError::AllocationFailed)?;
            self.slots.push(Slot::default());
            index
        };
        let slot = self
            .slots
            .get_mut(index)
            .ok_or(NativeRecordingError::InvalidLifecycle(opcode))?;
        if slot.live {
            return Err(NativeRecordingError::InvalidLifecycle(opcode));
        }
        slot.generation = slot.generation.wrapping_add(1);
        slot.live = true;
        Ok((index, slot.generation))
    }

    fn require(&self, id: ObjectId, opcode: u8) -> Result<usize, NativeRecordingError> {
        let index = id
            .index1
            .checked_sub(1)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(NativeRecordingError::InvalidReference(opcode))?;
        let slot = self
            .slots
            .get(index)
            .ok_or(NativeRecordingError::InvalidReference(opcode))?;
        if !slot.live || slot.generation != id.generation {
            return Err(NativeRecordingError::InvalidReference(opcode));
        }
        Ok(index)
    }

    fn release(&mut self, index: usize, opcode: u8) -> Result<(), NativeRecordingError> {
        let slot = self
            .slots
            .get_mut(index)
            .ok_or(NativeRecordingError::InvalidLifecycle(opcode))?;
        if !slot.live {
            return Err(NativeRecordingError::InvalidLifecycle(opcode));
        }
        slot.live = false;
        self.free
            .try_reserve(1)
            .map_err(|_| NativeRecordingError::AllocationFailed)?;
        self.free.push(index);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct BodyState {
    body_type: u32,
    head_shape: i32,
    head_chain: i32,
    head_joint: i32,
}

impl Default for BodyState {
    fn default() -> Self {
        Self {
            body_type: STATIC_BODY,
            head_shape: NULL_INDEX,
            head_chain: NULL_INDEX,
            head_joint: NULL_INDEX,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ShapeState {
    body: i32,
    parent_chain: i32,
    chain_order: i32,
    prev: i32,
    next: i32,
    shape_type: u32,
}

impl Default for ShapeState {
    fn default() -> Self {
        Self {
            body: NULL_INDEX,
            parent_chain: NULL_INDEX,
            chain_order: NULL_INDEX,
            prev: NULL_INDEX,
            next: NULL_INDEX,
            shape_type: CIRCLE_SHAPE,
        }
    }
}

#[derive(Clone, Debug)]
struct ChainState {
    body: i32,
    next: i32,
    material_count: usize,
    shapes: Vec<i32>,
}

impl Default for ChainState {
    fn default() -> Self {
        Self {
            body: NULL_INDEX,
            next: NULL_INDEX,
            material_count: 0,
            shapes: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct JointState {
    bodies: [i32; 2],
    prev: [i32; 2],
    next: [i32; 2],
    joint_type: u32,
}

impl Default for JointState {
    fn default() -> Self {
        Self {
            bodies: [NULL_INDEX; 2],
            prev: [NULL_INDEX; 2],
            next: [NULL_INDEX; 2],
            joint_type: DISTANCE_JOINT,
        }
    }
}

#[derive(Debug)]
pub(super) struct Lifecycle {
    world: Option<WorldId>,
    destroyed: bool,
    bodies_pool: Pool,
    shapes_pool: Pool,
    chains_pool: Pool,
    joints_pool: Pool,
    bodies: Vec<BodyState>,
    shapes: Vec<ShapeState>,
    chains: Vec<ChainState>,
    joints: Vec<JointState>,
}

pub(super) struct Observation<'a> {
    pub(super) rule: OperationRule,
    pub(super) return_kind: ReturnKind,
    pub(super) tail_kind: TailKind,
    pub(super) argument_defs: &'a [Argument],
    pub(super) payload: &'a [u8],
    pub(super) arguments: &'a [Range<usize>],
    pub(super) tail: &'a [u8],
    pub(super) double_precision: bool,
    pub(super) opcode: u8,
}

impl Lifecycle {
    pub(super) fn from_snapshot(
        snapshot: &SnapshotValidation,
    ) -> Result<Self, NativeRecordingError> {
        let bodies_pool = Pool::from_snapshot(snapshot, SNAPSHOT_ENTRY_BODY)?;
        let shapes_pool = Pool::from_snapshot(snapshot, SNAPSHOT_ENTRY_SHAPE)?;
        let chains_pool = Pool::from_snapshot(snapshot, SNAPSHOT_ENTRY_CHAIN)?;
        let joints_pool = Pool::from_snapshot(snapshot, SNAPSHOT_ENTRY_JOINT)?;
        let bodies = filled_vec(bodies_pool.slots.len(), BodyState::default())?;
        let shapes = filled_vec(shapes_pool.slots.len(), ShapeState::default())?;
        let chains = filled_vec(chains_pool.slots.len(), ChainState::default())?;
        let joints = filled_vec(joints_pool.slots.len(), JointState::default())?;
        let mut lifecycle = Self {
            world: None,
            destroyed: false,
            bodies,
            shapes,
            chains,
            joints,
            bodies_pool,
            shapes_pool,
            chains_pool,
            joints_pool,
        };
        lifecycle.load_snapshot_metadata(snapshot)?;
        lifecycle.validate_snapshot_graph()?;
        Ok(lifecycle)
    }

    pub(super) fn observe(
        &mut self,
        observation: Observation<'_>,
    ) -> Result<(), NativeRecordingError> {
        let Observation {
            rule,
            return_kind,
            tail_kind,
            argument_defs,
            payload,
            arguments,
            tail,
            double_precision,
            opcode,
        } = observation;
        if self.destroyed {
            return Err(NativeRecordingError::InvalidLifecycle(opcode));
        }
        for (index, argument_def) in argument_defs.iter().enumerate() {
            let bytes = argument(payload, arguments, index, opcode)?;
            match argument_def.tag {
                ArgumentTag::WorldId => {
                    let id = read_world_id(bytes, opcode)?;
                    self.require_world(id, rule == OperationRule::StateHash, opcode)?;
                }
                ArgumentTag::BodyId => {
                    self.require_body(read_object_id(bytes, opcode)?, opcode)?;
                }
                ArgumentTag::ShapeId => {
                    self.require_shape(read_object_id(bytes, opcode)?, opcode)?;
                }
                ArgumentTag::ChainId => {
                    self.require_chain(read_object_id(bytes, opcode)?, opcode)?;
                }
                ArgumentTag::JointId => {
                    self.require_joint(read_object_id(bytes, opcode)?, opcode)?;
                }
                _ => {}
            }
        }

        use OperationRule::*;
        match rule {
            CreateBody => self.create_body(
                argument(payload, arguments, 1, opcode)?,
                tail,
                return_kind,
                opcode,
            )?,
            DestroyBody => {
                let index = self.require_body(
                    read_object_id(argument(payload, arguments, 0, opcode)?, opcode)?,
                    opcode,
                )?;
                self.destroy_body(index, opcode)?;
            }
            BodySetType => {
                let index = self.require_body(
                    read_object_id(argument(payload, arguments, 0, opcode)?, opcode)?,
                    opcode,
                )?;
                self.bodies[index].body_type =
                    read_i32(argument(payload, arguments, 1, opcode)?, 0, opcode)? as u32;
            }
            CreateCircleShape => self.create_shape_from_record(
                payload,
                arguments,
                tail,
                return_kind,
                CIRCLE_SHAPE,
                opcode,
            )?,
            CreateCapsuleShape => self.create_shape_from_record(
                payload,
                arguments,
                tail,
                return_kind,
                CAPSULE_SHAPE,
                opcode,
            )?,
            CreateSegmentShape => self.create_shape_from_record(
                payload,
                arguments,
                tail,
                return_kind,
                SEGMENT_SHAPE,
                opcode,
            )?,
            CreatePolygonShape => self.create_shape_from_record(
                payload,
                arguments,
                tail,
                return_kind,
                POLYGON_SHAPE,
                opcode,
            )?,
            CreateChainSegmentShape => self.create_shape_from_record(
                payload,
                arguments,
                tail,
                return_kind,
                CHAIN_SEGMENT_SHAPE,
                opcode,
            )?,
            DestroyShape => {
                let index = self.require_shape(
                    read_object_id(argument(payload, arguments, 0, opcode)?, opcode)?,
                    opcode,
                )?;
                if self.shapes[index].parent_chain != NULL_INDEX {
                    return Err(NativeRecordingError::InvalidLifecycle(opcode));
                }
                self.destroy_shape_internal(index, opcode)?;
            }
            ShapeSetCircle => self.set_shape_type(payload, arguments, CIRCLE_SHAPE, opcode)?,
            ShapeSetCapsule => self.set_shape_type(payload, arguments, CAPSULE_SHAPE, opcode)?,
            ShapeSetSegment => self.set_shape_type(payload, arguments, SEGMENT_SHAPE, opcode)?,
            ShapeSetPolygon => self.set_shape_type(payload, arguments, POLYGON_SHAPE, opcode)?,
            ShapeSetChainSegment => {
                self.set_shape_type(payload, arguments, CHAIN_SEGMENT_SHAPE, opcode)?
            }
            CreateChain => self.create_chain(payload, arguments, tail, return_kind, opcode)?,
            DestroyChain => {
                let index = self.require_chain(
                    read_object_id(argument(payload, arguments, 0, opcode)?, opcode)?,
                    opcode,
                )?;
                self.destroy_chain(index, opcode)?;
            }
            ChainSetSurfaceMaterial => {
                let index = self.require_chain(
                    read_object_id(argument(payload, arguments, 0, opcode)?, opcode)?,
                    opcode,
                )?;
                let material_index = read_i32(argument(payload, arguments, 2, opcode)?, 0, opcode)?;
                if material_index < 0
                    || usize::try_from(material_index).ok()
                        >= Some(self.chains[index].material_count)
                {
                    return Err(NativeRecordingError::InvalidRange(opcode));
                }
            }
            CreateDistanceJoint => self.create_joint(
                payload,
                arguments,
                tail,
                return_kind,
                DISTANCE_JOINT,
                opcode,
            )?,
            CreateMotorJoint => {
                self.create_joint(payload, arguments, tail, return_kind, MOTOR_JOINT, opcode)?
            }
            CreateFilterJoint => {
                self.create_joint(payload, arguments, tail, return_kind, FILTER_JOINT, opcode)?
            }
            CreatePrismaticJoint => self.create_joint(
                payload,
                arguments,
                tail,
                return_kind,
                PRISMATIC_JOINT,
                opcode,
            )?,
            CreateRevoluteJoint => self.create_joint(
                payload,
                arguments,
                tail,
                return_kind,
                REVOLUTE_JOINT,
                opcode,
            )?,
            CreateWeldJoint => {
                self.create_joint(payload, arguments, tail, return_kind, WELD_JOINT, opcode)?
            }
            CreateWheelJoint => {
                self.create_joint(payload, arguments, tail, return_kind, WHEEL_JOINT, opcode)?
            }
            DestroyJoint => {
                let index = self.require_joint(
                    read_object_id(argument(payload, arguments, 0, opcode)?, opcode)?,
                    opcode,
                )?;
                self.destroy_joint(index, opcode)?;
            }
            DistanceJointSetLength
            | DistanceJointEnableSpring
            | DistanceJointSetSpringForceRange
            | DistanceJointSetSpringHertz
            | DistanceJointSetSpringDampingRatio
            | DistanceJointEnableLimit
            | DistanceJointSetLengthRange
            | DistanceJointEnableMotor
            | DistanceJointSetMotorSpeed
            | DistanceJointSetMaxMotorForce => {
                self.require_joint_type(payload, arguments, DISTANCE_JOINT, opcode)?;
            }
            MotorJointSetLinearVelocity
            | MotorJointSetAngularVelocity
            | MotorJointSetMaxVelocityForce
            | MotorJointSetMaxVelocityTorque
            | MotorJointSetLinearHertz
            | MotorJointSetLinearDampingRatio
            | MotorJointSetAngularHertz
            | MotorJointSetAngularDampingRatio
            | MotorJointSetMaxSpringForce
            | MotorJointSetMaxSpringTorque => {
                self.require_joint_type(payload, arguments, MOTOR_JOINT, opcode)?;
            }
            PrismaticJointEnableSpring
            | PrismaticJointSetSpringHertz
            | PrismaticJointSetSpringDampingRatio
            | PrismaticJointSetTargetTranslation
            | PrismaticJointEnableLimit
            | PrismaticJointSetLimits
            | PrismaticJointEnableMotor
            | PrismaticJointSetMotorSpeed
            | PrismaticJointSetMaxMotorForce => {
                self.require_joint_type(payload, arguments, PRISMATIC_JOINT, opcode)?;
            }
            RevoluteJointEnableSpring
            | RevoluteJointSetSpringHertz
            | RevoluteJointSetSpringDampingRatio
            | RevoluteJointSetTargetAngle
            | RevoluteJointEnableLimit
            | RevoluteJointSetLimits
            | RevoluteJointEnableMotor
            | RevoluteJointSetMotorSpeed
            | RevoluteJointSetMaxMotorTorque => {
                self.require_joint_type(payload, arguments, REVOLUTE_JOINT, opcode)?;
            }
            WeldJointSetLinearHertz
            | WeldJointSetLinearDampingRatio
            | WeldJointSetAngularHertz
            | WeldJointSetAngularDampingRatio => {
                self.require_joint_type(payload, arguments, WELD_JOINT, opcode)?;
            }
            WheelJointEnableSpring
            | WheelJointSetSpringHertz
            | WheelJointSetSpringDampingRatio
            | WheelJointEnableLimit
            | WheelJointSetLimits
            | WheelJointEnableMotor
            | WheelJointSetMotorSpeed
            | WheelJointSetMaxMotorTorque => {
                self.require_joint_type(payload, arguments, WHEEL_JOINT, opcode)?;
            }
            DestroyWorld => self.destroyed = true,
            _ => {}
        }

        self.validate_tail_ids(tail_kind, tail, double_precision, opcode)
    }

    pub(super) fn finish(&self) -> Result<(), NativeRecordingError> {
        if self.world.is_none() || !self.destroyed {
            return Err(NativeRecordingError::StreamGrammar);
        }
        Ok(())
    }

    fn load_snapshot_metadata(
        &mut self,
        snapshot: &SnapshotValidation,
    ) -> Result<(), NativeRecordingError> {
        for entry in &snapshot.entries {
            if entry.flags & SNAPSHOT_ENTRY_LIVE == 0 {
                continue;
            }
            let index =
                usize::try_from(entry.index).map_err(|_| NativeRecordingError::ContractMismatch)?;
            match entry.kind {
                SNAPSHOT_ENTRY_BODY => {
                    let body = self
                        .bodies
                        .get_mut(index)
                        .ok_or(NativeRecordingError::ContractMismatch)?;
                    body.body_type = entry.subtype;
                }
                SNAPSHOT_ENTRY_SHAPE => {
                    let shape = self
                        .shapes
                        .get_mut(index)
                        .ok_or(NativeRecordingError::ContractMismatch)?;
                    *shape = ShapeState {
                        body: entry.owner_a,
                        parent_chain: entry.owner_b,
                        chain_order: entry.owner_b_order,
                        prev: entry.owner_a_prev,
                        next: entry.owner_a_next,
                        shape_type: entry.subtype,
                    };
                }
                SNAPSHOT_ENTRY_CHAIN => {
                    let chain = self
                        .chains
                        .get_mut(index)
                        .ok_or(NativeRecordingError::ContractMismatch)?;
                    chain.body = entry.owner_a;
                    chain.next = entry.owner_a_next;
                    chain.material_count = usize::try_from(entry.subtype)
                        .map_err(|_| NativeRecordingError::ContractMismatch)?;
                    chain.shapes = filled_vec(
                        usize::try_from(entry.color_index)
                            .map_err(|_| NativeRecordingError::ContractMismatch)?,
                        NULL_INDEX,
                    )?;
                }
                SNAPSHOT_ENTRY_JOINT => {
                    let joint = self
                        .joints
                        .get_mut(index)
                        .ok_or(NativeRecordingError::ContractMismatch)?;
                    *joint = JointState {
                        bodies: [entry.owner_a, entry.owner_b],
                        prev: [entry.owner_a_prev, entry.owner_b_prev],
                        next: [entry.owner_a_next, entry.owner_b_next],
                        joint_type: entry.subtype,
                    };
                }
                _ => {}
            }
        }
        for (shape_index, shape) in self.shapes.iter().enumerate() {
            if !self.shapes_pool.slots[shape_index].live || shape.parent_chain == NULL_INDEX {
                continue;
            }
            let chain_index = usize::try_from(shape.parent_chain)
                .map_err(|_| NativeRecordingError::ContractMismatch)?;
            let order = usize::try_from(shape.chain_order)
                .map_err(|_| NativeRecordingError::ContractMismatch)?;
            let target = self
                .chains
                .get_mut(chain_index)
                .and_then(|chain| chain.shapes.get_mut(order))
                .ok_or(NativeRecordingError::ContractMismatch)?;
            if *target != NULL_INDEX {
                return Err(NativeRecordingError::ContractMismatch);
            }
            *target =
                i32::try_from(shape_index).map_err(|_| NativeRecordingError::ContractMismatch)?;
        }
        Ok(())
    }

    fn validate_snapshot_graph(&mut self) -> Result<(), NativeRecordingError> {
        let mut shape_heads = filled_vec(self.bodies.len(), NULL_INDEX)?;
        let mut shape_counts = filled_vec(self.bodies.len(), 0usize)?;
        let mut shape_seen = filled_vec(self.shapes.len(), false)?;
        let mut chain_heads = filled_vec(self.bodies.len(), NULL_INDEX)?;
        let mut chain_counts = filled_vec(self.bodies.len(), 0usize)?;
        let mut chain_referenced = filled_vec(self.chains.len(), false)?;
        let mut chain_seen = filled_vec(self.chains.len(), false)?;
        let mut joint_heads = filled_vec(self.bodies.len(), NULL_INDEX)?;
        let mut joint_counts = filled_vec(self.bodies.len(), 0usize)?;
        let edge_count = self
            .joints
            .len()
            .checked_mul(2)
            .ok_or(NativeRecordingError::ContractMismatch)?;
        let mut joint_seen = filled_vec(edge_count, false)?;

        for (index, slot) in self.bodies_pool.slots.iter().enumerate() {
            if slot.live && self.bodies[index].body_type > DYNAMIC_BODY {
                return Err(NativeRecordingError::ContractMismatch);
            }
        }
        for index in 0..self.shapes.len() {
            if !self.shapes_pool.slots[index].live {
                continue;
            }
            let shape = self.shapes[index];
            let body = self.require_live_index(&self.bodies_pool, shape.body)?;
            shape_counts[body] = shape_counts[body]
                .checked_add(1)
                .ok_or(NativeRecordingError::ContractMismatch)?;
            if shape.prev == NULL_INDEX {
                if shape_heads[body] != NULL_INDEX {
                    return Err(NativeRecordingError::ContractMismatch);
                }
                shape_heads[body] =
                    i32::try_from(index).map_err(|_| NativeRecordingError::ContractMismatch)?;
            }
            if shape.shape_type > CHAIN_SEGMENT_SHAPE {
                return Err(NativeRecordingError::ContractMismatch);
            }
            for linked in [shape.prev, shape.next] {
                if linked != NULL_INDEX {
                    let linked_index = self.require_live_index(&self.shapes_pool, linked)?;
                    if self.shapes[linked_index].body != shape.body {
                        return Err(NativeRecordingError::ContractMismatch);
                    }
                }
            }
            if shape.parent_chain != NULL_INDEX {
                let chain = self.require_live_index(&self.chains_pool, shape.parent_chain)?;
                if self.chains[chain].body != shape.body || shape.shape_type != CHAIN_SEGMENT_SHAPE
                {
                    return Err(NativeRecordingError::ContractMismatch);
                }
            }
        }
        for index in 0..self.chains.len() {
            if !self.chains_pool.slots[index].live {
                continue;
            }
            let chain = &self.chains[index];
            let body = self.require_live_index(&self.bodies_pool, chain.body)?;
            chain_counts[body] = chain_counts[body]
                .checked_add(1)
                .ok_or(NativeRecordingError::ContractMismatch)?;
            if chain.material_count == 0
                || chain.shapes.is_empty()
                || (chain.material_count != 1 && chain.material_count != chain.shapes.len())
                || chain.shapes.contains(&NULL_INDEX)
            {
                return Err(NativeRecordingError::ContractMismatch);
            }
            if chain.next != NULL_INDEX {
                let next = self.require_live_index(&self.chains_pool, chain.next)?;
                if self.chains[next].body != chain.body {
                    return Err(NativeRecordingError::ContractMismatch);
                }
                if chain_referenced[next] {
                    return Err(NativeRecordingError::ContractMismatch);
                }
                chain_referenced[next] = true;
            }
        }
        for (index, referenced) in chain_referenced.iter().copied().enumerate() {
            if !self.chains_pool.slots[index].live || referenced {
                continue;
            }
            let body = usize::try_from(self.chains[index].body)
                .map_err(|_| NativeRecordingError::ContractMismatch)?;
            if chain_heads[body] != NULL_INDEX {
                return Err(NativeRecordingError::ContractMismatch);
            }
            chain_heads[body] =
                i32::try_from(index).map_err(|_| NativeRecordingError::ContractMismatch)?;
        }
        for index in 0..self.joints.len() {
            if !self.joints_pool.slots[index].live {
                continue;
            }
            let joint = self.joints[index];
            if joint.joint_type > WHEEL_JOINT || joint.bodies[0] == joint.bodies[1] {
                return Err(NativeRecordingError::ContractMismatch);
            }
            for edge in 0..2 {
                let body = self.require_live_index(&self.bodies_pool, joint.bodies[edge])?;
                joint_counts[body] = joint_counts[body]
                    .checked_add(1)
                    .ok_or(NativeRecordingError::ContractMismatch)?;
                if joint.prev[edge] == NULL_INDEX {
                    if joint_heads[body] != NULL_INDEX {
                        return Err(NativeRecordingError::ContractMismatch);
                    }
                    joint_heads[body] = edge_key(index, edge)?;
                }
                for key in [joint.prev[edge], joint.next[edge]] {
                    if key != NULL_INDEX {
                        let (linked_joint, linked_edge) = self.decode_edge_key(key)?;
                        if self.joints[linked_joint].bodies[linked_edge] != joint.bodies[edge] {
                            return Err(NativeRecordingError::ContractMismatch);
                        }
                    }
                }
            }
        }

        for body in 0..self.bodies.len() {
            if !self.bodies_pool.slots[body].live {
                continue;
            }
            let body_i32 =
                i32::try_from(body).map_err(|_| NativeRecordingError::ContractMismatch)?;

            let mut shape = shape_heads[body];
            let mut previous = NULL_INDEX;
            let mut observed_shapes = 0usize;
            while shape != NULL_INDEX {
                let index =
                    usize::try_from(shape).map_err(|_| NativeRecordingError::ContractMismatch)?;
                if shape_seen[index]
                    || self.shapes[index].body != body_i32
                    || self.shapes[index].prev != previous
                {
                    return Err(NativeRecordingError::ContractMismatch);
                }
                shape_seen[index] = true;
                observed_shapes = observed_shapes
                    .checked_add(1)
                    .ok_or(NativeRecordingError::ContractMismatch)?;
                previous = shape;
                shape = self.shapes[index].next;
            }
            if observed_shapes != shape_counts[body] {
                return Err(NativeRecordingError::ContractMismatch);
            }

            let mut chain = chain_heads[body];
            let mut observed_chains = 0usize;
            while chain != NULL_INDEX {
                let index =
                    usize::try_from(chain).map_err(|_| NativeRecordingError::ContractMismatch)?;
                if chain_seen[index] || self.chains[index].body != body_i32 {
                    return Err(NativeRecordingError::ContractMismatch);
                }
                chain_seen[index] = true;
                observed_chains = observed_chains
                    .checked_add(1)
                    .ok_or(NativeRecordingError::ContractMismatch)?;
                chain = self.chains[index].next;
            }
            if observed_chains != chain_counts[body] {
                return Err(NativeRecordingError::ContractMismatch);
            }

            let mut key = joint_heads[body];
            let mut previous_key = NULL_INDEX;
            let mut observed_joints = 0usize;
            while key != NULL_INDEX {
                let (joint, edge) = self.decode_edge_key(key)?;
                let seen_index =
                    usize::try_from(key).map_err(|_| NativeRecordingError::ContractMismatch)?;
                if joint_seen[seen_index]
                    || self.joints[joint].bodies[edge] != body_i32
                    || self.joints[joint].prev[edge] != previous_key
                {
                    return Err(NativeRecordingError::ContractMismatch);
                }
                joint_seen[seen_index] = true;
                observed_joints = observed_joints
                    .checked_add(1)
                    .ok_or(NativeRecordingError::ContractMismatch)?;
                previous_key = key;
                key = self.joints[joint].next[edge];
            }
            if observed_joints != joint_counts[body] {
                return Err(NativeRecordingError::ContractMismatch);
            }

            self.bodies[body].head_shape = shape_heads[body];
            self.bodies[body].head_chain = chain_heads[body];
            self.bodies[body].head_joint = joint_heads[body];
        }
        Ok(())
    }

    fn require_world(
        &mut self,
        id: WorldId,
        may_initialize: bool,
        opcode: u8,
    ) -> Result<(), NativeRecordingError> {
        if id.index1 == 0 {
            return Err(NativeRecordingError::InvalidReference(opcode));
        }
        match self.world {
            Some(expected) if expected == id => Ok(()),
            Some(_) => Err(NativeRecordingError::InvalidReference(opcode)),
            None if may_initialize => {
                self.world = Some(id);
                Ok(())
            }
            None => Err(NativeRecordingError::InvalidLifecycle(opcode)),
        }
    }

    fn require_object_world(&self, id: ObjectId, opcode: u8) -> Result<(), NativeRecordingError> {
        let world = self
            .world
            .ok_or(NativeRecordingError::InvalidLifecycle(opcode))?;
        if id.index1 <= 0 || id.world0 != world.index1 - 1 {
            return Err(NativeRecordingError::InvalidReference(opcode));
        }
        Ok(())
    }

    fn require_body(&self, id: ObjectId, opcode: u8) -> Result<usize, NativeRecordingError> {
        self.require_object_world(id, opcode)?;
        self.bodies_pool.require(id, opcode)
    }

    fn require_shape(&self, id: ObjectId, opcode: u8) -> Result<usize, NativeRecordingError> {
        self.require_object_world(id, opcode)?;
        self.shapes_pool.require(id, opcode)
    }

    fn require_chain(&self, id: ObjectId, opcode: u8) -> Result<usize, NativeRecordingError> {
        self.require_object_world(id, opcode)?;
        self.chains_pool.require(id, opcode)
    }

    fn require_joint(&self, id: ObjectId, opcode: u8) -> Result<usize, NativeRecordingError> {
        self.require_object_world(id, opcode)?;
        self.joints_pool.require(id, opcode)
    }

    fn create_body(
        &mut self,
        body_def: &[u8],
        returned: &[u8],
        return_kind: ReturnKind,
        opcode: u8,
    ) -> Result<(), NativeRecordingError> {
        if return_kind != ReturnKind::Body {
            return Err(NativeRecordingError::GeneratedContract);
        }
        let body_type = read_i32(body_def, 0, opcode)?;
        let (index, generation) = self.bodies_pool.allocate(opcode)?;
        ensure_len(&mut self.bodies, index)?;
        self.bodies[index] = BodyState {
            body_type: u32::try_from(body_type)
                .map_err(|_| NativeRecordingError::InvalidEnum(opcode))?,
            ..BodyState::default()
        };
        self.require_returned_id(returned, index, generation, opcode)
    }

    fn create_shape_from_record(
        &mut self,
        payload: &[u8],
        arguments: &[Range<usize>],
        returned: &[u8],
        return_kind: ReturnKind,
        shape_type: u32,
        opcode: u8,
    ) -> Result<(), NativeRecordingError> {
        if return_kind != ReturnKind::Shape {
            return Err(NativeRecordingError::GeneratedContract);
        }
        let body = self.require_body(
            read_object_id(argument(payload, arguments, 0, opcode)?, opcode)?,
            opcode,
        )?;
        let (index, generation) =
            self.allocate_shape(body, NULL_INDEX, NULL_INDEX, shape_type, opcode)?;
        self.require_returned_id(returned, index, generation, opcode)
    }

    fn allocate_shape(
        &mut self,
        body: usize,
        parent_chain: i32,
        chain_order: i32,
        shape_type: u32,
        opcode: u8,
    ) -> Result<(usize, u16), NativeRecordingError> {
        let (index, generation) = self.shapes_pool.allocate(opcode)?;
        ensure_len(&mut self.shapes, index)?;
        let old_head = self.bodies[body].head_shape;
        let index_i32 =
            i32::try_from(index).map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
        if old_head != NULL_INDEX {
            let old = usize::try_from(old_head)
                .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
            self.shapes[old].prev = index_i32;
        }
        self.shapes[index] = ShapeState {
            body: i32::try_from(body)
                .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?,
            parent_chain,
            chain_order,
            prev: NULL_INDEX,
            next: old_head,
            shape_type,
        };
        self.bodies[body].head_shape = index_i32;
        Ok((index, generation))
    }

    fn set_shape_type(
        &mut self,
        payload: &[u8],
        arguments: &[Range<usize>],
        shape_type: u32,
        opcode: u8,
    ) -> Result<(), NativeRecordingError> {
        let index = self.require_shape(
            read_object_id(argument(payload, arguments, 0, opcode)?, opcode)?,
            opcode,
        )?;
        if self.shapes[index].parent_chain != NULL_INDEX {
            return Err(NativeRecordingError::InvalidLifecycle(opcode));
        }
        self.shapes[index].shape_type = shape_type;
        Ok(())
    }

    fn create_chain(
        &mut self,
        payload: &[u8],
        arguments: &[Range<usize>],
        returned: &[u8],
        return_kind: ReturnKind,
        opcode: u8,
    ) -> Result<(), NativeRecordingError> {
        if return_kind != ReturnKind::Chain {
            return Err(NativeRecordingError::GeneratedContract);
        }
        let body = self.require_body(
            read_object_id(argument(payload, arguments, 0, opcode)?, opcode)?,
            opcode,
        )?;
        let (point_count, material_count, is_loop) =
            read_chain_def(argument(payload, arguments, 1, opcode)?, opcode)?;
        let shape_count = if is_loop {
            point_count
        } else {
            point_count
                .checked_sub(3)
                .ok_or(NativeRecordingError::InvalidCount(opcode))?
        };
        let (chain_index, generation) = self.chains_pool.allocate(opcode)?;
        ensure_len(&mut self.chains, chain_index)?;
        let chain_i32 = i32::try_from(chain_index)
            .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
        let old_head = self.bodies[body].head_chain;
        let mut shapes = Vec::new();
        shapes
            .try_reserve_exact(shape_count)
            .map_err(|_| NativeRecordingError::AllocationFailed)?;
        self.bodies[body].head_chain = chain_i32;
        for order in 0..shape_count {
            let (shape, _) = self.allocate_shape(
                body,
                chain_i32,
                i32::try_from(order).map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?,
                CHAIN_SEGMENT_SHAPE,
                opcode,
            )?;
            shapes.push(
                i32::try_from(shape).map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?,
            );
        }
        self.chains[chain_index] = ChainState {
            body: i32::try_from(body)
                .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?,
            next: old_head,
            material_count,
            shapes,
        };
        self.require_returned_id(returned, chain_index, generation, opcode)
    }

    fn create_joint(
        &mut self,
        payload: &[u8],
        arguments: &[Range<usize>],
        returned: &[u8],
        return_kind: ReturnKind,
        joint_type: u32,
        opcode: u8,
    ) -> Result<(), NativeRecordingError> {
        if return_kind != ReturnKind::Joint {
            return Err(NativeRecordingError::GeneratedContract);
        }
        let definition = argument(payload, arguments, 1, opcode)?;
        let body_a_id = read_object_id_at(definition, 8, opcode)?;
        let body_b_id = read_object_id_at(definition, 16, opcode)?;
        if body_a_id == body_b_id {
            return Err(NativeRecordingError::InvalidLifecycle(opcode));
        }
        let body_a = self.require_body(body_a_id, opcode)?;
        let body_b = self.require_body(body_b_id, opcode)?;
        let (index, generation) = self.joints_pool.allocate(opcode)?;
        ensure_len(&mut self.joints, index)?;
        let key_a = edge_key(index, 0)?;
        let key_b = edge_key(index, 1)?;
        let next_a = self.bodies[body_a].head_joint;
        let next_b = self.bodies[body_b].head_joint;
        if next_a != NULL_INDEX {
            let (joint, edge) = self.decode_edge_key(next_a)?;
            self.joints[joint].prev[edge] = key_a;
        }
        if next_b != NULL_INDEX {
            let (joint, edge) = self.decode_edge_key(next_b)?;
            self.joints[joint].prev[edge] = key_b;
        }
        self.joints[index] = JointState {
            bodies: [
                i32::try_from(body_a)
                    .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?,
                i32::try_from(body_b)
                    .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?,
            ],
            prev: [NULL_INDEX; 2],
            next: [next_a, next_b],
            joint_type,
        };
        self.bodies[body_a].head_joint = key_a;
        self.bodies[body_b].head_joint = key_b;
        self.require_returned_id(returned, index, generation, opcode)
    }

    fn destroy_body(&mut self, index: usize, opcode: u8) -> Result<(), NativeRecordingError> {
        // Match b2DestroyBody exactly: joints first, then the complete shape list (including
        // chain-owned segments), then chain slots, and finally the body slot. Pool free lists are
        // LIFO, so changing this order would predict different returned IDs for later creates.
        while self.bodies[index].head_joint != NULL_INDEX {
            let (joint, _) = self.decode_edge_key(self.bodies[index].head_joint)?;
            self.destroy_joint(joint, opcode)?;
        }
        while self.bodies[index].head_shape != NULL_INDEX {
            let shape = usize::try_from(self.bodies[index].head_shape)
                .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
            self.destroy_shape_internal(shape, opcode)?;
        }
        while self.bodies[index].head_chain != NULL_INDEX {
            let chain = usize::try_from(self.bodies[index].head_chain)
                .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
            self.bodies[index].head_chain = self.chains[chain].next;
            self.chains_pool.release(chain, opcode)?;
            self.chains[chain] = ChainState::default();
        }
        self.bodies_pool.release(index, opcode)?;
        self.bodies[index] = BodyState::default();
        Ok(())
    }

    fn destroy_shape_internal(
        &mut self,
        index: usize,
        opcode: u8,
    ) -> Result<(), NativeRecordingError> {
        let shape = self.shapes[index];
        let body = usize::try_from(shape.body)
            .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
        if shape.prev != NULL_INDEX {
            let prev = usize::try_from(shape.prev)
                .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
            self.shapes[prev].next = shape.next;
        }
        if shape.next != NULL_INDEX {
            let next = usize::try_from(shape.next)
                .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
            self.shapes[next].prev = shape.prev;
        }
        if self.bodies[body].head_shape
            == i32::try_from(index).map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?
        {
            self.bodies[body].head_shape = shape.next;
        }
        self.shapes_pool.release(index, opcode)?;
        self.shapes[index] = ShapeState::default();
        Ok(())
    }

    fn destroy_chain(&mut self, index: usize, opcode: u8) -> Result<(), NativeRecordingError> {
        let body = usize::try_from(self.chains[index].body)
            .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
        let index_i32 =
            i32::try_from(index).map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
        if self.bodies[body].head_chain == index_i32 {
            self.bodies[body].head_chain = self.chains[index].next;
        } else {
            let mut cursor = self.bodies[body].head_chain;
            let mut found = false;
            while cursor != NULL_INDEX {
                let chain = usize::try_from(cursor)
                    .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
                if self.chains[chain].next == index_i32 {
                    self.chains[chain].next = self.chains[index].next;
                    found = true;
                    break;
                }
                cursor = self.chains[chain].next;
            }
            if !found {
                return Err(NativeRecordingError::InvalidLifecycle(opcode));
            }
        }
        let shapes = std::mem::take(&mut self.chains[index].shapes);
        for shape in shapes {
            let shape = usize::try_from(shape)
                .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
            if !self
                .shapes_pool
                .slots
                .get(shape)
                .is_some_and(|slot| slot.live)
                || self.shapes[shape].parent_chain != index_i32
            {
                return Err(NativeRecordingError::InvalidLifecycle(opcode));
            }
            self.destroy_shape_internal(shape, opcode)?;
        }
        self.chains_pool.release(index, opcode)?;
        self.chains[index] = ChainState::default();
        Ok(())
    }

    fn destroy_joint(&mut self, index: usize, opcode: u8) -> Result<(), NativeRecordingError> {
        let joint = self.joints[index];
        for edge in 0..2 {
            let body = usize::try_from(joint.bodies[edge])
                .map_err(|_| NativeRecordingError::InvalidLifecycle(opcode))?;
            if joint.prev[edge] != NULL_INDEX {
                let (prev_joint, prev_edge) = self.decode_edge_key(joint.prev[edge])?;
                self.joints[prev_joint].next[prev_edge] = joint.next[edge];
            }
            if joint.next[edge] != NULL_INDEX {
                let (next_joint, next_edge) = self.decode_edge_key(joint.next[edge])?;
                self.joints[next_joint].prev[next_edge] = joint.prev[edge];
            }
            let key = edge_key(index, edge)?;
            if self.bodies[body].head_joint == key {
                self.bodies[body].head_joint = joint.next[edge];
            }
        }
        self.joints_pool.release(index, opcode)?;
        self.joints[index] = JointState::default();
        Ok(())
    }

    fn require_joint_type(
        &self,
        payload: &[u8],
        arguments: &[Range<usize>],
        expected: u32,
        opcode: u8,
    ) -> Result<(), NativeRecordingError> {
        let index = self.require_joint(
            read_object_id(argument(payload, arguments, 0, opcode)?, opcode)?,
            opcode,
        )?;
        if self.joints[index].joint_type != expected {
            return Err(NativeRecordingError::InvalidLifecycle(opcode));
        }
        Ok(())
    }

    fn require_returned_id(
        &self,
        bytes: &[u8],
        index: usize,
        generation: u16,
        opcode: u8,
    ) -> Result<(), NativeRecordingError> {
        let actual = read_object_id(bytes, opcode)?;
        let world = self
            .world
            .ok_or(NativeRecordingError::InvalidLifecycle(opcode))?;
        let expected = ObjectId {
            index1: i32::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(NativeRecordingError::InvalidLifecycle(opcode))?,
            world0: world.index1 - 1,
            generation,
        };
        if actual != expected {
            return Err(NativeRecordingError::InvalidLifecycle(opcode));
        }
        Ok(())
    }

    fn validate_tail_ids(
        &self,
        kind: TailKind,
        bytes: &[u8],
        double_precision: bool,
        opcode: u8,
    ) -> Result<(), NativeRecordingError> {
        let position_width = if double_precision { 16 } else { 8 };
        let mut cursor = TailCursor::new(bytes, opcode);
        match kind {
            TailKind::OverlapHits => {
                let count = cursor.u32()?;
                for _ in 0..count {
                    self.require_shape(cursor.object_id()?, opcode)?;
                    cursor.skip(1)?;
                }
                cursor.skip(8)?;
            }
            TailKind::CastHits => {
                let count = cursor.u32()?;
                for _ in 0..count {
                    self.require_shape(cursor.object_id()?, opcode)?;
                    cursor.skip(position_width + 8 + 4 + 4)?;
                }
                cursor.skip(8)?;
            }
            TailKind::PlaneHits => {
                let count = cursor.u32()?;
                for _ in 0..count {
                    self.require_shape(cursor.object_id()?, opcode)?;
                    cursor.skip(8 + 4 + 8 + 1 + 1)?;
                }
            }
            TailKind::ClosestRayResult => {
                let id = cursor.object_id()?;
                cursor.skip(position_width + 8 + 4 + 4 + 4)?;
                let hit = cursor.u8()? != 0;
                if hit {
                    self.require_shape(id, opcode)?;
                } else if id.index1 != 0 || id.world0 != 0 || id.generation != 0 {
                    return Err(NativeRecordingError::InvalidReference(opcode));
                }
            }
            TailKind::ReturnedId
            | TailKind::None
            | TailKind::MoverResult
            | TailKind::BoolResult
            | TailKind::ShapeCastResult => return Ok(()),
        }
        cursor.finish()
    }

    fn require_live_index(&self, pool: &Pool, index: i32) -> Result<usize, NativeRecordingError> {
        let index = usize::try_from(index).map_err(|_| NativeRecordingError::ContractMismatch)?;
        if !pool.slots.get(index).is_some_and(|slot| slot.live) {
            return Err(NativeRecordingError::ContractMismatch);
        }
        Ok(index)
    }

    fn decode_edge_key(&self, key: i32) -> Result<(usize, usize), NativeRecordingError> {
        if key < 0 {
            return Err(NativeRecordingError::ContractMismatch);
        }
        let joint =
            usize::try_from(key >> 1).map_err(|_| NativeRecordingError::ContractMismatch)?;
        let edge = usize::try_from(key & 1).map_err(|_| NativeRecordingError::ContractMismatch)?;
        if !self
            .joints_pool
            .slots
            .get(joint)
            .is_some_and(|slot| slot.live)
        {
            return Err(NativeRecordingError::ContractMismatch);
        }
        Ok((joint, edge))
    }
}

fn edge_key(joint: usize, edge: usize) -> Result<i32, NativeRecordingError> {
    if edge > 1 {
        return Err(NativeRecordingError::ContractMismatch);
    }
    joint
        .checked_mul(2)
        .and_then(|value| value.checked_add(edge))
        .and_then(|value| i32::try_from(value).ok())
        .ok_or(NativeRecordingError::ContractMismatch)
}

fn filled_vec<T: Clone>(length: usize, value: T) -> Result<Vec<T>, NativeRecordingError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| NativeRecordingError::AllocationFailed)?;
    values.resize(length, value);
    Ok(values)
}

fn ensure_len<T: Default>(values: &mut Vec<T>, index: usize) -> Result<(), NativeRecordingError> {
    if index >= values.len() {
        values
            .try_reserve(index + 1 - values.len())
            .map_err(|_| NativeRecordingError::AllocationFailed)?;
        values.resize_with(index + 1, T::default);
    }
    Ok(())
}

fn read_chain_def(bytes: &[u8], opcode: u8) -> Result<(usize, usize, bool), NativeRecordingError> {
    let point_count = usize::try_from(read_i32(bytes, 8, opcode)?)
        .map_err(|_| NativeRecordingError::InvalidCount(opcode))?;
    let material_count_offset = 12usize
        .checked_add(
            point_count
                .checked_mul(8)
                .ok_or(NativeRecordingError::InvalidCount(opcode))?,
        )
        .ok_or(NativeRecordingError::InvalidCount(opcode))?;
    let material_count = usize::try_from(read_i32(bytes, material_count_offset, opcode)?)
        .map_err(|_| NativeRecordingError::InvalidCount(opcode))?;
    let is_loop_offset = material_count_offset
        .checked_add(4)
        .and_then(|value| value.checked_add(material_count.checked_mul(28)?))
        .and_then(|value| value.checked_add(20))
        .ok_or(NativeRecordingError::InvalidCount(opcode))?;
    let is_loop = *bytes
        .get(is_loop_offset)
        .ok_or(NativeRecordingError::InvalidCount(opcode))?
        != 0;
    Ok((point_count, material_count, is_loop))
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
        .ok_or(NativeRecordingError::PayloadTruncated(opcode))
}

fn read_world_id(bytes: &[u8], opcode: u8) -> Result<WorldId, NativeRecordingError> {
    let raw: [u8; 4] = bytes
        .try_into()
        .map_err(|_| NativeRecordingError::InvalidReference(opcode))?;
    let raw = u32::from_le_bytes(raw);
    Ok(WorldId {
        index1: (raw >> 16) as u16,
        generation: raw as u16,
    })
}

fn read_object_id(bytes: &[u8], opcode: u8) -> Result<ObjectId, NativeRecordingError> {
    read_object_id_at(bytes, 0, opcode)
}

fn read_object_id_at(
    bytes: &[u8],
    offset: usize,
    opcode: u8,
) -> Result<ObjectId, NativeRecordingError> {
    let raw: [u8; 8] = bytes
        .get(
            offset
                ..offset
                    .checked_add(8)
                    .ok_or(NativeRecordingError::InvalidReference(opcode))?,
        )
        .ok_or(NativeRecordingError::InvalidReference(opcode))?
        .try_into()
        .map_err(|_| NativeRecordingError::InvalidReference(opcode))?;
    let raw = u64::from_le_bytes(raw);
    Ok(ObjectId {
        index1: (raw >> 32) as u32 as i32,
        world0: (raw >> 16) as u16,
        generation: raw as u16,
    })
}

fn read_i32(bytes: &[u8], offset: usize, opcode: u8) -> Result<i32, NativeRecordingError> {
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

struct TailCursor<'a> {
    bytes: &'a [u8],
    cursor: usize,
    opcode: u8,
}

impl<'a> TailCursor<'a> {
    fn new(bytes: &'a [u8], opcode: u8) -> Self {
        Self {
            bytes,
            cursor: 0,
            opcode,
        }
    }

    fn take(&mut self, width: usize) -> Result<&'a [u8], NativeRecordingError> {
        let end = self
            .cursor
            .checked_add(width)
            .ok_or(NativeRecordingError::PayloadTruncated(self.opcode))?;
        let bytes = self
            .bytes
            .get(self.cursor..end)
            .ok_or(NativeRecordingError::PayloadTruncated(self.opcode))?;
        self.cursor = end;
        Ok(bytes)
    }

    fn skip(&mut self, width: usize) -> Result<(), NativeRecordingError> {
        self.take(width).map(|_| ())
    }

    fn u8(&mut self) -> Result<u8, NativeRecordingError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, NativeRecordingError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(
            |_| NativeRecordingError::PayloadTruncated(self.opcode),
        )?))
    }

    fn object_id(&mut self) -> Result<ObjectId, NativeRecordingError> {
        read_object_id(self.take(8)?, self.opcode)
    }

    fn finish(self) -> Result<(), NativeRecordingError> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(NativeRecordingError::PayloadMismatch {
                opcode: self.opcode,
                operation: "query result tail",
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_reuse_invalidates_the_old_generation_and_preserves_lifo_order() {
        const OPCODE: u8 = 0xA5;
        let mut pool = Pool {
            slots: Vec::new(),
            free: Vec::new(),
        };

        let (first_index, first_generation) = pool.allocate(OPCODE).unwrap();
        let first = ObjectId {
            index1: i32::try_from(first_index).unwrap() + 1,
            world0: 0,
            generation: first_generation,
        };
        assert_eq!(pool.require(first, OPCODE), Ok(first_index));

        pool.release(first_index, OPCODE).unwrap();
        assert_eq!(
            pool.require(first, OPCODE),
            Err(NativeRecordingError::InvalidReference(OPCODE))
        );

        let (reused_index, reused_generation) = pool.allocate(OPCODE).unwrap();
        assert_eq!(reused_index, first_index);
        assert_ne!(reused_generation, first_generation);
        assert_eq!(
            pool.require(first, OPCODE),
            Err(NativeRecordingError::InvalidReference(OPCODE))
        );
        assert_eq!(
            pool.require(
                ObjectId {
                    generation: reused_generation,
                    ..first
                },
                OPCODE,
            ),
            Ok(reused_index)
        );
    }
}
