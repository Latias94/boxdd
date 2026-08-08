#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    use boxdd::{Foundation, Position, QueryFilter, Vec2};

    let foundation = Foundation::initialize_default().unwrap();
    let world = foundation.create_world(foundation.world_def()).unwrap();
    let query = world.query().unwrap();
    let filter = QueryFilter::default();
    let translation = Vec2::new(1.0, 0.0);

    let compatibility_hit = query
        .cast_ray_closest(Position::ZERO, translation, filter)
        .unwrap();
    if let Some(hit) = compatibility_hit {
        let _ = (hit.hit, hit.fraction);
    }
    let closest = query
        .cast_ray_closest_with_stats(Position::ZERO, translation, filter)
        .unwrap();
    let _ = (closest.hit, closest.node_visits, closest.leaf_visits);
    let _ = query.cast_mover(
        Position::ZERO,
        Vec2::ZERO,
        Vec2::ZERO,
        0.5,
        translation,
        filter,
    );
}
