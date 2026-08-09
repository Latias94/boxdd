use boxdd::ReplayPlayer;
use static_assertions::assert_not_impl_any;

assert_not_impl_any!(ReplayPlayer: Send, Sync);

#[test]
fn replay_player_remains_thread_affine() {}
