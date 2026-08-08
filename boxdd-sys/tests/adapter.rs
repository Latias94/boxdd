use std::mem::{offset_of, size_of, size_of_val};

use boxdd_sys::adapter::{
    self, SNAPSHOT_ABI_MISMATCH, SNAPSHOT_BAD_HEADER, SNAPSHOT_BUFFER_TOO_SMALL,
    SNAPSHOT_ENTRY_BODY, SNAPSHOT_ENTRY_LIVE, SNAPSHOT_ENTRY_SHAPE, SNAPSHOT_INVALID_REFERENCE,
    SNAPSHOT_INVALID_VALUE, SNAPSHOT_LIMIT_EXCEEDED, SNAPSHOT_OK, SNAPSHOT_TRAILING_BYTES,
    SNAPSHOT_TRUNCATED, SnapshotEntry, SnapshotFacts, SnapshotLimits, SnapshotValidationError,
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

fn populated_snapshot_with_shapes(shape_count: usize) -> (Vec<u8>, i32, Vec<i32>) {
    assert!(shape_count > 0);
    // SAFETY: each definition is initialized by Box2D, all created ids belong to this local world,
    // and the guard destroys that world only after its snapshot bytes have been copied.
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
        let shapes = (0..shape_count)
            .map(|index| {
                let circle = ffi::b2Circle {
                    center: ffi::b2Vec2 {
                        x: index as f32 * 2.0,
                        y: 0.0,
                    },
                    radius: 0.5,
                };
                ffi::b2CreateCircleShape(body, &shape_def, &circle).index1 - 1
            })
            .collect();
        ffi::b2World_Step(world.0, 1.0 / 60.0, 4);

        let required = ffi::b2World_Snapshot(world.0, std::ptr::null_mut(), 0);
        assert!(required > 0);
        let mut bytes = vec![0; required as usize];
        assert_eq!(
            ffi::b2World_Snapshot(world.0, bytes.as_mut_ptr(), required),
            required
        );
        (bytes, body.index1 - 1, shapes)
    }
}

fn populated_snapshot() -> (Vec<u8>, i32, i32) {
    let (bytes, body, shapes) = populated_snapshot_with_shapes(1);
    (bytes, body, shapes[0])
}

#[derive(Clone, Copy, Debug)]
struct SerializedTree {
    header: usize,
    root: i32,
    node_count: i32,
    capacity: i32,
    free_list: i32,
    proxy_count: i32,
}

fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_ne_bytes(bytes[offset..offset + size_of::<i32>()].try_into().unwrap())
}

fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + size_of::<i32>()].copy_from_slice(&value.to_ne_bytes());
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_ne_bytes(bytes[offset..offset + size_of::<u16>()].try_into().unwrap())
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + size_of::<u16>()].copy_from_slice(&value.to_ne_bytes());
}

fn serialized_trees(bytes: &[u8]) -> [SerializedTree; 3] {
    let locating_limits = SnapshotLimits {
        max_tree_nodes: 1,
        ..SnapshotLimits::default()
    };
    let mut facts = SnapshotFacts::default();
    let mut ignored_entries = 0usize;
    // SAFETY: the input image and output facts are live for the call. Rejecting the first tree by
    // capacity leaves consumed_bytes immediately after its five serialized scalar fields.
    let status = unsafe {
        adapter::boxddSnapshot_Validate(
            bytes.as_ptr(),
            bytes.len(),
            &locating_limits,
            &mut facts,
            std::ptr::null_mut(),
            0,
            &mut ignored_entries,
        )
    };
    assert_eq!(status, SNAPSHOT_INVALID_VALUE);

    let scalar_bytes = 5 * size_of::<i32>();
    let first_header = usize::try_from(facts.consumed_bytes)
        .unwrap()
        .checked_sub(scalar_bytes)
        .expect("tree header precedes the consumed cursor");
    let mut next_header = first_header;
    std::array::from_fn(|_| {
        let tree = SerializedTree {
            header: next_header,
            root: read_i32(bytes, next_header),
            node_count: read_i32(bytes, next_header + size_of::<i32>()),
            capacity: read_i32(bytes, next_header + 2 * size_of::<i32>()),
            free_list: read_i32(bytes, next_header + 3 * size_of::<i32>()),
            proxy_count: read_i32(bytes, next_header + 4 * size_of::<i32>()),
        };
        assert!(tree.capacity >= 0);
        next_header += scalar_bytes + tree.capacity as usize * size_of::<ffi::b2TreeNode>();
        tree
    })
}

fn tree_node_offset(tree: SerializedTree, node: i32) -> usize {
    assert!((0..tree.capacity).contains(&node));
    tree.header + 5 * size_of::<i32>() + node as usize * size_of::<ffi::b2TreeNode>()
}

fn tree_flags(bytes: &[u8], tree: SerializedTree, node: i32) -> u16 {
    read_u16(
        bytes,
        tree_node_offset(tree, node) + offset_of!(ffi::b2TreeNode, flags),
    )
}

fn tree_children(bytes: &[u8], tree: SerializedTree, node: i32) -> (i32, i32) {
    let children = tree_node_offset(tree, node) + offset_of!(ffi::b2TreeNode, __bindgen_anon_1);
    (
        read_i32(bytes, children),
        read_i32(bytes, children + size_of::<i32>()),
    )
}

fn write_tree_child(bytes: &mut [u8], tree: SerializedTree, node: i32, child: usize, value: i32) {
    assert!(child < 2);
    let children = tree_node_offset(tree, node) + offset_of!(ffi::b2TreeNode, __bindgen_anon_1);
    write_i32(bytes, children + child * size_of::<i32>(), value);
}

fn write_tree_parent(bytes: &mut [u8], tree: SerializedTree, node: i32, parent: i32) {
    let offset = tree_node_offset(tree, node) + offset_of!(ffi::b2TreeNode, __bindgen_anon_2);
    write_i32(bytes, offset, parent);
}

fn assert_invalid_tree_snapshot(bytes: &[u8], mutation: &str) {
    assert_eq!(
        adapter::validate_snapshot(bytes, &SnapshotLimits::default()).unwrap_err(),
        SnapshotValidationError::Status(SNAPSHOT_INVALID_REFERENCE),
        "mutation unexpectedly passed: {mutation}"
    );
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
fn validator_rejects_malformed_dynamic_tree_topology() {
    let (bytes, _, _) = populated_snapshot_with_shapes(3);
    let trees = serialized_trees(&bytes);
    let tree = *trees
        .iter()
        .find(|tree| tree.proxy_count == 3)
        .expect("dynamic tree with three proxies");
    assert_eq!(tree.node_count, 5);
    assert!((0..tree.capacity).contains(&tree.root));
    assert!((0..tree.capacity).contains(&tree.free_list));

    let allocated = ffi::b2TreeNodeFlags_b2_allocatedNode as u16;
    let leaf = ffi::b2TreeNodeFlags_b2_leafNode as u16;
    assert_ne!(tree_flags(&bytes, tree, tree.root) & allocated, 0);
    assert_eq!(tree_flags(&bytes, tree, tree.free_list) & allocated, 0);

    let root_children = tree_children(&bytes, tree, tree.root);
    let (branch, root_leaf, root_leaf_slot) = match (
        tree_flags(&bytes, tree, root_children.0) & leaf != 0,
        tree_flags(&bytes, tree, root_children.1) & leaf != 0,
    ) {
        (false, true) => (root_children.0, root_children.1, 1),
        (true, false) => (root_children.1, root_children.0, 0),
        topology => panic!("unexpected three-proxy root topology: {topology:?}"),
    };
    let branch_child = tree_children(&bytes, tree, branch).0;

    let mut free_root = bytes.clone();
    write_i32(&mut free_root, tree.header, tree.free_list);
    write_tree_parent(&mut free_root, tree, tree.root, tree.free_list);
    assert_invalid_tree_snapshot(&free_root, "free root");

    let mut free_child = bytes.clone();
    write_tree_child(
        &mut free_child,
        tree,
        tree.root,
        root_leaf_slot,
        tree.free_list,
    );
    assert_invalid_tree_snapshot(&free_child, "free child");

    let mut self_child = bytes.clone();
    write_tree_child(&mut self_child, tree, tree.root, root_leaf_slot, tree.root);
    assert_invalid_tree_snapshot(&self_child, "self child");

    let mut mismatched_parent = bytes.clone();
    write_tree_parent(&mut mismatched_parent, tree, root_leaf, branch);
    assert_invalid_tree_snapshot(&mismatched_parent, "parent does not own child");

    let mut duplicate_and_unreachable = bytes.clone();
    write_tree_child(
        &mut duplicate_and_unreachable,
        tree,
        tree.root,
        root_leaf_slot,
        branch_child,
    );
    assert_invalid_tree_snapshot(
        &duplicate_and_unreachable,
        "duplicate child leaves an allocated node unreachable",
    );

    for (node, mutation) in [(root_leaf, "leaf height"), (tree.root, "internal height")] {
        let mut invalid_height = bytes.clone();
        let height = tree_node_offset(tree, node) + offset_of!(ffi::b2TreeNode, height);
        write_u16(&mut invalid_height, height, u16::from(node == root_leaf));
        assert_eq!(
            adapter::validate_snapshot(&invalid_height, &SnapshotLimits::default()).unwrap_err(),
            SnapshotValidationError::Status(SNAPSHOT_INVALID_VALUE),
            "mutation unexpectedly passed: {mutation}"
        );
    }
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

#[test]
fn validator_rejects_a_truncated_pair_set_without_reading_past_the_image() {
    let (bytes, _, _) = populated_snapshot();
    let locating_limits = SnapshotLimits {
        max_hash_capacity: 1,
        ..SnapshotLimits::default()
    };
    let mut pair_set_facts = SnapshotFacts::default();
    let mut ignored_entries = 0usize;
    // SAFETY: the input image and output facts are live for the call. A null entries pointer is
    // the documented sizing form and lets the restrictive limit locate the pair-set payload.
    let locating_status = unsafe {
        adapter::boxddSnapshot_Validate(
            bytes.as_ptr(),
            bytes.len(),
            &locating_limits,
            &mut pair_set_facts,
            std::ptr::null_mut(),
            0,
            &mut ignored_entries,
        )
    };
    assert_eq!(locating_status, SNAPSHOT_INVALID_VALUE);
    let pair_set_payload_offset = usize::try_from(pair_set_facts.consumed_bytes).unwrap();
    assert!(pair_set_payload_offset < bytes.len());
    let truncated = &bytes[..pair_set_payload_offset];

    let limits = SnapshotLimits::default();
    let mut facts = SnapshotFacts::default();
    let mut required_entries = 0usize;
    // SAFETY: all pointers describe live caller-owned storage. The truncated slice ends exactly
    // before the pair-set payload whose declared capacity remains in the image.
    let sizing_status = unsafe {
        adapter::boxddSnapshot_Validate(
            truncated.as_ptr(),
            truncated.len(),
            &limits,
            &mut facts,
            std::ptr::null_mut(),
            0,
            &mut required_entries,
        )
    };
    assert_eq!(sizing_status, SNAPSHOT_TRUNCATED);

    let valid = adapter::validate_snapshot(&bytes, &limits).unwrap();
    let mut entries = vec![SnapshotEntry::default(); valid.entries.len()];
    // SAFETY: the output slice is writable and accurately described by its capacity. Validation
    // must stop at the truncated pair-set payload before reading or populating later state.
    let populated_status = unsafe {
        adapter::boxddSnapshot_Validate(
            truncated.as_ptr(),
            truncated.len(),
            &limits,
            &mut facts,
            entries.as_mut_ptr(),
            entries.len(),
            &mut required_entries,
        )
    };
    assert_eq!(populated_status, SNAPSHOT_TRUNCATED);
}
