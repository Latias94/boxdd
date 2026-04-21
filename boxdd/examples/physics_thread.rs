use boxdd::prelude::*;
use std::sync::mpsc;
use std::thread;

enum PhysicsCmd {
    SpawnBox { position: Vec2 },
    Step { dt: f32, sub_steps: i32 },
    HighestBodyY,
    Shutdown,
}

enum PhysicsReply {
    Spawned,
    Stepped,
    HighestBodyY(Option<f32>),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (cmd_tx, cmd_rx) = mpsc::channel::<PhysicsCmd>();
    let (reply_tx, reply_rx) = mpsc::channel::<PhysicsReply>();

    let physics_thread = thread::spawn(move || {
        // Keep the world on one dedicated thread. This example focuses on the ownership model;
        // Box2D worker threads require explicit raw task-system callbacks and are intentionally
        // not part of this safe-threading example.
        let mut world = World::new(WorldDef::builder().gravity([0.0_f32, -10.0]).build())
            .expect("failed to create world");

        let ground = world.create_body_id(BodyBuilder::new().build());
        let _ground_shape = world.create_segment_shape_for(
            ground,
            &ShapeDef::builder().build(),
            &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0]),
        );

        let shape_def = ShapeDef::builder().density(1.0).build();
        let box_shape = shapes::box_polygon(0.5, 0.5);
        let mut dynamic_bodies = Vec::new();

        loop {
            match cmd_rx.recv().expect("physics command channel closed") {
                PhysicsCmd::SpawnBox { position } => {
                    let body = world.create_body_id(
                        BodyBuilder::new()
                            .body_type(BodyType::Dynamic)
                            .position(position)
                            .build(),
                    );
                    let _shape = world.create_polygon_shape_for(body, &shape_def, &box_shape);
                    dynamic_bodies.push(body);
                    reply_tx.send(PhysicsReply::Spawned).unwrap();
                }
                PhysicsCmd::Step { dt, sub_steps } => {
                    world.step(dt, sub_steps);
                    reply_tx.send(PhysicsReply::Stepped).unwrap();
                }
                PhysicsCmd::HighestBodyY => {
                    let highest_y = dynamic_bodies.iter().fold(None::<f32>, |current, body| {
                        let y = world.body_position(*body).y;
                        Some(current.map_or(y, |best| best.max(y)))
                    });
                    reply_tx
                        .send(PhysicsReply::HighestBodyY(highest_y))
                        .unwrap();
                }
                PhysicsCmd::Shutdown => break,
            }
        }
    });

    for height in [4.0_f32, 6.0] {
        cmd_tx.send(PhysicsCmd::SpawnBox {
            position: Vec2::new(0.0, height),
        })?;
        match reply_rx.recv()? {
            PhysicsReply::Spawned => {}
            _ => unreachable!("unexpected physics reply"),
        }
    }

    for _ in 0..120 {
        cmd_tx.send(PhysicsCmd::Step {
            dt: 1.0 / 60.0,
            sub_steps: 4,
        })?;
        match reply_rx.recv()? {
            PhysicsReply::Stepped => {}
            _ => unreachable!("unexpected physics reply"),
        }
    }

    cmd_tx.send(PhysicsCmd::HighestBodyY)?;
    let highest_y = match reply_rx.recv()? {
        PhysicsReply::HighestBodyY(value) => value,
        _ => unreachable!("unexpected physics reply"),
    };

    println!("physics_thread: highest_dynamic_body_y={highest_y:?}");

    cmd_tx.send(PhysicsCmd::Shutdown)?;
    physics_thread.join().expect("physics thread panicked");
    Ok(())
}
