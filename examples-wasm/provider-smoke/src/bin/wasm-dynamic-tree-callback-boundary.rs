#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    use boxdd::{Aabb, DynamicTree, TreeBoxCastInput, TreeRayCastInput, Vec2};

    let tree = DynamicTree::new();
    let unit_x = Vec2::new(1.0, 0.0);
    let aabb = Aabb::from_center_half_extents(Vec2::ZERO, Vec2::new(1.0, 1.0));
    let mut query = |_, _| true;
    let _ = tree.query(aabb, u64::MAX, &mut query);
    let _ = tree.try_query(aabb, u64::MAX, &mut query);
    let _ = tree.query_all(aabb, &mut query);
    let _ = tree.try_query_all(aabb, &mut query);

    let mut ray_cast = |_, _, _| boxdd::TreeCastControl::Continue;
    let ray = TreeRayCastInput::new(Vec2::ZERO, unit_x);
    let _ = tree.ray_cast(ray, u64::MAX, &mut ray_cast);
    let _ = tree.try_ray_cast(ray, u64::MAX, &mut ray_cast);
    let mut box_cast = |_, _, _| boxdd::TreeCastControl::Continue;
    let swept = TreeBoxCastInput::new(aabb, unit_x);
    let _ = tree.box_cast(swept, u64::MAX, &mut box_cast);
    let _ = tree.try_box_cast(swept, u64::MAX, &mut box_cast);
}
