use boxdd::prelude::*;
use std::sync::mpsc;
use std::thread;

enum PhysicsCmd {
    SpawnBox { position: Position },
    Step { dt: f32, sub_steps: i32 },
    HighestBodyY,
    Shutdown,
}

enum PhysicsReply {
    Spawned,
    Stepped,
    HighestBodyY(Option<WorldScalar>),
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let foundation = Foundation::initialize_default()?;
    let (cmd_tx, cmd_rx) = mpsc::channel::<PhysicsCmd>();
    let (reply_tx, reply_rx) = mpsc::channel::<PhysicsReply>();

    let physics_thread = thread::spawn(move || -> boxdd::Result<()> {
        // The owner world remains on this dedicated thread. Box2D's validated built-in scheduler
        // may use native workers internally without making World sendable.
        let mut world = foundation.create_world(
            WorldBuilder::from(foundation.world_def())
                .gravity([0.0_f32, -10.0])
                .worker_count(WorkerCount::new(2).expect("native worker count"))
                .build()?,
        )?;

        let ground = world.create_body(BodyBuilder::from(foundation.body_def()).build()?)?;
        let _ground_shape = world.body(ground)?.create_segment(
            &ShapeDef::builder().build()?,
            &shapes::segment([-20.0_f32, 0.0], [20.0, 0.0])?,
        )?;

        let shape_def = ShapeDef::builder().density(1.0).build()?;
        let box_shape = shapes::box_polygon(0.5, 0.5).expect("valid polygon geometry");
        let mut dynamic_bodies = Vec::new();

        loop {
            match cmd_rx.recv().expect("physics command channel closed") {
                PhysicsCmd::SpawnBox { position } => {
                    let body = world.create_body(
                        BodyBuilder::from(foundation.body_def())
                            .body_type(BodyType::Dynamic)
                            .position(position)
                            .build()?,
                    )?;
                    let _shape = world.body(body)?.create_polygon(&shape_def, &box_shape)?;
                    dynamic_bodies.push(body);
                    reply_tx.send(PhysicsReply::Spawned).unwrap();
                }
                PhysicsCmd::Step { dt, sub_steps } => {
                    drop(
                        world
                            .step(dt, sub_steps)
                            .expect("valid physics-thread step"),
                    );
                    reply_tx.send(PhysicsReply::Stepped).unwrap();
                }
                PhysicsCmd::HighestBodyY => {
                    let mut highest_y = None::<WorldScalar>;
                    for body in &dynamic_bodies {
                        let y = world.body(*body)?.position()?.y;
                        highest_y = Some(highest_y.map_or(y, |best| best.max(y)));
                    }
                    reply_tx
                        .send(PhysicsReply::HighestBodyY(highest_y))
                        .unwrap();
                }
                PhysicsCmd::Shutdown => break,
            }
        }
        Ok(())
    });

    let spawn_heights: [WorldScalar; 2] = [4.0, 6.0];
    for height in spawn_heights {
        cmd_tx.send(PhysicsCmd::SpawnBox {
            position: Position::new(0.0, height),
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
    physics_thread.join().expect("physics thread panicked")?;
    Ok(())
}
