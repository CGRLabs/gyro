// Key hook: handle A / D rudder keys during elytra flight.
fn on_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    let _ = game;
    let left = key == "key.keyboard.a" || key == "key.a" || key == "key.left";
    let right = key == "key.keyboard.d" || key == "key.d" || key == "key.right";
    if left {
        horizon_client::gyro::RUDDER_LEFT.store(pressed, std::sync::atomic::Ordering::Relaxed);
    }
    if right {
        horizon_client::gyro::RUDDER_RIGHT.store(pressed, std::sync::atomic::Ordering::Relaxed);
    }
}
