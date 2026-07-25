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
| `Benchmark` | `Barrel` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:336` |
| `Benchmark` | `Barrel 2.4` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:420` |
| `Benchmark` | `Capacity` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:2119` |
| `Benchmark` | `Cast` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1583` |
| `Benchmark` | `Compounds` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:444` |
| `Benchmark` | `CreateDestroy` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:870` |
| `Benchmark` | `Joint Grid` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1011` |
| `Benchmark` | `Junkyard` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:2152` |
| `Benchmark` | `Kinematic` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1190` |
| `Benchmark` | `Large Compounds` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1134` |
| `Benchmark` | `Large Pyramid` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:709` |
| `Benchmark` | `Many Pyramids` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:739` |
| `Benchmark` | `Many Tumblers` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:685` |
| `Benchmark` | `Rain` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1665` |
| `Benchmark` | `Sensor` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:2025` |
| `Benchmark` | `Shape Distance` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1800` |
| `Benchmark` | `Sleep` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:987` |
| `Benchmark` | `Smash` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1034` |
| `Benchmark` | `Spinner` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:1624` |
| `Benchmark` | `Tumbler` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:467` |
| `Benchmark` | `Washer` | `UpstreamReference` | Upstream performance sample indexed; exact benchmark parity is not assigned to the safe API examples. | `boxdd-sys/third-party/box2d/samples/sample_benchmark.cpp:498` |
| `Bodies` | `Bad` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:735` |
| `Bodies` | `Body Type` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:293` |
| `Bodies` | `Kinematic` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:877` |
| `Bodies` | `Mixed Locks` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:987` |
| `Bodies` | `Pivot` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:806` |
| `Bodies` | `Set Velocity` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:1041` |
| `Bodies` | `Sleep` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:658` |
| `Bodies` | `Wake Touching` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:1103` |
| `Bodies` | `Weeble` | `TeachingAdaptation` | [`boxdd/examples/bodies.rs`](boxdd/examples/bodies.rs) | `boxdd-sys/third-party/box2d/samples/sample_bodies.cpp:413` |
| `Character` | `Mover` | `TeachingAdaptation` | [`boxdd/examples/character_mover.rs`](boxdd/examples/character_mover.rs) | `boxdd-sys/third-party/box2d/samples/sample_character.cpp:624` |
| `Collision` | `Cast World` | `TeachingAdaptation` | [`boxdd/examples/query_casts.rs`](boxdd/examples/query_casts.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:1856` |
| `Collision` | `Dynamic Tree` | `TeachingAdaptation` | [`boxdd/examples/dynamic_tree.rs`](boxdd/examples/dynamic_tree.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:867` |
| `Collision` | `Manifold` | `TeachingAdaptation` | [`boxdd/tests/manifold_collision.rs`](boxdd/tests/manifold_collision.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:2880` |
| `Collision` | `Overlap World` | `TeachingAdaptation` | [`boxdd/examples/queries.rs`](boxdd/examples/queries.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:2218` |
| `Collision` | `Ray Cast` | `TeachingAdaptation` | [`boxdd/examples/raycast.rs`](boxdd/examples/raycast.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:1188` |
| `Collision` | `Shape Cast` | `TeachingAdaptation` | [`boxdd/examples/shapecast.rs`](boxdd/examples/shapecast.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:3570` |
| `Collision` | `Shape Distance` | `TeachingAdaptation` | [`boxdd/tests/distance.rs`](boxdd/tests/distance.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:439` |
| `Collision` | `Smooth Manifold` | `TeachingAdaptation` | [`boxdd/tests/manifold_collision.rs`](boxdd/tests/manifold_collision.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:3171` |
| `Collision` | `Time of Impact` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_collision.cpp:3659` |
| `Continuous` | `Bounce House` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:190` |
| `Continuous` | `Bounce Humans` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:273` |
| `Continuous` | `Chain Drop` | `TeachingAdaptation` | [`boxdd/examples/chain_walkway.rs`](boxdd/examples/chain_walkway.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:364` |
| `Continuous` | `Chain Slide` | `TeachingAdaptation` | [`boxdd/examples/chain_walkway.rs`](boxdd/examples/chain_walkway.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:449` |
| `Continuous` | `Drop` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1536` |
| `Continuous` | `Ghost Bumps` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:917` |
| `Continuous` | `Pinball` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1710` |
| `Continuous` | `Pixel Imperfect` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1128` |
| `Continuous` | `Restitution Threshold` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1198` |
| `Continuous` | `Segment Slide` | `TeachingAdaptation` | [`boxdd/examples/chain_walkway.rs`](boxdd/examples/chain_walkway.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:504` |
| `Continuous` | `Skinny Box` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:627` |
| `Continuous` | `Speculative Fallback` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:968` |
| `Continuous` | `Speculative Ghost` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1061` |
| `Continuous` | `Speculative Sliver` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1012` |
| `Continuous` | `Wedge` | `TeachingAdaptation` | [`boxdd/examples/continuous_bullet.rs`](boxdd/examples/continuous_bullet.rs) | `boxdd-sys/third-party/box2d/samples/sample_continuous.cpp:1758` |
| `Determinism` | `Falling Hinges` | `TeachingAdaptation` | [`boxdd/examples/determinism.rs`](boxdd/examples/determinism.rs) | `boxdd-sys/third-party/box2d/samples/sample_determinism.cpp:94` |
| `Determinism` | `SnapShot` | `TeachingAdaptation` | [`boxdd/examples/determinism.rs`](boxdd/examples/determinism.rs) | `boxdd-sys/third-party/box2d/samples/sample_determinism.cpp:167` |
| `Events` | `Body Move` | `TeachingAdaptation` | [`boxdd/examples/events_summary.rs`](boxdd/examples/events_summary.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1634` |
| `Events` | `Circle Impulse` | `TeachingAdaptation` | [`boxdd/examples/events_summary.rs`](boxdd/examples/events_summary.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2696` |
| `Events` | `Contact` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/contacts.rs`](boxdd/examples/contacts.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1219` |
| `Events` | `Foot Sensor` | `TeachingAdaptation` | [`boxdd/examples/sensors.rs`](boxdd/examples/sensors.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:797` |
| `Events` | `Joint` | `TeachingAdaptation` | [`boxdd/examples/events_summary.rs`](boxdd/examples/events_summary.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2059` |
| `Events` | `Persistent Contact` | `TeachingAdaptation` | [`boxdd/examples/contacts.rs`](boxdd/examples/contacts.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2162` |
| `Events` | `Platformer` | `TeachingAdaptation` | [`boxdd/examples/events_summary.rs`](boxdd/examples/events_summary.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1449` |
| `Events` | `Projectile Event` | `TeachingAdaptation` | [`boxdd/examples/events_summary.rs`](boxdd/examples/events_summary.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2555` |
| `Events` | `Sensor Bookend` | `TeachingAdaptation` | [`boxdd/examples/sensors.rs`](boxdd/examples/sensors.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:662` |
| `Events` | `Sensor Funnel` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/sensors.rs`](boxdd/examples/sensors.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:333` |
| `Events` | `Sensor Hits` | `TeachingAdaptation` | [`boxdd/examples/sensors.rs`](boxdd/examples/sensors.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:2389` |
| `Events` | `Sensor Types` | `TeachingAdaptation` | [`boxdd/examples/sensors.rs`](boxdd/examples/sensors.rs) | `boxdd-sys/third-party/box2d/samples/sample_events.cpp:1829` |
| `Geometry` | `Convex Hull` | `TeachingAdaptation` | [`boxdd/examples/convex_hull.rs`](boxdd/examples/convex_hull.rs) | `boxdd-sys/third-party/box2d/samples/sample_geometry.cpp:214` |
| `Issues` | `Bad Steiner` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:54` |
| `Issues` | `Crash01` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:264` |
| `Issues` | `Disable` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:133` |
| `Issues` | `StaticVsBulletBug` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:326` |
| `Issues` | `Unstable Prismatic Joints` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:423` |
| `Issues` | `Unstable Windmill` | `TeachingAdaptation` | [`boxdd/examples/issues.rs`](boxdd/examples/issues.rs) | `boxdd-sys/third-party/box2d/samples/sample_issues.cpp:507` |
| `Joints` | `Ball & Chain` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1354` |
| `Joints` | `Breakable` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1968` |
| `Joints` | `Bridge` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/bridge.rs`](boxdd/examples/bridge.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1246` |
| `Joints` | `Cantilever` | `TeachingAdaptation` | [`boxdd/examples/bridge.rs`](boxdd/examples/bridge.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1510` |
| `Joints` | `Distance Joint` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:240` |
| `Joints` | `Doohickey` | `TeachingAdaptation` | [`boxdd/examples/doohickey.rs`](boxdd/examples/doohickey.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2732` |
| `Joints` | `Door` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3398` |
| `Joints` | `Driving` | `TeachingAdaptation` | [`boxdd/examples/car.rs`](boxdd/examples/car.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2569` |
| `Joints` | `Filter Joint` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:577` |
| `Joints` | `Gear Lift` | `TeachingAdaptation` | [`boxdd/examples/prismatic_elevator.rs`](boxdd/examples/prismatic_elevator.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3274` |
| `Joints` | `Motion Locks` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1736` |
| `Joints` | `Motor Joint` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:413` |
| `Joints` | `Prismatic` | `TeachingAdaptation` | [`boxdd/examples/prismatic_elevator.rs`](boxdd/examples/prismatic_elevator.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:942` |
| `Joints` | `Ragdoll` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2653` |
| `Joints` | `Revolute` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/revolute_motor.rs`](boxdd/examples/revolute_motor.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:785` |
| `Joints` | `Scale Ragdoll` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:3461` |
| `Joints` | `Scissor Lift` | `TeachingAdaptation` | [`boxdd/examples/prismatic_elevator.rs`](boxdd/examples/prismatic_elevator.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2950` |
| `Joints` | `Separation` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2194` |
| `Joints` | `Soft Body` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2686` |
| `Joints` | `Top Down Friction` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:524` |
| `Joints` | `User Constraint` | `TeachingAdaptation` | [`boxdd/examples/joints.rs`](boxdd/examples/joints.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:2315` |
| `Joints` | `Wheel` | `TeachingAdaptation` | [`boxdd/examples/prismatic_wheel.rs`](boxdd/examples/prismatic_wheel.rs) | `boxdd-sys/third-party/box2d/samples/sample_joints.cpp:1073` |
| `Replay` | `Viewer` | `TestOnly` | [`boxdd/tests/replay.rs`](boxdd/tests/replay.rs) | `boxdd-sys/third-party/box2d/samples/sample_replay.cpp:1243` |
| `Robustness` | `Cart` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:543` |
| `Robustness` | `HighMassRatio1` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:73` |
| `Robustness` | `HighMassRatio2` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:131` |
| `Robustness` | `HighMassRatio3` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:191` |
| `Robustness` | `Multiple Prismatic` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:602` |
| `Robustness` | `Overlap Recovery` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:309` |
| `Robustness` | `Tiny Pyramid` | `TeachingAdaptation` | [`boxdd/examples/robustness.rs`](boxdd/examples/robustness.rs) | `boxdd-sys/third-party/box2d/samples/sample_robustness.cpp:375` |
| `Shapes` | `Box Restitution` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:2057` |
| `Shapes` | `Chain Link` | `TeachingAdaptation` | [`boxdd/examples/chain_walkway.rs`](boxdd/examples/chain_walkway.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1628` |
| `Shapes` | `Chain Segment` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:404` |
| `Shapes` | `Chain Shape` | `TeachingAdaptation` | [`boxdd/examples/chain_walkway.rs`](boxdd/examples/chain_walkway.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:223` |
| `Shapes` | `Compound Shapes` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:619` |
| `Shapes` | `Conveyor Belt` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1255` |
| `Shapes` | `Custom Filter` | `TeachingAdaptation` | [`boxdd/tests/world_callbacks.rs`](boxdd/tests/world_callbacks.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:920` |
| `Shapes` | `Ellipse` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1763` |
| `Shapes` | `Explosion` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1933` |
| `Shapes` | `Filter` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/tests/world_callbacks.rs`](boxdd/tests/world_callbacks.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:825` |
| `Shapes` | `Friction` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1105` |
| `Shapes` | `Modify Geometry` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1547` |
| `Shapes` | `Offset` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1821` |
| `Shapes` | `Recreate Static` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1991` |
| `Shapes` | `Restitution` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1038` |
| `Shapes` | `Rolling Resistance` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1196` |
| `Shapes` | `Rounded` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1698` |
| `Shapes` | `Tangent Speed` | `TeachingAdaptation` | [`boxdd/examples/contacts.rs`](boxdd/examples/contacts.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:1390` |
| `Shapes` | `Wind` | `TeachingAdaptation` | [`boxdd/examples/shapes_variety.rs`](boxdd/examples/shapes_variety.rs) | `boxdd-sys/third-party/box2d/samples/sample_shapes.cpp:2222` |
| `Stacking` | `Arch` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:809` |
| `Stacking` | `Capsule Stack` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:569` |
| `Stacking` | `Card House` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:1011` |
| `Stacking` | `Circle Stack` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:508` |
| `Stacking` | `Cliff` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:711` |
| `Stacking` | `Confined` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:934` |
| `Stacking` | `Double Domino` | `TeachingAdaptation` | [`boxdd/examples/pyramid.rs`](boxdd/examples/pyramid.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:862` |
| `Stacking` | `Single Box` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/basic.rs`](boxdd/examples/basic.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:65` |
| `Stacking` | `Tilted Stack` | `TeachingAdaptation` | [`bevy_boxdd/examples/testbed_2d/scenes.rs`](bevy_boxdd/examples/testbed_2d/scenes.rs), [`boxdd/examples/stacking.rs`](boxdd/examples/stacking.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:136` |
| `Stacking` | `Vertical Stack` | `TeachingAdaptation` | [`boxdd/examples/stacking.rs`](boxdd/examples/stacking.rs) | `boxdd-sys/third-party/box2d/samples/sample_stacking.cpp:408` |
| `World` | `Far Gate` | `TeachingAdaptation` | [`boxdd/examples/world_basics.rs`](boxdd/examples/world_basics.rs) | `boxdd-sys/third-party/box2d/samples/sample_world.cpp:741` |
| `World` | `Far Pyramid` | `TeachingAdaptation` | [`boxdd/examples/world_basics.rs`](boxdd/examples/world_basics.rs) | `boxdd-sys/third-party/box2d/samples/sample_world.cpp:315` |
| `World` | `Far Ragdolls` | `TeachingAdaptation` | [`boxdd/examples/world_basics.rs`](boxdd/examples/world_basics.rs) | `boxdd-sys/third-party/box2d/samples/sample_world.cpp:390` |
| `World` | `Tiles` | `TeachingAdaptation` | [`boxdd/examples/world_basics.rs`](boxdd/examples/world_basics.rs) | `boxdd-sys/third-party/box2d/samples/sample_world.cpp:243` |
