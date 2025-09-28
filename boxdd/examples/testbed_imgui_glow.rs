//! Dear ImGui + Winit 0.30 + Glow testbed (dear-imgui 0.2 APIs)
//! Enable with: `cargo run -p boxdd --example testbed_imgui_glow --features imgui-glow-testbed`

#[cfg(feature = "imgui-glow-testbed")]
#[path = "testbed/app.rs"]
mod app;

#[cfg(feature = "imgui-glow-testbed")]
fn main() {
    app::run();
}

#[cfg(not(feature = "imgui-glow-testbed"))]
fn main() {
    println!("Enable with --features imgui-glow-testbed");
}
