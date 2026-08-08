# WASM Examples

This directory contains low-level WASM provider smoke code that is built through the Box2D provider runtime instead of the native example runner. User-facing browser examples live in `bevy_boxdd/examples/testbed_2d` and are published through `docs/pages`.

## Browser Provider Runtime

Browser builds use `BOXDD_SYS_PROVIDER=wasm-provider`. In that mode the Rust wasm
module imports Box2D C API symbols from an Emscripten-built provider module named
`box2d-sys-v2-single` (or `box2d-sys-v2-double` for the double-precision route), and
both modules share one `WebAssembly.Memory`.
Provider ABI v2 partitions that memory: Emscripten static data, stack, and `emmalloc` stay below
64 MiB, while Rust data, stack, and `System` allocation start at 64 MiB. Only Rust may grow the
memory. The smoke retains 81 MiB of Rust allocations, including a 64 KiB-aligned block, while
stepping a live Box2D world; it verifies Rust-owned growth, retained contents, refreshed provider
views, and release back to the pre-pressure Box2D byte count.
The provider selector is set internally by the listed `xtask` entry points together with a
versioned final-link opt-in and the complete shared-memory linker contract. Setting it directly in
an ordinary Cargo build is unsupported and fails closed. The final Wasm is structurally checked so
that neither module can place data, stack, or heap state in the other module's partition.

```bash
rustup target add wasm32-unknown-unknown
cargo run -p xtask -- provider-smoke-app

# Runtime commands use an activated Emscripten 6.0.4 installation.
git clone https://github.com/emscripten-core/emsdk.git /absolute/path/emsdk
git -C /absolute/path/emsdk checkout 224ec5f9f2f72f09f9ce0e26d66bae7dbd8b692f
/absolute/path/emsdk/emsdk install 6.0.4
/absolute/path/emsdk/emsdk activate 6.0.4
source /absolute/path/emsdk/emsdk_env.sh
cargo run -p xtask -- wasm-provider-contract --check
cargo run -p xtask -- provider-smoke
cargo run -p xtask -- verify-wasm --runtime

# Pages generation uses the same activated toolchain.
cargo run -p xtask -- build-pages-wasm
npm run test:pages-browser
```

The smoke runtime verifies world stepping, closest ray casts, standalone collision helpers, distance joints, and fail-closed rejection of raw task/material callbacks without relying on cross-module callback tables. Rust callback registration, callback-backed world and recording-session queries, dynamic-tree traversal, and debug draw are compile-time unavailable on `wasm32` until that table ABI has its own runtime proof. `verify-wasm --compile-only` includes negative probes for those boundaries. `build-pages-wasm` reuses the provider path, then builds the Bevy + egui testbed and writes current browser assets under `docs/pages/wasm/generated` and `docs/pages/bevy-testbed/generated-v2`. `test:pages-browser` runs the generated testbed, grows the shared memory through its explicit proof path, verifies detached-view refresh and shared read/write visibility, and requires a post-growth physics step.
