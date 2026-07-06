use boxdd::{
    Aabb, DynamicTree, ShapeProxy, TreeProxyId, TreeRayCastInput, TreeShapeCastInput, Vec2,
};

fn aabb(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Aabb {
    Aabb::new(Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
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
fn ray_cast_and_shape_cast_visit_tree_proxies() {
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

    let shape = ShapeProxy::new([Vec2::ZERO], 0.25).expect("valid shape proxy");
    let mut shape_hits = Vec::new();
    tree.shape_cast(
        TreeShapeCastInput::new(shape, Vec2::new(4.0, 0.0)),
        u64::MAX,
        &mut |_, id, data| {
            shape_hits.push((id, data));
            1.0
        },
    );
    assert!(shape_hits.contains(&(proxy, 7)));
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

    let shape = ShapeProxy::new([Vec2::ZERO], 0.25).expect("valid shape proxy");
    let shape_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tree.shape_cast(
            TreeShapeCastInput::new(shape, Vec2::new(4.0, 0.0)),
            u64::MAX,
            &mut |_, _, _| -> f32 {
                panic!("boom in dynamic tree shape cast");
            },
        );
    }));
    assert!(shape_result.is_err());
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
