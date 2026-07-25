//! Complete Rust-owned authorization boundary for native recording replay.

mod lifecycle;
mod semantic;

use boxdd_sys::{
    adapter::{self, AdapterIdentity, SnapshotLimits},
    ffi,
};

const RECORDING_HEADER_BYTES: usize = 32;
const RECORDING_MAGIC: u32 = 0x4352_3242;
const RECORDING_VERSION_MAJOR: u16 = 3;
const RECORDING_VERSION_MINOR: u16 = 2;
const MAX_RECORDING_BYTES: usize = 256 * 1024 * 1024;
const MAX_RECORDS: usize = 1_000_000;
const MAX_VALIDATION_WORK: u64 = 16_000_000;
const FRAME_HEADER_BYTES: usize = 4;
const PROGRAM_TAKE: u8 = 1;
const PROGRAM_BOOL: u8 = 2;
const PROGRAM_PRECISION: u8 = 3;
const PROGRAM_STRING: u8 = 4;
const PROGRAM_NATIVE_POD: u8 = 5;
const PROGRAM_COUNTED: u8 = 6;
const MAX_PROGRAM_DEPTH: usize = 16;
const MAX_OPERATION_ARGUMENTS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SemanticClass {
    Mutation,
    Query,
    Step,
    StateHash,
    Metadata,
    Terminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StreamRole {
    Ordinary,
    Initial,
    FinalMetadata,
    Terminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArgumentTag {
    Aabb,
    BodyDef,
    BodyId,
    Bool,
    Capsule,
    ChainDef,
    ChainId,
    ChainSegment,
    Circle,
    DistanceJointDef,
    ExplosionDef,
    F32,
    Filter,
    FilterJointDef,
    I32,
    JointId,
    Locks,
    MassData,
    Material,
    MotorJointDef,
    Polygon,
    Position,
    PrismaticJointDef,
    QueryFilter,
    RevoluteJointDef,
    Rot,
    Segment,
    ShapeDef,
    ShapeId,
    ShapeProxy,
    Str,
    U64,
    Vec2,
    WeldJointDef,
    WheelJointDef,
    WorldId,
    WorldTransform,
    Transform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TailKind {
    None,
    ReturnedId,
    OverlapHits,
    CastHits,
    PlaneHits,
    ClosestRayResult,
    MoverResult,
    BoolResult,
    ShapeCastResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReturnKind {
    None,
    Body,
    Shape,
    Chain,
    Joint,
}

#[derive(Clone, Copy, Debug)]
struct Argument {
    tag: ArgumentTag,
    program_end: u16,
}

#[derive(Clone, Copy, Debug)]
struct Operation {
    opcode: u8,
    name: &'static str,
    semantic: SemanticClass,
    role: StreamRole,
    rule: generated::OperationRule,
    return_kind: ReturnKind,
    tail: TailKind,
    arguments: &'static [Argument],
    program: &'static [u8],
}

mod generated {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/generated/recording_wire.rs"
    ));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PreflightError {
    InputTooLarge,
    Truncated,
    HeaderMismatch,
    ReservedField,
    InvalidLengthScale,
    AdapterUnavailable,
    AdapterMismatch,
    SnapshotLength,
    SnapshotRejected(u32),
    ContractMismatch,
    FrameTruncated,
    UnknownOpcode(u8),
    PayloadTruncated(u8),
    PayloadMismatch { opcode: u8, operation: &'static str },
    InvalidBoolean(u8),
    InvalidString(u8),
    InvalidCount(u8),
    InvalidValue(u8),
    InvalidEnum(u8),
    InvalidRange(u8),
    InvalidReference(u8),
    InvalidLifecycle(u8),
    UnsupportedCallbacks,
    StreamGrammar,
    GeneratedContract,
    WorkLimit,
    AllocationFailed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PreflightInfo {
    pub(super) length_units_per_meter: f32,
    pub(super) snapshot_offset: usize,
    pub(super) snapshot_bytes: usize,
    pub(super) records: usize,
    pub(super) steps: usize,
    pub(super) queries: usize,
}

#[derive(Debug)]
pub(super) struct ValidatedRecording {
    bytes: Box<[u8]>,
    info: PreflightInfo,
}

impl ValidatedRecording {
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(super) const fn info(&self) -> PreflightInfo {
        self.info
    }
}

pub(super) fn preflight_recording(bytes: &[u8]) -> Result<ValidatedRecording, PreflightError> {
    if bytes.len() > MAX_RECORDING_BYTES {
        return Err(PreflightError::InputTooLarge);
    }
    let identity = adapter::verify_runtime_identity().map_err(map_adapter_identity_error)?;
    validate_generated_identity()?;
    let header = parse_header(bytes, &identity)?;
    let snapshot_end = header
        .snapshot_offset
        .checked_add(header.snapshot_bytes)
        .ok_or(PreflightError::SnapshotLength)?;
    let snapshot_bytes = bytes
        .get(header.snapshot_offset..snapshot_end)
        .ok_or(PreflightError::SnapshotLength)?;
    let snapshot = adapter::validate_snapshot(snapshot_bytes, &SnapshotLimits::default())
        .map_err(map_snapshot_validation_error)?;
    if snapshot.facts.requires_custom_filter != 0 || snapshot.facts.requires_pre_solve != 0 {
        return Err(PreflightError::UnsupportedCallbacks);
    }
    let mut lifecycle = lifecycle::Lifecycle::from_snapshot(&snapshot)?;
    let stream = validate_stream(
        bytes,
        snapshot_end,
        identity.double_precision != 0,
        header.length_units_per_meter,
        &mut lifecycle,
    )?;

    let mut owned = Vec::new();
    owned
        .try_reserve_exact(bytes.len())
        .map_err(|_| PreflightError::AllocationFailed)?;
    owned.extend_from_slice(bytes);
    Ok(ValidatedRecording {
        bytes: owned.into_boxed_slice(),
        info: PreflightInfo {
            records: stream.records,
            steps: stream.steps,
            queries: stream.queries,
            ..header
        },
    })
}

fn map_snapshot_validation_error(error: adapter::SnapshotValidationError) -> PreflightError {
    match error {
        adapter::SnapshotValidationError::AdapterIdentity(error) => {
            map_adapter_identity_error(error)
        }
        adapter::SnapshotValidationError::Status(status) => {
            PreflightError::SnapshotRejected(status)
        }
        _ => PreflightError::AdapterMismatch,
    }
}

fn map_adapter_identity_error(error: adapter::AdapterIdentityError) -> PreflightError {
    match error {
        adapter::AdapterIdentityError::Unavailable => PreflightError::AdapterUnavailable,
        adapter::AdapterIdentityError::Mismatch(_) => PreflightError::AdapterMismatch,
        _ => PreflightError::AdapterMismatch,
    }
}

fn validate_generated_identity() -> Result<(), PreflightError> {
    if generated::UPSTREAM_SHA != boxdd_sys::UPSTREAM_SHA
        || generated::EFFECTIVE_SOURCE_SHA256 != boxdd_sys::EFFECTIVE_SOURCE_SHA256
        || generated::CONTRACT_BLAKE3 != boxdd_sys::RECORDING_CONTRACT_BLAKE3
    {
        return Err(PreflightError::AdapterMismatch);
    }
    Ok(())
}

struct StreamFacts {
    records: usize,
    steps: usize,
    queries: usize,
}

fn validate_stream(
    bytes: &[u8],
    stream_offset: usize,
    double_precision: bool,
    length_scale: f32,
    lifecycle: &mut lifecycle::Lifecycle,
) -> Result<StreamFacts, PreflightError> {
    if generated::OPERATIONS.is_empty()
        || generated::OPERATIONS
            .windows(2)
            .any(|pair| pair[0].opcode >= pair[1].opcode)
    {
        return Err(PreflightError::ContractMismatch);
    }

    let mut cursor = stream_offset;
    let mut records = 0usize;
    let mut steps = 0usize;
    let mut queries = 0usize;
    let mut work = 0u64;
    let mut final_metadata = 0usize;
    let mut terminals = 0usize;
    let mut previous_role = StreamRole::Ordinary;
    let mut last_role = StreamRole::Ordinary;
    let mut state_hash_required = false;

    while cursor < bytes.len() {
        if records >= MAX_RECORDS {
            return Err(PreflightError::WorkLimit);
        }
        work = work.checked_add(1).ok_or(PreflightError::WorkLimit)?;
        if work > MAX_VALIDATION_WORK {
            return Err(PreflightError::WorkLimit);
        }

        let frame_header_end = cursor
            .checked_add(FRAME_HEADER_BYTES)
            .ok_or(PreflightError::FrameTruncated)?;
        let frame_header = bytes
            .get(cursor..frame_header_end)
            .ok_or(PreflightError::FrameTruncated)?;
        let opcode = frame_header[0];
        let operation = operation(opcode).ok_or(PreflightError::UnknownOpcode(opcode))?;
        let payload_size = usize::from(frame_header[1])
            | (usize::from(frame_header[2]) << 8)
            | (usize::from(frame_header[3]) << 16);
        let payload_end = frame_header_end
            .checked_add(payload_size)
            .ok_or(PreflightError::PayloadTruncated(opcode))?;
        let payload = bytes
            .get(frame_header_end..payload_end)
            .ok_or(PreflightError::PayloadTruncated(opcode))?;
        let mut interpreter = PayloadInterpreter {
            bytes: payload,
            cursor: 0,
            double_precision,
            opcode,
            work: &mut work,
        };
        if operation.arguments.len() > MAX_OPERATION_ARGUMENTS {
            return Err(PreflightError::GeneratedContract);
        }
        let mut argument_ranges: [std::ops::Range<usize>; MAX_OPERATION_ARGUMENTS] =
            std::array::from_fn(|_| 0..0);
        let mut program_start = 0usize;
        for (argument_index, argument) in operation.arguments.iter().enumerate() {
            let program_end = usize::from(argument.program_end);
            let argument_program = operation
                .program
                .get(program_start..program_end)
                .ok_or(PreflightError::GeneratedContract)?;
            let argument_start = interpreter.cursor;
            interpreter.consume_program(argument_program, 0)?;
            let argument_end = interpreter.cursor;
            argument_ranges[argument_index] = argument_start..argument_end;
            semantic::validate_argument(
                argument.tag,
                payload
                    .get(argument_start..argument_end)
                    .ok_or(PreflightError::GeneratedContract)?,
                double_precision,
                length_scale,
                opcode,
            )?;
            program_start = program_end;
        }
        let tail_program = operation
            .program
            .get(program_start..)
            .ok_or(PreflightError::GeneratedContract)?;
        let tail_start = interpreter.cursor;
        interpreter.consume_program(tail_program, 0)?;
        if interpreter.cursor != payload.len() {
            return Err(PreflightError::PayloadMismatch {
                opcode,
                operation: operation.name,
            });
        }
        semantic::validate_operation(
            operation.rule,
            payload,
            &argument_ranges[..operation.arguments.len()],
            length_scale,
            opcode,
        )?;
        semantic::validate_tail(
            operation.tail,
            operation.return_kind,
            payload
                .get(tail_start..)
                .ok_or(PreflightError::GeneratedContract)?,
            double_precision,
            opcode,
        )?;
        lifecycle.observe(lifecycle::Observation {
            rule: operation.rule,
            return_kind: operation.return_kind,
            tail_kind: operation.tail,
            argument_defs: operation.arguments,
            payload,
            arguments: &argument_ranges[..operation.arguments.len()],
            tail: payload
                .get(tail_start..)
                .ok_or(PreflightError::GeneratedContract)?,
            double_precision,
            opcode,
        })?;

        if records == 0 && operation.role != StreamRole::Initial {
            return Err(PreflightError::StreamGrammar);
        }
        if records != 0 {
            if state_hash_required {
                if operation.semantic != SemanticClass::StateHash {
                    return Err(PreflightError::StreamGrammar);
                }
                state_hash_required = false;
            } else if operation.semantic == SemanticClass::StateHash {
                return Err(PreflightError::StreamGrammar);
            }
        }
        if operation.role == StreamRole::Terminal && payload_end != bytes.len() {
            return Err(PreflightError::StreamGrammar);
        }
        match operation.semantic {
            SemanticClass::Step => {
                steps = steps.checked_add(1).ok_or(PreflightError::WorkLimit)?;
                state_hash_required = true;
            }
            SemanticClass::Query => {
                queries = queries.checked_add(1).ok_or(PreflightError::WorkLimit)?;
            }
            SemanticClass::Mutation
            | SemanticClass::StateHash
            | SemanticClass::Metadata
            | SemanticClass::Terminal => {}
        }
        if operation.role == StreamRole::FinalMetadata {
            final_metadata = final_metadata
                .checked_add(1)
                .ok_or(PreflightError::WorkLimit)?;
        }
        if operation.role == StreamRole::Terminal {
            terminals = terminals.checked_add(1).ok_or(PreflightError::WorkLimit)?;
        }
        records += 1;
        previous_role = last_role;
        last_role = operation.role;
        cursor = payload_end;
    }

    if cursor != bytes.len()
        || records < 3
        || previous_role != StreamRole::FinalMetadata
        || last_role != StreamRole::Terminal
        || final_metadata != 1
        || terminals != 1
        || state_hash_required
    {
        return Err(PreflightError::StreamGrammar);
    }
    lifecycle.finish()?;
    Ok(StreamFacts {
        records,
        steps,
        queries,
    })
}

fn operation(opcode: u8) -> Option<&'static Operation> {
    generated::OPERATIONS
        .binary_search_by_key(&opcode, |operation| operation.opcode)
        .ok()
        .map(|index| &generated::OPERATIONS[index])
}

struct PayloadInterpreter<'payload, 'work> {
    bytes: &'payload [u8],
    cursor: usize,
    double_precision: bool,
    opcode: u8,
    work: &'work mut u64,
}

impl PayloadInterpreter<'_, '_> {
    fn consume_program(&mut self, program: &[u8], depth: usize) -> Result<(), PreflightError> {
        if depth > MAX_PROGRAM_DEPTH {
            return Err(PreflightError::GeneratedContract);
        }
        let mut program_cursor = 0usize;
        while program_cursor < program.len() {
            self.charge(1)?;
            let instruction = program_byte(program, &mut program_cursor)?;
            match instruction {
                PROGRAM_TAKE => {
                    let width = program_usize(program, &mut program_cursor)?;
                    self.take(width)?;
                }
                PROGRAM_BOOL => {
                    let value = self.take(1)?[0];
                    if value > 1 {
                        return Err(PreflightError::InvalidBoolean(self.opcode));
                    }
                }
                PROGRAM_PRECISION => {
                    let single = program_usize(program, &mut program_cursor)?;
                    let double = program_usize(program, &mut program_cursor)?;
                    self.take(if self.double_precision {
                        double
                    } else {
                        single
                    })?;
                }
                PROGRAM_STRING => {
                    let null_sentinel = program_u16(program, &mut program_cursor)?;
                    let max_bytes = program_usize(program, &mut program_cursor)?;
                    let encoded_length = self.read_u16()?;
                    if encoded_length != null_sentinel {
                        let length = usize::from(encoded_length);
                        if length > max_bytes {
                            return Err(PreflightError::InvalidString(self.opcode));
                        }
                        self.take(length)?;
                    }
                }
                PROGRAM_NATIVE_POD => {
                    let pod = program_byte(program, &mut program_cursor)?;
                    self.take(native_pod_size(pod)?)?;
                }
                PROGRAM_COUNTED => {
                    let signed = program_byte(program, &mut program_cursor)?;
                    if signed > 1 {
                        return Err(PreflightError::GeneratedContract);
                    }
                    let max_count = program_usize(program, &mut program_cursor)?;
                    let body_bytes = program_usize(program, &mut program_cursor)?;
                    let body_end = program_cursor
                        .checked_add(body_bytes)
                        .ok_or(PreflightError::GeneratedContract)?;
                    let body = program
                        .get(program_cursor..body_end)
                        .ok_or(PreflightError::GeneratedContract)?;
                    program_cursor = body_end;

                    let raw_count = self.read_u32()?;
                    if (signed != 0 && (raw_count as i32) < 0) || raw_count > i32::MAX as u32 {
                        return Err(PreflightError::InvalidCount(self.opcode));
                    }
                    let count = usize::try_from(raw_count)
                        .map_err(|_| PreflightError::InvalidCount(self.opcode))?;
                    if count > max_count {
                        return Err(PreflightError::InvalidCount(self.opcode));
                    }
                    let minimum = minimum_program_width(body, self.double_precision, depth + 1)?;
                    let required = count
                        .checked_mul(minimum)
                        .ok_or(PreflightError::InvalidCount(self.opcode))?;
                    if required > self.remaining() {
                        return Err(PreflightError::PayloadTruncated(self.opcode));
                    }
                    self.charge(u64::try_from(count).map_err(|_| PreflightError::WorkLimit)?)?;
                    for _ in 0..count {
                        self.consume_program(body, depth + 1)?;
                    }
                }
                _ => return Err(PreflightError::GeneratedContract),
            }
        }
        Ok(())
    }

    fn take(&mut self, width: usize) -> Result<&[u8], PreflightError> {
        let end = self
            .cursor
            .checked_add(width)
            .ok_or(PreflightError::PayloadTruncated(self.opcode))?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(PreflightError::PayloadTruncated(self.opcode))?;
        self.cursor = end;
        Ok(value)
    }

    fn read_u16(&mut self) -> Result<u16, PreflightError> {
        let value = self.take(2)?;
        Ok(u16::from_le_bytes([value[0], value[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, PreflightError> {
        let value = self.take(4)?;
        Ok(u32::from_le_bytes(value.try_into().map_err(|_| {
            PreflightError::PayloadTruncated(self.opcode)
        })?))
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.cursor)
    }

    fn charge(&mut self, amount: u64) -> Result<(), PreflightError> {
        *self.work = self
            .work
            .checked_add(amount)
            .ok_or(PreflightError::WorkLimit)?;
        if *self.work > MAX_VALIDATION_WORK {
            return Err(PreflightError::WorkLimit);
        }
        Ok(())
    }
}

fn minimum_program_width(
    program: &[u8],
    double_precision: bool,
    depth: usize,
) -> Result<usize, PreflightError> {
    if depth > MAX_PROGRAM_DEPTH {
        return Err(PreflightError::GeneratedContract);
    }
    let mut cursor = 0usize;
    let mut total = 0usize;
    while cursor < program.len() {
        let instruction = program_byte(program, &mut cursor)?;
        let width = match instruction {
            PROGRAM_TAKE => program_usize(program, &mut cursor)?,
            PROGRAM_BOOL => 1,
            PROGRAM_PRECISION => {
                let single = program_usize(program, &mut cursor)?;
                let double = program_usize(program, &mut cursor)?;
                if double_precision { double } else { single }
            }
            PROGRAM_STRING => {
                let _ = program_u16(program, &mut cursor)?;
                let _ = program_usize(program, &mut cursor)?;
                2
            }
            PROGRAM_NATIVE_POD => native_pod_size(program_byte(program, &mut cursor)?)?,
            PROGRAM_COUNTED => {
                let signed = program_byte(program, &mut cursor)?;
                if signed > 1 {
                    return Err(PreflightError::GeneratedContract);
                }
                let _ = program_usize(program, &mut cursor)?;
                let body_bytes = program_usize(program, &mut cursor)?;
                let body_end = cursor
                    .checked_add(body_bytes)
                    .ok_or(PreflightError::GeneratedContract)?;
                let body = program
                    .get(cursor..body_end)
                    .ok_or(PreflightError::GeneratedContract)?;
                let _ = minimum_program_width(body, double_precision, depth + 1)?;
                cursor = body_end;
                4
            }
            _ => return Err(PreflightError::GeneratedContract),
        };
        total = total
            .checked_add(width)
            .ok_or(PreflightError::GeneratedContract)?;
    }
    Ok(total)
}

fn native_pod_size(pod: u8) -> Result<usize, PreflightError> {
    match pod {
        1 => Ok(size_of::<ffi::b2Circle>()),
        2 => Ok(size_of::<ffi::b2Capsule>()),
        3 => Ok(size_of::<ffi::b2Segment>()),
        4 => Ok(size_of::<ffi::b2Polygon>()),
        5 => Ok(size_of::<ffi::b2ChainSegment>()),
        _ => Err(PreflightError::GeneratedContract),
    }
}

fn program_byte(program: &[u8], cursor: &mut usize) -> Result<u8, PreflightError> {
    let byte = program
        .get(*cursor)
        .copied()
        .ok_or(PreflightError::GeneratedContract)?;
    *cursor = cursor
        .checked_add(1)
        .ok_or(PreflightError::GeneratedContract)?;
    Ok(byte)
}

fn program_u16(program: &[u8], cursor: &mut usize) -> Result<u16, PreflightError> {
    let end = cursor
        .checked_add(2)
        .ok_or(PreflightError::GeneratedContract)?;
    let value = program
        .get(*cursor..end)
        .ok_or(PreflightError::GeneratedContract)?;
    *cursor = end;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn program_usize(program: &[u8], cursor: &mut usize) -> Result<usize, PreflightError> {
    let end = cursor
        .checked_add(4)
        .ok_or(PreflightError::GeneratedContract)?;
    let value = program
        .get(*cursor..end)
        .ok_or(PreflightError::GeneratedContract)?;
    *cursor = end;
    let value = u32::from_le_bytes(
        value
            .try_into()
            .map_err(|_| PreflightError::GeneratedContract)?,
    );
    usize::try_from(value).map_err(|_| PreflightError::GeneratedContract)
}

fn parse_header(bytes: &[u8], identity: &AdapterIdentity) -> Result<PreflightInfo, PreflightError> {
    let header = bytes
        .get(..RECORDING_HEADER_BYTES)
        .ok_or(PreflightError::Truncated)?;
    if read_u32(header, 0)? != RECORDING_MAGIC
        || read_u16(header, 4)? != RECORDING_VERSION_MAJOR
        || read_u16(header, 6)? != RECORDING_VERSION_MINOR
    {
        return Err(PreflightError::HeaderMismatch);
    }
    if read_u32(header, 8)? != 0 || header[16] != 0 || read_u32(header, 20)? != 0 {
        return Err(PreflightError::ReservedField);
    }
    let length_units_per_meter = f32::from_bits(read_u32(header, 12)?);
    if !length_units_per_meter.is_finite() || length_units_per_meter <= 0.0 {
        return Err(PreflightError::InvalidLengthScale);
    }
    let validation = header[19];
    if header[17] != identity.pointer_width
        || header[18] != 0
        || validation > 1
        || (validation != 0) != (identity.validation_enabled != 0)
    {
        return Err(PreflightError::AdapterMismatch);
    }
    let snapshot_bytes =
        usize::try_from(read_u64(header, 24)?).map_err(|_| PreflightError::SnapshotLength)?;
    if snapshot_bytes < 16 {
        return Err(PreflightError::SnapshotLength);
    }
    Ok(PreflightInfo {
        length_units_per_meter,
        snapshot_offset: RECORDING_HEADER_BYTES,
        snapshot_bytes,
        records: 0,
        steps: 0,
        queries: 0,
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, PreflightError> {
    let value = bytes
        .get(offset..offset.checked_add(2).ok_or(PreflightError::Truncated)?)
        .ok_or(PreflightError::Truncated)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, PreflightError> {
    let value = bytes
        .get(offset..offset.checked_add(4).ok_or(PreflightError::Truncated)?)
        .ok_or(PreflightError::Truncated)?;
    Ok(u32::from_le_bytes(
        value.try_into().map_err(|_| PreflightError::Truncated)?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, PreflightError> {
    let value = bytes
        .get(offset..offset.checked_add(8).ok_or(PreflightError::Truncated)?)
        .ok_or(PreflightError::Truncated)?;
    Ok(u64::from_le_bytes(
        value.try_into().map_err(|_| PreflightError::Truncated)?,
    ))
}

#[cfg(test)]
mod tests {
    use core::mem::{offset_of, size_of};
    use std::ops::Range;

    use super::*;
    use crate::{
        Aabb, BodyBuilder, BodyType, DistanceJointDef, JointBase, Position, QueryFilter,
        RecordingCapacity, ShapeDef, Transform, Vec2, World, WorldDef, shapes,
    };

    #[derive(Debug)]
    struct LifecycleFixture {
        bytes: Vec<u8>,
        shape: [u8; 8],
        chain: [u8; 8],
        segment: [u8; 8],
        joint: [u8; 8],
    }

    fn packed_object_id(index1: i32, world0: u16, generation: u16) -> [u8; 8] {
        ((u64::from(index1 as u32) << 32) | (u64::from(world0) << 16) | u64::from(generation))
            .to_le_bytes()
    }

    fn lifecycle_recording() -> LifecycleFixture {
        let mut world =
            World::new(WorldDef::builder().gravity(Vec2::ZERO).build()).expect("test world");
        let body = world.create_body_id(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let other = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([4.0_f32, 0.0])
                .build(),
        );
        let shape = world.create_circle_shape_for(
            body,
            &ShapeDef::builder().density(1.0).build(),
            &shapes::circle(Vec2::ZERO, 0.5),
        );
        let chain = world.create_chain_for_id(
            body,
            &shapes::chain::ChainDef::builder()
                .points([
                    [-3.0_f32, 0.0],
                    [-2.0_f32, 0.0],
                    [-1.0_f32, 0.0],
                    [0.0_f32, 0.0],
                    [1.0_f32, 0.0],
                    [2.0_f32, 0.0],
                ])
                .build(),
        );
        let segment = {
            let chain = world.chain(chain).expect("live test chain");
            *chain.segments().last().expect("test chain segment")
        };
        let joint = world.create_distance_joint_id(
            &DistanceJointDef::new(JointBase::new(body, other)).length(4.0),
        );

        let shape_raw = shape.unbind();
        let chain_raw = chain.unbind();
        let segment_raw = segment.unbind();
        let joint_raw = joint.unbind();

        let mut session = world
            .try_start_recording(RecordingCapacity::DEFAULT)
            .expect("recording session");
        session.shape_set_density(shape, 2.0, true);
        let _ = session.shape_test_point(segment, Position::ZERO);
        session.destroy_body(body);
        let replacement_body =
            session.create_body(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let replacement_shape = session.create_circle_shape(
            replacement_body,
            &ShapeDef::builder().density(1.0).build(),
            &shapes::circle(Vec2::ZERO, 0.25),
        );
        session
            .try_step(1.0 / 60.0, 2)
            .expect("recorded world step");
        assert_eq!(
            session.overlap_aabb(
                Position::ZERO,
                Aabb::new([-1.0_f32, -1.0], [1.0_f32, 1.0]),
                QueryFilter::default(),
            ),
            vec![replacement_shape]
        );
        let bytes = session
            .try_finish()
            .expect("finished recording")
            .into_bytes();

        LifecycleFixture {
            bytes,
            shape: packed_object_id(shape_raw.index1, shape_raw.world0, shape_raw.generation),
            chain: packed_object_id(chain_raw.index1, chain_raw.world0, chain_raw.generation),
            segment: packed_object_id(
                segment_raw.index1,
                segment_raw.world0,
                segment_raw.generation,
            ),
            joint: packed_object_id(joint_raw.index1, joint_raw.world0, joint_raw.generation),
        }
    }

    fn allocator_recording() -> Vec<u8> {
        let mut world = World::new(WorldDef::default()).expect("test world");
        let mut session = world
            .try_start_recording(RecordingCapacity::DEFAULT)
            .expect("recording session");
        let first = session.create_body(BodyBuilder::new().build());
        let second = session.create_body(BodyBuilder::new().build());
        session.destroy_body(first);
        session.destroy_body(second);
        let _replacement = session.create_body(BodyBuilder::new().build());
        session
            .try_step(1.0 / 60.0, 1)
            .expect("recorded world step");
        session
            .try_finish()
            .expect("finished recording")
            .into_bytes()
    }

    fn valid_recording() -> Vec<u8> {
        let mut world = World::new(WorldDef::default()).expect("test world");
        let mut session = world
            .try_start_recording(RecordingCapacity::DEFAULT)
            .expect("recording session");
        session
            .try_step(1.0 / 60.0, 4)
            .expect("recorded world step");
        session
            .try_finish()
            .expect("finished recording")
            .into_bytes()
    }

    fn polygon_recording(polygon: crate::Polygon) -> Vec<u8> {
        let mut world = World::new(WorldDef::default()).expect("test world");
        let mut session = world
            .try_start_recording(RecordingCapacity::DEFAULT)
            .expect("recording session");
        let body = session.create_body(BodyBuilder::new().body_type(BodyType::Dynamic).build());
        let _shape =
            session.create_polygon_shape(body, &ShapeDef::builder().density(1.0).build(), &polygon);
        session
            .try_step(1.0 / 60.0, 1)
            .expect("recorded world step");
        session
            .try_finish()
            .expect("finished recording")
            .into_bytes()
    }

    fn stream_offset(bytes: &[u8]) -> usize {
        RECORDING_HEADER_BYTES
            + usize::try_from(read_u64(bytes, 24).expect("snapshot length"))
                .expect("host snapshot length")
    }

    fn frames(bytes: &[u8]) -> Vec<(usize, usize, u8)> {
        let mut cursor = stream_offset(bytes);
        let mut frames = Vec::new();
        while cursor < bytes.len() {
            let payload = usize::from(bytes[cursor + 1])
                | (usize::from(bytes[cursor + 2]) << 8)
                | (usize::from(bytes[cursor + 3]) << 16);
            let end = cursor + FRAME_HEADER_BYTES + payload;
            frames.push((cursor, end, bytes[cursor]));
            cursor = end;
        }
        frames
    }

    fn frame(bytes: &[u8], opcode: u8) -> (usize, usize, u8) {
        let matching: Vec<_> = frames(bytes)
            .into_iter()
            .filter(|(_, _, candidate)| *candidate == opcode)
            .collect();
        assert_eq!(matching.len(), 1, "expected one {opcode:#04X} frame");
        matching[0]
    }

    fn frame_argument_range(bytes: &[u8], opcode: u8, tag: ArgumentTag) -> Range<usize> {
        let record = frame(bytes, opcode);
        let payload_start = record.0 + FRAME_HEADER_BYTES;
        let payload = &bytes[payload_start..record.1];
        let operation = operation(opcode).expect("known test opcode");
        let mut work = 0;
        let mut interpreter = PayloadInterpreter {
            bytes: payload,
            cursor: 0,
            double_precision: cfg!(feature = "double-precision"),
            opcode,
            work: &mut work,
        };
        let mut program_start = 0;
        for argument in operation.arguments {
            let program_end = usize::from(argument.program_end);
            let argument_start = interpreter.cursor;
            interpreter
                .consume_program(&operation.program[program_start..program_end], 0)
                .expect("valid native recording argument");
            if argument.tag == tag {
                return payload_start + argument_start..payload_start + interpreter.cursor;
            }
            program_start = program_end;
        }
        panic!("{tag:?} argument missing from opcode {opcode:#04X}");
    }

    fn polygon_f32_offset(field_offset: usize, index: usize) -> usize {
        field_offset + index * size_of::<ffi::b2Vec2>()
    }

    fn write_f32(bytes: &mut [u8], offset: usize, value: f32) {
        bytes[offset..offset + size_of::<f32>()].copy_from_slice(&value.to_le_bytes());
    }

    fn swap_vec2(bytes: &mut [u8], left: usize, right: usize) {
        for component in 0..2 {
            let component_offset = component * size_of::<f32>();
            for byte in 0..size_of::<f32>() {
                bytes.swap(
                    left + component_offset + byte,
                    right + component_offset + byte,
                );
            }
        }
    }

    fn insert_frame_at(bytes: &mut Vec<u8>, insertion: usize, opcode: u8, payload: &[u8]) {
        let size = payload.len();
        assert!(size < 1 << 24);
        let mut record = Vec::with_capacity(FRAME_HEADER_BYTES + size);
        record.extend_from_slice(&[opcode, size as u8, (size >> 8) as u8, (size >> 16) as u8]);
        record.extend_from_slice(payload);
        bytes.splice(insertion..insertion, record);
    }

    fn assert_rejected(bytes: &[u8], expected: PreflightError) {
        let actual = preflight_recording(bytes).expect_err("malicious recording must fail");
        assert_eq!(actual, expected);
    }

    fn insert_before_final_metadata(bytes: &mut Vec<u8>, opcode: u8, payload: &[u8]) {
        let records = frames(bytes);
        let insertion = records[records.len() - 2].0;
        let size = payload.len();
        assert!(size < 1 << 24);
        let mut frame = Vec::with_capacity(FRAME_HEADER_BYTES + size);
        frame.extend_from_slice(&[opcode, size as u8, (size >> 8) as u8, (size >> 16) as u8]);
        frame.extend_from_slice(payload);
        bytes.splice(insertion..insertion, frame);
    }

    fn double_precision_explosion_argument(
        position: (f64, f64),
        radius: f32,
        falloff: f32,
        impulse: f32,
    ) -> Vec<u8> {
        let mut payload = Vec::with_capacity(36);
        payload.extend_from_slice(&u64::MAX.to_le_bytes());
        payload.extend_from_slice(&position.0.to_le_bytes());
        payload.extend_from_slice(&position.1.to_le_bytes());
        payload.extend_from_slice(&radius.to_le_bytes());
        payload.extend_from_slice(&falloff.to_le_bytes());
        payload.extend_from_slice(&impulse.to_le_bytes());
        payload
    }

    #[test]
    fn adapter_identity_and_snapshot_status_errors_map_precisely() {
        assert_eq!(
            map_adapter_identity_error(adapter::AdapterIdentityError::Unavailable),
            PreflightError::AdapterUnavailable
        );
        assert_eq!(
            map_adapter_identity_error(adapter::AdapterIdentityError::Mismatch(
                adapter::AdapterIdentityField::Precision,
            )),
            PreflightError::AdapterMismatch
        );
        assert_eq!(
            map_snapshot_validation_error(adapter::SnapshotValidationError::AdapterIdentity(
                adapter::AdapterIdentityError::Unavailable,
            )),
            PreflightError::AdapterUnavailable
        );
        assert_eq!(
            map_snapshot_validation_error(adapter::SnapshotValidationError::AdapterIdentity(
                adapter::AdapterIdentityError::Mismatch(
                    adapter::AdapterIdentityField::SnapshotLayout,
                ),
            )),
            PreflightError::AdapterMismatch
        );
        assert_eq!(
            map_snapshot_validation_error(adapter::SnapshotValidationError::Status(
                adapter::SNAPSHOT_BAD_HEADER,
            )),
            PreflightError::SnapshotRejected(adapter::SNAPSHOT_BAD_HEADER)
        );
    }

    #[test]
    fn actual_native_recording_passes_complete_preflight_and_is_owned() {
        let mut source = valid_recording();
        let expected = source.clone();
        let validated = preflight_recording(&source).expect("complete preflight");
        assert_eq!(validated.bytes(), expected);
        assert!(validated.info().records >= 4);
        assert_eq!(validated.info().steps, 1);
        assert_eq!(validated.info().queries, 0);
        source.fill(0);
        assert_eq!(validated.bytes(), expected);
    }

    #[test]
    fn malicious_frame_corpus_is_rejected_before_native_dispatch() {
        let valid = valid_recording();
        preflight_recording(&valid).expect("baseline must be valid");

        let mut unknown = valid.clone();
        let first_opcode = stream_offset(&unknown);
        unknown[first_opcode] = 0xFF;
        assert!(matches!(
            preflight_recording(&unknown),
            Err(PreflightError::UnknownOpcode(0xFF))
        ));

        let mut truncated_payload = valid.clone();
        let first = stream_offset(&truncated_payload);
        truncated_payload[first + 1..first + 4].copy_from_slice(&[0xFF, 0xFF, 0x7F]);
        assert!(matches!(
            preflight_recording(&truncated_payload),
            Err(PreflightError::PayloadTruncated(_))
        ));

        let mut extra_payload_byte = valid.clone();
        insert_before_final_metadata(&mut extra_payload_byte, 0x0C, &[0; 5]);
        assert!(matches!(
            preflight_recording(&extra_payload_byte),
            Err(PreflightError::PayloadMismatch { opcode: 0x0C, .. })
        ));

        let mut non_canonical_bool = valid.clone();
        insert_before_final_metadata(&mut non_canonical_bool, 0x02, &[0, 0, 0, 0, 2]);
        assert!(matches!(
            preflight_recording(&non_canonical_bool),
            Err(PreflightError::InvalidBoolean(0x02))
        ));

        let position_bytes = if cfg!(feature = "double-precision") {
            16
        } else {
            8
        };
        let count_offset = 4 + position_bytes;
        let mut excessive_count_payload = vec![0; count_offset + 4];
        excessive_count_payload[count_offset..].copy_from_slice(&i32::MAX.to_le_bytes());
        let mut excessive_count = valid.clone();
        insert_before_final_metadata(&mut excessive_count, 0xE1, &excessive_count_payload);
        assert!(matches!(
            preflight_recording(&excessive_count),
            Err(PreflightError::InvalidCount(0xE1))
        ));

        let mut missing_count_elements_payload = vec![0; count_offset + 4];
        missing_count_elements_payload[count_offset..].copy_from_slice(&1_u32.to_le_bytes());
        let mut missing_count_elements = valid.clone();
        insert_before_final_metadata(
            &mut missing_count_elements,
            0xE1,
            &missing_count_elements_payload,
        );
        assert!(matches!(
            preflight_recording(&missing_count_elements),
            Err(PreflightError::PayloadTruncated(0xE1))
        ));

        let mut bytes_after_terminal = valid.clone();
        bytes_after_terminal.extend_from_slice(&[0x0C, 4, 0, 0, 0, 0, 0, 0]);
        assert!(matches!(
            preflight_recording(&bytes_after_terminal),
            Err(PreflightError::StreamGrammar)
        ));

        let mut missing_metadata = valid;
        let records = frames(&missing_metadata);
        let metadata = records[records.len() - 2];
        missing_metadata.drain(metadata.0..metadata.1);
        assert!(matches!(
            preflight_recording(&missing_metadata),
            Err(PreflightError::StreamGrammar)
        ));
    }

    #[test]
    fn explosion_preflight_rejects_double_positions_outside_native_query_bounds() {
        const OPCODE: u8 = 0x90;
        let valid = double_precision_explosion_argument((10_000_000_000.25, 0.0), 1.0, 0.5, 2.0);
        semantic::validate_argument(ArgumentTag::ExplosionDef, &valid, true, 1.0, OPCODE)
            .expect("representable double-precision explosion");

        for invalid in [
            double_precision_explosion_argument((f64::from(f32::MAX) * 2.0, 0.0), 1.0, 0.5, 2.0),
            double_precision_explosion_argument((f64::from(f32::MAX), 0.0), f32::MAX, 0.0, 2.0),
        ] {
            assert_eq!(
                semantic::validate_argument(ArgumentTag::ExplosionDef, &invalid, true, 1.0, OPCODE,),
                Err(PreflightError::InvalidValue(OPCODE))
            );
        }
    }

    #[test]
    fn malformed_lifecycle_corpus_is_rejected_before_native_dispatch() {
        const DESTROY_BODY: u8 = 0x11;
        const SHAPE_SET_DENSITY: u8 = 0x50;
        const DESTROY_CHAIN: u8 = 0x71;
        const DISTANCE_ENABLE_SPRING: u8 = 0xA1;
        const JOINT_SET_CONSTRAINT_TUNING: u8 = 0x9C;
        const PRISMATIC_ENABLE_SPRING: u8 = 0xB4;
        const OVERLAP_AABB: u8 = 0xE0;
        const SHAPE_TEST_POINT: u8 = 0xE7;

        let fixture = lifecycle_recording();
        preflight_recording(&fixture.bytes).expect("lifecycle baseline must be valid");

        let mut wrong_world = fixture.bytes.clone();
        let destroy = frame(&wrong_world, DESTROY_BODY);
        let world_offset = destroy.0 + FRAME_HEADER_BYTES + 2;
        let world0 = u16::from_le_bytes(
            wrong_world[world_offset..world_offset + 2]
                .try_into()
                .unwrap(),
        );
        wrong_world[world_offset..world_offset + 2]
            .copy_from_slice(&world0.wrapping_add(1).to_le_bytes());
        assert_rejected(&wrong_world, PreflightError::InvalidReference(DESTROY_BODY));

        let mut wrong_generation = fixture.bytes.clone();
        let destroy = frame(&wrong_generation, DESTROY_BODY);
        let generation_offset = destroy.0 + FRAME_HEADER_BYTES;
        let generation = u16::from_le_bytes(
            wrong_generation[generation_offset..generation_offset + 2]
                .try_into()
                .unwrap(),
        );
        wrong_generation[generation_offset..generation_offset + 2]
            .copy_from_slice(&generation.wrapping_add(1).to_le_bytes());
        assert_rejected(
            &wrong_generation,
            PreflightError::InvalidReference(DESTROY_BODY),
        );

        let mut wrong_kind = fixture.bytes.clone();
        let destroy = frame(&wrong_kind, DESTROY_BODY);
        wrong_kind[destroy.0 + FRAME_HEADER_BYTES..destroy.0 + FRAME_HEADER_BYTES + 8]
            .copy_from_slice(&fixture.segment);
        assert_rejected(&wrong_kind, PreflightError::InvalidReference(DESTROY_BODY));

        let mut stale_body = fixture.bytes.clone();
        let destroy = frame(&stale_body, DESTROY_BODY);
        let duplicate = stale_body[destroy.0..destroy.1].to_vec();
        stale_body.splice(destroy.1..destroy.1, duplicate);
        assert_rejected(&stale_body, PreflightError::InvalidReference(DESTROY_BODY));

        let mut stale_shape = fixture.bytes.clone();
        let destroy = frame(&stale_shape, DESTROY_BODY);
        let density = frame(&stale_shape, SHAPE_SET_DENSITY);
        let duplicate = stale_shape[density.0..density.1].to_vec();
        stale_shape.splice(destroy.1..destroy.1, duplicate);
        assert_rejected(
            &stale_shape,
            PreflightError::InvalidReference(SHAPE_SET_DENSITY),
        );

        let mut stale_chain = fixture.bytes.clone();
        let destroy = frame(&stale_chain, DESTROY_BODY);
        insert_frame_at(&mut stale_chain, destroy.1, DESTROY_CHAIN, &fixture.chain);
        assert_rejected(
            &stale_chain,
            PreflightError::InvalidReference(DESTROY_CHAIN),
        );

        let mut stale_joint = fixture.bytes.clone();
        let destroy = frame(&stale_joint, DESTROY_BODY);
        let mut payload = fixture.joint.to_vec();
        payload.push(1);
        insert_frame_at(
            &mut stale_joint,
            destroy.1,
            DISTANCE_ENABLE_SPRING,
            &payload,
        );
        assert_rejected(
            &stale_joint,
            PreflightError::InvalidReference(DISTANCE_ENABLE_SPRING),
        );

        let mut wrong_joint_subtype = fixture.bytes.clone();
        let destroy = frame(&wrong_joint_subtype, DESTROY_BODY);
        let mut payload = fixture.joint.to_vec();
        payload.push(1);
        insert_frame_at(
            &mut wrong_joint_subtype,
            destroy.0,
            PRISMATIC_ENABLE_SPRING,
            &payload,
        );
        assert_rejected(
            &wrong_joint_subtype,
            PreflightError::InvalidLifecycle(PRISMATIC_ENABLE_SPRING),
        );

        let mut invalid_joint_tuning = fixture.bytes.clone();
        let destroy = frame(&invalid_joint_tuning, DESTROY_BODY);
        let mut payload = fixture.joint.to_vec();
        payload.extend_from_slice(&1.0_f32.to_le_bytes());
        payload.extend_from_slice(&(-1.0_f32).to_le_bytes());
        insert_frame_at(
            &mut invalid_joint_tuning,
            destroy.0,
            JOINT_SET_CONSTRAINT_TUNING,
            &payload,
        );
        assert_rejected(
            &invalid_joint_tuning,
            PreflightError::InvalidRange(JOINT_SET_CONSTRAINT_TUNING),
        );

        let mut stale_chain_segment = fixture.bytes.clone();
        let shape_query = frame(&stale_chain_segment, SHAPE_TEST_POINT);
        insert_frame_at(
            &mut stale_chain_segment,
            shape_query.0,
            DESTROY_CHAIN,
            &fixture.chain,
        );
        assert_rejected(
            &stale_chain_segment,
            PreflightError::InvalidReference(SHAPE_TEST_POINT),
        );

        let mut bad_query_tail = fixture.bytes.clone();
        let overlap = frame(&bad_query_tail, OVERLAP_AABB);
        let position_bytes = if cfg!(feature = "double-precision") {
            16
        } else {
            8
        };
        let tail_offset = overlap.0 + FRAME_HEADER_BYTES + 4 + position_bytes + 16 + 16;
        assert_eq!(
            u32::from_le_bytes(
                bad_query_tail[tail_offset..tail_offset + 4]
                    .try_into()
                    .unwrap()
            ),
            1
        );
        bad_query_tail[tail_offset + 4..tail_offset + 12].copy_from_slice(&fixture.shape);
        assert_rejected(
            &bad_query_tail,
            PreflightError::InvalidReference(OVERLAP_AABB),
        );
    }

    #[test]
    fn returned_ids_must_follow_the_exact_native_free_list_order() {
        const CREATE_BODY: u8 = 0x10;

        let valid = allocator_recording();
        preflight_recording(&valid).expect("allocator baseline must be valid");
        let creates: Vec<_> = frames(&valid)
            .into_iter()
            .filter(|(_, _, opcode)| *opcode == CREATE_BODY)
            .collect();
        assert_eq!(creates.len(), 3);

        let first_return = valid[creates[0].1 - 8..creates[0].1].to_vec();
        let mut forged = valid;
        let mut wrong_lifo_id: [u8; 8] = first_return.try_into().unwrap();
        let generation = u16::from_le_bytes(wrong_lifo_id[..2].try_into().unwrap());
        wrong_lifo_id[..2].copy_from_slice(&generation.wrapping_add(1).to_le_bytes());
        let final_return = creates[2].1 - 8;
        forged[final_return..final_return + 8].copy_from_slice(&wrong_lifo_id);
        assert_rejected(&forged, PreflightError::InvalidLifecycle(CREATE_BODY));
    }

    #[test]
    fn malformed_lifecycle_never_reaches_native_player_creation() {
        const DESTROY_BODY: u8 = 0x11;

        let fixture = lifecycle_recording();
        let mut malformed = fixture.bytes;
        let destroy = frame(&malformed, DESTROY_BODY);
        let generation_offset = destroy.0 + FRAME_HEADER_BYTES;
        let generation = u16::from_le_bytes(
            malformed[generation_offset..generation_offset + 2]
                .try_into()
                .unwrap(),
        );
        malformed[generation_offset..generation_offset + 2]
            .copy_from_slice(&generation.wrapping_add(1).to_le_bytes());

        super::super::REPLAY_CREATE_CALLS.with(|calls| calls.set(0));
        let error = crate::ReplayPlayer::open_bytes(
            &malformed,
            crate::MixerRequirements::default(),
            crate::ReplayConfig::default(),
        )
        .expect_err("malformed recording must fail before native creation");
        assert!(matches!(error, crate::ReplayError::Malformed(_)));
        assert_eq!(
            super::super::REPLAY_CREATE_CALLS.with(|calls| calls.get()),
            0
        );
    }

    #[test]
    fn malformed_polygon_semantics_never_reach_native_player_creation() {
        const CREATE_POLYGON_SHAPE: u8 = 0x43;

        let valid = polygon_recording(shapes::box_polygon(1.0, 1.0));
        let polygon = frame_argument_range(&valid, CREATE_POLYGON_SHAPE, ArgumentTag::Polygon);
        assert_eq!(polygon.len(), size_of::<ffi::b2Polygon>());
        preflight_recording(&valid).expect("canonical polygon recording must pass preflight");

        super::super::REPLAY_CREATE_CALLS.with(|calls| calls.set(0));
        let player = crate::ReplayPlayer::open_bytes(
            &valid,
            crate::MixerRequirements::default(),
            crate::ReplayConfig::default(),
        )
        .expect("canonical polygon recording must reach native player creation");
        assert_eq!(
            super::super::REPLAY_CREATE_CALLS.with(|calls| calls.get()),
            1
        );
        drop(player);

        let vertices = polygon.start + offset_of!(ffi::b2Polygon, vertices);
        let normals = polygon.start + offset_of!(ffi::b2Polygon, normals);
        let centroid = polygon.start + offset_of!(ffi::b2Polygon, centroid);
        let radius = polygon.start + offset_of!(ffi::b2Polygon, radius);

        let mut clockwise = valid.clone();
        swap_vec2(
            &mut clockwise,
            polygon_f32_offset(vertices, 0),
            polygon_f32_offset(vertices, 3),
        );
        swap_vec2(
            &mut clockwise,
            polygon_f32_offset(vertices, 1),
            polygon_f32_offset(vertices, 2),
        );

        let mut concave = valid.clone();
        write_f32(&mut concave, polygon_f32_offset(vertices, 2), 0.0);
        write_f32(
            &mut concave,
            polygon_f32_offset(vertices, 2) + size_of::<f32>(),
            -0.5,
        );

        let mut wrong_normal = valid.clone();
        write_f32(&mut wrong_normal, polygon_f32_offset(normals, 0), 1.0);
        write_f32(
            &mut wrong_normal,
            polygon_f32_offset(normals, 0) + size_of::<f32>(),
            0.0,
        );

        let mut wrong_centroid = valid.clone();
        write_f32(&mut wrong_centroid, centroid, 0.25);

        let mut negative_radius = valid;
        write_f32(&mut negative_radius, radius, -0.25);

        for malformed in [
            clockwise,
            concave,
            wrong_normal,
            wrong_centroid,
            negative_radius,
        ] {
            super::super::REPLAY_CREATE_CALLS.with(|calls| calls.set(0));
            let error = crate::ReplayPlayer::open_bytes(
                &malformed,
                crate::MixerRequirements::default(),
                crate::ReplayConfig::default(),
            )
            .expect_err("malformed polygon must fail before native creation");
            assert!(matches!(error, crate::ReplayError::Malformed(_)));
            assert_eq!(
                super::super::REPLAY_CREATE_CALLS.with(|calls| calls.get()),
                0
            );
        }
    }

    #[test]
    fn large_offset_native_polygon_reaches_native_player_creation() {
        let valid = polygon_recording(shapes::offset_box_polygon(
            1.5,
            0.625,
            Transform::from_pos_angle([1_000.25_f32, -750.5], 0.37),
        ));
        preflight_recording(&valid).expect("large-offset native polygon must pass preflight");

        super::super::REPLAY_CREATE_CALLS.with(|calls| calls.set(0));
        let player = crate::ReplayPlayer::open_bytes(
            &valid,
            crate::MixerRequirements::default(),
            crate::ReplayConfig::default(),
        )
        .expect("large-offset native polygon must reach native player creation");
        assert_eq!(
            super::super::REPLAY_CREATE_CALLS.with(|calls| calls.get()),
            1
        );
        drop(player);
    }

    #[test]
    fn every_prefix_of_the_fixed_header_fails_as_truncated() {
        let valid = valid_recording();
        for length in 0..RECORDING_HEADER_BYTES {
            assert!(matches!(
                preflight_recording(&valid[..length]),
                Err(PreflightError::Truncated)
            ));
        }
    }
}
