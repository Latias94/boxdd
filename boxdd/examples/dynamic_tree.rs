use boxdd::{Aabb, DynamicTree, TreeRayCastInput, Vec2};

fn main() {
    let mut tree = DynamicTree::new();
    let player = tree.create_proxy(
        Aabb::from_center_half_extents(Vec2::new(0.0, 0.0), Vec2::new(0.5, 0.5)),
        0b01,
        100,
    );
    let crate_proxy = tree.create_proxy(
        Aabb::from_center_half_extents(Vec2::new(3.0, 0.0), Vec2::new(0.5, 0.5)),
        0b10,
        200,
    );

    println!("created proxies: {:?}, {:?}", player, crate_proxy);

    let mut nearby = Vec::new();
    tree.query(
        Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(2.0, 2.0)),
        u64::MAX,
        &mut |id, user_data| {
            nearby.push((id, user_data));
            true
        },
    );
    println!("nearby proxies: {nearby:?}");

    let mut first_hit = None;
    tree.ray_cast(
        TreeRayCastInput::new(Vec2::new(-2.0, 0.0), Vec2::new(8.0, 0.0)),
        u64::MAX,
        &mut |_, id, user_data| {
            first_hit = Some((id, user_data));
            0.0
        },
    );
    println!("first broad-phase ray hit: {first_hit:?}");
}
