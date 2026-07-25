use boxdd::{prelude::*, shapes};

fn orphan_segment(offset: f32) -> ChainSegment {
    shapes::chain_segment(
        [offset - 2.0, 0.0],
        [offset - 1.0, 0.0],
        [offset + 1.0, 0.0],
        [offset + 2.0, 0.0],
    )
}

#[test]
fn orphan_chain_segment_creation_has_receiver_parity_and_no_parent() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body_id = world.create_body_id(BodyDef::default());
    let shape_def = ShapeDef::default();
    let geometry = orphan_segment(0.0);

    let id = world.create_chain_segment_shape_for(body_id, &shape_def, &geometry);
    {
        let shape = world.shape(id).unwrap();
        assert_eq!(shape.shape_type(), ShapeType::ChainSegment);
        assert_eq!(shape.parent_chain_id(), None);
        assert_eq!(shape.chain_segment(), geometry);
        assert_eq!(shape.chain_segment().chain_id_raw(), -1);
    }

    let owned = world.create_chain_segment_shape_for_owned(body_id, &shape_def, &geometry);
    assert_eq!(owned.shape_type(), ShapeType::ChainSegment);
    assert_eq!(owned.parent_chain_id(), None);
    drop(owned);

    {
        let mut body = world.body(body_id).unwrap();
        let shape = body.create_chain_segment_shape(&shape_def, &geometry);
        assert_eq!(shape.parent_chain_id(), None);
    }

    let mut owned_body = world.create_body_owned(BodyDef::default());
    let owned_shape = owned_body
        .try_create_chain_segment_shape(&shape_def, &geometry)
        .unwrap();
    assert_eq!(owned_shape.parent_chain_id(), None);
}

#[test]
fn invalid_orphan_chain_segment_is_rejected_before_creation() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyDef::default());
    let invalid = shapes::chain_segment([-1.0_f32, 0.0], [0.0, 0.0], [0.0, 0.0], [1.0, 0.0]);
    let before = world.counters().shape_count;

    assert_eq!(
        world.try_create_chain_segment_shape_for(body, &ShapeDef::default(), &invalid),
        Err(ApiError::InvalidArgument)
    );
    assert_eq!(world.counters().shape_count, before);
}

#[test]
fn orphan_mutation_updates_or_converts_shapes_without_forging_a_parent() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyDef::default());
    let mut shape = world.create_circle_shape_for_owned(
        body,
        &ShapeDef::default(),
        &shapes::circle([0.0_f32, 0.0], 0.5),
    );

    let first = orphan_segment(0.0);
    shape.set_chain_segment(&first);
    assert_eq!(shape.shape_type(), ShapeType::ChainSegment);
    assert_eq!(shape.parent_chain_id(), None);
    assert_eq!(shape.chain_segment(), first);

    let second = orphan_segment(3.0);
    shape.try_set_chain_segment(&second).unwrap();
    assert_eq!(shape.chain_segment(), second);
    assert_eq!(shape.chain_segment().chain_id_raw(), -1);

    let invalid = shapes::chain_segment([-1.0_f32, 0.0], [0.0, 0.0], [0.0, 0.0], [1.0, 0.0]);
    assert_eq!(
        world.try_shape_set_chain_segment(shape.id(), &invalid),
        Err(ApiError::InvalidArgument)
    );
    assert_eq!(shape.chain_segment(), second);

    let scoped_id = world.create_circle_shape_for(
        body,
        &ShapeDef::default(),
        &shapes::circle([8.0_f32, 0.0], 0.5),
    );
    let mut scoped = world.shape(scoped_id).unwrap();
    scoped.try_set_chain_segment(&orphan_segment(8.0)).unwrap();
    assert_eq!(scoped.shape_type(), ShapeType::ChainSegment);
    assert_eq!(scoped.parent_chain_id(), None);
}

#[test]
fn chain_owned_segments_reject_orphan_mutation_and_independent_destroy() {
    let mut world = World::new(WorldDef::default()).unwrap();
    let body = world.create_body_id(BodyDef::default());
    let chain = world.create_chain_for_owned(
        body,
        &boxdd::shapes::chain::ChainDef::builder()
            .points([
                Vec2::new(-2.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
            ])
            .build(),
    );
    let segment = chain.segments()[0];
    let replacement = orphan_segment(4.0);

    assert_eq!(
        world.try_shape_set_chain_segment(segment, &replacement),
        Err(ApiError::ChainOwnedShape)
    );
    assert_eq!(
        world.try_destroy_shape_id(segment, true),
        Err(ApiError::ChainOwnedShape)
    );

    let circle = shapes::circle([0.0_f32, 0.0], 0.5);
    let regular_segment = shapes::segment([-0.5_f32, 0.0], [0.5_f32, 0.0]);
    let capsule = shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25);
    let polygon = shapes::box_polygon(0.5, 0.5);
    assert_eq!(
        world.try_shape_set_circle(segment, &circle),
        Err(ApiError::ChainOwnedShape)
    );
    assert_eq!(
        world.try_shape_set_segment(segment, &regular_segment),
        Err(ApiError::ChainOwnedShape)
    );
    assert_eq!(
        world.try_shape_set_capsule(segment, &capsule),
        Err(ApiError::ChainOwnedShape)
    );
    assert_eq!(
        world.try_shape_set_polygon(segment, &polygon),
        Err(ApiError::ChainOwnedShape)
    );

    let mutation = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.shape_set_chain_segment(segment, &replacement);
    }));
    assert!(mutation.is_err());
    let mutation = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.shape_set_circle(segment, &circle);
    }));
    assert!(mutation.is_err());

    {
        let mut shape = world.shape(segment).unwrap();
        assert_eq!(
            shape.try_set_circle(&circle),
            Err(ApiError::ChainOwnedShape)
        );
        assert_eq!(
            shape.try_set_segment(&regular_segment),
            Err(ApiError::ChainOwnedShape)
        );
        assert_eq!(
            shape.try_set_capsule(&capsule),
            Err(ApiError::ChainOwnedShape)
        );
        assert_eq!(
            shape.try_set_polygon(&polygon),
            Err(ApiError::ChainOwnedShape)
        );
        assert_eq!(shape.parent_chain_id(), Some(chain.id()));
        assert_eq!(shape.shape_type(), ShapeType::ChainSegment);
        assert!(shape.is_valid());
    }

    chain.destroy();
    assert_eq!(world.body_shape_count(body), 0);
    assert!(world.shape(segment).is_none());
}
