# Gyro

**Fly an elytra like an aircraft.** Put on elytra, jump, and the mouse stops being a head-turner: moving it
left and right rolls you around the direction you are looking, moving it up and down pulls you up or pushes
you down — and because that pull goes through your bank, a wing-down tilt turns you into a carve. Push the
nose past straight up and you loop. `A` and `D` are the rudder. In third person you stay upright on screen
while the world spins around you. The moment you stop gliding, the view levels out and the mouse is an
ordinary camera again.

Written for the feel of the Minecraft Java mod *Do a Barrel Roll*, on Horizon's native hook points.

## Controls

| While gliding | What it does |
|---|---|
| Mouse left / right | Roll: bank and barrel roll, a full 360°, no snap-back |
| Mouse up / down | Elevator: pitch through your current roll, so a banked pull-up carves |
| `A` / `D` | Rudder yaw |
| Anything else | Vanilla — walking, flying and swimming controls are untouched |

Nothing to bind and nothing to configure: one row in **Options > Mods** tells you it is installed.

## Play it

**Through the Horizon launcher** (the usual way for a native mod): Mods → **Add from GitHub** →
`https://github.com/CGRLabs/gyro`, then Settings → Game → **Native mods (experimental)** → **Build**. The
launcher downloads its own Rust and compiles the mod into a game of your own; the first build takes 20 to 30
minutes, later ones are incremental. Its `.hnmod` asks for hook points v3, so it builds on any Horizon that
has them rather than one exact commit — a new Nightly does not strand it.

Native mods are unsandboxed code that runs with your rights, and this one is not reviewed by the Horizon
registry — the launcher will say so in red before it builds. The bundle is eight small `.rs` files under
`mixins/gyro/`; read them first.

**Without the launcher:** `horizon-client-gyro.exe` in
[the latest release](https://github.com/CGRLabs/gyro/releases) is a Horizon
client with Gyro already compiled in. Drop it into a prepared Horizon 26.3 folder (one where
`horizon-prepare` has run) and start it in place of `horizon-client`. It is built for commit `ba6344a5`, so
it will not match a newer install.

Either way: `/give @s minecraft:elytra 1`, jump off something high, and hold Space to glide.

## Getting it running

Gyro only moves the camera and the player's own model; the server never hears about the roll, so it is safe
on a strict server — but the rudder and the carved turns do change where you go, so an anticheat that
dislikes fast yaw changes may complain, the same as with any mouse-sensitive mod. It is client-side only: no
server component, and dedicated-server players just do not get it (native mods are not built for dedicated
servers yet).

---

## For developers

### How the roll is drawn

Camera roll rides in the level's render pose:

```text
clip = projection · (bob · spin) · view_matrix(yaw, pitch) · model_matrix
```

`Game::spinning_matrix` gets multiplied by a Z rotation of the current roll, so terrain, sky, entities and
frustum culling all turn around the screen centre, while `game.yaw` and `game.pitch` stay clean — mouse aim
is not repurposed, only reinterpreted while gliding.

Because that roll is in the projection, every model on screen tilts with the world, the player's own avatar
included. To pin the gliding avatar to the camera instead, `mobs::render::base_transform` is
*left*-multiplied by `view⁻¹ · Rz(-roll) · view`: a rotation about the render camera's look axis through the
camera, which unwinds the projection roll for that one model — both its tilt and its orbit about the screen
centre. The yaw, pitch and camera-mob id it needs are captured inside the spin hook itself, which runs right
after `camera_view`, so third-person front-view flips and spectator cameras come out right, and other
players' avatars are left alone.

| Injection target | Kind | Helper |
|---|---|---|
| `native_hooks::Game::native_mods` | `HEAD` | `list_helper.rs` — the row in Options > Mods |
| `native_hooks::Game::native_key` | `HEAD` | `key_helper.rs` — rudder key state |
| `native_hooks::Game::native_frame` | `HEAD` | `frame_helper.rs` — level-out tick, rudder yaw |
| `game::fov::Game::spinning_matrix` | `RETURN` | `spin_helper.rs` — camera roll, render view capture |
| `game::Game::apply_mouse_look` | `OVERWRITE` | `turn_helper.rs` — mouse to roll and pitch |
| `mobs::render::base_transform` | `RETURN` | `model_roll_helper.rs` — the avatar lock |

Roll physics and the atomics behind it are in `runtime.rs`, injected as `horizon_client::gyro`, and it is
also this crate's library so the numbers can be tested on their own.

### Files

```
mixins/gyro/            the bundle: horizon-mixin.toml, runtime.rs, one helper per hook
compatibility.json      the pin, the hook-point level it needs, the horizon-mixins commit
proof.py                drives a real modded game and proves the roll happens
tools/horizon.py        stage the pinned source, apply the bundle, build a patched client
tools/package.py        the deterministic .hnmod for the registry and "Add from GitHub"
tools/audit             checks the staged tree against the pins
```

### Build, test, prove

```sh
cargo test                                   # the flight maths, standalone

python3 tools/horizon.py build --horizon PATH/to/horizon     # patched client
#   HORIZON_MIXINS=... for the horizon-mixins tool; --profile fast output under target/horizon-cargo/

python3 tools/package.py --check-hooks       # target/gyro-<version>.hnmod; checks the pin, the hook level
                                             # and that the Options > Mods version is the bundle's

python3 proof.py --resources PATH/to/.local/horizon-client/26.3 \
                 --game target/horizon-cargo/fast/horizon-client.exe   # needs horizon-ctl
```

The proof starts a game on throwaway data folders, glides, and asserts that a horizontal mouse move rolls the
frame without turning the player, that rolling back returns to the same picture, and that the mouse looks
again once you are on the ground. Screenshots land in `target/proof/`.

Windows notes: `tools/horizon.py` runs with a no-op lock (no `fcntl`) and exports the pinned tree with
`core.autocrlf=false` so source hashes match the Linux pin; the extensionless Linux `horizon-mixins` binary
in the mixins target dir must not shadow the `.exe` build.

## License

MIT, Horizon's own licence for a Horizon mod. Contains none of Mojang's code, assets or data.
