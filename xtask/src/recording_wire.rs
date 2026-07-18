use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    Error, Result,
    config::RECORDING_WIRE_SCHEMA,
    recording_ops::{
        ARGUMENT_TAGS, RETURN_TAGS, RecordingArgument, RecordingOp, validate_operations,
    },
};

const MAX_SEQUENCE_ELEMENTS: usize = 1_000_000;
const NATIVE_POD_TAGS: [(&str, &str); 5] = [
    ("CIRCLE", "b2Circle"),
    ("CAPSULE", "b2Capsule"),
    ("SEGMENT", "b2Segment"),
    ("POLYGON", "b2Polygon"),
    ("CHAINSEG", "b2ChainSegment"),
];
const DESTROY_OPERATIONS: &[(u8, &str)] = &[
    (0x01, "DestroyWorld"),
    (0x11, "DestroyBody"),
    (0x54, "DestroyShape"),
    (0x71, "DestroyChain"),
    (0x88, "DestroyJoint"),
];
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

/// Exact, canonical source set whose Git blob identities authorize a wire-contract schema.
pub const REVIEWED_RECORDING_INPUT_PATHS: &[&str] = &[
    "src/recording.c",
    "src/recording.h",
    "src/recording_ops.inl",
    "src/recording_replay.c",
    "src/recording_replay.h",
    "src/world_snapshot.c",
    "src/world_snapshot.h",
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordingPrecision {
    Single,
    Double,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingAbi {
    pub precision: RecordingPrecision,
    pub pointer_width: u8,
    pub validation_enabled: bool,
    pub snapshot_layout_hash: u32,
    pub native_pod_sizes: BTreeMap<String, usize>,
}

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
#[serde(rename_all = "kebab-case")]
pub enum IdAction {
    Create,
    Destroy,
    Use,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdEffect {
    pub action: IdAction,
    pub id_kind: String,
    pub source: String,
    pub repeated_by: Option<String>,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireOpcode {
    pub opcode: u8,
    pub name: String,
    pub return_tag: String,
    pub arguments: Vec<RecordingArgument>,
    pub tail_program: String,
    pub payload_termination: String,
    pub semantic_validator: ReplaySemanticClass,
    pub id_effects: Vec<IdEffect>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralPreflight {
    pub records: usize,
    pub snapshot_offset: usize,
    pub snapshot_bytes: usize,
    pub snapshot_requires_semantic_validation: bool,
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
        opcodes: operations.iter().map(wire_opcode).collect(),
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
            "recording opcode arguments, tail program, ID effects, or semantic validator drifted"
                .to_owned(),
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
}

/// Validates wire framing and codec boundaries only.
///
/// The returned value is deliberately not a replay authorization token. U8 must still validate
/// the native snapshot structure, live IDs, enum/value invariants, and replay semantics before any
/// input can be dispatched to Box2D.
pub fn preflight_structure(
    bytes: &[u8],
    contract: &RecordingWireContract,
    abi: &RecordingAbi,
) -> Result<StructuralPreflight> {
    validate_abi(abi)?;
    validate_contract_shape(contract)?;
    let snapshot_bytes = validate_header_and_snapshot(bytes, contract, abi)?;
    let snapshot_offset = contract.framing.header_bytes;
    let mut cursor = snapshot_offset
        .checked_add(snapshot_bytes)
        .ok_or_else(|| Error::message("recording snapshot offset overflow"))?;
    let opcodes = contract
        .opcodes
        .iter()
        .map(|operation| (operation.opcode, operation))
        .collect::<BTreeMap<_, _>>();
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
    let mut records = 0;
    let mut operation_names = Vec::new();

    while cursor < bytes.len() {
        let frame_header = contract.framing.opcode_bytes + contract.framing.payload_size_bytes;
        if bytes.len() - cursor < frame_header {
            return Err(Error::message("recording has a truncated opcode/u24 frame"));
        }
        let opcode = bytes[cursor];
        let operation = opcodes.get(&opcode).ok_or_else(|| {
            Error::message(format!("recording contains unknown opcode 0x{opcode:02X}"))
        })?;
        let payload_size = usize::from(bytes[cursor + 1])
            | (usize::from(bytes[cursor + 2]) << 8)
            | (usize::from(bytes[cursor + 3]) << 16);
        if payload_size >= contract.framing.max_payload_exclusive {
            return Err(Error::message("recording payload exceeds the u24 contract"));
        }
        let payload_start = cursor
            .checked_add(frame_header)
            .ok_or_else(|| Error::message("recording frame offset overflow"))?;
        let payload_end = payload_start
            .checked_add(payload_size)
            .ok_or_else(|| Error::message("recording payload offset overflow"))?;
        if payload_end > bytes.len() {
            return Err(Error::message("recording payload extends beyond input"));
        }
        let payload = &bytes[payload_start..payload_end];
        let mut interpreter = Interpreter::new(payload, abi, &tags);
        for argument in &operation.arguments {
            interpreter.consume_tag(&argument.tag)?;
        }
        let tail = tails.get(operation.tail_program.as_str()).ok_or_else(|| {
            Error::message(format!(
                "opcode `{}` references missing tail program `{}`",
                operation.name, operation.tail_program
            ))
        })?;
        interpreter.consume(tail)?;
        if interpreter.cursor != payload.len() {
            return Err(Error::message(format!(
                "opcode 0x{opcode:02X} `{}` consumed {} of {} payload bytes; exact payload EOF is required",
                operation.name,
                interpreter.cursor,
                payload.len()
            )));
        }
        cursor = payload_end;
        records += 1;
        operation_names.push(operation.name.as_str());
    }

    if cursor != bytes.len() {
        return Err(Error::message(
            "recording stream did not terminate at exact EOF",
        ));
    }
    validate_stream_grammar(&operation_names, &contract.stream_grammar)?;
    Ok(StructuralPreflight {
        records,
        snapshot_offset,
        snapshot_bytes,
        snapshot_requires_semantic_validation: true,
    })
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

fn wire_opcode(operation: &RecordingOp) -> WireOpcode {
    WireOpcode {
        opcode: operation.opcode,
        name: operation.name.clone(),
        return_tag: operation.return_tag.clone(),
        arguments: operation.arguments.clone(),
        tail_program: tail_program(operation).to_owned(),
        payload_termination: "exact-eof".to_owned(),
        semantic_validator: semantic_class(operation),
        id_effects: id_effects(operation),
    }
}

fn tail_program(operation: &RecordingOp) -> &'static str {
    match operation.opcode {
        0xE0 | 0xE1 => "overlap-hits",
        0xE2 | 0xE3 => "cast-hits",
        0xE4 => "plane-hits",
        0xE5 => "closest-ray-result",
        0xE6 => "mover-result",
        0xE7 => "bool-result",
        0xE8 => "shape-cast-result",
        _ if operation.return_tag != "RET_NONE" => "returned-id",
        _ => "none",
    }
}

fn semantic_class(operation: &RecordingOp) -> ReplaySemanticClass {
    match operation.name.as_str() {
        "DestroyWorld" => ReplaySemanticClass::Terminal,
        "Step" => ReplaySemanticClass::Step,
        "StateHash" => ReplaySemanticClass::StateHash,
        "RecordingBounds" => ReplaySemanticClass::Metadata,
        name if QUERY_OPERATIONS.contains(&name) => ReplaySemanticClass::Query,
        _ => ReplaySemanticClass::Mutation,
    }
}

fn id_effects(operation: &RecordingOp) -> Vec<IdEffect> {
    let destroying = DESTROY_OPERATIONS.contains(&(operation.opcode, operation.name.as_str()));
    let mut effects = operation
        .arguments
        .iter()
        .filter_map(|argument| id_kind(&argument.tag).map(|kind| (argument, kind)))
        .map(|(argument, kind)| IdEffect {
            action: if destroying && operation.arguments.first() == Some(argument) {
                IdAction::Destroy
            } else {
                IdAction::Use
            },
            id_kind: kind.to_owned(),
            source: format!("argument:{}", argument.name),
            repeated_by: None,
            condition: None,
        })
        .collect::<Vec<_>>();
    for argument in &operation.arguments {
        if argument.tag.ends_with("JOINTDEF") {
            for field in ["bodyIdA", "bodyIdB"] {
                effects.push(IdEffect {
                    action: IdAction::Use,
                    id_kind: "body".to_owned(),
                    source: format!("argument:{}.base.{field}", argument.name),
                    repeated_by: None,
                    condition: None,
                });
            }
        }
    }
    if operation.return_tag != "RET_NONE" {
        effects.push(IdEffect {
            action: IdAction::Create,
            id_kind: return_id_kind(&operation.return_tag)
                .expect("validated return ID tag")
                .to_owned(),
            source: "tail:returned-id".to_owned(),
            repeated_by: None,
            condition: None,
        });
    }
    match operation.opcode {
        0xE0..=0xE4 => effects.push(IdEffect {
            action: IdAction::Use,
            id_kind: "shape".to_owned(),
            source: format!("tail:{}[].shapeId", tail_program(operation)),
            repeated_by: Some("tail.count".to_owned()),
            condition: None,
        }),
        0xE5 => effects.push(IdEffect {
            action: IdAction::Use,
            id_kind: "shape".to_owned(),
            source: "tail:closest-ray-result.shapeId".to_owned(),
            repeated_by: None,
            condition: Some("tail.hit == 1".to_owned()),
        }),
        _ => {}
    }
    effects
}

fn return_id_kind(tag_name: &str) -> Option<&'static str> {
    match tag_name {
        "RET_BODYID" => Some("body"),
        "RET_SHAPEID" => Some("shape"),
        "RET_CHAINID" => Some("chain"),
        "RET_JOINTID" => Some("joint"),
        _ => None,
    }
}

fn id_kind(tag_name: &str) -> Option<&'static str> {
    match tag_name {
        "WORLDID" => Some("world"),
        "BODYID" => Some("body"),
        "SHAPEID" => Some("shape"),
        "CHAINID" => Some("chain"),
        "JOINTID" => Some("joint"),
        _ => None,
    }
}

fn validate_abi(abi: &RecordingAbi) -> Result<()> {
    let mut errors = Vec::new();
    if !matches!(abi.pointer_width, 4 | 8) {
        errors.push("recording ABI pointer width must be 4 or 8".to_owned());
    }
    for (_, abi_type) in NATIVE_POD_TAGS {
        if abi.native_pod_sizes.get(abi_type).copied().unwrap_or(0) == 0 {
            errors.push(format!(
                "recording ABI is missing generated native POD size for {abi_type}"
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::message(errors.join("\n")))
    }
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
            if actual != &wire_opcode(operation) {
                errors.push(format!(
                    "recording opcode 0x{:02X} `{}` has inconsistent tail, semantics, or ID effects",
                    actual.opcode, actual.name
                ));
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

fn validate_stream_grammar(
    operation_names: &[&str],
    grammar: &RecordingStreamGrammar,
) -> Result<()> {
    if operation_names.len() < 3 {
        return Err(Error::message(
            "recording stream must contain its initial state hash, final bounds, and terminal record",
        ));
    }
    if operation_names.first().copied() != Some(grammar.initial_operation.as_str()) {
        return Err(Error::message(format!(
            "recording stream must begin with `{}`",
            grammar.initial_operation
        )));
    }
    if operation_names.get(operation_names.len() - 2).copied()
        != Some(grammar.final_metadata_operation.as_str())
    {
        return Err(Error::message(format!(
            "recording stream must end metadata with `{}`",
            grammar.final_metadata_operation
        )));
    }
    if operation_names.last().copied() != Some(grammar.terminal_operation.as_str()) {
        return Err(Error::message(format!(
            "recording stream must terminate with `{}`",
            grammar.terminal_operation
        )));
    }
    if grammar.terminal_must_be_last
        && operation_names
            .iter()
            .filter(|name| **name == grammar.terminal_operation)
            .count()
            != 1
    {
        return Err(Error::message(format!(
            "recording terminal `{}` must appear exactly once and last",
            grammar.terminal_operation
        )));
    }
    if operation_names
        .iter()
        .filter(|name| **name == grammar.final_metadata_operation)
        .count()
        != 1
    {
        return Err(Error::message(format!(
            "recording final metadata `{}` must appear exactly once",
            grammar.final_metadata_operation
        )));
    }
    Ok(())
}

fn validate_header_and_snapshot(
    bytes: &[u8],
    contract: &RecordingWireContract,
    abi: &RecordingAbi,
) -> Result<usize> {
    let framing = &contract.framing;
    let header = &contract.header;
    if bytes.len() < framing.header_bytes {
        return Err(Error::message("recording is shorter than its fixed header"));
    }
    if read_u32(bytes, header.magic_offset)? != header.magic
        || read_u16(bytes, header.version_major_offset)? != header.version_major
        || read_u16(bytes, header.version_minor_offset)? != header.version_minor
    {
        return Err(Error::message("recording header magic or version mismatch"));
    }
    if read_u32(bytes, header.reserved2_offset)? != 0
        || bytes[header.reserved3_offset] != 0
        || read_u32(bytes, header.reserved1_offset)? != 0
    {
        return Err(Error::message(
            "recording header reserved fields must be zero",
        ));
    }
    let scale = f32::from_bits(read_u32(bytes, header.length_scale_offset)?);
    if !scale.is_finite() || scale <= 0.0 {
        return Err(Error::message(
            "recording length scale must be finite and positive",
        ));
    }
    if bytes[header.pointer_width_offset] != abi.pointer_width {
        return Err(Error::message("recording pointer-width ABI mismatch"));
    }
    if bytes[header.big_endian_offset] != 0 {
        return Err(Error::message("big-endian recordings are unsupported"));
    }
    let validation = bytes[header.validation_enabled_offset];
    if validation > 1 || (validation != 0) != abi.validation_enabled {
        return Err(Error::message("recording validation-mode ABI mismatch"));
    }
    let snapshot_u64 = read_u64(bytes, header.snapshot_size_offset)?;
    let snapshot_bytes = usize::try_from(snapshot_u64)
        .map_err(|_| Error::message("recording snapshot size does not fit this host"))?;
    if snapshot_bytes < contract.snapshot.minimum_bytes {
        return Err(Error::message(
            "recording snapshot is shorter than its identity header",
        ));
    }
    let snapshot_end = framing
        .header_bytes
        .checked_add(snapshot_bytes)
        .ok_or_else(|| Error::message("recording snapshot offset overflow"))?;
    if snapshot_end > bytes.len() {
        return Err(Error::message("recording snapshot extends beyond input"));
    }
    let snapshot = &bytes[framing.header_bytes..snapshot_end];
    let identity = &contract.snapshot;
    if read_u32(snapshot, identity.magic_offset)? != identity.magic
        || read_u32(snapshot, identity.version_offset)? != identity.version
    {
        return Err(Error::message("snapshot magic or version mismatch"));
    }
    if read_u32(snapshot, identity.layout_hash_offset)? != abi.snapshot_layout_hash {
        return Err(Error::message("snapshot private-layout identity mismatch"));
    }
    let flags = read_u32(snapshot, identity.flags_offset)?;
    if flags & !identity.known_flags_mask != 0 {
        return Err(Error::message("snapshot contains unknown flags"));
    }
    let expected_flags = if abi.validation_enabled {
        identity.validation_flag
    } else {
        0
    } | if abi.precision == RecordingPrecision::Double {
        identity.double_precision_flag
    } else {
        0
    };
    if flags != expected_flags {
        return Err(Error::message(
            "snapshot precision or validation flags do not match the selected ABI",
        ));
    }
    Ok(snapshot_bytes)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| Error::message("u16 offset overflow"))?;
    let value = bytes
        .get(offset..end)
        .ok_or_else(|| Error::message("truncated u16 field"))?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| Error::message("u32 offset overflow"))?;
    let value = bytes
        .get(offset..end)
        .ok_or_else(|| Error::message("truncated u32 field"))?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| Error::message("u64 offset overflow"))?;
    let value = bytes
        .get(offset..end)
        .ok_or_else(|| Error::message("truncated u64 field"))?;
    Ok(u64::from_le_bytes(
        value
            .try_into()
            .map_err(|_| Error::message("invalid u64 field width"))?,
    ))
}

struct Interpreter<'a> {
    bytes: &'a [u8],
    cursor: usize,
    abi: &'a RecordingAbi,
    tags: &'a BTreeMap<&'a str, &'a Codec>,
    stack: Vec<String>,
}

impl<'a> Interpreter<'a> {
    fn new(bytes: &'a [u8], abi: &'a RecordingAbi, tags: &'a BTreeMap<&'a str, &'a Codec>) -> Self {
        Self {
            bytes,
            cursor: 0,
            abi,
            tags,
            stack: Vec::new(),
        }
    }

    fn consume_tag(&mut self, tag_name: &str) -> Result<()> {
        if self.stack.iter().any(|entry| entry == tag_name) {
            return Err(Error::message(format!(
                "recursive recording codec tag `{tag_name}`"
            )));
        }
        let codec = self.tags.get(tag_name).copied().ok_or_else(|| {
            Error::message(format!("recording codec is missing tag `{tag_name}`"))
        })?;
        self.stack.push(tag_name.to_owned());
        let result = self.consume(codec);
        self.stack.pop();
        result
    }

    fn consume(&mut self, codec: &Codec) -> Result<()> {
        match codec {
            Codec::Fixed { bytes, boolean } => {
                let slice = self.take(*bytes)?;
                if *boolean && slice != [0] && slice != [1] {
                    return Err(Error::message("recording BOOL must be encoded as 0 or 1"));
                }
                Ok(())
            }
            Codec::Precision {
                single_bytes,
                double_bytes,
            } => self
                .take(match self.abi.precision {
                    RecordingPrecision::Single => *single_bytes,
                    RecordingPrecision::Double => *double_bytes,
                })
                .map(|_| ()),
            Codec::String {
                length_bytes,
                null_sentinel,
                max_bytes,
            } => {
                if *length_bytes != 2 {
                    return Err(Error::message("unsupported recording string length width"));
                }
                let length = u64::from(read_u16(self.bytes, self.cursor)?);
                self.cursor += 2;
                if length == *null_sentinel {
                    return Ok(());
                }
                let length = usize::try_from(length)
                    .map_err(|_| Error::message("recording string length does not fit host"))?;
                if length > *max_bytes {
                    return Err(Error::message("recording string exceeds codec limit"));
                }
                self.take(length).map(|_| ())
            }
            Codec::NativePod { abi_type, .. } => {
                let size = self
                    .abi
                    .native_pod_sizes
                    .get(abi_type)
                    .copied()
                    .ok_or_else(|| {
                        Error::message(format!("missing native POD size for {abi_type}"))
                    })?;
                self.take(size).map(|_| ())
            }
            Codec::Tag { tag } => self.consume_tag(tag),
            Codec::Sequence { steps } => {
                for step in steps {
                    self.consume(step)?;
                }
                Ok(())
            }
            Codec::Counted { count, element } => {
                if count.bytes != 4 || !count.remaining_bytes_bound || !count.checked_multiply {
                    return Err(Error::message("unsupported or unbounded count codec"));
                }
                let raw = read_u32(self.bytes, self.cursor)?;
                self.cursor += 4;
                if count.signed && (raw as i32) < 0 {
                    return Err(Error::message("recording count must not be negative"));
                }
                if raw > i32::MAX as u32 {
                    return Err(Error::message("recording count exceeds i32::MAX"));
                }
                let count_value = usize::try_from(raw)
                    .map_err(|_| Error::message("recording count does not fit host"))?;
                if count_value > count.max_count {
                    return Err(Error::message(format!(
                        "recording count {count_value} exceeds configured limit {}",
                        count.max_count
                    )));
                }
                let minimum = minimum_width(element, self.abi, self.tags, &mut BTreeSet::new())?;
                let required = count_value
                    .checked_mul(minimum)
                    .ok_or_else(|| Error::message("recording count byte-size overflow"))?;
                if required > self.remaining() {
                    return Err(Error::message(
                        "recording counted sequence exceeds remaining payload",
                    ));
                }
                for _ in 0..count_value {
                    self.consume(element)?;
                }
                Ok(())
            }
        }
    }

    fn take(&mut self, bytes: usize) -> Result<&'a [u8]> {
        let end = self
            .cursor
            .checked_add(bytes)
            .ok_or_else(|| Error::message("recording codec cursor overflow"))?;
        let slice = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| Error::message("recording payload is shorter than its codec"))?;
        self.cursor = end;
        Ok(slice)
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.cursor)
    }
}

fn minimum_width(
    codec: &Codec,
    abi: &RecordingAbi,
    tags: &BTreeMap<&str, &Codec>,
    stack: &mut BTreeSet<String>,
) -> Result<usize> {
    match codec {
        Codec::Fixed { bytes, .. } => Ok(*bytes),
        Codec::Precision {
            single_bytes,
            double_bytes,
        } => Ok(match abi.precision {
            RecordingPrecision::Single => *single_bytes,
            RecordingPrecision::Double => *double_bytes,
        }),
        Codec::String { length_bytes, .. } => Ok(*length_bytes),
        Codec::NativePod { abi_type, .. } => abi
            .native_pod_sizes
            .get(abi_type)
            .copied()
            .ok_or_else(|| Error::message(format!("missing native POD size for {abi_type}"))),
        Codec::Tag { tag } => {
            if !stack.insert(tag.clone()) {
                return Err(Error::message(format!("recursive codec tag `{tag}`")));
            }
            let result = minimum_width(
                tags.get(tag.as_str())
                    .copied()
                    .ok_or_else(|| Error::message(format!("missing codec tag `{tag}`")))?,
                abi,
                tags,
                stack,
            );
            stack.remove(tag);
            result
        }
        Codec::Sequence { steps } => steps.iter().try_fold(0_usize, |total, step| {
            total
                .checked_add(minimum_width(step, abi, tags, stack)?)
                .ok_or_else(|| Error::message("recording codec minimum-width overflow"))
        }),
        Codec::Counted { count, .. } => Ok(count.bytes),
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

    fn abi(precision: RecordingPrecision) -> RecordingAbi {
        RecordingAbi {
            precision,
            pointer_width: 8,
            validation_enabled: false,
            snapshot_layout_hash: 0x1234_5678,
            native_pod_sizes: [
                ("b2Circle".to_owned(), 12),
                ("b2Capsule".to_owned(), 20),
                ("b2Segment".to_owned(), 16),
                ("b2Polygon".to_owned(), 144),
                ("b2ChainSegment".to_owned(), 36),
            ]
            .into_iter()
            .collect(),
        }
    }

    fn recording(abi: &RecordingAbi) -> Vec<u8> {
        let mut bytes = vec![0; 32];
        bytes[0..4].copy_from_slice(&0x4352_3242_u32.to_le_bytes());
        bytes[4..6].copy_from_slice(&3_u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&2_u16.to_le_bytes());
        bytes[12..16].copy_from_slice(&1.0_f32.to_bits().to_le_bytes());
        bytes[17] = abi.pointer_width;
        bytes[19] = u8::from(abi.validation_enabled);
        bytes[24..32].copy_from_slice(&16_u64.to_le_bytes());
        bytes.extend_from_slice(&0x3253_4E42_u32.to_le_bytes());
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&abi.snapshot_layout_hash.to_le_bytes());
        let flags = u32::from(abi.validation_enabled)
            | if abi.precision == RecordingPrecision::Double {
                2
            } else {
                0
            };
        bytes.extend_from_slice(&flags.to_le_bytes());
        push_frame(&mut bytes, 0xF1, &[0; 12]);
        bytes
    }

    fn finish_recording(bytes: &mut Vec<u8>) {
        push_frame(bytes, 0xF2, &[0; 16]);
        push_frame(bytes, 0x01, &[0; 4]);
    }

    fn push_frame(bytes: &mut Vec<u8>, opcode: u8, payload: &[u8]) {
        bytes.push(opcode);
        let size = payload.len();
        bytes.extend_from_slice(&[size as u8, (size >> 8) as u8, (size >> 16) as u8]);
        bytes.extend_from_slice(payload);
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
    fn id_effects_cover_nested_joint_ids_return_ids_and_query_tail_ids() {
        let contract = wire_contract(&operations());
        let create = contract
            .opcodes
            .iter()
            .find(|opcode| opcode.opcode == 0x90)
            .expect("joint create opcode");
        assert!(create.id_effects.iter().any(|effect| {
            effect.action == IdAction::Use
                && effect.id_kind == "body"
                && effect.source == "argument:def.base.bodyIdA"
        }));
        assert!(
            create
                .id_effects
                .iter()
                .any(|effect| { effect.action == IdAction::Create && effect.id_kind == "joint" })
        );

        let overlap = contract
            .opcodes
            .iter()
            .find(|opcode| opcode.opcode == 0xE0)
            .expect("overlap opcode");
        assert!(overlap.id_effects.iter().any(|effect| {
            effect.id_kind == "shape" && effect.repeated_by.as_deref() == Some("tail.count")
        }));
        let closest = contract
            .opcodes
            .iter()
            .find(|opcode| opcode.opcode == 0xE5)
            .expect("closest query opcode");
        assert!(closest.id_effects.iter().any(|effect| {
            effect.id_kind == "shape" && effect.condition.as_deref() == Some("tail.hit == 1")
        }));
    }

    #[test]
    fn exact_payload_rejects_cross_frame_short_read_and_trailing_bytes() {
        let operations = operations();
        let contract = wire_contract(&operations);
        let abi = abi(RecordingPrecision::Single);

        let mut valid = recording(&abi);
        push_frame(&mut valid, 0x80, &[0; 12]);
        finish_recording(&mut valid);
        assert_eq!(
            preflight_structure(&valid, &contract, &abi)
                .unwrap()
                .records,
            4
        );

        let mut short = recording(&abi);
        push_frame(&mut short, 0x80, &[]);
        push_frame(&mut short, 0x80, &[0; 12]);
        assert!(
            preflight_structure(&short, &contract, &abi)
                .unwrap_err()
                .to_string()
                .contains("shorter than its codec")
        );

        let mut trailing = recording(&abi);
        push_frame(&mut trailing, 0x80, &[0; 13]);
        assert!(
            preflight_structure(&trailing, &contract, &abi)
                .unwrap_err()
                .to_string()
                .contains("exact payload EOF")
        );
    }

    #[test]
    fn rejects_unknown_opcode_partial_frame_and_invalid_bool() {
        let operations = operations();
        let contract = wire_contract(&operations);
        let abi = abi(RecordingPrecision::Single);

        let mut unknown = recording(&abi);
        push_frame(&mut unknown, 0xFF, &[]);
        assert!(preflight_structure(&unknown, &contract, &abi).is_err());

        let mut partial = recording(&abi);
        partial.extend_from_slice(&[0x80, 0, 0]);
        assert!(preflight_structure(&partial, &contract, &abi).is_err());

        let mut invalid_bool = recording(&abi);
        push_frame(&mut invalid_bool, 0x02, &[0, 0, 0, 0, 2]);
        assert!(
            preflight_structure(&invalid_bool, &contract, &abi)
                .unwrap_err()
                .to_string()
                .contains("BOOL")
        );
    }

    #[test]
    fn rejects_string_count_and_tail_mismatches() {
        let operations = operations();
        let contract = wire_contract(&operations);
        let abi = abi(RecordingPrecision::Single);

        let mut body_payload = vec![0; 4 + 4 + 8 + 8 + 8 + 20];
        body_payload.extend_from_slice(&10_u16.to_le_bytes());
        let mut body = recording(&abi);
        push_frame(&mut body, 0x10, &body_payload);
        assert!(preflight_structure(&body, &contract, &abi).is_err());

        let mut chain_payload = vec![0; 8 + 8];
        chain_payload.extend_from_slice(&u32::MAX.to_le_bytes());
        let mut chain = recording(&abi);
        push_frame(&mut chain, 0x70, &chain_payload);
        assert!(
            preflight_structure(&chain, &contract, &abi)
                .unwrap_err()
                .to_string()
                .contains("count")
        );

        let mut query_payload = vec![0; 4 + 8 + 16 + 16];
        query_payload.extend_from_slice(&1_u32.to_le_bytes());
        let mut query = recording(&abi);
        push_frame(&mut query, 0xE0, &query_payload);
        assert!(
            preflight_structure(&query, &contract, &abi)
                .unwrap_err()
                .to_string()
                .contains("counted sequence")
        );
    }

    #[test]
    fn rejects_shape_proxy_limit_and_precision_width_mismatch() {
        let operations = operations();
        let contract = wire_contract(&operations);
        let single = abi(RecordingPrecision::Single);

        let mut proxy_payload = vec![0; 4 + 8];
        proxy_payload.extend_from_slice(&9_u32.to_le_bytes());
        let mut proxy = recording(&single);
        push_frame(&mut proxy, 0xE1, &proxy_payload);
        assert!(
            preflight_structure(&proxy, &contract, &single)
                .unwrap_err()
                .to_string()
                .contains("configured limit")
        );

        let double = abi(RecordingPrecision::Double);
        let mut wrong_width = recording(&double);
        push_frame(&mut wrong_width, 0x20, &[0; 24]);
        assert!(preflight_structure(&wrong_width, &contract, &double).is_err());
    }

    #[test]
    fn validates_header_and_snapshot_identity_without_claiming_snapshot_safety() {
        let operations = operations();
        let contract = wire_contract(&operations);
        let abi = abi(RecordingPrecision::Single);
        let mut valid = recording(&abi);
        finish_recording(&mut valid);
        let summary =
            preflight_structure(&valid, &contract, &abi).expect("structural-only preflight");
        assert!(summary.snapshot_requires_semantic_validation);

        let mut invalid_scale = valid.clone();
        invalid_scale[12..16].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        assert!(preflight_structure(&invalid_scale, &contract, &abi).is_err());

        for mutate in [17_usize, 18, 19, 32, 36, 40, 44] {
            let mut invalid = valid.clone();
            invalid[mutate] ^= 0xFF;
            assert!(
                preflight_structure(&invalid, &contract, &abi).is_err(),
                "offset {mutate}"
            );
        }
        let mut empty_snapshot = valid.clone();
        empty_snapshot[24..32].copy_from_slice(&0_u64.to_le_bytes());
        assert!(preflight_structure(&empty_snapshot, &contract, &abi).is_err());
    }

    #[test]
    fn structural_preflight_requires_producer_stream_grammar() {
        let operations = operations();
        let contract = wire_contract(&operations);
        let abi = abi(RecordingPrecision::Single);

        let missing_terminal = recording(&abi);
        let error = preflight_structure(&missing_terminal, &contract, &abi)
            .expect_err("unterminated stream must fail");
        assert!(error.to_string().contains("final bounds, and terminal"));

        let mut terminal_then_data = recording(&abi);
        push_frame(&mut terminal_then_data, 0xF2, &[0; 16]);
        push_frame(&mut terminal_then_data, 0x01, &[0; 4]);
        push_frame(&mut terminal_then_data, 0x80, &[0; 12]);
        let error = preflight_structure(&terminal_then_data, &contract, &abi)
            .expect_err("records after terminal must fail");
        assert!(error.to_string().contains("end metadata"));

        let mut duplicate_terminal = recording(&abi);
        push_frame(&mut duplicate_terminal, 0x01, &[0; 4]);
        finish_recording(&mut duplicate_terminal);
        let error = preflight_structure(&duplicate_terminal, &contract, &abi)
            .expect_err("duplicate terminal must fail");
        assert!(error.to_string().contains("exactly once and last"));
    }

    #[test]
    fn structural_preflight_rejects_forged_contract_before_indexing_offsets() {
        let operations = operations();
        let mut contract = wire_contract(&operations);
        contract.header.magic_offset = usize::MAX;
        let abi = abi(RecordingPrecision::Single);
        let mut valid = recording(&abi);
        finish_recording(&mut valid);

        let error = preflight_structure(&valid, &contract, &abi)
            .expect_err("forged offsets must fail as contract drift");
        assert!(
            error
                .to_string()
                .contains("framing or identity layout drifted")
        );
    }
}
