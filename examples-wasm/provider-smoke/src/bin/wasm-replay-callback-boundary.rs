#[cfg(not(target_arch = "wasm32"))]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {
    let _ = boxdd::ReplayConfig::default()
        .with_friction_mixer(|left, right| left.coefficient.max(right.coefficient));
    let _ = boxdd::ReplayConfig::default()
        .with_restitution_mixer(|left, right| left.coefficient.max(right.coefficient));

    fn draw(player: &mut boxdd::ReplayPlayer) {
        let _ = player.draw(&mut (), boxdd::DebugDrawOptions::default(), None);
    }
    let _ = draw;
}
