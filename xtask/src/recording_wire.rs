use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

use serde::{Deserialize, Serialize};

use crate::{
    Error, Result,
    config::RECORDING_WIRE_SCHEMA,
    recording_ops::{
        ARGUMENT_TAGS, RECORDING_OPS_PATH, RETURN_TAGS, RecordingArgument, RecordingOp,
        validate_operations,
    },
};

const MAX_SEQUENCE_ELEMENTS: usize = 1_000_000;
const QUERY_OPERATIONS: &[&str] = &[
    "QueryOverlapAABB",
    "QueryOverlapShape",
    "QueryCastRay",
    "QueryCastShape",
    "QueryCollideMover",
    "QueryCastRayClosest",
    "QueryCastMover",
    "ShapeTestPoint",
    "ShapeRayCast",
];
const REVIEWED_SOURCE_AGGREGATE_DOMAIN: &[u8] = b"boxdd.recording-wire.reviewed-sources\0";

/// Exact, canonical source set whose Git blob identities authorize the wire schema and replay.
///
/// Producer callsites are checked separately across the complete effective C-source inventory;
/// keeping that check separate avoids duplicating every producer file in this schema identity.
pub const REVIEWED_RECORDING_INPUT_PATHS: &[&str] = &[
    "src/recording.c",
    "src/recording.h",
    RECORDING_OPS_PATH,
    "src/recording_replay.c",
    "src/recording_replay.h",
    "src/world_snapshot.c",
    "src/world_snapshot.h",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingFraming {
    pub header_bytes: usize,
    pub opcode_bytes: usize,
    pub payload_size_bytes: usize,
    pub max_payload_exclusive: usize,
    pub byte_order: String,
    pub stream_termination: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingHeaderIdentity {
    pub magic_offset: usize,
    pub magic: u32,
    pub version_major_offset: usize,
    pub version_major: u16,
    pub version_minor_offset: usize,
    pub version_minor: u16,
    pub reserved2_offset: usize,
    pub length_scale_offset: usize,
    pub reserved3_offset: usize,
    pub pointer_width_offset: usize,
    pub big_endian_offset: usize,
    pub validation_enabled_offset: usize,
    pub reserved1_offset: usize,
    pub snapshot_size_offset: usize,
    pub snapshot_size_bytes: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotIdentity {
    pub minimum_bytes: usize,
    pub header_bytes: usize,
    pub magic_offset: usize,
    pub magic: u32,
    pub version_offset: usize,
    pub version: u32,
    pub layout_hash_offset: usize,
    pub flags_offset: usize,
    pub known_flags_mask: u32,
    pub validation_flag: u32,
    pub double_precision_flag: u32,
    pub length_source: String,
    pub slice_termination: String,
    pub layout_metadata_source: String,
    pub validation_requirement: String,
    pub native_dispatch_before_validation: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingStreamGrammar {
    pub initial_operation: String,
    pub final_metadata_operation: String,
    pub terminal_operation: String,
    pub terminal_must_be_last: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CountCodec {
    pub bytes: usize,
    pub signed: bool,
    pub max_count: usize,
    pub remaining_bytes_bound: bool,
    pub checked_multiply: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Codec {
    Fixed {
        bytes: usize,
        boolean: bool,
    },
    Precision {
        single_bytes: usize,
        double_bytes: usize,
    },
    String {
        length_bytes: usize,
        null_sentinel: u64,
        max_bytes: usize,
    },
    NativePod {
        abi_type: String,
        size_source: String,
    },
    Tag {
        tag: String,
    },
    Sequence {
        steps: Vec<Codec>,
    },
    Counted {
        count: CountCodec,
        element: Box<Codec>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TagCodec {
    pub tag: String,
    pub codec: Codec,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TailCodec {
    pub name: String,
    pub codec: Codec,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReplaySemanticClass {
    Mutation,
    Query,
    Step,
    StateHash,
    Metadata,
    Terminal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireOpcode {
    pub opcode: u8,
    pub name: String,
    pub return_tag: String,
    pub arguments: Vec<RecordingArgument>,
    pub tail_program: String,
    pub semantic_validator: ReplaySemanticClass,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedRecordingSource {
    pub path: String,
    pub git_blob: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingWireContract {
    pub schema_version: u32,
    pub upstream_sha: String,
    pub reviewed_sources: Vec<ReviewedRecordingSource>,
    pub reviewed_sources_aggregate_blake3: String,
    pub framing: RecordingFraming,
    pub header: RecordingHeaderIdentity,
    pub snapshot: SnapshotIdentity,
    pub stream_grammar: RecordingStreamGrammar,
    pub argument_tags: Vec<String>,
    pub return_tags: Vec<String>,
    pub tag_codecs: Vec<TagCodec>,
    pub tail_codecs: Vec<TailCodec>,
    pub opcodes: Vec<WireOpcode>,
}

pub fn generate_wire_contract(
    upstream_sha: &str,
    operations: &[RecordingOp],
    reviewed_source_git_blobs: &BTreeMap<String, String>,
    reviewed_sources_aggregate_blake3: &str,
) -> Result<RecordingWireContract> {
    let reviewed_sources = reviewed_sources_from_map(reviewed_source_git_blobs)?;
    validate_aggregate(
        &reviewed_sources,
        reviewed_sources_aggregate_blake3,
        "provided reviewed-source",
    )?;

    Ok(RecordingWireContract {
        schema_version: RECORDING_WIRE_SCHEMA,
        upstream_sha: upstream_sha.to_owned(),
        reviewed_sources,
        reviewed_sources_aggregate_blake3: reviewed_sources_aggregate_blake3.to_owned(),
        framing: expected_framing(),
        header: expected_header(),
        snapshot: expected_snapshot(),
        stream_grammar: expected_stream_grammar(),
        argument_tags: ARGUMENT_TAGS.iter().map(|tag| (*tag).to_owned()).collect(),
        return_tags: RETURN_TAGS.iter().map(|tag| (*tag).to_owned()).collect(),
        tag_codecs: expected_tag_codecs(),
        tail_codecs: expected_tail_codecs(),
        opcodes: operations
            .iter()
            .map(wire_opcode)
            .collect::<Result<Vec<_>>>()?,
    })
}

pub fn validate_wire_contract(
    contract: &RecordingWireContract,
    operations: &[RecordingOp],
    expected_upstream: &str,
    expected_source_git_blobs: &BTreeMap<String, String>,
    expected_sources_aggregate_blake3: &str,
) -> Result<()> {
    validate_operations(operations)?;
    let expected = generate_wire_contract(
        expected_upstream,
        operations,
        expected_source_git_blobs,
        expected_sources_aggregate_blake3,
    )?;
    let mut errors = Vec::new();

    if contract.schema_version != RECORDING_WIRE_SCHEMA {
        errors.push(format!(
            "recording wire schema {} does not match supported schema {RECORDING_WIRE_SCHEMA}",
            contract.schema_version
        ));
    }
    if contract.upstream_sha != expected_upstream {
        errors.push(format!(
            "recording wire upstream {} does not match {expected_upstream}",
            contract.upstream_sha
        ));
    }
    if let Err(error) = validate_reviewed_sources(
        &contract.reviewed_sources,
        &contract.reviewed_sources_aggregate_blake3,
    ) {
        errors.push(error.to_string());
    }
    if contract.reviewed_sources != expected.reviewed_sources {
        errors.push(
            "recording wire reviewed-source Git blob identities differ from the reviewed manifest"
                .to_owned(),
        );
    }
    if contract.reviewed_sources_aggregate_blake3 != expected.reviewed_sources_aggregate_blake3 {
        errors.push(
            "recording wire reviewed-source aggregate differs from the reviewed manifest"
                .to_owned(),
        );
    }
    if contract.framing != expected.framing
        || contract.header != expected.header
        || contract.snapshot != expected.snapshot
        || contract.stream_grammar != expected.stream_grammar
    {
        errors.push("recording header, snapshot identity, or u8/u24 framing drifted".to_owned());
    }
    if contract.argument_tags != expected.argument_tags
        || contract.return_tags != expected.return_tags
        || contract.tag_codecs != expected.tag_codecs
        || contract.tail_codecs != expected.tail_codecs
    {
        errors.push("recording precision-aware codec registry drifted".to_owned());
    }

    let mut opcodes = BTreeSet::new();
    let mut names = BTreeSet::new();
    for opcode in &contract.opcodes {
        if !opcodes.insert(opcode.opcode) {
            errors.push(format!(
                "duplicate wire-contract opcode 0x{:02X}",
                opcode.opcode
            ));
        }
        if !names.insert(opcode.name.as_str()) {
            errors.push(format!(
                "duplicate wire-contract operation `{}`",
                opcode.name
            ));
        }
    }
    if contract.opcodes != expected.opcodes {
        errors.push(
            "recording opcode arguments, tail program, or semantic validator drifted".to_owned(),
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

/// Renders the allocation-free runtime codec table consumed by Safe Rust replay preflight.
pub fn render_runtime_parser(
    contract: &RecordingWireContract,
    contract_blake3: &str,
    effective_source_sha256: &str,
) -> Result<String> {
    validate_contract_shape(contract)?;
    if !is_lower_hex(contract_blake3, 64) {
        return Err(Error::message(
            "recording contract artifact digest must be lowercase 64-character BLAKE3",
        ));
    }
    if !is_lower_hex(effective_source_sha256, 64) {
        return Err(Error::message(
            "effective source digest must be lowercase 64-character SHA-256",
        ));
    }

    let tags = contract
        .tag_codecs
        .iter()
        .map(|codec| (codec.tag.as_str(), &codec.codec))
        .collect::<BTreeMap<_, _>>();
    let tails = contract
        .tail_codecs
        .iter()
        .map(|codec| (codec.name.as_str(), &codec.codec))
        .collect::<BTreeMap<_, _>>();
    let mut operations = contract.opcodes.iter().collect::<Vec<_>>();
    operations.sort_by_key(|operation| operation.opcode);

    let mut output = String::new();
    writeln!(
        output,
        "// @generated by cargo run -p xtask -- recording-wire-codegen --write; do not edit."
    )
    .expect("writing to a String cannot fail");
    writeln!(
        output,
        "use super::{{Argument, ArgumentTag, Operation, ReturnKind, SemanticClass, StreamRole, TailKind}};\n"
    )
    .expect("writing to a String cannot fail");
    writeln!(
        output,
        "pub(super) const CONTRACT_BLAKE3: &str = {contract_blake3:?};"
    )
    .expect("writing to a String cannot fail");
    writeln!(
        output,
        "pub(super) const UPSTREAM_SHA: &str = {:?};\n",
        contract.upstream_sha
    )
    .expect("writing to a String cannot fail");
    writeln!(
        output,
        "pub(super) const EFFECTIVE_SOURCE_SHA256: &str = {effective_source_sha256:?};\n"
    )
    .expect("writing to a String cannot fail");
    writeln!(
        output,
        "pub(super) const PROGRAM_TAKE: u8 = {RUNTIME_TAKE};\n\
         pub(super) const PROGRAM_BOOL: u8 = {RUNTIME_BOOL};\n\
         pub(super) const PROGRAM_PRECISION: u8 = {RUNTIME_PRECISION};\n\
         pub(super) const PROGRAM_STRING: u8 = {RUNTIME_STRING};\n\
         pub(super) const PROGRAM_NATIVE_POD: u8 = {RUNTIME_NATIVE_POD};\n\
         pub(super) const PROGRAM_COUNTED: u8 = {RUNTIME_COUNTED};\n\
         pub(super) const NATIVE_POD_CIRCLE: u8 = {RUNTIME_POD_CIRCLE};\n\
         pub(super) const NATIVE_POD_CAPSULE: u8 = {RUNTIME_POD_CAPSULE};\n\
         pub(super) const NATIVE_POD_SEGMENT: u8 = {RUNTIME_POD_SEGMENT};\n\
         pub(super) const NATIVE_POD_POLYGON: u8 = {RUNTIME_POD_POLYGON};\n\
         pub(super) const NATIVE_POD_CHAIN_SEGMENT: u8 = {RUNTIME_POD_CHAIN_SEGMENT};\n"
    )
    .expect("writing to a String cannot fail");
    writeln!(
        output,
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]\npub(super) enum OperationRule {{"
    )
    .expect("writing to a String cannot fail");
    for operation in &operations {
        writeln!(output, "    {},", operation.name).expect("writing to a String cannot fail");
    }
    writeln!(output, "}}\n").expect("writing to a String cannot fail");
    writeln!(output, "pub(super) static OPERATIONS: &[Operation] = &[")
        .expect("writing to a String cannot fail");
    for operation in operations {
        let mut program = Vec::new();
        let mut arguments = Vec::new();
        let mut stack = BTreeSet::new();
        for argument in &operation.arguments {
            encode_runtime_codec(
                tags.get(argument.tag.as_str()).copied().ok_or_else(|| {
                    Error::message(format!(
                        "operation {} references missing tag {}",
                        operation.name, argument.tag
                    ))
                })?,
                &tags,
                &mut stack,
                &mut program,
            )?;
            let program_end = u16::try_from(program.len()).map_err(|_| {
                Error::message(format!(
                    "runtime program for {} exceeds u16 argument offsets",
                    operation.name
                ))
            })?;
            arguments.push((runtime_tag_variant(&argument.tag)?, program_end));
        }
        encode_runtime_codec(
            tails
                .get(operation.tail_program.as_str())
                .copied()
                .ok_or_else(|| {
                    Error::message(format!(
                        "operation {} references missing tail {}",
                        operation.name, operation.tail_program
                    ))
                })?,
            &tags,
            &mut stack,
            &mut program,
        )?;
        let semantic = match operation.semantic_validator {
            ReplaySemanticClass::Mutation => "Mutation",
            ReplaySemanticClass::Query => "Query",
            ReplaySemanticClass::Step => "Step",
            ReplaySemanticClass::StateHash => "StateHash",
            ReplaySemanticClass::Metadata => "Metadata",
            ReplaySemanticClass::Terminal => "Terminal",
        };
        let role = if operation.name == contract.stream_grammar.initial_operation {
            "Initial"
        } else if operation.name == contract.stream_grammar.final_metadata_operation {
            "FinalMetadata"
        } else if operation.name == contract.stream_grammar.terminal_operation {
            "Terminal"
        } else {
            "Ordinary"
        };
        let tail = runtime_tail_variant(&operation.tail_program)?;
        let return_kind = runtime_return_variant(&operation.return_tag)?;
        writeln!(
            output,
            "    Operation {{ opcode: 0x{:02X}, name: {:?}, semantic: SemanticClass::{semantic}, role: StreamRole::{role}, rule: OperationRule::{}, return_kind: ReturnKind::{return_kind}, tail: TailKind::{tail}, arguments: &[",
            operation.opcode, operation.name, operation.name
        )
        .expect("writing to a String cannot fail");
        for (tag, program_end) in arguments {
            writeln!(
                output,
                "        Argument {{ tag: ArgumentTag::{tag}, program_end: {program_end} }},"
            )
            .expect("writing to a String cannot fail");
        }
        output.push_str("    ], program: &[\n");
        for chunk in program.chunks(16) {
            output.push_str("        ");
            for (index, byte) in chunk.iter().enumerate() {
                if index != 0 {
                    output.push(' ');
                }
                write!(output, "0x{byte:02X},").expect("writing to a String cannot fail");
            }
            output.push('\n');
        }
        output.push_str("    ] },\n");
    }
    output.push_str("];\n");
    Ok(output)
}

fn runtime_tag_variant(tag: &str) -> Result<&'static str> {
    match tag {
        "AABB" => Ok("Aabb"),
        "BODYDEF" => Ok("BodyDef"),
        "BODYID" => Ok("BodyId"),
        "BOOL" => Ok("Bool"),
        "CAPSULE" => Ok("Capsule"),
        "CHAINDEF" => Ok("ChainDef"),
        "CHAINID" => Ok("ChainId"),
        "CHAINSEG" => Ok("ChainSegment"),
        "CIRCLE" => Ok("Circle"),
        "DISTANCEJOINTDEF" => Ok("DistanceJointDef"),
        "EXPLOSIONDEF" => Ok("ExplosionDef"),
        "F32" => Ok("F32"),
        "FILTER" => Ok("Filter"),
        "FILTERJOINTDEF" => Ok("FilterJointDef"),
        "I32" => Ok("I32"),
        "JOINTID" => Ok("JointId"),
        "LOCKS" => Ok("Locks"),
        "MASSDATA" => Ok("MassData"),
        "MATERIAL" => Ok("Material"),
        "MOTORJOINTDEF" => Ok("MotorJointDef"),
        "POLYGON" => Ok("Polygon"),
        "POSITION" => Ok("Position"),
        "PRISMATICJOINTDEF" => Ok("PrismaticJointDef"),
        "QUERYFILTER" => Ok("QueryFilter"),
        "REVOLUTEJOINTDEF" => Ok("RevoluteJointDef"),
        "ROT" => Ok("Rot"),
        "SEGMENT" => Ok("Segment"),
        "SHAPEDEF" => Ok("ShapeDef"),
        "SHAPEID" => Ok("ShapeId"),
        "SHAPEPROXY" => Ok("ShapeProxy"),
        "STR" => Ok("Str"),
        "U64" => Ok("U64"),
        "VEC2" => Ok("Vec2"),
        "WELDJOINTDEF" => Ok("WeldJointDef"),
        "WHEELJOINTDEF" => Ok("WheelJointDef"),
        "WORLDID" => Ok("WorldId"),
        "WORLDXF" => Ok("WorldTransform"),
        "XF" => Ok("Transform"),
        _ => Err(Error::message(format!(
            "argument tag {tag} has no runtime semantic variant"
        ))),
    }
}

fn runtime_tail_variant(tail: &str) -> Result<&'static str> {
    match tail {
        "none" => Ok("None"),
        "returned-id" => Ok("ReturnedId"),
        "overlap-hits" => Ok("OverlapHits"),
        "cast-hits" => Ok("CastHits"),
        "plane-hits" => Ok("PlaneHits"),
        "closest-ray-result" => Ok("ClosestRayResult"),
        "mover-result" => Ok("MoverResult"),
        "bool-result" => Ok("BoolResult"),
        "shape-cast-result" => Ok("ShapeCastResult"),
        _ => Err(Error::message(format!(
            "tail program {tail} has no runtime semantic variant"
        ))),
    }
}

fn runtime_return_variant(return_tag: &str) -> Result<&'static str> {
    match return_tag {
        "RET_NONE" => Ok("None"),
        "RET_BODYID" => Ok("Body"),
        "RET_SHAPEID" => Ok("Shape"),
        "RET_CHAINID" => Ok("Chain"),
        "RET_JOINTID" => Ok("Joint"),
        _ => Err(Error::message(format!(
            "return tag {return_tag} has no runtime semantic variant"
        ))),
    }
}

const RUNTIME_TAKE: u8 = 1;
const RUNTIME_BOOL: u8 = 2;
const RUNTIME_PRECISION: u8 = 3;
const RUNTIME_STRING: u8 = 4;
const RUNTIME_NATIVE_POD: u8 = 5;
const RUNTIME_COUNTED: u8 = 6;
const RUNTIME_POD_CIRCLE: u8 = 1;
const RUNTIME_POD_CAPSULE: u8 = 2;
const RUNTIME_POD_SEGMENT: u8 = 3;
const RUNTIME_POD_POLYGON: u8 = 4;
const RUNTIME_POD_CHAIN_SEGMENT: u8 = 5;

fn encode_runtime_codec(
    codec: &Codec,
    tags: &BTreeMap<&str, &Codec>,
    stack: &mut BTreeSet<String>,
    output: &mut Vec<u8>,
) -> Result<()> {
    match codec {
        Codec::Fixed { bytes, boolean } => {
            if *boolean {
                if *bytes != 1 {
                    return Err(Error::message("runtime BOOL codec must occupy one byte"));
                }
                output.push(RUNTIME_BOOL);
            } else {
                output.push(RUNTIME_TAKE);
                push_runtime_u32(output, *bytes, "fixed codec width")?;
            }
        }
        Codec::Precision {
            single_bytes,
            double_bytes,
        } => {
            output.push(RUNTIME_PRECISION);
            push_runtime_u32(output, *single_bytes, "single-precision codec width")?;
            push_runtime_u32(output, *double_bytes, "double-precision codec width")?;
        }
        Codec::String {
            length_bytes,
            null_sentinel,
            max_bytes,
        } => {
            if *length_bytes != 2 || *null_sentinel > u64::from(u16::MAX) {
                return Err(Error::message("runtime string codec requires a u16 length"));
            }
            output.push(RUNTIME_STRING);
            output.extend_from_slice(&(*null_sentinel as u16).to_le_bytes());
            push_runtime_u32(output, *max_bytes, "string codec maximum")?;
        }
        Codec::NativePod { abi_type, .. } => {
            let pod = match abi_type.as_str() {
                "b2Circle" => RUNTIME_POD_CIRCLE,
                "b2Capsule" => RUNTIME_POD_CAPSULE,
                "b2Segment" => RUNTIME_POD_SEGMENT,
                "b2Polygon" => RUNTIME_POD_POLYGON,
                "b2ChainSegment" => RUNTIME_POD_CHAIN_SEGMENT,
                _ => {
                    return Err(Error::message(format!(
                        "unsupported runtime native POD {abi_type}"
                    )));
                }
            };
            output.extend_from_slice(&[RUNTIME_NATIVE_POD, pod]);
        }
        Codec::Tag { tag } => {
            if !stack.insert(tag.clone()) {
                return Err(Error::message(format!("recursive runtime codec tag {tag}")));
            }
            let result = encode_runtime_codec(
                tags.get(tag.as_str())
                    .copied()
                    .ok_or_else(|| Error::message(format!("missing runtime codec tag {tag}")))?,
                tags,
                stack,
                output,
            );
            stack.remove(tag);
            result?;
        }
        Codec::Sequence { steps } => {
            for step in steps {
                encode_runtime_codec(step, tags, stack, output)?;
            }
        }
        Codec::Counted { count, element } => {
            if count.bytes != 4 || !count.remaining_bytes_bound || !count.checked_multiply {
                return Err(Error::message("runtime count codec is not bounded"));
            }
            let mut element_program = Vec::new();
            encode_runtime_codec(element, tags, stack, &mut element_program)?;
            output.extend_from_slice(&[RUNTIME_COUNTED, u8::from(count.signed)]);
            push_runtime_u32(output, count.max_count, "count codec maximum")?;
            push_runtime_u32(output, element_program.len(), "count element program width")?;
            output.extend_from_slice(&element_program);
        }
    }
    Ok(())
}

fn push_runtime_u32(output: &mut Vec<u8>, value: usize, label: &str) -> Result<()> {
    output.extend_from_slice(
        &u32::try_from(value)
            .map_err(|_| Error::message(format!("{label} does not fit u32")))?
            .to_le_bytes(),
    );
    Ok(())
}

/// Computes the canonical aggregate for an exact reviewed source-path to Git-blob map.
pub fn reviewed_sources_aggregate_blake3(
    source_git_blobs: &BTreeMap<String, String>,
) -> Result<String> {
    let sources = reviewed_sources_from_map(source_git_blobs)?;
    aggregate_reviewed_sources(&sources)
}

fn reviewed_sources_from_map(
    source_git_blobs: &BTreeMap<String, String>,
) -> Result<Vec<ReviewedRecordingSource>> {
    let actual_paths = source_git_blobs
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if actual_paths != REVIEWED_RECORDING_INPUT_PATHS {
        return Err(Error::message(format!(
            "reviewed recording sources must be the exact set and order {REVIEWED_RECORDING_INPUT_PATHS:?}, observed {actual_paths:?}"
        )));
    }

    let sources = source_git_blobs
        .iter()
        .map(|(path, git_blob)| ReviewedRecordingSource {
            path: path.clone(),
            git_blob: git_blob.clone(),
        })
        .collect::<Vec<_>>();
    validate_reviewed_source_entries(&sources)?;
    Ok(sources)
}

fn validate_reviewed_sources(
    sources: &[ReviewedRecordingSource],
    aggregate_blake3: &str,
) -> Result<()> {
    let mut errors = Vec::new();
    let actual_paths = sources
        .iter()
        .map(|source| source.path.as_str())
        .collect::<Vec<_>>();
    if actual_paths != REVIEWED_RECORDING_INPUT_PATHS {
        errors.push(format!(
            "recording wire reviewed sources must be the exact set and order {REVIEWED_RECORDING_INPUT_PATHS:?}, observed {actual_paths:?}"
        ));
    }

    let mut unique_paths = BTreeSet::new();
    for source in sources {
        if !unique_paths.insert(source.path.as_str()) {
            errors.push(format!(
                "recording wire repeats reviewed source `{}`",
                source.path
            ));
        }
        if !is_lower_hex(&source.git_blob, 40) {
            errors.push(format!(
                "recording wire reviewed source `{}` must use a lowercase 40-character Git blob object ID",
                source.path
            ));
        }
    }

    if !is_lower_hex(aggregate_blake3, 64) {
        errors.push(
            "recording wire reviewed-source aggregate must be a lowercase 64-character BLAKE3 digest"
                .to_owned(),
        );
    }
    match aggregate_reviewed_sources(sources) {
        Ok(expected) if expected != aggregate_blake3 => errors.push(format!(
            "recording wire reviewed-source aggregate drifted: expected {expected}, observed {aggregate_blake3}"
        )),
        Ok(_) => {}
        Err(error) => errors.push(error.to_string()),
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

fn validate_reviewed_source_entries(sources: &[ReviewedRecordingSource]) -> Result<()> {
    let mut errors = Vec::new();
    for source in sources {
        if !is_lower_hex(&source.git_blob, 40) {
            errors.push(format!(
                "reviewed recording source `{}` must use a lowercase 40-character Git blob object ID",
                source.path
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

fn validate_aggregate(
    sources: &[ReviewedRecordingSource],
    aggregate_blake3: &str,
    label: &str,
) -> Result<()> {
    if !is_lower_hex(aggregate_blake3, 64) {
        return Err(Error::message(format!(
            "{label} aggregate must be a lowercase 64-character BLAKE3 digest"
        )));
    }
    let expected = aggregate_reviewed_sources(sources)?;
    if aggregate_blake3 != expected {
        return Err(Error::message(format!(
            "{label} aggregate does not match its path/Git-blob identities: expected {expected}, observed {aggregate_blake3}"
        )));
    }
    Ok(())
}

fn aggregate_reviewed_sources(sources: &[ReviewedRecordingSource]) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    // Fixed-width schema/count fields and length-prefixed entry fields make the encoding
    // unambiguous without relying on a textual serialization format.
    hasher.update(REVIEWED_SOURCE_AGGREGATE_DOMAIN);
    hasher.update(&RECORDING_WIRE_SCHEMA.to_le_bytes());
    hasher.update(
        &u64::try_from(sources.len())
            .map_err(|_| Error::message("reviewed-source count does not fit u64"))?
            .to_le_bytes(),
    );
    for source in sources {
        hash_length_prefixed(&mut hasher, source.path.as_bytes())?;
        hash_length_prefixed(&mut hasher, source.git_blob.as_bytes())?;
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn hash_length_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) -> Result<()> {
    hasher.update(
        &u64::try_from(bytes.len())
            .map_err(|_| Error::message("reviewed-source identity field does not fit u64"))?
            .to_le_bytes(),
    );
    hasher.update(bytes);
    Ok(())
}

fn is_lower_hex(value: &str, width: usize) -> bool {
    value.len() == width
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn expected_framing() -> RecordingFraming {
    RecordingFraming {
        header_bytes: 32,
        opcode_bytes: 1,
        payload_size_bytes: 3,
        max_payload_exclusive: 1 << 24,
        byte_order: "little".to_owned(),
        stream_termination: "exact-eof".to_owned(),
    }
}

fn expected_header() -> RecordingHeaderIdentity {
    RecordingHeaderIdentity {
        magic_offset: 0,
        magic: 0x4352_3242,
        version_major_offset: 4,
        version_major: 3,
        version_minor_offset: 6,
        version_minor: 2,
        reserved2_offset: 8,
        length_scale_offset: 12,
        reserved3_offset: 16,
        pointer_width_offset: 17,
        big_endian_offset: 18,
        validation_enabled_offset: 19,
        reserved1_offset: 20,
        snapshot_size_offset: 24,
        snapshot_size_bytes: 8,
    }
}

fn expected_snapshot() -> SnapshotIdentity {
    SnapshotIdentity {
        minimum_bytes: 16,
        header_bytes: 16,
        magic_offset: 0,
        magic: 0x3253_4E42,
        version_offset: 4,
        version: 3,
        layout_hash_offset: 8,
        flags_offset: 12,
        known_flags_mask: 0x3,
        validation_flag: 0x1,
        double_precision_flag: 0x2,
        length_source: "recording-header-u64".to_owned(),
        slice_termination: "exact-declared-length".to_owned(),
        layout_metadata_source: "boxdd-sys-generated-abi".to_owned(),
        validation_requirement: "u8-structural-and-semantic-validator-required".to_owned(),
        native_dispatch_before_validation: "forbidden".to_owned(),
    }
}

fn expected_stream_grammar() -> RecordingStreamGrammar {
    RecordingStreamGrammar {
        initial_operation: "StateHash".to_owned(),
        final_metadata_operation: "RecordingBounds".to_owned(),
        terminal_operation: "DestroyWorld".to_owned(),
        terminal_must_be_last: true,
    }
}

fn fixed(bytes: usize) -> Codec {
    Codec::Fixed {
        bytes,
        boolean: false,
    }
}

fn boolean() -> Codec {
    Codec::Fixed {
        bytes: 1,
        boolean: true,
    }
}

fn precision(single_bytes: usize, double_bytes: usize) -> Codec {
    Codec::Precision {
        single_bytes,
        double_bytes,
    }
}

fn tag(name: &str) -> Codec {
    Codec::Tag {
        tag: name.to_owned(),
    }
}

fn sequence(steps: Vec<Codec>) -> Codec {
    Codec::Sequence { steps }
}

fn repeated(tag_name: &str, count: usize) -> Vec<Codec> {
    (0..count).map(|_| tag(tag_name)).collect()
}

fn counted(signed: bool, max_count: usize, element: Codec) -> Codec {
    Codec::Counted {
        count: CountCodec {
            bytes: 4,
            signed,
            max_count,
            remaining_bytes_bound: true,
            checked_multiply: true,
        },
        element: Box::new(element),
    }
}

fn joint_base() -> Vec<Codec> {
    let mut steps = vec![fixed(8), tag("BODYID"), tag("BODYID"), tag("XF"), tag("XF")];
    steps.extend(repeated("F32", 5));
    steps.push(tag("BOOL"));
    steps
}

fn with_joint_base(mut remainder: Vec<Codec>) -> Codec {
    let mut steps = joint_base();
    steps.append(&mut remainder);
    sequence(steps)
}

fn expected_tag_codecs() -> Vec<TagCodec> {
    let mut codecs = BTreeMap::new();
    codecs.insert("AABB", sequence(vec![tag("VEC2"), tag("VEC2")]));
    let mut body = vec![tag("I32"), tag("POSITION"), tag("ROT"), tag("VEC2")];
    body.extend(repeated("F32", 5));
    body.push(tag("STR"));
    body.push(tag("U64"));
    body.push(tag("LOCKS"));
    body.extend(repeated("BOOL", 6));
    codecs.insert("BODYDEF", sequence(body));
    codecs.insert("BODYID", fixed(8));
    codecs.insert("BOOL", boolean());
    codecs.insert(
        "CHAINDEF",
        sequence(vec![
            fixed(8),
            counted(true, MAX_SEQUENCE_ELEMENTS, tag("VEC2")),
            counted(true, MAX_SEQUENCE_ELEMENTS, tag("MATERIAL")),
            tag("FILTER"),
            tag("BOOL"),
            tag("BOOL"),
        ]),
    );
    codecs.insert("CHAINID", fixed(8));
    codecs.insert(
        "DISTANCEJOINTDEF",
        with_joint_base(vec![
            tag("F32"),
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
            tag("F32"),
            tag("F32"),
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
        ]),
    );
    codecs.insert(
        "EXPLOSIONDEF",
        sequence(vec![
            tag("U64"),
            tag("POSITION"),
            tag("F32"),
            tag("F32"),
            tag("F32"),
        ]),
    );
    codecs.insert("F32", fixed(4));
    codecs.insert("FILTER", fixed(20));
    codecs.insert("FILTERJOINTDEF", with_joint_base(Vec::new()));
    codecs.insert("I32", fixed(4));
    codecs.insert("JOINTID", fixed(8));
    codecs.insert(
        "LOCKS",
        sequence(vec![tag("BOOL"), tag("BOOL"), tag("BOOL")]),
    );
    codecs.insert("MASSDATA", fixed(16));
    codecs.insert("MATERIAL", fixed(28));
    let mut motor = vec![tag("VEC2")];
    motor.extend(repeated("F32", 9));
    codecs.insert("MOTORJOINTDEF", with_joint_base(motor));
    codecs.insert("POLYGON", native_pod("b2Polygon"));
    codecs.insert("POSITION", precision(8, 16));
    codecs.insert(
        "PRISMATICJOINTDEF",
        with_joint_base(vec![
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
            tag("F32"),
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
        ]),
    );
    codecs.insert("QUERYFILTER", fixed(16));
    codecs.insert(
        "REVOLUTEJOINTDEF",
        with_joint_base(vec![
            tag("F32"),
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
        ]),
    );
    codecs.insert("ROT", fixed(8));
    codecs.insert("SEGMENT", native_pod("b2Segment"));
    codecs.insert(
        "SHAPEDEF",
        sequence({
            let mut steps = vec![fixed(8), tag("MATERIAL"), tag("F32"), tag("FILTER")];
            steps.extend(repeated("BOOL", 8));
            steps
        }),
    );
    codecs.insert("SHAPEID", fixed(8));
    codecs.insert(
        "SHAPEPROXY",
        sequence(vec![counted(true, 8, tag("VEC2")), tag("F32")]),
    );
    codecs.insert(
        "STR",
        Codec::String {
            length_bytes: 2,
            null_sentinel: 0xFFFF,
            max_bytes: 65_534,
        },
    );
    codecs.insert("U64", fixed(8));
    codecs.insert("VEC2", fixed(8));
    codecs.insert("WELDJOINTDEF", with_joint_base(repeated("F32", 4)));
    codecs.insert(
        "WHEELJOINTDEF",
        with_joint_base(vec![
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
            tag("BOOL"),
            tag("F32"),
            tag("F32"),
        ]),
    );
    codecs.insert("WORLDID", fixed(4));
    codecs.insert("WORLDXF", sequence(vec![tag("POSITION"), tag("ROT")]));
    codecs.insert("XF", fixed(16));
    codecs.insert("CIRCLE", native_pod("b2Circle"));
    codecs.insert("CAPSULE", native_pod("b2Capsule"));
    codecs.insert("CHAINSEG", native_pod("b2ChainSegment"));

    ARGUMENT_TAGS
        .iter()
        .map(|tag_name| TagCodec {
            tag: (*tag_name).to_owned(),
            codec: codecs
                .remove(tag_name)
                .unwrap_or_else(|| panic!("missing codec for {tag_name}")),
        })
        .collect()
}

fn native_pod(abi_type: &str) -> Codec {
    Codec::NativePod {
        abi_type: abi_type.to_owned(),
        size_source: "boxdd-sys-generated-abi".to_owned(),
    }
}

fn expected_tail_codecs() -> Vec<TailCodec> {
    let plane_result = sequence(vec![tag("VEC2"), tag("F32"), tag("VEC2"), tag("BOOL")]);
    let tree_stats = fixed(8);
    vec![
        TailCodec {
            name: "none".to_owned(),
            codec: sequence(Vec::new()),
        },
        TailCodec {
            name: "returned-id".to_owned(),
            codec: fixed(8),
        },
        TailCodec {
            name: "overlap-hits".to_owned(),
            codec: sequence(vec![
                counted(
                    false,
                    MAX_SEQUENCE_ELEMENTS,
                    sequence(vec![tag("SHAPEID"), tag("BOOL")]),
                ),
                tree_stats.clone(),
            ]),
        },
        TailCodec {
            name: "cast-hits".to_owned(),
            codec: sequence(vec![
                counted(
                    false,
                    MAX_SEQUENCE_ELEMENTS,
                    sequence(vec![
                        tag("SHAPEID"),
                        tag("POSITION"),
                        tag("VEC2"),
                        tag("F32"),
                        tag("F32"),
                    ]),
                ),
                tree_stats,
            ]),
        },
        TailCodec {
            name: "plane-hits".to_owned(),
            codec: counted(
                false,
                MAX_SEQUENCE_ELEMENTS,
                sequence(vec![tag("SHAPEID"), plane_result, tag("BOOL")]),
            ),
        },
        TailCodec {
            name: "closest-ray-result".to_owned(),
            codec: sequence(vec![
                tag("SHAPEID"),
                tag("POSITION"),
                tag("VEC2"),
                tag("F32"),
                tag("I32"),
                tag("I32"),
                tag("BOOL"),
            ]),
        },
        TailCodec {
            name: "mover-result".to_owned(),
            codec: tag("F32"),
        },
        TailCodec {
            name: "bool-result".to_owned(),
            codec: tag("BOOL"),
        },
        TailCodec {
            name: "shape-cast-result".to_owned(),
            codec: sequence(vec![
                tag("VEC2"),
                tag("POSITION"),
                tag("F32"),
                tag("I32"),
                tag("BOOL"),
            ]),
        },
    ]
}

fn wire_opcode(operation: &RecordingOp) -> Result<WireOpcode> {
    let tail_program = tail_program(operation)?;
    Ok(WireOpcode {
        opcode: operation.opcode,
        name: operation.name.clone(),
        return_tag: operation.return_tag.clone(),
        arguments: operation.arguments.clone(),
        tail_program: tail_program.to_owned(),
        semantic_validator: semantic_class(operation)?,
    })
}

fn tail_program(operation: &RecordingOp) -> Result<&'static str> {
    let tail = match operation.opcode {
        0xE0 | 0xE1 => "overlap-hits",
        0xE2 | 0xE3 => "cast-hits",
        0xE4 => "plane-hits",
        0xE5 => "closest-ray-result",
        0xE6 => "mover-result",
        0xE7 => "bool-result",
        0xE8 => "shape-cast-result",
        0xE9..=0xEF => {
            return Err(Error::message(format!(
                "recording query opcode 0x{:02X} `{}` has no reviewed tail program",
                operation.opcode, operation.name
            )));
        }
        _ if operation.return_tag != "RET_NONE" => "returned-id",
        0x00..=0xDF | 0xF0..=0xFF => "none",
    };
    Ok(tail)
}

fn semantic_class(operation: &RecordingOp) -> Result<ReplaySemanticClass> {
    let class = match operation.opcode {
        0x01 => ReplaySemanticClass::Terminal,
        0x80 => ReplaySemanticClass::Step,
        0xE0..=0xEF => ReplaySemanticClass::Query,
        0xF1 => ReplaySemanticClass::StateHash,
        0xF2 => ReplaySemanticClass::Metadata,
        0x00..=0xDF | 0xF0 | 0xF3..=0xFF => ReplaySemanticClass::Mutation,
    };
    let registered_query = QUERY_OPERATIONS.contains(&operation.name.as_str());
    if (class == ReplaySemanticClass::Query) != registered_query {
        return Err(Error::message(format!(
            "recording opcode 0x{:02X} `{}` disagrees with the reviewed query registry",
            operation.opcode, operation.name,
        )));
    }
    Ok(class)
}

fn validate_contract_shape(contract: &RecordingWireContract) -> Result<()> {
    let mut errors = Vec::new();
    if contract.schema_version != RECORDING_WIRE_SCHEMA {
        errors.push(format!(
            "recording wire schema {} does not match supported schema {RECORDING_WIRE_SCHEMA}",
            contract.schema_version
        ));
    }
    if contract.upstream_sha.len() != 40
        || !contract
            .upstream_sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        errors.push("recording contract upstream must be a lowercase full Git SHA".to_owned());
    }
    if let Err(error) = validate_reviewed_sources(
        &contract.reviewed_sources,
        &contract.reviewed_sources_aggregate_blake3,
    ) {
        errors.push(error.to_string());
    }
    if contract.framing != expected_framing()
        || contract.header != expected_header()
        || contract.snapshot != expected_snapshot()
        || contract.stream_grammar != expected_stream_grammar()
    {
        errors.push("recording contract framing or identity layout drifted".to_owned());
    }
    if contract.argument_tags
        != ARGUMENT_TAGS
            .iter()
            .map(|tag| (*tag).to_owned())
            .collect::<Vec<_>>()
        || contract.return_tags
            != RETURN_TAGS
                .iter()
                .map(|tag| (*tag).to_owned())
                .collect::<Vec<_>>()
        || contract.tag_codecs != expected_tag_codecs()
        || contract.tail_codecs != expected_tail_codecs()
    {
        errors.push("recording contract codec registry is incomplete or drifted".to_owned());
    }

    let expected_tags = ARGUMENT_TAGS.iter().copied().collect::<BTreeSet<_>>();
    let actual_tags = contract
        .tag_codecs
        .iter()
        .map(|codec| codec.tag.as_str())
        .collect::<BTreeSet<_>>();
    if actual_tags != expected_tags || contract.tag_codecs.len() != expected_tags.len() {
        errors.push("recording contract codec tags are missing or duplicated".to_owned());
    }
    let tail_names = contract
        .tail_codecs
        .iter()
        .map(|codec| codec.name.as_str())
        .collect::<BTreeSet<_>>();
    if tail_names.len() != contract.tail_codecs.len() {
        errors.push("recording contract tail programs contain duplicates".to_owned());
    }

    let operations = contract
        .opcodes
        .iter()
        .map(|opcode| RecordingOp {
            opcode: opcode.opcode,
            name: opcode.name.clone(),
            return_tag: opcode.return_tag.clone(),
            arguments: opcode.arguments.clone(),
        })
        .collect::<Vec<_>>();
    if let Err(error) = validate_operations(&operations) {
        errors.push(error.to_string());
    } else {
        for (actual, operation) in contract.opcodes.iter().zip(&operations) {
            match wire_opcode(operation) {
                Ok(expected) if actual == &expected => {}
                Ok(_) => errors.push(format!(
                    "recording opcode 0x{:02X} `{}` has inconsistent tail, semantics, or ID effects",
                    actual.opcode, actual.name
                )),
                Err(error) => errors.push(error.to_string()),
            }
        }
    }
    for required in [
        contract.stream_grammar.initial_operation.as_str(),
        contract.stream_grammar.final_metadata_operation.as_str(),
        contract.stream_grammar.terminal_operation.as_str(),
    ] {
        if !contract
            .opcodes
            .iter()
            .any(|opcode| opcode.name == required)
        {
            errors.push(format!(
                "recording stream grammar references missing operation `{required}`"
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recording_ops::parse;

    const SHA: &str = "0123456789012345678901234567890123456789";

    fn source_git_blobs() -> BTreeMap<String, String> {
        REVIEWED_RECORDING_INPUT_PATHS
            .iter()
            .enumerate()
            .map(|(index, path)| ((*path).to_owned(), format!("{:040x}", index + 1)))
            .collect()
    }

    fn wire_contract(operations: &[RecordingOp]) -> RecordingWireContract {
        let sources = source_git_blobs();
        let aggregate = reviewed_sources_aggregate_blake3(&sources).expect("source aggregate");
        generate_wire_contract(SHA, operations, &sources, &aggregate).expect("wire contract")
    }

    fn validate_test_contract(
        contract: &RecordingWireContract,
        operations: &[RecordingOp],
    ) -> Result<()> {
        let sources = source_git_blobs();
        let aggregate = reviewed_sources_aggregate_blake3(&sources)?;
        validate_wire_contract(contract, operations, SHA, &sources, &aggregate)
    }

    fn operations() -> Vec<RecordingOp> {
        parse(
            r#"
                B2_REC_OP(0x01, DestroyWorld, RET_NONE, ARG(WORLDID, world))
                B2_REC_OP(0x80, Step, RET_NONE,
                    ARG(WORLDID, world) ARG(F32, dt) ARG(I32, subSteps))
                B2_REC_OP(0x02, WorldEnableSleeping, RET_NONE,
                    ARG(WORLDID, world) ARG(BOOL, flag))
                B2_REC_OP(0x10, CreateBody, RET_BODYID,
                    ARG(WORLDID, world) ARG(BODYDEF, def))
                B2_REC_OP(0x20, BodySetTransform, RET_NONE,
                    ARG(BODYID, body) ARG(POSITION, position) ARG(ROT, rotation))
                B2_REC_OP(0x70, CreateChain, RET_CHAINID,
                    ARG(BODYID, body) ARG(CHAINDEF, def))
                B2_REC_OP(0x90, CreateDistanceJoint, RET_JOINTID,
                    ARG(WORLDID, world) ARG(DISTANCEJOINTDEF, def))
                B2_REC_OP(0xE0, QueryOverlapAABB, RET_NONE,
                    ARG(WORLDID, world) ARG(POSITION, origin) ARG(AABB, aabb)
                    ARG(QUERYFILTER, filter))
                B2_REC_OP(0xE1, QueryOverlapShape, RET_NONE,
                    ARG(WORLDID, world) ARG(POSITION, origin) ARG(SHAPEPROXY, proxy)
                    ARG(QUERYFILTER, filter))
                B2_REC_OP(0xE5, QueryCastRayClosest, RET_NONE,
                    ARG(WORLDID, world) ARG(POSITION, origin) ARG(VEC2, translation)
                    ARG(QUERYFILTER, filter))
                B2_REC_OP(0xF1, StateHash, RET_NONE,
                    ARG(WORLDID, world) ARG(U64, hash))
                B2_REC_OP(0xF2, RecordingBounds, RET_NONE, ARG(AABB, bounds))
            "#,
        )
        .expect("fixture operations")
    }

    #[test]
    fn runtime_parser_binds_canonical_effective_source_identity() {
        let contract = wire_contract(&operations());
        let contract_blake3 = "a".repeat(64);
        let effective_source_sha256 = "b".repeat(64);
        let rendered = render_runtime_parser(&contract, &contract_blake3, &effective_source_sha256)
            .expect("render runtime parser");

        assert!(rendered.contains(&format!(
            "pub(super) const EFFECTIVE_SOURCE_SHA256: &str = {effective_source_sha256:?};"
        )));
        let error = render_runtime_parser(&contract, &contract_blake3, &"A".repeat(64))
            .expect_err("non-canonical effective source digest must fail");
        assert!(error.to_string().contains("effective source digest"));
    }

    #[test]
    fn runtime_parser_has_no_trailing_whitespace() {
        let contract = wire_contract(&operations());
        let rendered = render_runtime_parser(&contract, &"a".repeat(64), &"b".repeat(64))
            .expect("render runtime parser");

        assert!(
            rendered
                .lines()
                .all(|line| line.trim_end_matches([' ', '\t']) == line),
            "generated runtime parser must not contain trailing whitespace"
        );
    }

    #[test]
    fn contract_rejects_duplicate_opcode_and_codec_drift() {
        let operations = operations();
        let mut contract = wire_contract(&operations);
        contract.opcodes.push(contract.opcodes[0].clone());
        let error =
            validate_test_contract(&contract, &operations).expect_err("duplicate opcode must fail");
        assert!(error.to_string().contains("duplicate wire-contract opcode"));

        let mut contract = wire_contract(&operations);
        contract
            .opcodes
            .iter_mut()
            .find(|opcode| opcode.opcode == 0xE0)
            .expect("query opcode")
            .tail_program = "none".to_owned();
        let error =
            validate_test_contract(&contract, &operations).expect_err("tail drift must fail");
        assert!(error.to_string().contains("tail program"));
    }

    #[test]
    fn reviewed_source_contract_is_deterministic() {
        let forward = source_git_blobs();
        let reverse = REVIEWED_RECORDING_INPUT_PATHS
            .iter()
            .enumerate()
            .rev()
            .map(|(index, path)| ((*path).to_owned(), format!("{:040x}", index + 1)))
            .collect::<BTreeMap<_, _>>();
        let forward_aggregate =
            reviewed_sources_aggregate_blake3(&forward).expect("forward aggregate");
        let reverse_aggregate =
            reviewed_sources_aggregate_blake3(&reverse).expect("reverse aggregate");
        assert_eq!(forward_aggregate, reverse_aggregate);

        let operations = operations();
        let forward_contract =
            generate_wire_contract(SHA, &operations, &forward, &forward_aggregate)
                .expect("forward contract");
        let reverse_contract =
            generate_wire_contract(SHA, &operations, &reverse, &reverse_aggregate)
                .expect("reverse contract");
        assert_eq!(forward_contract, reverse_contract);
        let forward_toml = toml::to_string_pretty(&forward_contract).expect("forward TOML");
        let reverse_toml = toml::to_string_pretty(&reverse_contract).expect("reverse TOML");
        assert_eq!(forward_toml, reverse_toml);
        assert_eq!(
            toml::from_str::<RecordingWireContract>(&forward_toml).expect("round-trip TOML"),
            forward_contract
        );
    }

    #[test]
    fn every_reviewed_source_path_and_blob_identity_is_bound_to_the_manifest() {
        let operations = operations();
        let expected = wire_contract(&operations);

        for index in 0..expected.reviewed_sources.len() {
            let mut forged = expected.clone();
            forged.reviewed_sources[index].git_blob = format!("{:040x}", index + 100);
            forged.reviewed_sources_aggregate_blake3 =
                aggregate_reviewed_sources(&forged.reviewed_sources).expect("forged aggregate");
            let error = validate_test_contract(&forged, &operations)
                .expect_err("a changed blob identity must fail");
            assert!(
                error
                    .to_string()
                    .contains("differ from the reviewed manifest"),
                "blob index {index}: {error}"
            );

            let mut forged = expected.clone();
            forged.reviewed_sources[index].path.push_str(".forged");
            forged.reviewed_sources_aggregate_blake3 =
                aggregate_reviewed_sources(&forged.reviewed_sources).expect("forged aggregate");
            let error = validate_test_contract(&forged, &operations)
                .expect_err("a changed source path must fail");
            assert!(
                error.to_string().contains("exact set and order"),
                "path index {index}: {error}"
            );
        }
    }

    #[test]
    fn reviewed_source_contract_rejects_missing_extra_duplicate_and_reordered_entries() {
        let operations = operations();
        let expected = wire_contract(&operations);
        let extra = ReviewedRecordingSource {
            path: "src/unreviewed.c".to_owned(),
            git_blob: "f".repeat(40),
        };

        let mut variants = Vec::new();
        let mut missing = expected.clone();
        missing.reviewed_sources.pop();
        variants.push(("missing", missing));

        let mut additional = expected.clone();
        additional.reviewed_sources.push(extra);
        variants.push(("extra", additional));

        let mut duplicate = expected.clone();
        duplicate.reviewed_sources[6] = duplicate.reviewed_sources[0].clone();
        variants.push(("duplicate", duplicate));

        let mut reordered = expected.clone();
        reordered.reviewed_sources.swap(0, 1);
        variants.push(("reordered", reordered));

        for (label, mut contract) in variants {
            contract.reviewed_sources_aggregate_blake3 =
                aggregate_reviewed_sources(&contract.reviewed_sources).expect("variant aggregate");
            let error = validate_test_contract(&contract, &operations)
                .expect_err("reviewed-source structural drift must fail");
            assert!(
                error.to_string().contains("exact set and order"),
                "{label}: {error}"
            );
            if label == "duplicate" {
                assert!(error.to_string().contains("repeats reviewed source"));
            }
        }
    }

    #[test]
    fn reviewed_source_aggregate_cannot_be_forged_or_auto_accepted() {
        let operations = operations();
        let mut contract = wire_contract(&operations);
        contract.reviewed_sources_aggregate_blake3 = "0".repeat(64);
        let error = validate_test_contract(&contract, &operations)
            .expect_err("a forged contract aggregate must fail");
        assert!(error.to_string().contains("aggregate drifted"));

        let sources = source_git_blobs();
        let error = generate_wire_contract(SHA, &operations, &sources, &"0".repeat(64))
            .expect_err("generation must not derive or accept a forged aggregate");
        assert!(error.to_string().contains("does not match"));

        let mut malformed_sources = sources;
        malformed_sources.insert(REVIEWED_RECORDING_INPUT_PATHS[0].to_owned(), "A".repeat(40));
        let error = reviewed_sources_aggregate_blake3(&malformed_sources)
            .expect_err("non-canonical Git blob identities must fail");
        assert!(error.to_string().contains("lowercase 40-character"));
    }

    #[test]
    fn reviewed_source_aggregate_length_prefixes_path_and_blob_fields() {
        let first = vec![
            ReviewedRecordingSource {
                path: "a".to_owned(),
                git_blob: "bc".to_owned(),
            },
            ReviewedRecordingSource {
                path: "de".to_owned(),
                git_blob: "f".to_owned(),
            },
        ];
        let second = vec![
            ReviewedRecordingSource {
                path: "ab".to_owned(),
                git_blob: "c".to_owned(),
            },
            ReviewedRecordingSource {
                path: "d".to_owned(),
                git_blob: "ef".to_owned(),
            },
        ];
        assert_eq!(
            first
                .iter()
                .flat_map(|source| [source.path.as_str(), source.git_blob.as_str()])
                .collect::<String>(),
            second
                .iter()
                .flat_map(|source| [source.path.as_str(), source.git_blob.as_str()])
                .collect::<String>()
        );
        assert_ne!(
            aggregate_reviewed_sources(&first).expect("first aggregate"),
            aggregate_reviewed_sources(&second).expect("second aggregate")
        );
    }

    #[test]
    fn unreviewed_query_opcodes_fail_closed_instead_of_inheriting_mutation_defaults() {
        let operations = vec![RecordingOp {
            opcode: 0xE9,
            name: "QueryNewOperation".to_owned(),
            return_tag: "RET_NONE".to_owned(),
            arguments: vec![],
        }];
        let sources = source_git_blobs();
        let aggregate = reviewed_sources_aggregate_blake3(&sources).expect("source aggregate");
        let error = generate_wire_contract(SHA, &operations, &sources, &aggregate)
            .expect_err("unreviewed query opcode must fail closed");
        assert!(error.to_string().contains("no reviewed tail program"));
    }
}
