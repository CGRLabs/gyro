// RETURN hook on mobs::render::base_transform:
// Locks the gliding player's avatar to the camera roll.
//
// The roll rides in the level's projection (`clip = P · bob · spin · view · model`),
// so the avatar already spins on screen with the world. To pin it to the camera
// instead, cancel the projection roll for this model alone: left-multiply
// `view⁻¹ · Rz(-roll) · view`, a rotation about the render camera's look axis
// through the camera itself. That unwinds both the avatar's on-screen tilt and
// its orbit about the screen centre, leaving the world rolling around it.

fn on_return(m: &crate::mobs::Mob, value: glam::Mat4) -> glam::Mat4 {
    if !m.extra.fall_flying || m.id != crate::gyro::get_lock_mob() {
        return value;
    }
    let roll = crate::gyro::get_roll();
    if roll.abs() <= 0.001 {
        return value;
    }
    let (yaw, pitch) = crate::gyro::get_view();
    let view = crate::camera::view_matrix(yaw, pitch);
    view.inverse() * glam::Mat4::from_rotation_z(-roll.to_radians()) * view * value
}
