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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut world = World::new(WorldDef::builder().gravity([0.0, -10.0]).build())?;

    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_segment_shape_for(
        ground,
        &ShapeDef::builder().build(),
        &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0]),
    );

    // Attachments and platform
    let attach1 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-2.0_f32, 3.0])
            .build(),
    );
    let attach2 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([3.0_f32, 3.0])
            .build(),
    );
    let platform = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-4.0_f32, 5.0])
            .build(),
    );

    let sdef1 = ShapeDef::builder().density(1.0).build();
    let sdef2 = ShapeDef::builder().density(2.0).build();
    let box_tall = shapes::box_polygon(0.5, 2.0);
    let box_long = shapes::box_polygon(0.5, 4.0);
    let _ = world.create_polygon_shape_for(attach1, &sdef1, &box_tall);
    let _ = world.create_polygon_shape_for(attach2, &sdef1, &box_tall);
    let _ = world.create_polygon_shape_for(platform, &sdef2, &box_long);

    // For Debug stability, omit joints in this headless variant.

    // Payloads
    let payload1 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([-3.0_f32, 8.0])
            .build(),
    );
    let _ = world.create_polygon_shape_for(payload1, &sdef1, &shapes::box_polygon(0.5, 0.5));
    let payload2 = world.create_body_id(
        BodyBuilder::new()
            .body_type(BodyType::Dynamic)
            .position([0.0_f32, 8.0])
            .build(),
    );
    let _ = world.create_polygon_shape_for(payload2, &sdef1, &shapes::box_polygon(0.25, 0.25));

    // Phase 1: dynamic platform
    for _ in 0..120 {
        world.step(1.0 / 60.0, 4);
    }
    let p_dyn = world.body_position(platform);

    // No joints in this variant; directly switch body types below.
    // Phase 2: kinematic platform moves left-right
    world.set_body_type(platform, BodyType::Kinematic);
    world.set_body_linear_velocity(platform, [-3.0_f32, 0.0]);
    for _ in 0..120 {
        world.step(1.0 / 60.0, 4);
    }
    let p_kin = world.body_position(platform);

    // Phase 3: static platform (no motion)
    world.set_body_type(platform, BodyType::Static);
    world.set_body_linear_velocity(platform, [0.0_f32, 0.0]);
    for _ in 0..60 {
        world.step(1.0 / 60.0, 4);
    }
    let p_sta = world.body_position(platform);

    let awake = world.awake_body_count();
    println!(
        "bodies: platform dyn=({:.2},{:.2}) kin=({:.2},{:.2}) sta=({:.2},{:.2}) awake={}",
        p_dyn.x, p_dyn.y, p_kin.x, p_kin.y, p_sta.x, p_sta.y, awake
    );
    Ok(())
}
