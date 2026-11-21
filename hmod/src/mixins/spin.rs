// RETURN mixin on Game::spinning_matrix: lean the whole rendered world around
// the screen centre by the current bank. Every model tilts with it.
fn on_return(value: glam::Mat4) -> glam::Mat4 {
    let roll = mod_gyro::get_roll();
    if roll.abs() > 0.001 {
        value * glam::Mat4::from_rotation_z(roll.to_radians())
    } else {
        value
    }
}
