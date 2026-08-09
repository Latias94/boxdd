use boxdd::prelude::*;
use boxdd::shapes::{self, chain::ChainDef};

#[derive(Copy, Clone)]
struct ObjectIds {
    body_a: BodyId,
    body_b: BodyId,
    shape: ShapeId,
    joint: JointId,
    chain: ChainId,
}

fn new_world() -> World {
    boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_builder()
                .gravity([0.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap()
}

fn chain_def() -> ChainDef {
    ChainDef::builder()
        .points([[-2.0_f32, -10.0], [-1.0, -10.0], [1.0, -10.0], [2.0, -10.0]])
        .build()
        .unwrap()
}

fn populate_world(world: &mut World, x: f32) -> ObjectIds {
    let body_a = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([x, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let body_b = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Static)
                .position([x + 3.0, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape = world
        .body(body_a)
        .unwrap()
        .create_circle(
            &ShapeDef::builder()
                .density(1.0)
                .enable_contact_events(true)
                .build()
                .unwrap(),
            &shapes::circle([0.0_f32, 0.0], 0.5).unwrap(),
        )
        .unwrap();
    let joint = world
        .create_distance_joint(&DistanceJointDef::new(
            world.joint_base(body_a, body_b).unwrap(),
        ))
        .unwrap();
    let chain = world
        .body(body_b)
        .unwrap()
        .create_chain(&chain_def())
        .unwrap();

    ObjectIds {
        body_a,
        body_b,
        shape,
        joint,
        chain,
    }
}

fn create_live_contact(
    world: &mut World,
    center_x: f32,
) -> (BodyId, ShapeId, ShapeId, ContactBeginTouchEvent) {
    let body_a = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([center_x - 1.0, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let body_b = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([center_x + 1.0, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let shape_def = ShapeDef::builder()
        .density(1.0)
        .enable_contact_events(true)
        .build()
        .unwrap();
    let polygon = shapes::box_polygon(0.5, 0.5).unwrap();
    let shape_a = world
        .body(body_a)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();
    let shape_b = world
        .body(body_b)
        .unwrap()
        .create_polygon(&shape_def, &polygon)
        .unwrap();
    world
        .body(body_a)
        .unwrap()
        .set_linear_velocity([2.0_f32, 0.0])
        .unwrap();
    world
        .body(body_b)
        .unwrap()
        .set_linear_velocity([-2.0_f32, 0.0])
        .unwrap();

    for _ in 0..180 {
        let completed = world.step(1.0 / 60.0, 4).unwrap();
        let events = completed.contact_events().unwrap().to_owned().unwrap();
        if let Some(event) = events.begin.into_iter().next() {
            return (body_a, shape_a, shape_b, event);
        }
    }

    panic!("expected a live contact id from contact begin events");
}

fn assert_error<T>(result: boxdd::Result<T>, expected: Error) {
    match result {
        Err(error) => assert_eq!(error, expected),
        Ok(_) => panic!("expected {expected:?}, got Ok"),
    }
}

#[test]
fn foreign_ids_are_rejected_at_capability_and_creation_boundaries() {
    let mut source = new_world();
    let source_ids = populate_world(&mut source, -20.0);
    let mut target = new_world();
    let target_ids = populate_world(&mut target, 20.0);
    let source_before = source.counters().unwrap();
    let target_before = target.counters().unwrap();

    assert_error(target.body(source_ids.body_a), Error::WrongWorld);
    assert_error(target.shape(source_ids.shape), Error::WrongWorld);
    assert_error(target.joint(source_ids.joint), Error::WrongWorld);
    assert_error(target.chain(source_ids.chain), Error::WrongWorld);
    let source_joint = DistanceJointDef::new(
        source
            .joint_base(source_ids.body_a, source_ids.body_b)
            .unwrap(),
    );
    assert_error(
        target.create_distance_joint(&source_joint),
        Error::WrongWorld,
    );
    assert_error(
        target.joint_base(target_ids.body_a, source_ids.body_b),
        Error::WrongWorld,
    );

    assert_eq!(source.counters().unwrap(), source_before);
    assert_eq!(target.counters().unwrap(), target_before);
    assert!(source.body(source_ids.body_a).unwrap().body_type().is_ok());
    assert!(target.body(target_ids.body_a).unwrap().body_type().is_ok());
}

#[test]
fn foreign_destruction_attempts_are_transactional_for_both_worlds() {
    let mut source = new_world();
    let source_ids = populate_world(&mut source, -20.0);
    let mut target = new_world();
    let target_ids = populate_world(&mut target, 20.0);

    source
        .body(source_ids.body_a)
        .unwrap()
        .set_user_data(String::from("source body"))
        .unwrap();
    source
        .shape(source_ids.shape)
        .unwrap()
        .set_user_data(String::from("source shape"))
        .unwrap();
    source
        .joint(source_ids.joint)
        .unwrap()
        .set_user_data(String::from("source joint"))
        .unwrap();

    let source_before = source.counters().unwrap();
    let target_before = target.counters().unwrap();
    assert_error(
        target
            .body(source_ids.body_a)
            .and_then(|body| body.destroy()),
        Error::WrongWorld,
    );
    assert_error(
        target
            .joint(source_ids.joint)
            .and_then(|joint| joint.destroy(true)),
        Error::WrongWorld,
    );
    assert_error(target.shape(source_ids.shape), Error::WrongWorld);
    assert_error(target.chain(source_ids.chain), Error::WrongWorld);

    assert_eq!(source.counters().unwrap(), source_before);
    assert_eq!(target.counters().unwrap(), target_before);
    assert_eq!(
        source
            .body(source_ids.body_a)
            .unwrap()
            .with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .as_deref(),
        Some("source body")
    );
    assert_eq!(
        source
            .shape(source_ids.shape)
            .unwrap()
            .with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .as_deref(),
        Some("source shape")
    );
    assert_eq!(
        source
            .joint(source_ids.joint)
            .unwrap()
            .with_user_data::<String, _>(Clone::clone)
            .unwrap()
            .as_deref(),
        Some("source joint")
    );
    assert!(target.body(target_ids.body_a).unwrap().body_type().is_ok());
}

#[test]
fn completed_step_event_snapshots_preserve_world_provenance() {
    let mut source = new_world();
    let (source_body, source_shape, other_shape, event) = create_live_contact(&mut source, -20.0);
    let mut target = new_world();
    let target_ids = populate_world(&mut target, 20.0);
    let target_before = target.counters().unwrap();

    assert!(event.shape_a == source_shape || event.shape_b == source_shape);
    assert!(event.shape_a == other_shape || event.shape_b == other_shape);
    assert!(source.contact_data(event.contact_id).is_ok());
    assert_error(target.shape(event.shape_a), Error::WrongWorld);
    assert_error(target.shape(event.shape_b), Error::WrongWorld);
    assert_error(target.contact_data(event.contact_id), Error::WrongWorld);

    assert!(source.body(source_body).unwrap().body_type().is_ok());
    assert!(source.shape(source_shape).unwrap().shape_type().is_ok());
    assert_eq!(target.counters().unwrap(), target_before);
    assert!(target.body(target_ids.body_a).unwrap().body_type().is_ok());
}

#[test]
fn a_new_world_cannot_rebrand_an_id_from_a_dropped_world() {
    let old_body = {
        let mut old_world = new_world();
        old_world
            .create_body(
                boxdd::Foundation::get()
                    .expect("Foundation must be initialized before constructing a BodyDef")
                    .body_builder()
                    .body_type(BodyType::Dynamic)
                    .build()
                    .unwrap(),
            )
            .unwrap()
    };

    let mut new_world = new_world();
    let replacement = new_world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_builder()
                .body_type(BodyType::Dynamic)
                .position([7.0_f32, 0.0])
                .build()
                .unwrap(),
        )
        .unwrap();
    let replacement_position = new_world.body(replacement).unwrap().position().unwrap();
    assert_ne!(old_body, replacement);

    assert_error(new_world.body(old_body), Error::WrongWorld);
    assert_eq!(
        new_world.body(replacement).unwrap().position().unwrap(),
        replacement_position
    );
}
