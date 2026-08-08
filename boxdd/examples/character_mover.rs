use boxdd::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, -10.0])
            .build()?,
    )?;

    // Ground + step obstacle
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _g = world.body(ground)?.create_polygon(
        &ShapeDef::builder().density(0.0).build()?,
        &shapes::box_polygon(50.0, 0.5)?,
    )?;
    let step = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .position([1.0_f32, 0.5])
            .build()?,
    )?;
    let _s = world.body(step)?.create_polygon(
        &ShapeDef::builder().density(0.0).build()?,
        &shapes::box_polygon(0.5, 0.5)?,
    )?;

    // Character mover capsule. This starts slightly overlapping the ground so the
    // plane solver has something to resolve.
    let c1 = Vec2::new(0.0, 0.7);
    let c2 = Vec2::new(0.0, 1.5);
    let radius = 0.25;
    let desired_move = Vec2::new(2.0, 0.0);

    // Minimal API demo:
    // 1. cast the mover against future movement
    // 2. collect current contact planes
    // 3. solve planes
    // 4. clip a velocity / movement vector
    let query = world.query()?;
    let frac = query.cast_mover(
        Position::ZERO,
        c1,
        c2,
        radius,
        desired_move,
        QueryFilter::default(),
    )?;
    let plane_results =
        query.collide_mover(Position::ZERO, c1, c2, radius, QueryFilter::default())?;
    let mut planes = Vec::with_capacity(plane_results.len());
    for result in plane_results {
        if let Some(plane) = result.into_rigid_collision_plane()? {
            planes.push(plane);
        }
    }
    let solved = solve_planes([0.0_f32, -0.15], &mut planes)?;
    let clipped = clip_vector(desired_move, &planes)?;

    println!("character mover fraction: {:.3}", frac);
    println!("collision planes: {}", planes.len());
    println!(
        "solve translation: ({:.3}, {:.3})",
        solved.translation().x,
        solved.translation().y
    );
    println!("clipped vector: ({:.3}, {:.3})", clipped.x, clipped.y);
    Ok(())
}
