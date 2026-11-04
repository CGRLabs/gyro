// Gyro runtime: elytra flight on the mouse, in any direction.
// While gliding, mouse X rolls the view around the look axis, mouse Y pitches
// it through that roll (so a banked pull-up carves a turn), and A / D work the
// rudder. Rolling ends the moment you stop gliding: the view levels out.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

/// The mod's row in Options > Mods. `version` here and in horizon-mixin.toml are kept the same.
pub const ID: &str = "gyro";
pub const VERSION: &str = "1.4.1";
pub const DESCRIPTION: &str =
    "Fly an elytra like an aircraft: while you glide, the mouse rolls and pitches the view instead of only turning it, A and D work the rudder, and you stay upright on screen while the world spins around you.";

/// Whether rudder left (A key) is held.
pub static RUDDER_LEFT: AtomicBool = AtomicBool::new(false);

/// Whether rudder right (D key) is held.
pub static RUDDER_RIGHT: AtomicBool = AtomicBool::new(false);

/// Current camera roll angle in degrees [0, 360).
pub static ROLL_DEGREES: AtomicU32 = AtomicU32::new(0);

/// The render camera's yaw in degrees, captured each frame from the view
/// the level is drawn with (third-person flips included).
pub static VIEW_YAW: AtomicU32 = AtomicU32::new(0);

/// The render camera's pitch in degrees.
pub static VIEW_PITCH: AtomicU32 = AtomicU32::new(0);

/// Entity id of the camera's mob: only that avatar locks to the roll.
pub static LOCK_MOB: AtomicU32 = AtomicU32::new(u32::MAX);

/// Get the current camera roll angle in degrees.
pub fn get_roll() -> f32 {
    f32::from_bits(ROLL_DEGREES.load(Ordering::Relaxed))
}

/// Set the current camera roll angle in degrees.
pub fn set_roll(deg: f32) {
    let normalized = deg.rem_euclid(360.0);
    ROLL_DEGREES.store(normalized.to_bits(), Ordering::Relaxed);
}

/// Record the camera orientation the frame renders with.
pub fn set_view(yaw: f32, pitch: f32) {
    VIEW_YAW.store(yaw.to_bits(), Ordering::Relaxed);
    VIEW_PITCH.store(pitch.to_bits(), Ordering::Relaxed);
}

/// The camera orientation recorded by `set_view`.
pub fn get_view() -> (f32, f32) {
    (
        f32::from_bits(VIEW_YAW.load(Ordering::Relaxed)),
        f32::from_bits(VIEW_PITCH.load(Ordering::Relaxed)),
    )
}

/// Record which entity the camera belongs to (`u32::MAX`: none).
pub fn set_lock_mob(id: u32) {
    LOCK_MOB.store(id, Ordering::Relaxed);
}

/// The entity id whose avatar locks to the camera roll.
pub fn get_lock_mob() -> u32 {
    LOCK_MOB.load(Ordering::Relaxed)
}

/// Transform mouse movements `(xo, yo)` into flight updates:
/// - `xo` directly adds to roll (barrel rolling / banking with the mouse).
/// - `yo` pitches the aircraft in body coordinates.
/// - Returns `(pitch_delta, yaw_delta)` transformed to world coordinates based on current roll.
pub fn calculate_flight_turn(xo: f64, yo: f64, current_roll: f32) -> (f32, f32, f32) {
    // Mouse X directly rotates the roll axis (0.15 deg per unit)
    let roll_delta = (xo as f32) * 0.15;
    let new_roll = (current_roll + roll_delta).rem_euclid(360.0);

    let phi = new_roll.to_radians();
    let cos_phi = phi.cos();
    let sin_phi = phi.sin();

    // Mouse Y is body pitch input (yo * 0.15)
    let pitch_input = (yo as f32) * 0.15;

    // Body pitch projected onto world pitch and world yaw:
    // When level (phi = 0): pitch = pitch_input, yaw = 0
    // When banked 90 deg right (phi = 90 deg): pulling up (pitch_input < 0) turns right in yaw
    let pitch_delta = pitch_input * cos_phi;
    let yaw_delta = -pitch_input * sin_phi;

    (pitch_delta, yaw_delta, new_roll)
}

/// Advance flight state per frame.
/// While gliding: does NOT auto-snap roll; applies rudder turn if A/D are pressed.
/// When landed on the ground: smoothly returns roll back to 0.
pub fn tick(gliding: bool, elapsed: Duration) -> f32 {
    let dt = elapsed.as_secs_f32().min(0.5);
    let mut current_roll = get_roll();

    if !gliding {
        // Only level out to 0.0 when on the ground / not flying
        let mut norm = current_roll;
        if norm > 180.0 {
            norm -= 360.0;
        }

        if norm.abs() > 0.001 {
            let speed = 270.0 * dt;
            if norm.abs() <= speed {
                current_roll = 0.0;
            } else if norm > 0.0 {
                current_roll = norm - speed;
            } else {
                current_roll = norm + speed;
            }
            current_roll = current_roll.rem_euclid(360.0);
        } else {
            current_roll = 0.0;
        }
        set_roll(current_roll);
    }

    current_roll
}

/// Calculate rudder yaw delta from A/D keys (in degrees per frame)
pub fn rudder_yaw_delta(elapsed: Duration) -> f32 {
    let dt = elapsed.as_secs_f32().min(0.5);
    let rudder_speed = 90.0 * dt; // 90 degrees per second
    let left = RUDDER_LEFT.load(Ordering::Relaxed);
    let right = RUDDER_RIGHT.load(Ordering::Relaxed);
    match (left, right) {
        (true, false) => -rudder_speed,
        (false, true) => rudder_speed,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_x_controls_roll() {
        // Moving mouse right (xo = 100) rolls right by 15 degrees
        let (_, _, new_roll) = calculate_flight_turn(100.0, 0.0, 0.0);
        assert!((new_roll - 15.0).abs() < 1e-4);
    }

    #[test]
    fn test_banked_pitch_turns_yaw() {
        // Banked 90 deg right (phi = 90 deg):
        // Pulling up (yo = -100) turns yaw to the right
        let (p_delta, y_delta, _) = calculate_flight_turn(0.0, -100.0, 90.0);
        assert!(p_delta.abs() < 1e-3, "world pitch delta should be near 0");
        assert!(y_delta > 0.0, "should turn yaw right into the bank");
    }

    #[test]
    fn test_no_snap_back_while_gliding() {
        set_roll(180.0);
        let r = tick(true, Duration::from_secs_f32(0.5));
        assert_eq!(r, 180.0, "roll must stay at 180 deg without snapping back while flying");
    }

    #[test]
    fn test_levels_out_on_ground() {
        set_roll(90.0);
        let r = tick(false, Duration::from_secs_f32(0.5));
        assert!(r < 90.0, "roll should return toward 0 when on the ground");
    }

    #[test]
    fn test_view_round_trip() {
        set_view(-135.25, 41.75);
        let (yaw, pitch) = get_view();
        assert_eq!((yaw, pitch), (-135.25, 41.75));
    }

    #[test]
    fn test_lock_mob_round_trip() {
        set_lock_mob(7);
        assert_eq!(get_lock_mob(), 7);
        set_lock_mob(u32::MAX);
        assert_eq!(get_lock_mob(), u32::MAX);
    }
}
