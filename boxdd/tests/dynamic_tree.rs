use boxdd::{
    Aabb, DynamicTree, Error, TreeBoxCastInput, TreeCastControl, TreeProxyId, TreeRayCastInput,
    TreeStats, Vec2,
};

fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Aabb {
    Aabb::new(Vec2::new(min_x, min_y), Vec2::new(max_x, max_y)).unwrap()
}

fn initialize_foundation() {
    boxdd::Foundation::initialize_default().expect("default foundation should initialize");
}

fn assert_proxy_unchanged(tree: &mut DynamicTree, proxy: TreeProxyId, expected_aabb: Aabb) {
    assert!(tree.contains_proxy(proxy));
    assert_eq!(tree.user_data(proxy), Ok(11));
    assert_eq!(tree.aabb(proxy), Ok(expected_aabb));
    assert_eq!(tree.proxy_count(), Ok(1));
}

#[test]
fn dynamic_tree_value_types_round_trip_raw_abi_fields() {
    let stats = TreeStats::from_raw(boxdd_sys::ffi::b2TreeStats {
        nodeVisits: 13,
        leafVisits: 7,
    })
    .unwrap();
    assert_eq!(stats.node_visits(), 13);
    assert_eq!(stats.leaf_visits(), 7);
    let raw_stats = stats.into_raw();
    assert_eq!(raw_stats.nodeVisits, 13);
    assert_eq!(raw_stats.leafVisits, 7);

    let input = TreeRayCastInput::new([1.0_f32, 2.0], [3.0_f32, 4.0])
        .unwrap()
        .with_max_fraction(0.75)
        .unwrap();
    let raw_input = input.into_raw();
    assert_eq!(raw_input.origin.x, 1.0);
    assert_eq!(raw_input.origin.y, 2.0);
    assert_eq!(raw_input.translation.x, 3.0);
    assert_eq!(raw_input.translation.y, 4.0);
    assert_eq!(raw_input.maxFraction, 0.75);
    assert_eq!(TreeRayCastInput::from_raw(raw_input).unwrap(), input);

    let input = TreeBoxCastInput::new(aabb(-2.0, -1.0, 2.0, 1.0), [3.0_f32, 4.0])
        .unwrap()
        .with_max_fraction(0.5)
        .unwrap();
    let raw_input = input.into_raw();
    assert_eq!(Aabb::from_raw(raw_input.box_).unwrap(), input.aabb());
    assert_eq!(raw_input.translation.x, 3.0);
    assert_eq!(raw_input.translation.y, 4.0);
    assert_eq!(raw_input.maxFraction, 0.5);
    assert_eq!(TreeBoxCastInput::from_raw(raw_input).unwrap(), input);
}

#[test]
fn tree_stats_reject_negative_and_inconsistent_counts() {
    assert!(matches!(
        TreeStats::from_raw(boxdd_sys::ffi::b2TreeStats {
            nodeVisits: -1,
            leafVisits: 0,
        }),
        Err(Error::InvalidArgument {
            operation: "TreeStats::from_raw",
            argument: "node_visits",
            ..
        })
    ));
    assert!(matches!(
        TreeStats::from_raw(boxdd_sys::ffi::b2TreeStats {
            nodeVisits: 2,
            leafVisits: 3,
        }),
        Err(Error::InvalidArgument {
            operation: "TreeStats::from_raw",
            argument: "leaf_visits",
            ..
        })
    ));
}

#[test]
fn dynamic_tree_capacity_is_explicit_and_checked() {
    initialize_foundation();

    let tree = DynamicTree::with_capacity(32).expect("valid capacity should create a tree");
    assert_eq!(tree.proxy_count(), Ok(0));
    tree.validate().expect("new tree should be valid");
    assert_eq!(DynamicTree::DEFAULT_PROXY_CAPACITY, 16);
    const {
        assert!(DynamicTree::MAX_PROXY_CAPACITY >= DynamicTree::DEFAULT_PROXY_CAPACITY);
    }
    assert!(DynamicTree::with_capacity(DynamicTree::MAX_PROXY_CAPACITY + 1).is_err());
}

#[test]
fn query_and_mask_return_matching_proxies() {
    initialize_foundation();

    let mut tree = DynamicTree::new().expect("tree creation should succeed");
    let a = tree
        .create_proxy(aabb(-1.0, -1.0, 1.0, 1.0), 0b01, 10)
        .expect("valid proxy should be created");
    let b = tree
        .create_proxy(aabb(3.0, -1.0, 5.0, 1.0), 0b10, 20)
        .expect("valid proxy should be created");

    let mut hits = Vec::new();
    let stats = tree
        .query(aabb(-2.0, -2.0, 2.0, 2.0), u64::MAX, &mut |id, data| {
            hits.push((id, data));
            true
        })
        .expect("valid query should succeed");

    assert!(stats.leaf_visits() >= 1);
    assert_eq!(hits, vec![(a, 10)]);

    let mut masked = Vec::new();
    tree.query_all(aabb(-10.0, -10.0, 10.0, 10.0), &mut |id, _| {
        masked.push(id);
        true
    })
    .expect("valid query should succeed");
    masked.sort();
    assert_eq!(masked, vec![a, b]);
}

#[test]
fn moving_and_destroying_proxy_updates_tree_state() {
    initialize_foundation();

    let mut tree = DynamicTree::new().expect("tree creation should succeed");
    let proxy = tree
        .create_proxy(aabb(-1.0, -1.0, 1.0, 1.0), u64::MAX, 42)
        .expect("valid proxy should be created");

    let mut before = Vec::new();
    tree.query_all(aabb(-2.0, -2.0, 2.0, 2.0), &mut |id, _| {
        before.push(id);
        true
    })
    .expect("valid query should succeed");
    assert_eq!(before, vec![proxy]);

    tree.move_proxy(proxy, aabb(10.0, 10.0, 12.0, 12.0))
        .expect("valid proxy move should succeed");

    let mut after = Vec::new();
    tree.query_all(aabb(-2.0, -2.0, 2.0, 2.0), &mut |id, _| {
        after.push(id);
        true
    })
    .expect("valid query should succeed");
    assert!(after.is_empty());
    assert_eq!(tree.user_data(proxy), Ok(42));

    tree.destroy_proxy(proxy)
        .expect("live proxy should be destroyed");
    assert!(!tree.contains_proxy(proxy));
    assert!(tree.aabb(proxy).is_err());
    assert!(tree.destroy_proxy(proxy).is_err());
}

#[test]
fn proxy_ids_from_another_tree_are_rejected_without_mutating_the_local_proxy() {
    initialize_foundation();

    let original_aabb = aabb(-1.0, -1.0, 1.0, 1.0);
    let mut foreign_tree = DynamicTree::new().expect("tree creation should succeed");
    let foreign = foreign_tree
        .create_proxy(original_aabb, u64::MAX, 7)
        .expect("valid proxy should be created");
    let mut tree = DynamicTree::new().expect("tree creation should succeed");
    let local = tree
        .create_proxy(original_aabb, u64::MAX, 11)
        .expect("valid proxy should be created");

    assert!(!tree.contains_proxy(foreign));
    assert_eq!(tree.user_data(foreign), Err(Error::WrongTree));
    assert_eq!(tree.aabb(foreign), Err(Error::WrongTree));
    assert_eq!(tree.category_bits(foreign), Err(Error::WrongTree));
    assert_eq!(
        tree.move_proxy(foreign, aabb(3.0, 3.0, 5.0, 5.0)),
        Err(Error::WrongTree)
    );
    assert_eq!(
        tree.enlarge_proxy(foreign, aabb(-2.0, -2.0, 2.0, 2.0)),
        Err(Error::WrongTree)
    );
    assert_eq!(
        tree.replace_category_bits(foreign, 0b10),
        Err(Error::WrongTree)
    );
    assert_eq!(tree.destroy_proxy(foreign), Err(Error::WrongTree));
    assert_proxy_unchanged(&mut tree, local, original_aabb);
}

#[test]
fn recycled_proxy_slots_do_not_revive_destroyed_ids() {
    initialize_foundation();

    let original_aabb = aabb(-1.0, -1.0, 1.0, 1.0);
    let mut tree = DynamicTree::new().expect("tree creation should succeed");
    let stale = tree
        .create_proxy(original_aabb, u64::MAX, 7)
        .expect("valid proxy should be created");
    tree.destroy_proxy(stale)
        .expect("live proxy should be destroyed");
    let live = tree
        .create_proxy(original_aabb, u64::MAX, 11)
        .expect("valid proxy should be created");

    assert_ne!(stale, live);
    let mut callback_ids = Vec::new();
    tree.query_all(original_aabb, &mut |proxy, _| {
        callback_ids.push(proxy);
        true
    })
    .expect("valid query should succeed");
    assert_eq!(callback_ids, vec![live]);
    assert!(!tree.contains_proxy(stale));
    assert_eq!(tree.user_data(stale), Err(Error::InvalidTreeProxyId));
    assert_eq!(tree.aabb(stale), Err(Error::InvalidTreeProxyId));
    assert_eq!(tree.category_bits(stale), Err(Error::InvalidTreeProxyId));
    assert_eq!(
        tree.move_proxy(stale, aabb(3.0, 3.0, 5.0, 5.0)),
        Err(Error::InvalidTreeProxyId)
    );
    assert_eq!(
        tree.enlarge_proxy(stale, aabb(-2.0, -2.0, 2.0, 2.0)),
        Err(Error::InvalidTreeProxyId)
    );
    assert_eq!(
        tree.replace_category_bits(stale, 0b10),
        Err(Error::InvalidTreeProxyId)
    );
    assert_eq!(tree.destroy_proxy(stale), Err(Error::InvalidTreeProxyId));
    assert_proxy_unchanged(&mut tree, live, original_aabb);
}

#[test]
fn dropping_a_tree_does_not_make_its_proxy_valid_in_a_new_tree() {
    initialize_foundation();

    let original_aabb = aabb(-1.0, -1.0, 1.0, 1.0);
    let stale = {
        let mut tree = DynamicTree::new().expect("tree creation should succeed");
        tree.create_proxy(original_aabb, u64::MAX, 7)
            .expect("valid proxy should be created")
    };
    let mut replacement = DynamicTree::new().expect("tree creation should succeed");
    let live = replacement
        .create_proxy(original_aabb, u64::MAX, 11)
        .expect("valid proxy should be created");

    assert_eq!(replacement.user_data(stale), Err(Error::WrongTree));
    assert_proxy_unchanged(&mut replacement, live, original_aabb);
}

#[test]
fn move_and_enlarge_reject_native_precondition_violations() {
    let foundation =
        boxdd::Foundation::initialize_default().expect("default foundation should initialize");
    let mut tree = DynamicTree::new().expect("tree creation should succeed");
    let initial = aabb(-1.0, -1.0, 1.0, 1.0);
    let proxy = tree
        .create_proxy(initial, u64::MAX, 42)
        .expect("valid proxy should be created");
    let contained = aabb(-0.5, -0.5, 0.5, 0.5);
    let huge_factor = if cfg!(feature = "double-precision") {
        1.0e9_f32
    } else {
        1.0e5_f32
    };
    let huge = huge_factor * foundation.config().length_units_per_meter();
    let oversized = aabb(0.0, 0.0, huge, 1.0);

    assert!(tree.move_proxy(proxy, oversized).is_err());
    assert!(tree.enlarge_proxy(proxy, initial).is_err());
    assert!(tree.enlarge_proxy(proxy, contained).is_err());
    assert_eq!(tree.aabb(proxy), Ok(initial));
}

#[test]
fn ray_cast_and_box_cast_visit_tree_proxies() {
    initialize_foundation();

    let mut tree = DynamicTree::new().expect("tree creation should succeed");
    let proxy = tree
        .create_proxy(aabb(0.0, 0.0, 2.0, 2.0), u64::MAX, 7)
        .expect("valid proxy should be created");

    let mut ray_hits = Vec::new();
    tree.ray_cast(
        TreeRayCastInput::new(Vec2::new(-4.0, 1.0), Vec2::new(10.0, 0.0)).unwrap(),
        u64::MAX,
        &mut |input, id, data| {
            ray_hits.push((id, data, input.max_fraction()));
            TreeCastControl::Terminate
        },
    )
    .expect("valid ray cast should succeed");
    assert_eq!(ray_hits.len(), 1);
    assert_eq!(ray_hits[0].0, proxy);
    assert_eq!(ray_hits[0].1, 7);

    let mut box_hits = Vec::new();
    tree.box_cast(
        TreeBoxCastInput::new(aabb(-4.0, 0.5, -3.0, 1.5), Vec2::new(8.0, 0.0)).unwrap(),
        u64::MAX,
        &mut |_, id, data| {
            box_hits.push((id, data));
            TreeCastControl::Continue
        },
    )
    .expect("valid box cast should succeed");
    assert!(box_hits.contains(&(proxy, 7)));
}

#[test]
fn invalid_inputs_are_recoverable() {
    initialize_foundation();

    let tree = DynamicTree::new().expect("tree creation should succeed");
    assert!(Aabb::new([1.0_f32, 1.0], [-1.0_f32, -1.0]).is_err());

    assert!(TreeBoxCastInput::new(aabb(-1.0, -1.0, 1.0, 1.0), [f32::NAN, 0.0]).is_err());
    assert!(
        TreeBoxCastInput::new(aabb(-1.0, -1.0, 1.0, 1.0), Vec2::ZERO)
            .unwrap()
            .with_max_fraction(1.5)
            .is_err()
    );
    assert_eq!(tree.proxy_count(), Ok(0));
}

#[test]
fn dynamic_tree_callback_panics_are_caught_and_resumed() {
    initialize_foundation();

    let mut tree = DynamicTree::new().expect("tree creation should succeed");
    let proxy = tree
        .create_proxy(aabb(0.0, 0.0, 2.0, 2.0), u64::MAX, 7)
        .expect("valid proxy should be created");

    let query_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tree.query_all(aabb(-1.0, -1.0, 3.0, 3.0), &mut |_, _| -> bool {
            panic!("boom in dynamic tree query");
        })
        .expect("valid query should only unwind through its callback");
    }));
    assert!(query_result.is_err());
    assert_tree_query_finds_proxy(&tree, proxy);

    let ray_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tree.ray_cast(
            TreeRayCastInput::new(Vec2::new(-4.0, 1.0), Vec2::new(10.0, 0.0)).unwrap(),
            u64::MAX,
            &mut |_, _, _| -> TreeCastControl {
                panic!("boom in dynamic tree ray cast");
            },
        )
        .expect("valid ray cast should only unwind through its callback");
    }));
    assert!(ray_result.is_err());
    assert_tree_query_finds_proxy(&tree, proxy);

    let box_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tree.box_cast(
            TreeBoxCastInput::new(aabb(-4.0, 0.5, -3.0, 1.5), Vec2::new(8.0, 0.0)).unwrap(),
            u64::MAX,
            &mut |_, _, _| -> TreeCastControl {
                panic!("boom in dynamic tree box cast");
            },
        )
        .expect("valid box cast should only unwind through its callback");
    }));
    assert!(box_result.is_err());
    assert_tree_query_finds_proxy(&tree, proxy);
}

#[test]
fn invalid_dynamic_tree_clip_is_recoverable_and_tree_remains_reusable() {
    initialize_foundation();

    let mut tree = DynamicTree::new().expect("tree creation should succeed");
    let proxy = tree
        .create_proxy(aabb(0.0, 0.0, 2.0, 2.0), u64::MAX, 7)
        .expect("valid proxy should be created");

    for fraction in [f32::NAN, f32::INFINITY, -0.5, 1.5] {
        let result = tree.ray_cast(
            TreeRayCastInput::new(Vec2::new(-4.0, 1.0), Vec2::new(10.0, 0.0)).unwrap(),
            u64::MAX,
            &mut |_, _, _| TreeCastControl::Clip(fraction),
        );
        assert!(
            result.is_err(),
            "invalid clip fraction {fraction} must be rejected"
        );
        assert_tree_query_finds_proxy(&tree, proxy);
    }
}

#[test]
fn dynamic_tree_callbacks_reject_world_api_reentry() {
    let world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let mut tree = DynamicTree::new().expect("tree creation should succeed");
    tree.create_proxy(aabb(0.0, 0.0, 2.0, 2.0), u64::MAX, 7)
        .expect("valid proxy should be created");

    let mut query_error = None;
    tree.query(aabb(-1.0, -1.0, 3.0, 3.0), u64::MAX, &mut |_, _| {
        query_error = Some(world.counters().unwrap_err());
        false
    })
    .expect("valid query should succeed");

    let mut query_all_error = None;
    tree.query_all(aabb(-1.0, -1.0, 3.0, 3.0), &mut |_, _| {
        query_all_error = Some(world.counters().unwrap_err());
        false
    })
    .expect("valid query should succeed");

    let mut ray_error = None;
    tree.ray_cast(
        TreeRayCastInput::new(Vec2::new(-4.0, 1.0), Vec2::new(10.0, 0.0)).unwrap(),
        u64::MAX,
        &mut |_, _, _| {
            ray_error = Some(world.counters().unwrap_err());
            TreeCastControl::Terminate
        },
    )
    .expect("valid ray cast should succeed");

    let mut box_error = None;
    tree.box_cast(
        TreeBoxCastInput::new(aabb(-4.0, 0.5, -3.0, 1.5), Vec2::new(8.0, 0.0)).unwrap(),
        u64::MAX,
        &mut |_, _, _| {
            box_error = Some(world.counters().unwrap_err());
            TreeCastControl::Terminate
        },
    )
    .expect("valid box cast should succeed");

    assert_eq!(query_error, Some(Error::InCallback));
    assert_eq!(query_all_error, Some(Error::InCallback));
    assert_eq!(ray_error, Some(Error::InCallback));
    assert_eq!(box_error, Some(Error::InCallback));
}

fn assert_tree_query_finds_proxy(tree: &DynamicTree, expected: TreeProxyId) {
    let mut hits = Vec::new();
    tree.query_all(aabb(-1.0, -1.0, 3.0, 3.0), &mut |id, data| {
        hits.push((id, data));
        true
    })
    .expect("valid query should succeed");
    assert_eq!(hits, vec![(expected, 7)]);
}
