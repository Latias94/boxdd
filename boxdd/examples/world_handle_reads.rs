use boxdd::{Aabb, BodyBuilder, QueryFilter, ShapeDef, World, WorldDef, shapes};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -9.8]).build())?;

    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ground_shape = world.create_polygon_shape_for(
        ground,
        &ShapeDef::default(),
        &shapes::box_polygon(10.0, 0.5),
    );

    let dynamic = world.create_body_id(
        BodyBuilder::new()
            .body_type(boxdd::BodyType::Dynamic)
            .position([0.0, 2.0])
            .build(),
    );
    let _dynamic_shape = world.create_polygon_shape_for(
        dynamic,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );

    for _ in 0..10 {
        world.step(1.0 / 60.0, 4);
    }

    let handle = world.handle();
    let shape_ids =
        handle.overlap_aabb(Aabb::new([-1.0, -1.0], [1.0, 3.0]), QueryFilter::default());

    for shape in shape_ids {
        let body = handle.shape_body_id(shape);
        let position = handle.body_position(body);
        let aabb = handle.shape_aabb(shape);

        println!(
            "shape={shape:?} body={body:?} position=({}, {}) aabb=({}, {}) -> ({}, {})",
            position.x, position.y, aabb.lower.x, aabb.lower.y, aabb.upper.x, aabb.upper.y
        );
    }

    Ok(())
}
