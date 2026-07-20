use super::{Aabb, QueryFilter};
use crate::core::world_core::ActivityState;
use crate::error::ApiError;
use crate::{Position, World, WorldDef};

fn test_aabb() -> Aabb {
    Aabb::from_center_half_extents([0.0, 0.0], [1.0, 1.0])
}

#[test]
fn query_entry_points_reject_busy_worlds_before_native_queries() {
    let world = World::new(WorldDef::default()).unwrap();
    let handle = world.handle();

    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Recording)
        .unwrap();

    assert_eq!(
        world
            .try_overlap_aabb(Position::ZERO, test_aabb(), QueryFilter::default())
            .unwrap_err(),
        ApiError::WorldBusy
    );
    assert_eq!(
        handle
            .try_overlap_aabb(Position::ZERO, test_aabb(), QueryFilter::default())
            .unwrap_err(),
        ApiError::WorldBusy
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            world.overlap_aabb(Position::ZERO, test_aabb(), QueryFilter::default())
        }))
        .is_err()
    );

    world
        .core()
        .set_activity(ActivityState::Recording, ActivityState::Idle)
        .unwrap();
    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Restoring)
        .unwrap();
    assert_eq!(
        world
            .try_overlap_aabb(Position::ZERO, test_aabb(), QueryFilter::default())
            .unwrap_err(),
        ApiError::WorldBusy
    );
    world
        .core()
        .set_activity(ActivityState::Restoring, ActivityState::Idle)
        .unwrap();
}

#[test]
fn query_entry_points_reject_poisoned_worlds() {
    let world = World::new(WorldDef::default()).unwrap();
    let handle = world.handle();
    world.core().poison();

    assert_eq!(
        world
            .try_overlap_aabb(Position::ZERO, test_aabb(), QueryFilter::default())
            .unwrap_err(),
        ApiError::WorldPoisoned
    );
    assert_eq!(
        handle
            .try_overlap_aabb(Position::ZERO, test_aabb(), QueryFilter::default())
            .unwrap_err(),
        ApiError::WorldPoisoned
    );
}

#[test]
fn callback_error_takes_precedence_over_world_activity() {
    let world = World::new(WorldDef::default()).unwrap();
    world
        .core()
        .set_activity(ActivityState::Idle, ActivityState::Recording)
        .unwrap();

    {
        let _guard = crate::core::callback_state::CallbackGuard::enter();
        assert_eq!(
            world
                .try_overlap_aabb(Position::ZERO, test_aabb(), QueryFilter::default())
                .unwrap_err(),
            ApiError::InCallback
        );
    }

    world
        .core()
        .set_activity(ActivityState::Recording, ActivityState::Idle)
        .unwrap();
}
