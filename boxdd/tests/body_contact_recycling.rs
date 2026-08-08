use boxdd::{prelude::*, shapes};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn create_overlapping_circle_pair(
    world: &mut World,
    dynamic_recycling: bool,
    enable_pre_solve_events: bool,
) -> (BodyId, BodyId) {
    let static_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let dynamic_body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([0.75_f32, 0.0])
                .enable_contact_recycling(dynamic_recycling)
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_pre_solve_events(enable_pre_solve_events)
        .build()
        .unwrap();
    let circle = shapes::circle([0.0_f32, 0.0], 0.5).unwrap();
    world
        .body(static_body)
        .unwrap()
        .create_circle(&shape_def, &circle)
        .unwrap();
    world
        .body(dynamic_body)
        .unwrap()
        .create_circle(&shape_def, &circle)
        .unwrap();
    (static_body, dynamic_body)
}

fn step(world: &mut World) {
    drop(world.step(0.0, 1).unwrap());
}

fn recycled_contact_count(world: &World) -> i32 {
    world.counters().unwrap().recycled_contact_count
}

#[test]
fn body_definition_and_capability_control_contact_recycling() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let enabled = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let disabled = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .enable_contact_recycling(false)
                .build()
                .unwrap(),
        )
        .unwrap();

    assert!(
        world
            .body(enabled)
            .unwrap()
            .is_contact_recycling_enabled()
            .unwrap()
    );
    assert!(
        !world
            .body(disabled)
            .unwrap()
            .is_contact_recycling_enabled()
            .unwrap()
    );

    let mut body = world.body(enabled).unwrap();
    body.enable_contact_recycling(false).unwrap();
    assert!(!body.is_contact_recycling_enabled().unwrap());
    body.enable_contact_recycling(true).unwrap();
    assert!(body.is_contact_recycling_enabled().unwrap());
}

#[test]
fn contact_recycle_distance_is_a_strict_motion_threshold_and_zero_disables_reuse() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world.set_contact_recycle_distance(0.05).unwrap();
    let (_static_body, dynamic_body) = create_overlapping_circle_pair(&mut world, true, false);

    step(&mut world);
    assert_eq!(world.counters().unwrap().contact_count, 1);
    assert_eq!(recycled_contact_count(&world), 0);

    step(&mut world);
    assert_eq!(recycled_contact_count(&world), 1);

    world
        .body(dynamic_body)
        .unwrap()
        .set_position_and_rotation([0.76_f32, 0.0], 0.0)
        .unwrap();
    step(&mut world);
    assert_eq!(recycled_contact_count(&world), 1);

    world
        .body(dynamic_body)
        .unwrap()
        .set_position_and_rotation([0.81_f32, 0.0], 0.0)
        .unwrap();
    step(&mut world);
    assert_eq!(recycled_contact_count(&world), 0);

    step(&mut world);
    assert_eq!(recycled_contact_count(&world), 1);

    world.set_contact_recycle_distance(0.0).unwrap();
    step(&mut world);
    assert_eq!(recycled_contact_count(&world), 0);
}

#[test]
fn recycling_toggle_only_affects_contacts_created_after_the_toggle() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    world.set_contact_recycle_distance(0.05).unwrap();

    let pre_solve_calls = Arc::new(AtomicUsize::new(0));
    world
        .set_pre_solve({
            let pre_solve_calls = Arc::clone(&pre_solve_calls);
            move |_, _, _, _| {
                pre_solve_calls.fetch_add(1, Ordering::SeqCst);
                true
            }
        })
        .unwrap();

    let (_static_body, dynamic_body) = create_overlapping_circle_pair(&mut world, true, true);

    step(&mut world);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 1);
    assert_eq!(recycled_contact_count(&world), 0);

    step(&mut world);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 1);
    assert_eq!(recycled_contact_count(&world), 1);

    world
        .body(dynamic_body)
        .unwrap()
        .enable_contact_recycling(false)
        .unwrap();
    step(&mut world);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 1);
    assert_eq!(recycled_contact_count(&world), 1);

    world
        .body(dynamic_body)
        .unwrap()
        .set_position_and_rotation([3.0_f32, 0.0], 0.0)
        .unwrap();
    step(&mut world);
    assert_eq!(world.counters().unwrap().contact_count, 0);

    world
        .body(dynamic_body)
        .unwrap()
        .set_position_and_rotation([0.75_f32, 0.0], 0.0)
        .unwrap();
    step(&mut world);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 2);
    assert_eq!(recycled_contact_count(&world), 0);

    step(&mut world);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 3);
    assert_eq!(recycled_contact_count(&world), 0);

    world
        .body(dynamic_body)
        .unwrap()
        .enable_contact_recycling(true)
        .unwrap();
    step(&mut world);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 4);
    assert_eq!(recycled_contact_count(&world), 0);

    world
        .body(dynamic_body)
        .unwrap()
        .set_position_and_rotation([3.0_f32, 0.0], 0.0)
        .unwrap();
    step(&mut world);
    assert_eq!(world.counters().unwrap().contact_count, 0);

    world
        .body(dynamic_body)
        .unwrap()
        .set_position_and_rotation([0.75_f32, 0.0], 0.0)
        .unwrap();
    step(&mut world);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 5);
    assert_eq!(recycled_contact_count(&world), 0);

    step(&mut world);
    assert_eq!(pre_solve_calls.load(Ordering::SeqCst), 5);
    assert_eq!(recycled_contact_count(&world), 1);
}

#[test]
fn body_capability_rejects_foreign_and_stale_ids() {
    let mut source = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let source_body = source
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let mut target = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();

    assert_eq!(target.body(source_body).err().unwrap(), Error::WrongWorld);

    let stale = target
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    target.body(stale).unwrap().destroy().unwrap();
    assert_eq!(target.body(stale).err().unwrap(), Error::InvalidBodyId);
}
