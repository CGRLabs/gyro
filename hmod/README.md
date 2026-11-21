# Gyro — native `.hmod`

This directory is the 1.9.0 port of Gyro to Horizon's new native `.hmod` format. The old
`.hnmod` mixin tree at the repository root remains for history; this is the build published for
launchers 0.2.70 and later.

- **id:** `gyro`
- **kind:** native (`native-mod`, client side)
- **Horizon:** `0b14f8acfd97fa2c750e42795249dcc981178ca4`
- **Horizon Mixins:** `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`
- **Game:** 26.3

## What changed

- The frame helper advances the bank from the local player's turn rate once per rendered frame.
- The camera helper applies the bank to `Game::spinning_matrix`.
- A new mixin on `Game::turn_player` negates the pitch input when **Invert Y** is enabled.
- Options > Mods > Gyro > Config now has real settings: Invert Y, Bank strength and Max bank.
  The old fake keybind rows are gone; settings are read every frame and have usable defaults.

## Build and check

```sh
cd hmod
RUSTUP_TOOLCHAIN=1.99.1 cargo test
cargo horizon-mod build
RUSTUP_TOOLCHAIN=1.99.1 cargo horizon-mod check
cargo horizon-mod check target/horizon-mod/gyro.hmod --json
```

The repository root pins an older toolchain for the legacy `.hnmod` tree, so set
`RUSTUP_TOOLCHAIN=1.99.1` when running plain Cargo or the source check from `hmod/`.
`cargo horizon-mod build` selects the pinned toolchain itself.

`cargo horizon-mod fits` is intentionally not used: this is a single-build native mod with
`[[native.mixin]]` tables, so `fits` does not apply.

The checked 1.9.0 package has SHA-256
`eee4e7220eae67a667150b22642730201d6ee877ab7449a859d455ba391af170`.
