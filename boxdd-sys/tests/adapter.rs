#[path = "support/c_call_graph.rs"]
mod c_call_graph;

use std::collections::BTreeSet;
use std::fs;
use std::mem::{size_of, size_of_val};
use std::path::PathBuf;

use boxdd_sys::adapter::{
    self, SNAPSHOT_ABI_MISMATCH, SNAPSHOT_BAD_HEADER, SNAPSHOT_BUFFER_TOO_SMALL,
    SNAPSHOT_ENTRY_BODY, SNAPSHOT_ENTRY_LIVE, SNAPSHOT_ENTRY_SHAPE, SNAPSHOT_LIMIT_EXCEEDED,
    SNAPSHOT_OK, SNAPSHOT_TRAILING_BYTES, SNAPSHOT_TRUNCATED, SnapshotEntry, SnapshotFacts,
    SnapshotLimits, SnapshotValidationError,
};
use boxdd_sys::ffi;

struct TestWorld(ffi::b2WorldId);

impl Drop for TestWorld {
    fn drop(&mut self) {
        // SAFETY: this guard exclusively owns a live test world.
        unsafe { ffi::b2DestroyWorld(self.0) };
    }
}

fn c_string(bytes: &[u8]) -> &str {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).expect("adapter identities are ASCII")
}

fn populated_snapshot() -> (Vec<u8>, i32, i32) {
    // SAFETY: definitions are initialized by Box2D and all pointed-to values remain alive for
    // each call. The world is destroyed by the guard after the snapshot bytes are copied.
    unsafe {
        let mut world_def = ffi::b2DefaultWorldDef();
        world_def.gravity = ffi::b2Vec2 { x: 0.0, y: -10.0 };
        let world = TestWorld(ffi::b2CreateWorld(&world_def));
        assert!(ffi::b2World_IsValid(world.0));

        let mut body_def = ffi::b2DefaultBodyDef();
        body_def.type_ = ffi::b2BodyType_b2_dynamicBody;
        let body = ffi::b2CreateBody(world.0, &body_def);
        let discarded = ffi::b2CreateBody(world.0, &body_def);
        ffi::b2DestroyBody(discarded);

        let shape_def = ffi::b2DefaultShapeDef();
        let circle = ffi::b2Circle {
            center: ffi::b2Vec2 { x: 0.0, y: 0.0 },
            radius: 0.5,
        };
        let shape = ffi::b2CreateCircleShape(body, &shape_def, &circle);
        ffi::b2World_Step(world.0, 1.0 / 60.0, 4);

        let required = ffi::b2World_Snapshot(world.0, std::ptr::null_mut(), 0);
        assert!(required > 0);
        let mut bytes = vec![0; required as usize];
        assert_eq!(
            ffi::b2World_Snapshot(world.0, bytes.as_mut_ptr(), required),
            required
        );
        (bytes, body.index1 - 1, shape.index1 - 1)
    }
}

#[test]
fn snapshot_byte_preflight_has_a_pure_native_call_closure() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let adapter_path = "native/boxdd_adapter.c";
    let validator_path = "native/boxdd_snapshot_validate.c";
    let adapter_source =
        fs::read_to_string(manifest_dir.join(adapter_path)).expect("read adapter source");
    let validator_source =
        fs::read_to_string(manifest_dir.join(validator_path)).expect("read validator source");

    let report = c_call_graph::audit_pure_call_closure(
        &[
            (adapter_path, adapter_source.as_str()),
            (validator_path, validator_source.as_str()),
        ],
        &["boxddAdapter_GetIdentity", "boxddSnapshot_Validate"],
        &["b2IsDoublePrecision"],
        &["isfinite", "memcpy", "memset", "strncpy"],
        &[
            "BOXDD_ABI_FIELD",
            "BOXDD_ABI_TYPE",
            "BOXDD_ABI_VALUE",
            "BOXDD_LAYOUT_VALUE",
        ],
    )
    .expect("snapshot byte preflight must remain independent of mutable Box2D state");

    assert_eq!(
        report.native_calls,
        BTreeSet::from(["b2IsDoublePrecision".to_owned()])
    );
    assert_eq!(
        report.library_calls,
        BTreeSet::from([
            "isfinite".to_owned(),
            "memcpy".to_owned(),
            "memset".to_owned(),
            "strncpy".to_owned(),
        ])
    );
    assert!(
        report
            .reachable_functions
            .contains("boxddAdapter_GetSnapshotLayoutHash")
    );
    assert!(report.reachable_functions.contains("boxddParseImage"));
}

#[test]
fn adapter_abi_inputs_are_constant_only_macro_invocations() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let adapter_path = "native/boxdd_adapter.c";
    let validator_path = "native/boxdd_snapshot_validate.c";
    let adapter_source =
        fs::read_to_string(manifest_dir.join(adapter_path)).expect("read adapter source");
    let validator_source =
        fs::read_to_string(manifest_dir.join(validator_path)).expect("read validator source");
    let include_report = c_call_graph::audit_include_inventory(
        &[
            (adapter_path, adapter_source.as_str()),
            (validator_path, validator_source.as_str()),
        ],
        &[
            (adapter_path, "boxdd_adapter.h"),
            (adapter_path, "bitset.h"),
            (adapter_path, "body.h"),
            (adapter_path, "broad_phase.h"),
            (adapter_path, "constraint_graph.h"),
            (adapter_path, "contact.h"),
            (adapter_path, "id_pool.h"),
            (adapter_path, "island.h"),
            (adapter_path, "joint.h"),
            (adapter_path, "recording.h"),
            (adapter_path, "sensor.h"),
            (adapter_path, "shape.h"),
            (adapter_path, "solver_set.h"),
            (adapter_path, "table.h"),
            (adapter_path, "box2d/box2d.h"),
            (adapter_path, "box2d/collision.h"),
            (adapter_path, "box2d/types.h"),
            (validator_path, "boxdd_adapter.h"),
            (validator_path, "body.h"),
            (validator_path, "broad_phase.h"),
            (validator_path, "constraint_graph.h"),
            (validator_path, "contact.h"),
            (validator_path, "island.h"),
            (validator_path, "joint.h"),
            (validator_path, "sensor.h"),
            (validator_path, "shape.h"),
            (validator_path, "solver_set.h"),
            (validator_path, "table.h"),
            (validator_path, "box2d/collision.h"),
            (validator_path, "box2d/types.h"),
        ],
        &[
            (adapter_path, "stddef.h"),
            (adapter_path, "string.h"),
            (validator_path, "limits.h"),
            (validator_path, "math.h"),
            (validator_path, "stddef.h"),
            (validator_path, "string.h"),
        ],
        &[
            (adapter_path, "boxdd_private_abi.inl"),
            (adapter_path, "boxdd_snapshot_layout.inl"),
        ],
    )
    .expect("adapter include inventory must be exact");
    assert_eq!(
        include_report.body_quoted,
        BTreeSet::from([
            (adapter_path.to_owned(), "boxdd_private_abi.inl".to_owned()),
            (
                adapter_path.to_owned(),
                "boxdd_snapshot_layout.inl".to_owned(),
            ),
        ])
    );

    for (source, target) in include_report.body_quoted {
        assert_eq!(source, adapter_path);
        let relative_path = format!("native/{target}");
        let contents = fs::read_to_string(manifest_dir.join(&relative_path))
            .unwrap_or_else(|error| panic!("read audited include {relative_path}: {error}"));
        let report = match target.as_str() {
            "boxdd_private_abi.inl" => c_call_graph::audit_constant_macro_invocations(
                &relative_path,
                &contents,
                &["BOXDD_ABI_TYPE", "BOXDD_ABI_FIELD", "BOXDD_ABI_VALUE"],
                &["sizeof", "_Alignof", "offsetof"],
            ),
            "boxdd_snapshot_layout.inl" => c_call_graph::audit_constant_macro_invocations(
                &relative_path,
                &contents,
                &["BOXDD_LAYOUT_VALUE"],
                &["sizeof", "_Alignof", "offsetof"],
            ),
            _ => unreachable!("include inventory admitted an unaudited function-body include"),
        }
        .unwrap_or_else(|error| {
            panic!("audited include {relative_path} is not constant-only: {error}")
        });

        let expected_macros = match target.as_str() {
            "boxdd_private_abi.inl" => BTreeSet::from([
                "BOXDD_ABI_FIELD".to_owned(),
                "BOXDD_ABI_TYPE".to_owned(),
                "BOXDD_ABI_VALUE".to_owned(),
            ]),
            "boxdd_snapshot_layout.inl" => BTreeSet::from(["BOXDD_LAYOUT_VALUE".to_owned()]),
            _ => unreachable!(),
        };
        assert_eq!(report.macros, expected_macros);
    }
}

#[test]
fn runtime_identity_is_complete_and_versioned() {
    let identity = adapter::runtime_identity().expect("linked adapter identity");
    assert_eq!(identity.abi_version, adapter::ADAPTER_ABI_VERSION);
    assert_eq!(identity.struct_size as usize, size_of_val(&identity));
    assert_eq!(identity.pointer_width as usize, size_of::<usize>());
    assert_eq!(
        identity.little_endian,
        u8::from(cfg!(target_endian = "little"))
    );
    assert_eq!(
        identity.double_precision != 0,
        cfg!(feature = "double-precision")
    );
    assert_eq!(c_string(&identity.upstream_sha), boxdd_sys::UPSTREAM_SHA);
    assert_eq!(c_string(&identity.target_abi), boxdd_sys::TARGET_ABI);
    assert_eq!(
        c_string(&identity.adapter_source_sha256),
        boxdd_sys::ADAPTER_SOURCE_SHA256
    );
    assert_eq!(
        c_string(&identity.effective_source_sha256),
        boxdd_sys::EFFECTIVE_SOURCE_SHA256
    );
    // SAFETY: this exported fixed-width identity symbol is immutable for the process lifetime.
    let effective_source_symbol = unsafe { adapter::boxddEffectiveSourceSha256 };
    assert_eq!(effective_source_symbol[64], 0);
    assert!(
        effective_source_symbol[..64]
            .iter()
            .all(|byte| { byte.is_ascii_digit() || (b'a'..=b'f').contains(byte) })
    );
    assert_eq!(
        c_string(&effective_source_symbol),
        boxdd_sys::EFFECTIVE_SOURCE_SHA256
    );
    assert_eq!(
        c_string(&identity.recording_contract_blake3),
        boxdd_sys::RECORDING_CONTRACT_BLAKE3
    );
    assert_eq!(identity.private_abi_hash, boxdd_sys::PRIVATE_ABI_HASH);
    assert_eq!(
        identity.snapshot_layout_hash,
        boxdd_sys::SNAPSHOT_LAYOUT_HASH
    );
    assert_eq!(
        identity.snapshot_layout_hash,
        // SAFETY: the function has no inputs or mutable state.
        unsafe { adapter::boxddAdapter_GetSnapshotLayoutHash() }
    );
}

#[test]
fn validator_accepts_a_real_snapshot_and_returns_canonical_entries() {
    let (bytes, body_index, shape_index) = populated_snapshot();
    let limits = SnapshotLimits::default();

    let mut facts = SnapshotFacts::default();
    let mut required_entries = 0usize;
    // SAFETY: all pointers describe live caller-owned storage; a null entries pointer requests the
    // documented sizing pass.
    let sizing_status = unsafe {
        adapter::boxddSnapshot_Validate(
            bytes.as_ptr(),
            bytes.len(),
            &limits,
            &mut facts,
            std::ptr::null_mut(),
            0,
            &mut required_entries,
        )
    };
    assert_eq!(sizing_status, SNAPSHOT_BUFFER_TOO_SMALL);
    assert_eq!(required_entries as u64, facts.required_entries);

    let validated = adapter::validate_snapshot(&bytes, &limits).expect("valid snapshot");
    assert_eq!(validated.facts.image_bytes, bytes.len() as u64);
    assert_eq!(validated.facts.consumed_bytes, bytes.len() as u64);
    assert_eq!(validated.entries.len(), required_entries);
    assert!(validated.facts.pool_free[0] >= 1);

    let body = validated
        .entries
        .iter()
        .find(|entry| entry.kind == SNAPSHOT_ENTRY_BODY && entry.index == body_index)
        .expect("live body entry");
    assert_ne!(body.flags & SNAPSHOT_ENTRY_LIVE, 0);
    let shape = validated
        .entries
        .iter()
        .find(|entry| entry.kind == SNAPSHOT_ENTRY_SHAPE && entry.index == shape_index)
        .expect("live shape entry");
    assert_ne!(shape.flags & SNAPSHOT_ENTRY_LIVE, 0);
    assert_eq!(shape.owner_a, body_index);
    assert_eq!(shape.owner_b, -1);
}

#[test]
fn validator_fails_closed_for_boundaries_and_corruption() {
    let (bytes, _, _) = populated_snapshot();

    let too_small = SnapshotLimits {
        max_image_bytes: bytes.len() as u64 - 1,
        ..SnapshotLimits::default()
    };
    assert_eq!(
        adapter::validate_snapshot(&bytes, &too_small).unwrap_err(),
        SnapshotValidationError::Status(SNAPSHOT_LIMIT_EXCEEDED)
    );

    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        adapter::validate_snapshot(&trailing, &SnapshotLimits::default()).unwrap_err(),
        SnapshotValidationError::Status(SNAPSHOT_TRAILING_BYTES)
    );

    assert_eq!(
        adapter::validate_snapshot(&bytes[..bytes.len() - 1], &SnapshotLimits::default())
            .unwrap_err(),
        SnapshotValidationError::Status(SNAPSHOT_TRUNCATED)
    );

    let mut bad_magic = bytes.clone();
    bad_magic[0] ^= 0xff;
    assert_eq!(
        adapter::validate_snapshot(&bad_magic, &SnapshotLimits::default()).unwrap_err(),
        SnapshotValidationError::Status(SNAPSHOT_BAD_HEADER)
    );

    let mut bad_layout = bytes;
    bad_layout[8] ^= 0xff;
    assert_eq!(
        adapter::validate_snapshot(&bad_layout, &SnapshotLimits::default()).unwrap_err(),
        SnapshotValidationError::Status(SNAPSHOT_ABI_MISMATCH)
    );
}

#[test]
fn validator_rejects_an_undersized_entry_buffer_without_writing_past_it() {
    let (bytes, _, _) = populated_snapshot();
    let limits = SnapshotLimits::default();
    let mut facts = SnapshotFacts::default();
    let mut required_entries = 0usize;
    let mut sentinel = SnapshotEntry {
        flags: 0xfeed_beef,
        ..SnapshotEntry::default()
    };
    // SAFETY: the one-element output is accurately described by entry_capacity. The validator
    // must reject it before attempting to populate the canonical table.
    let status = unsafe {
        adapter::boxddSnapshot_Validate(
            bytes.as_ptr(),
            bytes.len(),
            &limits,
            &mut facts,
            &mut sentinel,
            1,
            &mut required_entries,
        )
    };
    assert_eq!(status, SNAPSHOT_BUFFER_TOO_SMALL);
    assert!(required_entries > 1);
    assert_eq!(sentinel.flags, 0xfeed_beef);
    assert_ne!(status, SNAPSHOT_OK);
}
