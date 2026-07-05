# Box2D Sample Parity Matrix

This matrix maps every official Box2D sample registered in `boxdd-sys/third-party/box2d/samples/sample_*.cpp` to the Rust artifact that covers it.
Rows are validated by `cargo run -p xtask -- sample-parity --check`.

## Status Values

- `FaithfulPort` means the Rust artifact is intended to match the official sample behavior.
- `TeachingAdaptation` means the Rust artifact teaches the same API surface with Rust-specific simplification.
- `TestOnly` means the sample is represented by a regression or API test rather than a user-facing example.
- `Deferred` means the sample is intentionally not covered yet and must carry a rationale in the artifact column.
- `UpstreamReference` means the upstream sample is indexed for traceability but has no Rust port yet.

## Matrix

| Category | Sample | Status | Artifact | Source |
|---|---|---|---|---|
| `Benchmark` | `Barrel` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:349` |
| `Benchmark` | `Barrel 2.4` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:433` |
| `Benchmark` | `Capacity` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:2109` |
| `Benchmark` | `Cast` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1564` |
| `Benchmark` | `Compound` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1113` |
| `Benchmark` | `CreateDestroy` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:849` |
| `Benchmark` | `Joint Grid` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:990` |
| `Benchmark` | `Kinematic` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1169` |
| `Benchmark` | `Large Pyramid` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:694` |
| `Benchmark` | `Many Pyramids` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:718` |
| `Benchmark` | `Many Tumblers` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:670` |
| `Benchmark` | `Rain` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1646` |
| `Benchmark` | `Sensor` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:2013` |
| `Benchmark` | `Shape Distance` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1788` |
| `Benchmark` | `Sleep` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:966` |
| `Benchmark` | `Smash` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1013` |
| `Benchmark` | `Spinner` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1605` |
| `Benchmark` | `Tumbler` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:456` |
| `Benchmark` | `Washer` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:479` |
| `Bodies` | `Bad` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:752` |
| `Bodies` | `Body Type` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:299` |
| `Bodies` | `Kinematic` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:894` |
| `Bodies` | `Mixed Locks` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:1004` |
| `Bodies` | `Pivot` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:823` |
| `Bodies` | `Set Velocity` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:1058` |
| `Bodies` | `Sleep` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:675` |
| `Bodies` | `Wake Touching` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:1127` |
| `Bodies` | `Weeble` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:424` |
| `Character` | `Mover` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_character.cpp:1595` |
| `Character` | `Mover` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_character.cpp:632` |
| `Collision` | `Cast World` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:1878` |
| `Collision` | `Dynamic Tree` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:872` |
| `Collision` | `Manifold` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:2911` |
| `Collision` | `Overlap World` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:2245` |
| `Collision` | `Ray Cast` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:1201` |
| `Collision` | `Shape Cast` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:3606` |
| `Collision` | `Shape Distance` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:438` |
| `Collision` | `Smooth Manifold` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:3205` |
| `Collision` | `Time of Impact` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:3728` |
| `Continuous` | `Bounce House` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:195` |
| `Continuous` | `Bounce Humans` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:278` |
| `Continuous` | `Chain Drop` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:374` |
| `Continuous` | `Chain Slide` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:459` |
| `Continuous` | `Drop` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1558` |
| `Continuous` | `Ghost Bumps` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:939` |
| `Continuous` | `Pinball` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1726` |
| `Continuous` | `Pixel Imperfect` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1150` |
| `Continuous` | `Restitution Threshold` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1220` |
| `Continuous` | `Segment Slide` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:514` |
| `Continuous` | `Skinny Box` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:644` |
| `Continuous` | `Speculative Fallback` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:990` |
| `Continuous` | `Speculative Ghost` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1083` |
| `Continuous` | `Speculative Sliver` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1034` |
| `Continuous` | `Wedge` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1774` |
| `Determinism` | `Falling Hinges` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_determinism.cpp:62` |
| `Events` | `Body Move` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1653` |
| `Events` | `Contact` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1228` |
| `Events` | `Foot Sensor` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:811` |
| `Events` | `Joint` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2078` |
| `Events` | `Persistent Contact` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2180` |
| `Events` | `Platformer` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1463` |
| `Events` | `Projectile Event` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2579` |
| `Events` | `Sensor Bookend` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:676` |
| `Events` | `Sensor Funnel` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:340` |
| `Events` | `Sensor Hits` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2414` |
| `Events` | `Sensor Types` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1848` |
| `Geometry` | `Convex Hull` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_geometry.cpp:214` |
| `Issues` | `Bad Steiner` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:289` |
| `Issues` | `Crash01` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:513` |
| `Issues` | `Disable` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:376` |
| `Issues` | `Shape Cast Chain` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:240` |
| `Issues` | `StaticVsBulletBug` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:575` |
| `Issues` | `Unstable Prismatic Joints` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:672` |
| `Issues` | `Unstable Windmill` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:756` |
| `Joints` | `Ball & Chain` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1390` |
| `Joints` | `Breakable` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2018` |
| `Joints` | `Bridge` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1277` |
| `Joints` | `Cantilever` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1551` |
| `Joints` | `Distance Joint` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:245` |
| `Joints` | `Doohickey` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2797` |
| `Joints` | `Door` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3472` |
| `Joints` | `Driving` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2630` |
| `Joints` | `Filter Joint` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:592` |
| `Joints` | `Gear Lift` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3345` |
| `Joints` | `Motion Locks` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1784` |
| `Joints` | `Motor Joint` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:421` |
| `Joints` | `Prismatic` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:963` |
| `Joints` | `Ragdoll` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2718` |
| `Joints` | `Revolute` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:803` |
| `Joints` | `Scale Ragdoll` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3540` |
| `Joints` | `Scissor Lift` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3018` |
| `Joints` | `Separation` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2247` |
| `Joints` | `Soft Body` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2751` |
| `Joints` | `Top Down Friction` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:539` |
| `Joints` | `User Constraint` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2369` |
| `Joints` | `Wheel` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1097` |
| `Robustness` | `Cart` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:550` |
| `Robustness` | `HighMassRatio1` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:73` |
| `Robustness` | `HighMassRatio2` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:131` |
| `Robustness` | `HighMassRatio3` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:191` |
| `Robustness` | `Multiple Prismatic` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:609` |
| `Robustness` | `Overlap Recovery` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:314` |
| `Robustness` | `Tiny Pyramid` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:377` |
| `Shapes` | `Box Restitution` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1910` |
| `Shapes` | `Chain Link` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1476` |
| `Shapes` | `Chain Shape` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:226` |
| `Shapes` | `Compound Shapes` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:448` |
| `Shapes` | `Conveyor Belt` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1094` |
| `Shapes` | `Custom Filter` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:756` |
| `Shapes` | `Ellipse` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1611` |
| `Shapes` | `Explosion` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1786` |
| `Shapes` | `Filter` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:661` |
| `Shapes` | `Friction` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:944` |
| `Shapes` | `Modify Geometry` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1395` |
| `Shapes` | `Offset` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1669` |
| `Shapes` | `Recreate Static` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1844` |
| `Shapes` | `Restitution` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:877` |
| `Shapes` | `Rolling Resistance` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1035` |
| `Shapes` | `Rounded` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1546` |
| `Shapes` | `Tangent Speed` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1233` |
| `Shapes` | `Wind` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:2080` |
| `Stacking` | `Arch` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:801` |
| `Stacking` | `Capsule Stack` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:554` |
| `Stacking` | `Card House` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:1003` |
| `Stacking` | `Circle Impulse` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2727` |
| `Stacking` | `Circle Stack` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:493` |
| `Stacking` | `Cliff` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:703` |
| `Stacking` | `Confined` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:926` |
| `Stacking` | `Double Domino` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:854` |
| `Stacking` | `Single Box` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:65` |
| `Stacking` | `Tilted Stack` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:136` |
| `Stacking` | `Vertical Stack` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:394` |
| `World` | `Large World` | `UpstreamReference` | Upstream sample indexed; Rust port not assigned yet. | `boxdd-sys/third-party/box2d/samples/sample_world.cpp:246` |
