// Invert the pitch (Y) mouse axis when the Gyro setting is on.
// Compiled in game::interface::perspective, so it sees $value = the `yo`
// parameter of Game::turn_player (issue #2).
fn invert_pitch(value: f64) -> f64 {
    if mod_gyro::invert_y() {
        -value
    } else {
        value
    }
}
