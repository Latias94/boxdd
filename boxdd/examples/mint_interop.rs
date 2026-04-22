use boxdd::{
    Aabb, BodyBuilder, BodyType, QueryFilter, Rot, ShapeDef, Transform, World, WorldDef, shapes,
};
use mint::{Point2, RowMatrix2, RowMatrix3x2, Vector2};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gravity = Vector2 {
        x: 0.0_f32,
        y: -9.8,
    };
    let spawn = Point2 { x: 1.0_f32, y: 3.0 };

    let mut world = World::new(WorldDef::builder().gravity(gravity).build())?;

    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_polygon_shape_for(
        ground,
        &ShapeDef::default(),
        &shapes::box_polygon(10.0, 0.5),
    );

    let body = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position(spawn)
            .build(),
    );
    let _ = world.create_polygon_shape_for(
        body,
        &ShapeDef::builder().density(1.0).build(),
        &shapes::box_polygon(0.5, 0.5),
    );

    for _ in 0..10 {
        world.step(1.0 / 60.0, 4);
    }

    let mut overlap_hits = Vec::with_capacity(8);
    world.overlap_aabb_into(
        Aabb::from_center_half_extents(Point2 { x: 1.0_f32, y: 2.5 }, Vector2 { x: 1.5, y: 1.5 }),
        QueryFilter::default(),
        &mut overlap_hits,
    );

    let position: Point2<f32> = world.body_position(body).into();

    let quarter_turn = Rot::try_from(RowMatrix2 {
        x: Vector2 { x: 0.0, y: -1.0 },
        y: Vector2 { x: 1.0, y: 0.0 },
    })?;
    let quarter_turn_matrix: RowMatrix2<f32> = quarter_turn.into();

    let transform = Transform::from_pos_angle(spawn, std::f32::consts::FRAC_PI_4);
    let mint_transform: RowMatrix3x2<f32> = transform.into();
    let recovered_transform = Transform::try_from(mint_transform)?;
    let recovered_position: Point2<f32> = recovered_transform.position().into();

    println!(
        "mint_interop: position=({:.2}, {:.2}) overlap_hits={} rot_x_axis=({:.2}, {:.2}) recovered_translation=({:.2}, {:.2})",
        position.x,
        position.y,
        overlap_hits.len(),
        quarter_turn_matrix.x.x,
        quarter_turn_matrix.x.y,
        recovered_position.x,
        recovered_position.y
    );

    Ok(())
}
