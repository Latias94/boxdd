use boxdd::{Aabb, DynamicTree, TreeBoxCastInput, TreeProxyId, TreeRayCastInput, TreeStats, Vec2};

fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Aabb {
    Aabb::new(Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
}

#[test]
fn dynamic_tree_value_types_round_trip_raw_abi_fields() {
    let stats = TreeStats::from_raw(boxdd_sys::ffi::b2TreeStats {
        nodeVisits: 13,
        leafVisits: 7,
    });
    assert_eq!(stats.node_visits, 13);
    assert_eq!(stats.leaf_visits, 7);
    let raw_stats = stats.into_raw();
    assert_eq!(raw_stats.nodeVisits, 13);
    assert_eq!(raw_stats.leafVisits, 7);

    let input = TreeRayCastInput::new([1.0_f32, 2.0], [3.0_f32, 4.0]).with_max_fraction(0.75);
    let raw_input = input.into_raw();
    assert_eq!(raw_input.origin.x, 1.0);
    assert_eq!(raw_input.origin.y, 2.0);
    assert_eq!(raw_input.translation.x, 3.0);
    assert_eq!(raw_input.translation.y, 4.0);
    assert_eq!(raw_input.maxFraction, 0.75);
    assert_eq!(TreeRayCastInput::from_raw(raw_input), input);

    let input =
        TreeBoxCastInput::new(aabb(-2.0, -1.0, 2.0, 1.0), [3.0_f32, 4.0]).with_max_fraction(0.5);
    let raw_input = input.into_raw();
    assert_eq!(Aabb::from_raw(raw_input.box_), input.aabb);
    assert_eq!(raw_input.translation.x, 3.0);
    assert_eq!(raw_input.translation.y, 4.0);
    assert_eq!(raw_input.maxFraction, 0.5);
    assert_eq!(TreeBoxCastInput::from_raw(raw_input), input);
}

#[test]
fn dynamic_tree_capacity_is_explicit_and_checked() {
    let tree = DynamicTree::with_capacity(32);
    assert_eq!(tree.proxy_count(), 0);
    assert_eq!(DynamicTree::DEFAULT_PROXY_CAPACITY, 16);
    const {
        assert!(DynamicTree::MAX_PROXY_CAPACITY >= DynamicTree::DEFAULT_PROXY_CAPACITY);
    }
    assert!(DynamicTree::try_with_capacity(DynamicTree::MAX_PROXY_CAPACITY + 1).is_err());
}

#[test]
fn query_and_mask_return_matching_proxies() {
    let mut tree = DynamicTree::new();
    let a = tree.create_proxy(aabb(-1.0, -1.0, 1.0, 1.0), 0b01, 10);
    let b = tree.create_proxy(aabb(3.0, -1.0, 5.0, 1.0), 0b10, 20);

    let mut hits = Vec::new();
    let stats = tree.query(aabb(-2.0, -2.0, 2.0, 2.0), u64::MAX, &mut |id, data| {
        hits.push((id, data));
        true
    });

    assert!(stats.leaf_visits >= 1);
    assert_eq!(hits, vec![(a, 10)]);

    let mut masked = Vec::new();
    tree.query_all(aabb(-10.0, -10.0, 10.0, 10.0), &mut |id, _| {
        masked.push(id);
        true
    });
    masked.sort();
    assert_eq!(masked, vec![a, b]);
}

#[test]
fn moving_and_destroying_proxy_updates_tree_state() {
    let mut tree = DynamicTree::new();
    let proxy = tree.create_proxy(aabb(-1.0, -1.0, 1.0, 1.0), u64::MAX, 42);

    let mut before = Vec::new();
    tree.query_all(aabb(-2.0, -2.0, 2.0, 2.0), &mut |id, _| {
        before.push(id);
        true
    });
    assert_eq!(before, vec![proxy]);

    tree.move_proxy(proxy, aabb(10.0, 10.0, 12.0, 12.0));

    let mut after = Vec::new();
    tree.query_all(aabb(-2.0, -2.0, 2.0, 2.0), &mut |id, _| {
        after.push(id);
        true
    });
    assert!(after.is_empty());
    assert_eq!(tree.user_data(proxy), 42);

    tree.destroy_proxy(proxy);
    assert!(!tree.contains_proxy(proxy));
    assert!(tree.try_aabb(proxy).is_err());
    assert!(tree.try_destroy_proxy(proxy).is_err());
}

#[test]
fn move_and_enlarge_reject_native_precondition_violations() {
    let mut tree = DynamicTree::new();
    let initial = aabb(-1.0, -1.0, 1.0, 1.0);
    let proxy = tree.create_proxy(initial, u64::MAX, 42);
    let contained = aabb(-0.5, -0.5, 0.5, 0.5);
    let huge_factor = if cfg!(feature = "double-precision") {
        1.0e9_f32
    } else {
        1.0e5_f32
    };
    let huge = huge_factor * boxdd::length_units_per_meter();
    let oversized = aabb(0.0, 0.0, huge, 1.0);

    assert!(tree.try_move_proxy(proxy, oversized).is_err());
    assert!(tree.try_enlarge_proxy(proxy, initial).is_err());
    assert!(tree.try_enlarge_proxy(proxy, contained).is_err());
    assert_eq!(tree.aabb(proxy), initial);

    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tree.move_proxy(proxy, oversized)
        }))
        .is_err()
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tree.enlarge_proxy(proxy, initial)
        }))
        .is_err()
    );
    assert_eq!(tree.aabb(proxy), initial);
}

#[test]
fn ray_cast_and_box_cast_visit_tree_proxies() {
    let mut tree = DynamicTree::new();
    let proxy = tree.create_proxy(aabb(0.0, 0.0, 2.0, 2.0), u64::MAX, 7);

    let mut ray_hits = Vec::new();
    tree.ray_cast(
        TreeRayCastInput::new(Vec2::new(-4.0, 1.0), Vec2::new(10.0, 0.0)),
        u64::MAX,
        &mut |input, id, data| {
            ray_hits.push((id, data, input.max_fraction));
            0.0
        },
    );
    assert_eq!(ray_hits.len(), 1);
    assert_eq!(ray_hits[0].0, proxy);
    assert_eq!(ray_hits[0].1, 7);

    let mut box_hits = Vec::new();
    tree.box_cast(
        TreeBoxCastInput::new(aabb(-4.0, 0.5, -3.0, 1.5), Vec2::new(8.0, 0.0)),
        u64::MAX,
        &mut |_, id, data| {
            box_hits.push((id, data));
            1.0
        },
    );
    assert!(box_hits.contains(&(proxy, 7)));
}

#[test]
fn invalid_inputs_are_recoverable() {
    let mut tree = DynamicTree::new();
    let invalid_aabb = aabb(1.0, 1.0, -1.0, -1.0);
    assert!(tree.try_create_proxy(invalid_aabb, u64::MAX, 0).is_err());
    assert!(
        tree.try_query_all(invalid_aabb, &mut |_: TreeProxyId, _| true)
            .is_err()
    );

    let mut visit = |_: TreeBoxCastInput, _: TreeProxyId, _: u64| 1.0;
    assert!(
        tree.try_box_cast(
            TreeBoxCastInput::new(invalid_aabb, Vec2::ZERO),
            u64::MAX,
            &mut visit,
        )
        .is_err()
    );
    assert!(
        tree.try_box_cast(
            TreeBoxCastInput::new(aabb(-1.0, -1.0, 1.0, 1.0), [f32::NAN, 0.0]),
            u64::MAX,
            &mut visit,
        )
        .is_err()
    );
    assert!(
        tree.try_box_cast(
            TreeBoxCastInput::new(aabb(-1.0, -1.0, 1.0, 1.0), Vec2::ZERO).with_max_fraction(1.5),
            u64::MAX,
            &mut visit,
        )
        .is_err()
    );
}

#[test]
fn dynamic_tree_callback_panics_are_caught_and_resumed() {
    let mut tree = DynamicTree::new();
    let proxy = tree.create_proxy(aabb(0.0, 0.0, 2.0, 2.0), u64::MAX, 7);

    let query_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tree.query_all(aabb(-1.0, -1.0, 3.0, 3.0), &mut |_, _| -> bool {
            panic!("boom in dynamic tree query");
        });
    }));
    assert!(query_result.is_err());
    assert_tree_query_finds_proxy(&tree, proxy);

    let ray_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tree.ray_cast(
            TreeRayCastInput::new(Vec2::new(-4.0, 1.0), Vec2::new(10.0, 0.0)),
            u64::MAX,
            &mut |_, _, _| -> f32 {
                panic!("boom in dynamic tree ray cast");
            },
        );
    }));
    assert!(ray_result.is_err());
    assert_tree_query_finds_proxy(&tree, proxy);

    let box_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tree.box_cast(
            TreeBoxCastInput::new(aabb(-4.0, 0.5, -3.0, 1.5), Vec2::new(8.0, 0.0)),
            u64::MAX,
            &mut |_, _, _| -> f32 {
                panic!("boom in dynamic tree box cast");
            },
        );
    }));
    assert!(box_result.is_err());
    assert_tree_query_finds_proxy(&tree, proxy);
}

fn assert_tree_query_finds_proxy(tree: &DynamicTree, expected: TreeProxyId) {
    let mut hits = Vec::new();
    tree.query_all(aabb(-1.0, -1.0, 3.0, 3.0), &mut |id, data| {
        hits.push((id, data));
        true
    });
    assert_eq!(hits, vec![(expected, 7)]);
}
