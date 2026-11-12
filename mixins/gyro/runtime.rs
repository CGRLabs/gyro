// Barrel Roll runtime: elytra flight on the mouse, in any direction.
//
// While gliding, mouse X turns you and mouse Y pitches you, exactly as the
// game's own look does — so you fly where you look. The camera banks into the
// turn, the world leaning around you, and holding the turn rolls the world
// right around and back: a barrel roll. The bank levels out when you stop
// turning, and the whole view levels out when you stop gliding.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// The mod's row in Options > Mods. `version` here and in horizon-mixin.toml are kept the same.
pub const ID: &str = "gyro";
pub const VERSION: &str = "1.6.0";
pub const DESCRIPTION: &str =
    "Fly an elytra like an aircraft: while you glide, the mouse turns and pitches you as usual, the world banks into your turns, and holding a turn rolls you right around.";

/// The game's own look sensitivity: 0.15 degrees per unit of mouse.
const DEG_PER_UNIT: f32 = 0.15;

/// How much the world banks per degree turned. A flick of the mouse tips it
/// over; a turn you hold rolls it right around and back.
const BANK_PER_TURN: f32 = 1.5;

/// How fast the world levels itself when you stop turning, in degrees a second.
/// Slow enough that the bank is still there when you pull up to carve.
const LEVEL_RATE: f32 = 25.0;

/// How fast the world levels itself when you land, in degrees a second.
const GROUND_LEVEL_RATE: f32 = 270.0;

/// Current camera bank, in degrees. Signed: positive is banked right.
static ROLL_DEGREES: AtomicU32 = AtomicU32::new(0);

/// The heading the last frame saw, so the turn can be measured per frame.
static LAST_YAW: AtomicU32 = AtomicU32::new(0);

/// The render camera's yaw in degrees, captured each frame from the view
/// the level is drawn with (third-person flips included).
pub static VIEW_YAW: AtomicU32 = AtomicU32::new(0);

/// The render camera's pitch in degrees.
pub static VIEW_PITCH: AtomicU32 = AtomicU32::new(0);

/// Entity id of the camera's mob: only that avatar locks to the roll.
pub static LOCK_MOB: AtomicU32 = AtomicU32::new(u32::MAX);

/// Get the current camera bank in degrees.
pub fn get_roll() -> f32 {
    f32::from_bits(ROLL_DEGREES.load(Ordering::Relaxed))
}

/// Set the current camera bank in degrees.
pub fn set_roll(deg: f32) {
    ROLL_DEGREES.store(deg.to_bits(), Ordering::Relaxed);
}

/// Turn the camera bank by `deg` degrees.
pub fn add_roll(deg: f32) {
    set_roll(get_roll() + deg);
}

/// Flip the camera bank over, for the top of a loop.
pub fn flip_roll() {
    set_roll(get_roll() + 180.0);
}

/// The heading the last frame saw.
pub fn get_last_yaw() -> f32 {
    f32::from_bits(LAST_YAW.load(Ordering::Relaxed))
}

/// Record the heading this frame saw.
pub fn set_last_yaw(yaw: f32) {
    LAST_YAW.store(yaw.to_bits(), Ordering::Relaxed);
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
/// - `xo` turns you (yaw), at the game's own look sensitivity.
/// - `yo` pitches you, in body coordinates, so a banked pull-up carves.
/// - Returns `(pitch_delta, yaw_delta)`.
///
/// The turn is the same 0.15 degrees per unit the game's own look uses, so
/// flying right feels exactly like looking right.
pub fn calculate_flight_turn(xo: f64, yo: f64, current_roll: f32) -> (f32, f32) {
    // Mouse X turns you. The elytra flies along the look vector, so this is
    // what makes you go where you look.
    let yaw_input = (xo as f32) * DEG_PER_UNIT;

    // Mouse Y is body pitch input.
    let pitch_input = (yo as f32) * DEG_PER_UNIT;

    let phi = current_roll.to_radians();
    let cos_phi = phi.cos();
    let sin_phi = phi.sin();

    // The body pitch projected onto world pitch and world yaw:
    // when level, a pull-up is a pure climb; when banked, the same pull-up
    // carves, because the lift comes off the tilted wing.
    let pitch_delta = pitch_input * cos_phi;
    let carve_yaw = -pitch_input * sin_phi;

    (pitch_delta, yaw_input + carve_yaw)
}

/// Advance the flight state per frame: the world banks into the turn, and
/// levels itself when you stop turning.
/// Returns the current bank in degrees.
pub fn tick(gliding: bool, elapsed: Duration, yaw: f32) -> f32 {
    let dt = elapsed.as_secs_f32().min(0.5);

    // How far the heading turned since the last frame.
    let mut turn = yaw - get_last_yaw();
    if turn > 180.0 {
        turn -= 360.0;
    } else if turn < -180.0 {
        turn += 360.0;
    }
    set_last_yaw(yaw);

    let mut roll = get_roll();
    if gliding {
        // The world leans into the turn, and keeps leaning while you keep
        // turning: a turn you hold rolls it right around and back.
        roll += turn * BANK_PER_TURN;
        if turn.abs() < 0.01 {
            // Straighten out: when you stop turning, the world levels again.
            let step = LEVEL_RATE * dt;
            if roll.abs() <= step {
                roll = 0.0;
            } else {
                roll -= roll.signum() * step;
            }
        }
    } else {
        // On the ground the view levels out.
        let step = GROUND_LEVEL_RATE * dt;
        if roll.abs() <= step {
            roll = 0.0;
        } else {
            roll -= roll.signum() * step;
        }
    }
    set_roll(roll);
    roll
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    /// The flight state lives in statics, and the tests run in parallel, so
    /// they take this lock and run one at a time.
    fn flight_state() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn test_mouse_x_turns_you() {
        let _guard = flight_state();
        // Looking right must fly you right: the same 15 degrees the game's
        // own look would give.
        let (_, yaw_delta) = calculate_flight_turn(100.0, 0.0, 0.0);
        assert!((yaw_delta - 15.0).abs() < 1e-4, "mouse X must turn you");
    }

    #[test]
    fn test_mouse_x_turns_you_the_other_way_too() {
        let _guard = flight_state();
        let (_, yaw_delta) = calculate_flight_turn(-100.0, 0.0, 0.0);
        assert!((yaw_delta + 15.0).abs() < 1e-4, "mouse X must turn you left");
    }

    #[test]
    fn test_banked_pitch_turns_yaw() {
        let _guard = flight_state();
        // Banked 90 deg right (phi = 90 deg):
        // Pulling up (yo = -100) turns yaw to the right
        let (p_delta, y_delta) = calculate_flight_turn(0.0, -100.0, 90.0);
        assert!(p_delta.abs() < 1e-3, "world pitch delta should be near 0");
        assert!(y_delta > 0.0, "should turn yaw right into the bank");
    }

    #[test]
    fn test_level_pitch_does_not_turn() {
        let _guard = flight_state();
        // With no rudder, a level pull-up is a pure climb: no yaw of its own.
        let (p_delta, y_delta) = calculate_flight_turn(0.0, -100.0, 0.0);
        assert!(p_delta < 0.0, "a level pull-up should pitch up");
        assert!(y_delta.abs() < 1e-3, "a level pull-up must not yaw");
    }

    #[test]
    fn test_the_world_banks_into_the_turn() {
        let _guard = flight_state();
        set_roll(0.0);
        set_last_yaw(0.0);
        // Turning right for a second leans the world to the right.
        let roll = tick(true, Duration::from_secs_f32(1.0), 90.0);
        assert!(roll > 30.0, "the world should bank into the turn");
    }

    #[test]
    fn test_the_world_levels_when_you_stop_turning() {
        let _guard = flight_state();
        set_roll(45.0);
        set_last_yaw(10.0);
        // No turn this frame: the world settles back to level.
        let roll = tick(true, Duration::from_secs_f32(0.5), 10.0);
        assert!(roll < 45.0, "the world should level when you stop turning");
        assert!(roll >= 0.0, "it should not level past level");
    }

    #[test]
    fn test_a_held_turn_is_a_barrel_roll() {
        let _guard = flight_state();
        set_roll(0.0);
        set_last_yaw(0.0);
        // Ten frames of turning right at 90 degrees a second: the world keeps
        // leaning until it has rolled all the way round.
        let mut yaw = 0.0;
        let mut roll = 0.0;
        for _ in 0..10 {
            yaw += 9.0;
            roll = tick(true, Duration::from_secs_f32(0.1), yaw);
        }
        assert!(
            roll > 180.0,
            "a held turn should roll the world right around, not 90"
        );
    }

    #[test]
    fn test_levels_out_on_ground() {
        let _guard = flight_state();
        set_roll(90.0);
        set_last_yaw(0.0);
        let roll = tick(false, Duration::from_secs_f32(0.5), 0.0);
        assert!(roll < 90.0, "roll should return toward 0 when on the ground");
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
