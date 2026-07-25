#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    use boxdd::{Position, QueryFilter, RecordingCapacity, Vec2, World, WorldDef};

    let mut world = World::new(WorldDef::default()).unwrap();
    let filter = QueryFilter::default();
    let translation = Vec2::new(1.0, 0.0);

    let _ = world.cast_ray_closest(Position::ZERO, translation, filter);
    let _ = world.try_cast_ray_closest(Position::ZERO, translation, filter);
    let _ = world.cast_mover(
        Position::ZERO,
        Vec2::ZERO,
        Vec2::ZERO,
        0.5,
        translation,
        filter,
    );
    let _ = world.try_cast_mover(
        Position::ZERO,
        Vec2::ZERO,
        Vec2::ZERO,
        0.5,
        translation,
        filter,
    );

    {
        let handle = world.handle();
        let _ = handle.cast_ray_closest(Position::ZERO, translation, filter);
        let _ = handle.try_cast_ray_closest(Position::ZERO, translation, filter);
        let _ = handle.cast_mover(
            Position::ZERO,
            Vec2::ZERO,
            Vec2::ZERO,
            0.5,
            translation,
            filter,
        );
        let _ = handle.try_cast_mover(
            Position::ZERO,
            Vec2::ZERO,
            Vec2::ZERO,
            0.5,
            translation,
            filter,
        );
    }

    let session = world.start_recording(RecordingCapacity::default());
    let _ = session.cast_ray_closest(Position::ZERO, translation, filter);
    let _ = session.try_cast_ray_closest(Position::ZERO, translation, filter);
    let _ = session.cast_mover(
        Position::ZERO,
        Vec2::ZERO,
        Vec2::ZERO,
        0.5,
        translation,
        filter,
    );
    let _ = session.try_cast_mover(
        Position::ZERO,
        Vec2::ZERO,
        Vec2::ZERO,
        0.5,
        translation,
        filter,
    );
}
