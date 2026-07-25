use boxdd::{prelude::*, shapes};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn create_overlapping_circle_pair(
    world: &mut World,
    dynamic_recycling: bool,
    enable_pre_solve_events: bool,
) -> (BodyId, BodyId) {
    let static_body = world.create_body_id(BodyDef::default());
    let dynamic_body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.75_f32, 0.0])
            .enable_contact_recycling(dynamic_recycling)
            .build(),
    );
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_pre_solve_events(enable_pre_solve_events)
        .build();
    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    world.create_circle_shape_for(static_body, &shape_def, &circle);
    world.create_circle_shape_for(dynamic_body, &shape_def, &circle);
    (static_body, dynamic_body)
}

#[test]
fn contact_recycling_defaults_on_and_has_receiver_parity() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyDef::default());
    let handle = world.handle();

    assert!(world.body_is_contact_recycling_enabled(body_id));
    assert!(
        world
            .try_body_is_contact_recycling_enabled(body_id)
            .unwrap()
    );
    assert!(handle.body_is_contact_recycling_enabled(body_id));
    assert!(
        handle
            .try_body_is_contact_recycling_enabled(body_id)
            .unwrap()
    );

    world.body_enable_contact_recycling(body_id, false);
    assert!(!world.body_is_contact_recycling_enabled(body_id));
    world
        .try_body_enable_contact_recycling(body_id, true)
        .unwrap();
    assert!(world.body_is_contact_recycling_enabled(body_id));

    {
        let mut body = world.body(body_id).unwrap();
        assert!(body.is_contact_recycling_enabled());
        assert!(body.try_is_contact_recycling_enabled().unwrap());
        body.enable_contact_recycling(false);
        assert!(!body.is_contact_recycling_enabled());
        body.try_enable_contact_recycling(true).unwrap();
        assert!(body.try_is_contact_recycling_enabled().unwrap());
    }

    let mut owned = world.create_body_owned(BodyDef::default());
    assert!(owned.is_contact_recycling_enabled());
    assert!(owned.try_is_contact_recycling_enabled().unwrap());
    owned.enable_contact_recycling(false);
    assert!(!owned.is_contact_recycling_enabled());
    owned.try_enable_contact_recycling(true).unwrap();
    assert!(owned.try_is_contact_recycling_enabled().unwrap());
}

#[test]
fn body_definition_controls_initial_contact_recycling_state() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let enabled = world.create_body_id(BodyDef::default());
    let disabled = world.create_body_id(BodyBuilder::new().enable_contact_recycling(false).build());

    assert!(world.body_is_contact_recycling_enabled(enabled));
    assert!(!world.body_is_contact_recycling_enabled(disabled));
}

#[test]
fn contact_recycle_distance_controls_the_strict_motion_threshold_and_zero_disables_it() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    world.set_contact_recycle_distance(0.05);
    let (_static_body, dynamic_body) = create_overlapping_circle_pair(&mut world, true, false);

    world.step(0.0, 1);
    assert_eq!(world.counters().contact_count, 1);
    assert_eq!(world.counters().recycled_contact_count, 0);

    world.step(0.0, 1);
    assert_eq!(world.counters().recycled_contact_count, 1);

    world.set_body_position_and_rotation(dynamic_body, [0.76_f32, 0.0], 0.0);
    world.step(0.0, 1);
    assert_eq!(world.counters().recycled_contact_count, 1);

    world.set_body_position_and_rotation(dynamic_body, [0.81_f32, 0.0], 0.0);
    world.step(0.0, 1);
    assert_eq!(world.counters().recycled_contact_count, 0);

    world.step(0.0, 1);
    assert_eq!(world.counters().recycled_contact_count, 1);

    world.set_contact_recycle_distance(0.0);
    world.step(0.0, 1);
    assert_eq!(world.counters().recycled_contact_count, 0);
}

#[test]
fn body_recycling_toggle_only_changes_contacts_created_after_the_toggle() {
    let mut world = World::new(WorldDef::builder().gravity([0.0_f32, 0.0]).build()).unwrap();
    world.set_contact_recycle_distance(0.05);

    let pre_solve_calls = Arc::new(AtomicUsize::new(0));
    world.set_pre_solve({
        let pre_solve_calls = Arc::clone(&pre_solve_calls);
        move |_, _, _, _| {
            pre_solve_calls.fetch_add(1, Ordering::SeqCst);
            true
        }
    });

    let (_static_body, dynamic_body) = create_overlapping_circle_pair(&mut world, true, true);

    world.step(0.0, 1);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 1);
    assert_eq!(world.counters().recycled_contact_count, 0);

    world.step(0.0, 1);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 1);
    assert_eq!(world.counters().recycled_contact_count, 1);

    world.body_enable_contact_recycling(dynamic_body, false);
    world.step(0.0, 1);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 1);
    assert_eq!(world.counters().recycled_contact_count, 1);

    world.set_body_position_and_rotation(dynamic_body, [3.0_f32, 0.0], 0.0);
    world.step(0.0, 1);
    assert_eq!(world.counters().contact_count, 0);

    world.set_body_position_and_rotation(dynamic_body, [0.75_f32, 0.0], 0.0);
    world.step(0.0, 1);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 2);
    assert_eq!(world.counters().recycled_contact_count, 0);

    world.step(0.0, 1);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 3);
    assert_eq!(world.counters().recycled_contact_count, 0);

    world.body_enable_contact_recycling(dynamic_body, true);
    world.step(0.0, 1);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 4);
    assert_eq!(world.counters().recycled_contact_count, 0);

    world.set_body_position_and_rotation(dynamic_body, [3.0_f32, 0.0], 0.0);
    world.step(0.0, 1);
    assert_eq!(world.counters().contact_count, 0);

    world.set_body_position_and_rotation(dynamic_body, [0.75_f32, 0.0], 0.0);
    world.step(0.0, 1);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 5);
    assert_eq!(world.counters().recycled_contact_count, 0);

    world.step(0.0, 1);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 5);
    assert_eq!(world.counters().recycled_contact_count, 1);
}

#[test]
fn contact_recycling_reports_wrong_world_stale_and_destroyed_states() {
    let mut source = World::new(WorldDef::default()).unwrap();
    let source_body = source.create_body_id(BodyDef::default());
    let mut target = World::new(WorldDef::default()).unwrap();
    let target_handle = target.handle();

    assert_eq!(
        target
            .try_body_is_contact_recycling_enabled(source_body)
            .unwrap_err(),
        ApiError::WrongWorld
    );
    assert_eq!(
        target
            .try_body_enable_contact_recycling(source_body, false)
            .unwrap_err(),
        ApiError::WrongWorld
    );
    assert_eq!(
        target_handle
            .try_body_is_contact_recycling_enabled(source_body)
            .unwrap_err(),
        ApiError::WrongWorld
    );

    let mut stale_owned = target.create_body_owned(BodyDef::default());
    let stale = stale_owned.id();
    target.destroy_body_id(stale);
    assert_eq!(
        target
            .try_body_is_contact_recycling_enabled(stale)
            .unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        target
            .try_body_enable_contact_recycling(stale, false)
            .unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        target_handle
            .try_body_is_contact_recycling_enabled(stale)
            .unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        stale_owned.try_is_contact_recycling_enabled().unwrap_err(),
        ApiError::InvalidBodyId
    );
    assert_eq!(
        stale_owned.try_enable_contact_recycling(false).unwrap_err(),
        ApiError::InvalidBodyId
    );

    let mut doomed_world = World::new(WorldDef::default()).unwrap();
    let mut doomed_body = doomed_world.create_body_owned(BodyDef::default());
    let doomed_id = doomed_body.id();
    let doomed_handle = doomed_world.handle();
    drop(doomed_world);
    assert_eq!(
        doomed_body.try_is_contact_recycling_enabled().unwrap_err(),
        ApiError::WorldDestroyed
    );
    assert_eq!(
        doomed_body.try_enable_contact_recycling(false).unwrap_err(),
        ApiError::WorldDestroyed
    );
    assert_eq!(
        doomed_handle
            .try_body_is_contact_recycling_enabled(doomed_id)
            .unwrap_err(),
        ApiError::WorldDestroyed
    );
}

#[test]
fn contact_recycling_rejects_callback_reentry_before_stale_validation() {
    struct Drawer {
        body: OwnedBody,
        handle: WorldHandle,
        stale: BodyId,
        errors: Vec<ApiError>,
    }

    impl DebugDraw for Drawer {
        fn draw_solid_polygon(
            &mut self,
            _transform: WorldTransform,
            _vertices: &[Vec2],
            _radius: f32,
            _color: HexColor,
        ) {
            if !self.errors.is_empty() {
                return;
            }
            self.errors.push(
                self.handle
                    .try_body_is_contact_recycling_enabled(self.stale)
                    .unwrap_err(),
            );
            self.errors
                .push(self.body.try_is_contact_recycling_enabled().unwrap_err());
            self.errors
                .push(self.body.try_enable_contact_recycling(false).unwrap_err());
        }
    }

    let mut world = World::new(WorldDef::default()).unwrap();
    let stale = world.create_body_id(BodyDef::default());
    world.destroy_body_id(stale);

    let body = world.create_body_owned(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 1.0])
            .build(),
    );
    world.create_polygon_shape_for(
        body.id(),
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );

    let mut drawer = Drawer {
        body,
        handle: world.handle(),
        stale,
        errors: Vec::new(),
    };
    world.debug_draw(&mut drawer, DebugDrawOptions::default());

    assert_eq!(
        drawer.errors,
        vec![
            ApiError::InCallback,
            ApiError::InCallback,
            ApiError::InCallback,
        ]
    );
    assert!(drawer.body.is_contact_recycling_enabled());
}
