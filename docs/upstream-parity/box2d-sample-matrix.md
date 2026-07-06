# Box2D Sample Parity Matrix

This matrix maps every official Box2D sample registered in `boxdd-sys/third-party/box2d/samples/sample_*.cpp` to the Rust artifact that covers it.
Rows are validated by `cargo run -p xtask -- sample-parity --check`.

## Status Values

- `FaithfulPort` means the Rust artifact is intended to match the official sample behavior.
- `TeachingAdaptation` means the Rust artifact teaches the same API surface with Rust-specific simplification.
- `TestOnly` means the sample is represented by a regression or API test rather than a user-facing example.
- `Deferred` means the sample is intentionally not covered yet and must carry a rationale in the artifact column.
- `UpstreamReference` means the upstream sample is indexed for traceability but has no Rust port yet.

`UpstreamReference` is allowed only for benchmark rows. All non-benchmark rows must name a Rust artifact or an explicit deferral rationale.

## Matrix

| Category | Sample | Status | Artifact | Source |
|---|---|---|---|---|
| `Benchmark` | `Barrel` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:349` |
| `Benchmark` | `Barrel 2.4` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:433` |
| `Benchmark` | `Capacity` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:2109` |
| `Benchmark` | `Cast` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1564` |
| `Benchmark` | `Compound` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1113` |
| `Benchmark` | `CreateDestroy` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:849` |
| `Benchmark` | `Joint Grid` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:990` |
| `Benchmark` | `Kinematic` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1169` |
| `Benchmark` | `Large Pyramid` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:694` |
| `Benchmark` | `Many Pyramids` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:718` |
| `Benchmark` | `Many Tumblers` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:670` |
| `Benchmark` | `Rain` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1646` |
| `Benchmark` | `Sensor` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:2013` |
| `Benchmark` | `Shape Distance` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1788` |
| `Benchmark` | `Sleep` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:966` |
| `Benchmark` | `Smash` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1013` |
| `Benchmark` | `Spinner` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1605` |
| `Benchmark` | `Tumbler` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:456` |
| `Benchmark` | `Washer` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:479` |
| `Bodies` | `Bad` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:752` |
| `Bodies` | `Body Type` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:299` |
| `Bodies` | `Kinematic` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:894` |
| `Bodies` | `Mixed Locks` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:1004` |
| `Bodies` | `Pivot` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:823` |
| `Bodies` | `Set Velocity` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:1058` |
| `Bodies` | `Sleep` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:675` |
| `Bodies` | `Wake Touching` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:1127` |
| `Bodies` | `Weeble` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:424` |
| `Character` | `Mover` | `TeachingAdaptation` | [`boxdd/examples/character_mover.rs`](boxdd/examples/character_mover.rs) | `boxdd-sys/third-party/box2d/samples/sample_character.cpp:1595` |
| `Character` | `Mover` | `TeachingAdaptation` | [`boxdd/examples/character_mover.rs`](boxdd/examples/character_mover.rs) | `boxdd-sys/third-party/box2d/samples/sample_character.cpp:632` |
| `Collision` | `Cast World` | `TeachingAdaptation` | [`boxdd/examples/query_casts.rs`](boxdd/examples/query_casts.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:1878` |
| `Collision` | `Dynamic Tree` | `TeachingAdaptation` | [`boxdd/examples/dynamic_tree.rs`](boxdd/examples/dynamic_tree.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:872` |
| `Collision` | `Manifold` | `TeachingAdaptation` | [`boxdd/tests/manifold_collision.rs`](boxdd/tests/manifold_collision.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:2911` |
| `Collision` | `Overlap World` | `TeachingAdaptation` | [`boxdd/examples/queries.rs`](boxdd/examples/queries.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:2245` |
| `Collision` | `Ray Cast` | `TeachingAdaptation` | [`boxdd/examples/raycast.rs`](boxdd/examples/raycast.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:1201` |
| `Collision` | `Shape Cast` | `TeachingAdaptation` | [`boxdd/examples/shapecast.rs`](boxdd/examples/shapecast.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:3606` |
| `Collision` | `Shape Distance` | `TeachingAdaptation` | [`boxdd/tests/distance.rs`](boxdd/tests/distance.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:438` |
| `Collision` | `Smooth Manifold` | `TeachingAdaptation` | [`boxdd/tests/manifold_collision.rs`](boxdd/tests/manifold_collision.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:3205` |
| `Collision` | `Time of Impact` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:3728` |
| `Continuous` | `Bounce House` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:195` |
| `Continuous` | `Bounce Humans` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:278` |
| `Continuous` | `Chain Drop` | `TeachingAdaptation` | [`boxdd/examples/chain_walkway.rs`](boxdd/examples/chain_walkway.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:374` |
| `Continuous` | `Chain Slide` | `TeachingAdaptation` | [`boxdd/examples/chain_walkway.rs`](boxdd/examples/chain_walkway.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:459` |
| `Continuous` | `Drop` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1558` |
| `Continuous` | `Ghost Bumps` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:939` |
| `Continuous` | `Pinball` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1726` |
| `Continuous` | `Pixel Imperfect` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1150` |
| `Continuous` | `Restitution Threshold` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1220` |
| `Continuous` | `Segment Slide` | `TeachingAdaptation` | [`boxdd/examples/chain_walkway.rs`](boxdd/examples/chain_walkway.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:514` |
| `Continuous` | `Skinny Box` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:644` |
| `Continuous` | `Speculative Fallback` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:990` |
| `Continuous` | `Speculative Ghost` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1083` |
| `Continuous` | `Speculative Sliver` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1034` |
| `Continuous` | `Wedge` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1774` |
| `Determinism` | `Falling Hinges` | `TeachingAdaptation` | [`boxdd/examples/determinism.rs`](boxdd/examples/determinism.rs) | `boxdd-sys/third-party/box2d/samples/sample_determinism.cpp:62` |
| `Events` | `Body Move` | `TeachingAdaptation` | [`boxdd/examples/events_summary.rs`](boxdd/examples/events_summary.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1653` |
| `Events` | `Contact` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/contacts.rs`](boxdd/examples/contacts.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1228` |
| `Events` | `Foot Sensor` | `TeachingAdaptation` | [`boxdd/examples/sensors.rs`](boxdd/examples/sensors.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:811` |
| `Events` | `Joint` | `TeachingAdaptation` | [`boxdd/examples/events_summary.rs`](boxdd/examples/events_summary.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2078` |
| `Events` | `Persistent Contact` | `TeachingAdaptation` | [`boxdd/examples/contacts.rs`](boxdd/examples/contacts.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2180` |
| `Events` | `Platformer` | `TeachingAdaptation` | [`boxdd/examples/events_summary.rs`](boxdd/examples/events_summary.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1463` |
| `Events` | `Projectile Event` | `TeachingAdaptation` | [`boxdd/examples/events_summary.rs`](boxdd/examples/events_summary.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2579` |
| `Events` | `Sensor Bookend` | `TeachingAdaptation` | [`boxdd/examples/sensors.rs`](boxdd/examples/sensors.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:676` |
| `Events` | `Sensor Funnel` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/sensors.rs`](boxdd/examples/sensors.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:340` |
| `Events` | `Sensor Hits` | `TeachingAdaptation` | [`boxdd/examples/sensors.rs`](boxdd/examples/sensors.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2414` |
| `Events` | `Sensor Types` | `TeachingAdaptation` | [`boxdd/examples/sensors.rs`](boxdd/examples/sensors.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1848` |
| `Geometry` | `Convex Hull` | `TeachingAdaptation` | [`boxdd/examples/convex_hull.rs`](boxdd/examples/convex_hull.rs) | `boxdd-sys/third-party/box2d/samples/sample_geometry.cpp:214` |
| `Issues` | `Bad Steiner` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:289` |
| `Issues` | `Crash01` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:513` |
| `Issues` | `Disable` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:376` |
| `Issues` | `Shape Cast Chain` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:240` |
| `Issues` | `StaticVsBulletBug` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:575` |
| `Issues` | `Unstable Prismatic Joints` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:672` |
| `Issues` | `Unstable Windmill` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:756` |
| `Joints` | `Ball & Chain` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1390` |
| `Joints` | `Breakable` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2018` |
| `Joints` | `Bridge` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/bridge.rs`](boxdd/examples/bridge.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1277` |
| `Joints` | `Cantilever` | `TeachingAdaptation` | [`boxdd/examples/bridge.rs`](boxdd/examples/bridge.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1551` |
| `Joints` | `Distance Joint` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:245` |
| `Joints` | `Doohickey` | `TeachingAdaptation` | [`boxdd/examples/doohickey.rs`](boxdd/examples/doohickey.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2797` |
| `Joints` | `Door` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3472` |
| `Joints` | `Driving` | `TeachingAdaptation` | [`boxdd/examples/car.rs`](boxdd/examples/car.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2630` |
| `Joints` | `Filter Joint` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:592` |
| `Joints` | `Gear Lift` | `TeachingAdaptation` | [`boxdd/examples/prismatic_elevator.rs`](boxdd/examples/prismatic_elevator.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3345` |
| `Joints` | `Motion Locks` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1784` |
| `Joints` | `Motor Joint` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:421` |
| `Joints` | `Prismatic` | `TeachingAdaptation` | [`boxdd/examples/prismatic_elevator.rs`](boxdd/examples/prismatic_elevator.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:963` |
| `Joints` | `Ragdoll` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2718` |
| `Joints` | `Revolute` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/revolute_motor.rs`](boxdd/examples/revolute_motor.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:803` |
| `Joints` | `Scale Ragdoll` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3540` |
| `Joints` | `Scissor Lift` | `TeachingAdaptation` | [`boxdd/examples/prismatic_elevator.rs`](boxdd/examples/prismatic_elevator.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3018` |
| `Joints` | `Separation` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2247` |
| `Joints` | `Soft Body` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2751` |
| `Joints` | `Top Down Friction` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:539` |
| `Joints` | `User Constraint` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2369` |
| `Joints` | `Wheel` | `TeachingAdaptation` | [`boxdd/examples/prismatic_wheel.rs`](boxdd/examples/prismatic_wheel.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1097` |
| `Robustness` | `Cart` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:550` |
| `Robustness` | `HighMassRatio1` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:73` |
| `Robustness` | `HighMassRatio2` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:131` |
| `Robustness` | `HighMassRatio3` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:191` |
| `Robustness` | `Multiple Prismatic` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:609` |
| `Robustness` | `Overlap Recovery` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:314` |
| `Robustness` | `Tiny Pyramid` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:377` |
| `Shapes` | `Box Restitution` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1910` |
| `Shapes` | `Chain Link` | `TeachingAdaptation` | [`boxdd/examples/chain_walkway.rs`](boxdd/examples/chain_walkway.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1476` |
| `Shapes` | `Chain Shape` | `TeachingAdaptation` | [`boxdd/examples/chain_walkway.rs`](boxdd/examples/chain_walkway.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:226` |
| `Shapes` | `Compound Shapes` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:448` |
| `Shapes` | `Conveyor Belt` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1094` |
| `Shapes` | `Custom Filter` | `TeachingAdaptation` | [`boxdd/tests/world_callbacks.rs`](boxdd/tests/world_callbacks.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:756` |
| `Shapes` | `Ellipse` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1611` |
| `Shapes` | `Explosion` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1786` |
| `Shapes` | `Filter` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/tests/world_callbacks.rs`](boxdd/tests/world_callbacks.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:661` |
| `Shapes` | `Friction` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:944` |
| `Shapes` | `Modify Geometry` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1395` |
| `Shapes` | `Offset` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1669` |
| `Shapes` | `Recreate Static` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1844` |
| `Shapes` | `Restitution` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:877` |
| `Shapes` | `Rolling Resistance` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1035` |
| `Shapes` | `Rounded` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1546` |
| `Shapes` | `Tangent Speed` | `TeachingAdaptation` | [`boxdd/examples/contacts.rs`](boxdd/examples/contacts.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1233` |
| `Shapes` | `Wind` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:2080` |
| `Stacking` | `Arch` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:801` |
| `Stacking` | `Capsule Stack` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:554` |
| `Stacking` | `Card House` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:1003` |
| `Stacking` | `Circle Impulse` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2727` |
| `Stacking` | `Circle Stack` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:493` |
| `Stacking` | `Cliff` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:703` |
| `Stacking` | `Confined` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:926` |
| `Stacking` | `Double Domino` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:854` |
| `Stacking` | `Single Box` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/basic.rs`](boxdd/examples/basic.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:65` |
| `Stacking` | `Tilted Stack` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/stacking.rs`](boxdd/examples/stacking.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:136` |
| `Stacking` | `Vertical Stack` | `TeachingAdaptation` | [`boxdd/examples/stacking.rs`](boxdd/examples/stacking.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:394` |
| `World` | `Large World` | `TeachingAdaptation` | [`boxdd/examples/world_basics.rs`](boxdd/examples/world_basics.rs) | `boxdd-sys/third-party/box2d/samples/sample_world.cpp:246` |
