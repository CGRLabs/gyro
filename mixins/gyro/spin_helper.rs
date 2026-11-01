// RETURN hook for Game::spinning_matrix: applies camera roll around Z axis.
// Runs after `camera_view` has aligned this frame's view, so it captures the
// exact yaw/pitch the level renders with (third-person front flip included)
// and the avatar's own mob: the model lock hook cancels the roll with them.
// The local player's body is drawn from `Game::player_model`, the synthetic
// Mob kept in step with the player; `camera_mob()` only names a mob a server
// attached the view to (spectator/remote cameras), never our own avatar.
fn on_return(game: &crate::game::Game<'_>, value: glam::Mat4) -> glam::Mat4 {
    horizon_client::gyro::set_view(game.view.y_rot, game.view.x_rot);
    horizon_client::gyro::set_lock_mob(game.player_model.as_ref().map_or(u32::MAX, |m| m.id));
    let roll = horizon_client::gyro::get_roll();
    if roll.abs() > 0.001 {
        value * glam::Mat4::from_rotation_z(roll.to_radians())
    } else {
        value
    }
}
