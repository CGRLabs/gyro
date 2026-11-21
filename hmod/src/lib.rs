//! Do a Barrel Roll — look where you want to go: while gliding, the mouse steers you just like it
//! does when you walk, and the world leans gently into your turns.
//!
//! Ported from the retired `.hnmod` (v1.8.2) to the new native `.hmod` format.
//! The frame and camera-roll work is done by two `[[native.mixin]]` rows in
//! horizon-mod.toml, which call into this crate. No hook in the API carries camera
//! roll or raw mouse deltas, so the native route is required (see the injected
//! helpers under src/mixins/).

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use mod_api::{Hooks, Registrar, RegistrationContext};

/// The most the world ever leans, in degrees (setting `max_bank`).
const MAX_BANK: f32 = 35.0;
/// Degrees of lean per degree-per-second of turn (setting `bank_strength`).
const BANK_PER_RATE: f32 = 0.25;
/// How fast the lean follows its target, per second (exponential ease).
const BANK_RESPONSE: f32 = 10.0;
/// How fast the world levels itself when you land, in degrees a second.
const GROUND_LEVEL_RATE: f32 = 270.0;

/// Setting values, refreshed from the game each frame by `refresh_settings`.
static INVERT_Y: AtomicU32 = AtomicU32::new(0);
static MAX_BANK_BITS: AtomicU32 = AtomicU32::new(MAX_BANK.to_bits());
static BANK_RATE_BITS: AtomicU32 = AtomicU32::new(BANK_PER_RATE.to_bits());

/// Read the current settings from the game host (Options > Mods > Do a Barrel Roll).
/// Missing or reset values fall back to the defaults, so a reset rebakes them.
pub fn refresh_settings(host: &dyn mod_api::ClientHost) {
    let invert = host.setting_bool("invert_y").unwrap_or(false);
    INVERT_Y.store(u32::from(invert), Ordering::Relaxed);
    let max = host.setting_f64("max_bank").unwrap_or(MAX_BANK as f64) as f32;
    MAX_BANK_BITS.store(max.max(0.0).min(90.0).to_bits(), Ordering::Relaxed);
    let strength = host.setting_f64("bank_strength").unwrap_or(BANK_PER_RATE as f64) as f32;
    BANK_RATE_BITS.store(strength.max(0.0).min(1.0).to_bits(), Ordering::Relaxed);
}

/// Whether to invert the pitch (Y) mouse axis. Read by the turn mixin.
pub fn invert_y() -> bool {
    INVERT_Y.load(Ordering::Relaxed) != 0
}

fn max_bank() -> f32 {
    f32::from_bits(MAX_BANK_BITS.load(Ordering::Relaxed))
}

fn bank_per_rate() -> f32 {
    f32::from_bits(BANK_RATE_BITS.load(Ordering::Relaxed))
}

/// Current camera bank in degrees. Signed: positive is banked right.
static ROLL_DEGREES: AtomicU32 = AtomicU32::new(0);
/// The heading the last frame saw, so the turn can be measured per frame.
static LAST_YAW: AtomicU32 = AtomicU32::new(0);

/// Current camera bank in degrees.
pub fn get_roll() -> f32 {
    f32::from_bits(ROLL_DEGREES.load(Ordering::Relaxed))
}

/// Heading the last frame saw.
fn get_last_yaw() -> f32 {
    f32::from_bits(LAST_YAW.load(Ordering::Relaxed))
}

fn set_last_yaw(yaw: f32) {
    LAST_YAW.store(yaw.to_bits(), Ordering::Relaxed);
}

/// Advance the lean per frame. While gliding the world banks toward a turn-rate
/// target, capped at MAX_BANK; stopping a turn holds the lean; landing levels it.
pub fn tick(gliding: bool, elapsed: Duration, yaw: f32) -> f32 {
    let dt = elapsed.as_secs_f32().clamp(0.001, 0.5);

    let mut turn = yaw - get_last_yaw();
    if turn > 180.0 {
        turn -= 360.0;
    } else if turn < -180.0 {
        turn += 360.0;
    }
    set_last_yaw(yaw);

    let mut roll = get_roll();
    if gliding {
        let rate = turn / dt;
        if rate.abs() > 1.0 {
            let target = (rate * bank_per_rate()).clamp(-max_bank(), max_bank());
            roll += (target - roll) * (dt * BANK_RESPONSE).min(1.0);
        }
    } else {
        let step = GROUND_LEVEL_RATE * dt;
        roll = if roll.abs() <= step {
            0.0
        } else {
            roll - roll.signum() * step
        };
    }
    ROLL_DEGREES.store(roll.to_bits(), Ordering::Relaxed);
    roll
}

/// Settings are registered here so Options > Mods > Do a Barrel Roll shows real, editable
/// rows with working defaults (the old `.hnmod` faked keybind rows that did
/// nothing and could not be reset — issues #1 and #2).
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        // Issue #2: invert the pitch (Y) mouse axis.
        let _ = r.register_setting(mod_api::SettingSpec::toggle(
            "invert_y",
            "options.gyro.invert_y",
            false,
        ));
        // Real, resettable replacements for the old fake keybind rows (#1).
        let _ = r.register_setting(mod_api::SettingSpec::range(
            "bank_strength",
            "options.gyro.bank_strength",
            0.0,
            1.0,
            0.01,
            BANK_PER_RATE as f64,
        ));
        let _ = r.register_setting(mod_api::SettingSpec::range(
            "max_bank",
            "options.gyro.max_bank",
            0.0,
            90.0,
            1.0,
            MAX_BANK as f64,
        ));
    }

    /// Refresh the cached settings before each frame so the frame/turn mixins
    /// see the player's current choices, including a reset to defaults.
    fn render_frame_pre(&self, host: &mut dyn mod_api::ClientHost, _e: &mut mod_api::RenderFramePreEvent) {
        refresh_settings(host);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_leans_into_a_turn() {
        ROLL_DEGREES.store(0.0f32.to_bits(), Ordering::Relaxed);
        set_last_yaw(0.0);
        let roll = tick(true, Duration::from_secs_f32(0.1), 9.0);
        assert!(roll > 10.0, "a steady turn should lean the world: {roll}");
    }

    #[test]
    fn glance_barely_tips() {
        ROLL_DEGREES.store(0.0f32.to_bits(), Ordering::Relaxed);
        set_last_yaw(0.0);
        let roll = tick(true, Duration::from_secs_f32(0.1), 1.0);
        assert!(roll.abs() < 10.0, "a glance should barely tip: {roll}");
    }

    #[test]
    fn bank_is_capped() {
        ROLL_DEGREES.store(0.0f32.to_bits(), Ordering::Relaxed);
        set_last_yaw(0.0);
        let mut roll = 0.0;
        for i in 0..50 {
            roll = tick(true, Duration::from_secs_f32(0.1), (i * 90) as f32);
        }
        assert!(roll.abs() <= MAX_BANK + 0.001, "bank must stay capped: {roll}");
    }

    #[test]
    fn levels_out_on_ground() {
        ROLL_DEGREES.store(30.0f32.to_bits(), Ordering::Relaxed);
        set_last_yaw(0.0);
        let roll = tick(false, Duration::from_secs_f32(0.5), 0.0);
        assert_eq!(roll, 0.0, "landing should level the world: {roll}");
    }

    #[test]
    fn invert_y_defaults_off_and_toggles() {
        INVERT_Y.store(0, Ordering::Relaxed);
        assert!(!invert_y(), "invert must default off");
        INVERT_Y.store(1, Ordering::Relaxed);
        assert!(invert_y(), "invert must turn on from the setting");
        INVERT_Y.store(0, Ordering::Relaxed);
    }

    #[test]
    fn settings_drive_bank_limits() {
        MAX_BANK_BITS.store(20.0f32.to_bits(), Ordering::Relaxed);
        BANK_RATE_BITS.store(0.5f32.to_bits(), Ordering::Relaxed);
        ROLL_DEGREES.store(0.0f32.to_bits(), Ordering::Relaxed);
        set_last_yaw(0.0);
        let mut roll = 0.0;
        for i in 0..50 {
            roll = tick(true, Duration::from_secs_f32(0.1), (i * 90) as f32);
        }
        assert!(roll.abs() <= 20.0 + 0.001, "the max_bank setting must cap the lean: {roll}");
        // Reset to the compiled defaults for the other tests.
        MAX_BANK_BITS.store(MAX_BANK.to_bits(), Ordering::Relaxed);
        BANK_RATE_BITS.store(BANK_PER_RATE.to_bits(), Ordering::Relaxed);
    }
}
