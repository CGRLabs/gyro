// Frame hook: handle flight rudder and ground leveling.
fn on_frame(game: &mut crate::game::Game<'_>, elapsed: std::time::Duration) {
    let gliding = match &game.player {
        Some(p) => p.fall_flying,
        None => false,
    };
    let _ = horizon_client::gyro::tick(gliding, elapsed);
    if gliding {
        let rudder = horizon_client::gyro::rudder_yaw_delta(elapsed);
        if rudder != 0.0 {
            game.yaw = (game.yaw + rudder).rem_euclid(360.0);
        }
    }
}
