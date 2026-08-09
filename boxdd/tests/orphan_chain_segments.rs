use boxdd::{prelude::*, shapes};

fn orphan_segment(offset: f32) -> ChainSegment {
    shapes::chain_segment(
        [offset - 2.0, 0.0],
        [offset - 1.0, 0.0],
        [offset + 1.0, 0.0],
        [offset + 2.0, 0.0],
    )
    .unwrap()
}

#[test]
fn safe_chain_segments_never_preserve_box2d_internal_ownership() {
    boxdd::Foundation::initialize_default().unwrap();
    let segment = orphan_segment(0.0);
    assert_eq!(segment.into_raw().chainId, boxdd_sys::ffi::B2_NULL_INDEX);

    let mut native = segment.into_raw();
    native.chainId = 42;
    assert_eq!(
        ChainSegment::from_raw(native).unwrap().into_raw().chainId,
        boxdd_sys::ffi::B2_NULL_INDEX
    );
}

#[test]
fn orphan_chain_segments_have_no_parent_and_validate_before_creation() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body_id = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let geometry = orphan_segment(0.0);
    let shape_id = world
        .body(body_id)
        .unwrap()
        .create_chain_segment(&ShapeDef::default(), &geometry)
        .unwrap();

    let shape = world.shape(shape_id).unwrap();
    assert_eq!(shape.shape_type().unwrap(), ShapeType::ChainSegment);
    assert_eq!(shape.parent_chain_id().unwrap(), None);
    assert_eq!(shape.chain_segment().unwrap(), geometry);
    let before = world.counters().unwrap().shape_count;
    assert_eq!(
        shapes::chain_segment([-1.0_f32, 0.0], [0.0, 0.0], [0.0, 0.0], [1.0, 0.0]).unwrap_err(),
        Error::invalid_argument(
            "ChainSegment::new",
            "chain_segment",
            "finite ghost points and segment endpoints separated by Box2D's minimum length",
        )
    );
    assert_eq!(world.counters().unwrap().shape_count, before);
}

#[test]
fn regular_shapes_can_become_orphan_segments_without_forging_a_parent() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let shape_id = world
        .body(body)
        .unwrap()
        .create_centered_circle(&ShapeDef::default(), 0.5)
        .unwrap();
    let first = orphan_segment(0.0);
    let second = orphan_segment(3.0);

    let mut shape = world.shape(shape_id).unwrap();
    shape.set_chain_segment(&first).unwrap();
    assert_eq!(shape.shape_type().unwrap(), ShapeType::ChainSegment);
    assert_eq!(shape.parent_chain_id().unwrap(), None);
    assert_eq!(shape.chain_segment().unwrap(), first);
    shape.set_chain_segment(&second).unwrap();
    assert_eq!(shape.chain_segment().unwrap(), second);

    assert_eq!(
        shapes::chain_segment([-1.0_f32, 0.0], [0.0, 0.0], [0.0, 0.0], [1.0, 0.0]).unwrap_err(),
        Error::invalid_argument(
            "ChainSegment::new",
            "chain_segment",
            "finite ghost points and segment endpoints separated by Box2D's minimum length",
        )
    );
    assert_eq!(shape.chain_segment().unwrap(), second);
}

#[test]
fn chain_owned_segments_reject_independent_mutation_and_destruction() {
    let mut world = boxdd::Foundation::initialize_default()
        .unwrap()
        .create_world(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a WorldDef")
                .world_def(),
        )
        .unwrap();
    let body = world
        .create_body(
            boxdd::Foundation::get()
                .expect("Foundation must be initialized before constructing a BodyDef")
                .body_def(),
        )
        .unwrap();
    let chain_id = world
        .body(body)
        .unwrap()
        .create_chain(
            &ChainDef::builder()
                .points([
                    Vec2::new(-2.0, 0.0),
                    Vec2::new(-1.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(2.0, 0.0),
                ])
                .build()
                .unwrap(),
        )
        .unwrap();
    let segment = world.chain(chain_id).unwrap().segments().unwrap()[0];

    assert_eq!(
        world
            .shape(segment)
            .unwrap()
            .set_chain_segment(&orphan_segment(4.0)),
        Err(Error::ChainOwnedShape)
    );
    assert_eq!(
        world.shape(segment).unwrap().destroy(true),
        Err(Error::ChainOwnedShape)
    );

    let circle = shapes::circle([0.0_f32, 0.0], 0.5).unwrap();
    let regular_segment = shapes::segment([-0.5_f32, 0.0], [0.5_f32, 0.0]).unwrap();
    let capsule = shapes::capsule([-0.5_f32, 0.0], [0.5_f32, 0.0], 0.25).unwrap();
    let polygon = shapes::box_polygon(0.5, 0.5).unwrap();
    let mut shape = world.shape(segment).unwrap();
    assert_eq!(shape.set_circle(&circle), Err(Error::ChainOwnedShape));
    assert_eq!(
        shape.set_segment(&regular_segment),
        Err(Error::ChainOwnedShape)
    );
    assert_eq!(shape.set_capsule(&capsule), Err(Error::ChainOwnedShape));
    assert_eq!(shape.set_polygon(&polygon), Err(Error::ChainOwnedShape));
    assert_eq!(shape.parent_chain_id().unwrap(), Some(chain_id));
    assert_eq!(shape.shape_type().unwrap(), ShapeType::ChainSegment);
    assert_eq!(shape.shape_type().unwrap(), ShapeType::ChainSegment);
    world.chain(chain_id).unwrap().destroy().unwrap();
    assert_eq!(world.body(body).unwrap().shape_count().unwrap(), 0);
    assert_eq!(world.shape(segment).err().unwrap(), Error::InvalidShapeId);
}
