#!/usr/bin/env python3
"""Gyro in a running modded game: its row is in Options > Mods, mouse X rolls the view while gliding
without turning the player, and the same mouse gives plain look controls back on the ground.

Starts the patched client with `horizon-ctl launch --game`, on throwaway data folders (nothing of the
player's own). Screenshots and the results go to --out. It needs prepared Minecraft 26.3 resources, the
same as Horizon's parity harnesses.

    python3 proof.py --resources C:/path/.local/horizon-client/26.3
    python3 proof.py --game target/horizon-cargo/fast/horizon-client.exe --ctl horizon-ctl
"""
import argparse
import json
import os
import struct
import subprocess
import sys
import tempfile
import time
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent


class Game:
    """One modded game on its own temp home, driven over horizon-ctl."""

    def __init__(self, ctl, game, resources, out, socket=None):
        self.ctl, self.game, self.resources, self.out = Path(ctl), Path(game), Path(resources), Path(out)
        self.socket = socket
        self.timeout = 420  # a cold resource load on a slow disk takes minutes

    def __enter__(self):
        self.out.mkdir(parents=True, exist_ok=True)
        dirs = [tempfile.mkdtemp(prefix=f"gyro-proof-{n}-") for n in ("home", "launcher", "data", "config")]
        self.home = Path(dirs[0])
        self.env = dict(os.environ, HORIZON_HOME=dirs[0], HORIZON_LAUNCHER_HOME=dirs[1],
                        XDG_DATA_HOME=dirs[2], XDG_CONFIG_HOME=dirs[3],
                        HORIZON_LAUNCHER_NO_SELF_UPDATE="1", HORIZON_NO_AUDIO="1")
        self.run("launch", "--game", str(self.game), "--world", "Proof", "--creative", "--offscreen",
                 "--wait", "--render-distance", "6", "--resources", str(self.resources))
        time.sleep(2)
        return self

    def __exit__(self, *_):
        try:
            self.run("quit")
        except Exception:  # noqa: BLE001 - a game that will not quit is still a finished run
            pass

    def run(self, *args):
        cmd = [str(self.ctl)]
        if self.socket:
            cmd += ["--socket", self.socket]
        cmd += list(args)
        # `launch` starts the game, which inherits our handles: on a pipe the read would wait for the
        # game to close it, so ctl writes to files and returns when it is done talking.
        out_path, err_path = self.out / "ctl.out", self.out / "ctl.err"
        with open(out_path, "w") as out, open(err_path, "w") as err:
            try:
                done = subprocess.run(cmd, env=self.env, cwd=self.home, stdout=out, stderr=err, timeout=self.timeout)
            except subprocess.TimeoutExpired:
                raise SystemExit(f"`horizon-ctl {' '.join(args)}` did not answer in {self.timeout}s") from None
        text, error = out_path.read_text(), err_path.read_text()
        if done.returncode != 0:
            raise SystemExit(f"`horizon-ctl {' '.join(args)}` failed ({done.returncode}): {(text or error).strip()}")
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return text

    def labels(self):
        return [w.get("label") for w in self.run("ui").get("widgets", [])]

    def play(self):
        """In the world, unpaused, no screen open: frames are only drawn when the game runs."""
        for _ in range(8):
            paused = self.run("state").get("paused")
            screen = self.run("status").get("screen")
            if not paused and screen in (None, "playing"):
                return
            self.run("key", "esc", "tap")
            time.sleep(0.6)
        raise SystemExit(f"still paused or on a screen: {self.run('status').get('screen')!r}, "
                         f"paused={self.run('state').get('paused')}")

    def shot(self, name):
        path = (self.out / f"{name}.png").resolve()
        self.run("screenshot", str(path))
        return path


def pixels(path):
    """An 8-bit truecolour PNG's (width, height, channels, rows of bytes)."""
    data = Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", f"{path} is not a PNG"
    pos, idat, width = 8, b"", 0
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, colour = struct.unpack(">IIBB", body[:10])
            assert depth == 8 and colour in (2, 6), f"{path}: only 8-bit RGB/RGBA is read"
            channels = 3 if colour == 2 else 4
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    stride = width * channels
    raw = zlib.decompress(idat)
    rows, prev = [], bytearray(stride)
    for y in range(height):
        line = bytearray(raw[y * (stride + 1) + 1: y * (stride + 1) + 1 + stride])
        filter_type = raw[y * (stride + 1)]
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up, upleft = prev[i], (prev[i - channels] if i >= channels else 0)
            if filter_type == 1:
                line[i] = (line[i] + left) & 255
            elif filter_type == 2:
                line[i] = (line[i] + up) & 255
            elif filter_type == 3:
                line[i] = (line[i] + (left + up) // 2) & 255
            elif filter_type == 4:
                p = left + up - upleft
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - upleft)
                line[i] = (line[i] + (left if pa <= pb and pa <= pc else up if pb <= pc else upleft)) & 255
        rows.append(line)
        prev = line
    return width, height, channels, rows


def difference(a, b):
    """The mean colour difference of two frames, 0..255, sampled on a grid."""
    wa, ha, ca, ra = pixels(a)
    wb, hb, cb, rb = pixels(b)
    if (wa, ha) != (wb, hb):
        raise SystemExit(f"{a} is {wa}x{ha}, {b} is {wb}x{hb}")
    total = count = 0
    for y in range(0, ha, 4):
        for x in range(0, wa, 4):
            total += sum(abs(ra[y][x * ca + k] - rb[y][x * cb + k]) for k in range(3))
            count += 3
    return total / max(count, 1)


def flight(state):
    """What the proof watches: is the player gliding, and where do they look."""
    player = state.get("player", state)
    return player.get("fall_flying"), player.get("yaw"), player.get("pitch")


def turned_degrees(a, b):
    """How far a yaw of `a` is from `b`, in degrees, across the 360 join."""
    return abs((b - a + 180.0) % 360.0 - 180.0)


def proof(g):
    out = {}
    # 1. The row players see in Options > Mods.
    g.run("options", "mods")
    labels = g.labels()
    row = next((l for l in labels if isinstance(l, str) and l.startswith("Gyro")), None)
    out["mods_row"] = row
    assert row and "1.5" in row, f"no Gyro row in Options > Mods: {labels}"
    g.shot("0-options-mods")

    g.play()
    g.run("command", "item replace entity @s armor.chest with minecraft:elytra 1")
    g.run("command", "tp @s 0 320 0 0 0")
    time.sleep(1.5)
    g.run("key", "space", "down")
    time.sleep(1.0)
    gliding, yaw, _ = flight(g.run("state"))
    out["gliding"] = gliding
    assert gliding, f"the player is not gliding: {g.run('state')}"

    # 2. Mouse X while gliding rolls the world instead of turning the player.
    level = g.shot("1-gliding-level")
    g.run("mouse-move", "200", "0")
    time.sleep(0.7)
    glide_rolled, yaw_rolled, _ = flight(g.run("state"))
    out["gliding_after_roll"] = glide_rolled
    assert glide_rolled, f"gliding ended during the roll: {out}"
    rolled = g.shot("2-gliding-rolled")
    out["yaw_drift_while_rolling"] = round(turned_degrees(yaw, yaw_rolled), 2)
    out["frame_difference_when_rolled"] = round(difference(level, rolled), 2)
    assert out["yaw_drift_while_rolling"] < 1.0, f"mouse X turned the player: {out}"
    assert out["frame_difference_when_rolled"] > 3.0, f"the view did not roll: {out}"

    # 3. Banked, a vertical mouse move carves: it turns the yaw as well as pitching.
    glide_level, yaw_before, pitch_before = flight(g.run("state"))
    out["gliding_before_carve"] = glide_level
    assert glide_level, f"gliding ended before the carve, so the roll had levelled out: {out}"
    g.run("mouse-move", "0", "300")
    time.sleep(0.7)
    glide_banked, yaw_banked, pitch_banked = flight(g.run("state"))
    out["gliding_after_carve"] = glide_banked
    out["yaw_turned_while_banked"] = round(turned_degrees(yaw_before, yaw_banked), 2)
    out["pitch_moved_while_banked"] = round(abs(pitch_banked - pitch_before), 2)
    assert out["pitch_moved_while_banked"] > 3, f"the elevator did nothing: {out}"
    assert out["yaw_turned_while_banked"] > 3, f"a banked pull-up did not carve: {out}"

    # 4. Unrolled, the same vertical move is plain pitch with no turn.
    g.run("mouse-move", "-200", "0")
    time.sleep(0.7)
    g.shot("3-gliding-unrolled")
    _, yaw_before, pitch_before = flight(g.run("state"))
    g.run("mouse-move", "0", "200")
    time.sleep(0.7)
    _, yaw_level, pitch_level = flight(g.run("state"))
    out["yaw_turned_while_level"] = round(turned_degrees(yaw_before, yaw_level), 2)
    out["pitch_moved_while_level"] = round(abs(pitch_level - pitch_before), 2)
    assert out["pitch_moved_while_level"] > 3 and out["yaw_turned_while_level"] < 1, out

    # 5. There is no rudder: A and D do nothing to the camera.
    glide_level, yaw_before, pitch_before = flight(g.run("state"))
    g.run("key", "key.keyboard.a", "down")
    time.sleep(0.8)
    g.run("key", "key.keyboard.a", "up")
    g.run("key", "key.keyboard.d", "down")
    time.sleep(0.8)
    g.run("key", "key.keyboard.d", "up")
    time.sleep(0.3)
    _, yaw_keys, pitch_keys = flight(g.run("state"))
    out["yaw_turned_by_a_and_d"] = round(turned_degrees(yaw_before, yaw_keys), 2)
    out["pitch_moved_by_a_and_d"] = round(abs(pitch_keys - pitch_before), 2)
    assert out["yaw_turned_by_a_and_d"] < 1.0, f"A and D still steered: {out}"
    assert out["pitch_moved_by_a_and_d"] < 1.0, f"A and D still pitched: {out}"

    # 6. On the ground the mouse is plain look again, and the roll has levelled out.
    g.run("key", "space", "up")
    g.run("command", "tp @s 0 5 0")
    time.sleep(2.0)
    landed = g.run("state").get("player", {})
    out["on_ground"] = landed.get("on_ground")
    out["gliding_after_landing"] = landed.get("fall_flying")
    yaw = landed.get("yaw")
    g.run("mouse-move", "200", "0")
    time.sleep(0.7)
    turned = g.run("state").get("player", {})
    out["yaw_drift_on_ground"] = round(turned_degrees(yaw, turned.get("yaw", 0)), 2)
    assert out["gliding_after_landing"] is False and out["yaw_drift_on_ground"] > 20, out
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--game", default=str(ROOT / "target/horizon-cargo/fast/horizon-client.exe"),
                        help="the patched client to play")
    parser.add_argument("--ctl", default="horizon-ctl", help="horizon-ctl to drive it with")
    parser.add_argument("--socket", help="the control socket, when this platform has no discovery file")
    parser.add_argument("--resources", required=True, help="prepared resources: .local/horizon-client/26.3")
    parser.add_argument("--out", default=str(ROOT / "target/proof"), help="where the screenshots go")
    args = parser.parse_args()
    with Game(args.ctl, args.game, args.resources, args.out, args.socket) as g:
        results = proof(g)
    print(json.dumps({"proof": "gyro", "ok": True, **results}, indent=2))


if __name__ == "__main__":
    main()
