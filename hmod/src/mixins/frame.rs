// HEAD mixin on Game::render_and_flip: advance the bank once per frame.
// Compiled in the game.rs module, so private Game fields are visible here.
fn on_frame(game: &mut crate::game::Game<'_>, elapsed: std::time::Duration) {
    let gliding = game.player.as_ref().is_some_and(|p| p.fall_flying);
    let _ = mod_gyro::tick(gliding, elapsed, game.yaw);
}
