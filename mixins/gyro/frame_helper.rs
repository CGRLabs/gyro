// Frame hook: the world banks into the turn, and levels out when you stop.
fn on_frame(game: &mut crate::game::Game<'_>, elapsed: std::time::Duration) {
    let gliding = match &game.player {
        Some(p) => p.fall_flying,
        None => false,
    };
    let _ = horizon_client::gyro::tick(gliding, elapsed, game.yaw);
}
