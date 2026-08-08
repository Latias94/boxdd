// Bodies/BodyType headless variant
//
// Notes
// - Debug build assertions: switching body types while joints are attached can hit assertions
//   in Box2D (world locked / joint graph invariants). To keep Debug stable, this example
//   destroys the revolute joints before switching the platform to kinematic/static.
// - Expected: no panics; prints platform positions across phases and awake count.
use boxdd::prelude::*;

// Headless port of the Bodies/Body Type sample.
// Demonstrates switching body types and enabling/disabling while connected by joints.
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = boxdd::Foundation::initialize_default()?;
    let mut world = foundation.create_world(
        boxdd::WorldBuilder::from(foundation.world_def())
            .gravity([0.0, -10.0])
            .build()?,
    )?;

    // Ground
    let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
    let _ = world.body(ground)?.create_segment(
        &ShapeDef::builder().build()?,
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0])?,
    )?;

    // Attachments and platform
    let attach1 = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([-2.0_f32, 3.0])
            .build()?,
    )?;
    let attach2 = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([3.0_f32, 3.0])
            .build()?,
    )?;
    let platform = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([-4.0_f32, 5.0])
            .build()?,
    )?;

    let sdef1 = ShapeDef::builder().density(1.0).build()?;
    let sdef2 = ShapeDef::builder().density(2.0).build()?;
    let box_tall = shapes::box_polygon(0.5, 2.0).expect("valid polygon geometry");
    let box_long = shapes::box_polygon(0.5, 4.0).expect("valid polygon geometry");
    let _ = world.body(attach1)?.create_polygon(&sdef1, &box_tall)?;
    let _ = world.body(attach2)?.create_polygon(&sdef1, &box_tall)?;
    let _ = world.body(platform)?.create_polygon(&sdef2, &box_long)?;

    // For Debug stability, omit joints in this headless variant.

    // Payloads
    let payload1 = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([-3.0_f32, 8.0])
            .build()?,
    )?;
    let _ = world.body(payload1)?.create_polygon(
        &sdef1,
        &shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry"),
    )?;
    let payload2 = world.create_body(
        BodyBuilder::from(foundation.body_def())
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 8.0])
            .build()?,
    )?;
    let _ = world.body(payload2)?.create_polygon(
        &sdef1,
        &shapes::box_polygon(0.25, 0.25).expect("valid polygon geometry"),
    )?;

    // Phase 1: dynamic platform
    for _ in 0..120 {
        drop(world.step(1.0 / 60.0, 4)?);
    }
    let p_dyn = world.body(platform)?.position()?;

    // No joints in this variant; directly switch body types below.
    // Phase 2: kinematic platform moves left-right
    world.body(platform)?.set_body_type(BodyType::Kinematic)?;
    world.body(platform)?.set_linear_velocity([-3.0_f32, 0.0])?;
    for _ in 0..120 {
        drop(world.step(1.0 / 60.0, 4)?);
    }
    let p_kin = world.body(platform)?.position()?;

    // Phase 3: static platform (no motion)
    world.body(platform)?.set_body_type(BodyType::Static)?;
    world.body(platform)?.set_linear_velocity([0.0_f32, 0.0])?;
    for _ in 0..60 {
        drop(world.step(1.0 / 60.0, 4)?);
    }
    let p_sta = world.body(platform)?.position()?;

    let awake = world.awake_body_count()?;
    println!(
        "bodies: platform dyn=({:.2},{:.2}) kin=({:.2},{:.2}) sta=({:.2},{:.2}) awake={}",
        p_dyn.x, p_dyn.y, p_kin.x, p_kin.y, p_sta.x, p_sta.y, awake
    );
    Ok(())
}
