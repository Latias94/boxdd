//! Dear ImGui + Winit 0.30 + Glow testbed (dear-imgui 0.11 stack)
//! Enable with: `cargo run -p boxdd --example testbed_imgui_glow --features imgui-glow-testbed`

mod app;
mod debug_draw;
mod scenes;

fn main() {
    app::run();
}
