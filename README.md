# boxdd — Rust bindings for Box2D

Crates
- `boxdd-sys`: low-level FFI built from the official Box2D v3 C API via a git submodule.
- `boxdd`: safe, ergonomic layer providing modular APIs for world, bodies, shapes, joints,
  queries, events, and debug draw.

Getting started
1) Initialize submodules
```
git submodule update --init --recursive
```
2) Build
```
cargo build
```
3) Run examples (optional)
```
cargo r --example testbed_imgui_glow --features imgui-glow-testbed
cargo r --example joints
```

Design notes
- This workspace targets Box2D v3 and binds to its C API for portability and simplicity.
- The safe crate exposes both RAII-style wrappers and ID-based APIs to suit different needs. ID
  APIs help avoid borrow/lifetime friction when storing handles in game state.
- `mint` integration: you can pass `mint::Vector2<f32>` and `mint::Point2<f32>` anywhere a vector
  is required; arrays and tuples are also supported.

License
- `boxdd`: MIT OR Apache-2.0
- `boxdd-sys`: MIT OR Apache-2.0

