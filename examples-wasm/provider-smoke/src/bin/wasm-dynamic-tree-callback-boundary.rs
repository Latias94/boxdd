#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    use boxdd::{Aabb, DynamicTree, TreeBoxCastInput, TreeRayCastInput, Vec2};

    let _foundation = boxdd::Foundation::initialize_default().unwrap();
    let tree = DynamicTree::new().expect("tree creation should succeed");
    let unit_x = Vec2::new(1.0, 0.0);
    let aabb = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0)).unwrap();
    let mut query = |_, _| true;
    tree.query(aabb, u64::MAX, &mut query)
        .expect("valid query should succeed");
    tree.query_all(aabb, &mut query)
        .expect("valid query should succeed");

    let mut ray_cast = |_, _, _| boxdd::TreeCastControl::Continue;
    let ray = TreeRayCastInput::new(Vec2::ZERO, unit_x).unwrap();
    tree.ray_cast(ray, u64::MAX, &mut ray_cast)
        .expect("valid ray cast should succeed");
    let mut box_cast = |_, _, _| boxdd::TreeCastControl::Continue;
    let swept = TreeBoxCastInput::new(aabb, unit_x).unwrap();
    tree.box_cast(swept, u64::MAX, &mut box_cast)
        .expect("valid box cast should succeed");
}
