# Gyro Native Mod — v1.4.0 Documentation

## What it does

A Horizon **native mod** (`.hnmod`) providing true 6-DOF elytra aerobatics — inspired by the Minecraft Java mod *"Do a Barrel Roll":

1. **Mouse X rolls**: while gliding, moving the mouse barrels/banks the camera freely around the look axis (no snap-back, full 360°).
2. **Mouse Y pitches**: body pitch projected through the current roll, so pulling up while banked carves a turn; passing straight up/down flips into a loop-de-loop.
3. **A/D rudder**: yaw the aircraft while gliding (90°/s).
4. **Avatar locked to the camera**: in third person, the player's model, elytra and held item stay pinned to the screen while the world rolls around them.
5. **Level-out on landing**: roll eases back to 0° whenever you stop gliding.

---

## How it works (Injections & Architecture)

Camera roll is injected into the level's render pose:

```text
clip = projection · (bob · spin) · view_matrix(yaw, pitch) · model_matrix
```

`Game::spinning_matrix` is multiplied with a Z rotation by the current roll, so terrain, sky, entities and frustum culling all spin around the screen centre without touching `game.yaw`/`game.pitch` (mouse aim stays clean).

The avatar lock follows from that chain: because the roll rides in the projection, every model — including the player's own — rolls on screen with the world. To pin the gliding avatar to the camera instead, `mobs::render::base_transform` is post-multiplied by nothing but **left**-multiplied by `view⁻¹ · Rz(-roll) · view`: a rotation about the render camera's look axis through the camera, which exactly unwinds the projection roll for that one model (tilt and orbit). The view yaw/pitch and camera-entity id it needs are captured each frame inside the spin hook itself, which runs right after `camera_view` — so third-person front-view flips and spectator cameras are automatically correct, and other players' avatars are left alone.

| Injection Target | Kind | Helper | Purpose |
|---|---|---|---|
| `horizon_client::game::native_hooks::Game<'_>::native_mods` | `HEAD` | `list_helper.rs` | Show the mod in Options > Mods |
| `horizon_client::game::native_hooks::Game<'_>::native_key` | `HEAD` | `key_helper.rs` | A/D rudder key state |
| `horizon_client::game::native_hooks::Game<'_>::native_frame` | `HEAD` | `frame_helper.rs` | Level-out tick + rudder yaw while gliding |
| `horizon_client::game::fov::Game<'_>::spinning_matrix` | `RETURN` | `spin_helper.rs` | Inject camera roll; capture render view angles + camera mob id |
| `horizon_client::game::Game<'_>::apply_mouse_look` | `OVERWRITE` | `turn_helper.rs` | 6-DOF flight controls while gliding |
| `horizon_client::mobs::render::base_transform` | `RETURN` | `model_roll_helper.rs` | Cancel the projection roll for the gliding avatar |

Shared state (`ROLL_DEGREES`, `VIEW_YAW`, `VIEW_PITCH`, `LOCK_MOB`, rudder flags) lives in atomics in `runtime.rs`, injected as `horizon_client::gyro`.

---

## Files

```
prototypes/gyro/
├── mixins/gyro/
│   ├── horizon-mixin.toml        # mod id, version, pinned commit, 6 injections + module
│   ├── runtime.rs                # roll physics, view/lock atomics (unit-tested standalone)
│   ├── list_helper.rs            # Options > Mods entry
│   ├── key_helper.rs             # rudder key adapter
│   ├── frame_helper.rs           # frame tick adapter
│   ├── spin_helper.rs            # camera roll + render-view capture
│   ├── turn_helper.rs            # mouse → roll/pitch flight model
│   └── model_roll_helper.rs      # avatar lock (view-space roll cancellation)
├── tools/                        # horizon.py (stage+build), package.py, audit
└── target/
    ├── gyro-1.4.0.hnmod   # packaged source bundle
    └── mixins/modded/            # staged patched Horizon source (build input)
```

---

## Build

The bundle pins Horizon `ba6344a5…` and horizon-mixins `d9c83d70…`.

```sh
# unit tests (physics + atomics, standalone)
cargo test

# stage the pinned source, apply the bundle, build a patched client
HORIZON_MIXINS=<path/to/horizon-mixins> \
python tools/horizon.py build --horizon <path/to/horizon>

# package the .hnmod
python tools/package.py
```

Windows notes: `tools/horizon.py` runs with a no-op lock (no `fcntl`) and exports the pinned tree with `core.autocrlf=false` so source hashes match the Linux pin; the extensionless Linux `horizon-mixins` binary in the mixins target dir must not shadow the `.exe` build.

## Launch & Testing

1. Copy the built `target/horizon-cargo/fast/horizon-client.exe` over `horizon-client-gyro.exe` in your play folder.
2. Equip Elytra (`/give <name> minecraft:elytra 1`) and rockets (`/give <name> minecraft:firework_rocket 64`).
3. Glide: mouse X rolls a full 360°, the world spins around you while your avatar stays pinned to the screen; A/D rudder; stop gliding to level out.
