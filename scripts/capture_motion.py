#!/usr/bin/env python3
"""Record Alt's motion as GIF clips through a real PTY (M05).

Usage: capture_motion.py [CLIP ...] [--out DIR] [--sizes 80x24,120x40] [--fps 20]
       capture_motion.py --check     # bandwidth and idle regression check, no GIFs

Runs the deterministic fixture engine (no weights, no network). Every byte Alt
writes is recorded with its arrival time, then replayed into a terminal emulator
and sampled at a fixed frame rate, so a clip shows exactly what a terminal would
have shown. It also reports bytes written per clip, the bandwidth cost of
motion, alongside a motion-off baseline. Needs pyte and Pillow with DejaVu fonts.
"""
import argparse, fcntl, json, os, pty, select, struct, subprocess, sys, tempfile, termios, time
from pathlib import Path

import pyte

repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / "scripts"))
from capture_screens import FIXTURE_CONFIG  # noqa: E402

binary = Path(os.environ.get("ALT_TEST_BINARY", repo / "target/debug/alt")).resolve()
fixture = repo / "tests/fixtures/acp_engine.py"

# (seconds after the previous step, keys); the clip ends `tail` seconds after the last key.
CLIPS = {
    "launch": {"steps": [], "tail": 1.6},
    "pages": {"steps": [(1.2, b"\x1b2"), (0.7, b"\x1b7"), (0.7, b"\x1b3"), (0.7, b"\x1b1")], "tail": 0.6},
    "palette": {"steps": [(1.2, b"\x10"), (0.6, b"the"), (0.5, b"me"), (0.7, b"\x1b")], "tail": 0.6},
    "theme": {"steps": [(1.2, b"\x10"), (0.4, b"setting: theme"), (0.6, b"\r"), (0.8, b"\x1b[B"),
                        (0.3, b"\x1b[B"), (0.3, b"\r"), (0.6, b"\x10"), (0.4, b"setting: theme"),
                        (0.6, b"\r"), (0.6, b"\r")], "tail": 1.4},
}


# Stills (seconds into the clip) saved as PNG at the wide size: the key moments.
STILLS = {
    "launch": {"splash": 0.45, "cascade": 0.8, "settled": 1.5},
    "palette": {"dialog-pop": 1.26, "dialog-open": 1.9},
    "theme": {"toast": 4.0},
}
# GIFs kept in git (each under the 1 MiB repository limit); others are stills.
GIF_SIZES = {"launch": ("80x24", "120x40"), "pages": ("80x24",), "palette": ("80x24",), "theme": ("80x24",)}


def record(clip, columns, lines, motion="full"):
    """Run one clip and return [(seconds, bytes)] of everything Alt wrote."""
    plan = CLIPS[clip]
    with tempfile.TemporaryDirectory(prefix="alt-motion-") as directory:
        root = Path(directory)
        project = root / "project"
        project.mkdir()
        (project / "README.md").write_text("Fixture project for motion clips.\n")
        state = root / "state"
        state.mkdir(mode=0o700)
        (state / "config.toml").write_text(FIXTURE_CONFIG)
        (state / "preferences.toml").write_text(f'project = "{project}"\nmouse = false\n')
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", lines, columns, 0, 0))
        env = {**os.environ, "TERM": "xterm-256color", "COLORTERM": "truecolor", "ALT_MOTION": motion}
        started = time.monotonic()
        process = subprocess.Popen([str(binary), "--data-dir", str(state), "--engine", str(fixture), "tui"],
                                   stdin=slave, stdout=slave, stderr=slave, cwd=project, env=env)
        chunks = []

        def pump(until):
            while (left := until - time.monotonic()) > 0:
                if select.select([master], [], [], min(left, 0.01))[0]:
                    chunks.append((time.monotonic() - started, os.read(master, 1 << 20)))

        try:
            for delay, keys in plan["steps"]:
                pump(time.monotonic() + delay)
                os.write(master, keys)
            pump(time.monotonic() + plan["tail"])
        finally:
            process.terminate()
            try:
                process.wait(timeout=8)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)
        return chunks


def frames(chunks, columns, lines, fps):
    """Replay recorded output and yield (seconds, screen) at a fixed rate."""
    screen = pyte.Screen(columns, lines)
    stream = pyte.ByteStream(screen)
    end = chunks[-1][0] if chunks else 0
    index = 0
    step = 1 / fps
    t = 0.0
    while t <= end + step:
        while index < len(chunks) and chunks[index][0] <= t:
            stream.feed(chunks[index][1])
            index += 1
        yield t, screen
        t += step


def image(screen, cell=(7, 15), size=12):
    from PIL import Image, ImageDraw, ImageFont
    fonts = Path("/usr/share/fonts/truetype/dejavu")
    normal = ImageFont.truetype(str(fonts / "DejaVuSansMono.ttf"), size)
    bold = ImageFont.truetype(str(fonts / "DejaVuSansMono-Bold.ttf"), size)
    w, h = cell
    picture = Image.new("RGB", (screen.columns * w, screen.lines * h), "#0f141d")
    draw = ImageDraw.Draw(picture)
    named = {"default": "0f141d", "black": "000000", "white": "e0e7ef"}
    for y in range(screen.lines):
        row = screen.buffer[y]
        for x in range(screen.columns):
            c = row[x]
            bg = named.get(c.bg, c.bg)
            fg = "e0e7ef" if c.fg == "default" else named.get(c.fg, c.fg)
            if len(bg) == 6:
                draw.rectangle((x * w, y * h, x * w + w, y * h + h), fill="#" + bg)
            if c.data.strip() and len(fg) == 6:
                draw.text((x * w, y * h), c.data, font=bold if c.bold else normal, fill="#" + fg)
    return picture


def gif(chunks, columns, lines, fps, destination):
    """Write a GIF; identical consecutive frames merge into one longer frame."""
    pictures, durations, last = [], [], None
    for _, screen in frames(chunks, columns, lines, fps):
        key = "\n".join(screen.display) + repr([[(c.fg, c.bg) for c in screen.buffer[y].values()] for y in range(lines)])
        if key == last:
            durations[-1] += round(1000 / fps)
            continue
        last = key
        pictures.append(image(screen).convert("P", palette=1, colors=128))  # 1 = Image.ADAPTIVE
        durations.append(round(1000 / fps))
    destination.parent.mkdir(parents=True, exist_ok=True)
    pictures[0].save(destination, save_all=True, append_images=pictures[1:], duration=durations,
                     loop=0, optimize=True, disposal=1)
    return len(pictures)


def idle_bytes(chunks, after):
    return sum(len(c) for t, c in chunks if t >= after)


def check(sizes):
    """Fail when motion costs more than ~4x the bytes of the same session with
    motion off, or when a settled, idle screen keeps writing."""
    failures = []
    for size in sizes:
        columns, lines = (int(n) for n in size.split("x"))
        for clip in ("pages", "palette", "theme"):
            full = sum(len(c) for _, c in record(clip, columns, lines))
            off = sum(len(c) for _, c in record(clip, columns, lines, motion="off"))
            ok = full <= 4 * off + 20_000
            print(f"{'PASS' if ok else 'FAIL'} {clip} {size}: {full} bytes with motion, {off} without")
            ok or failures.append(clip)
        CLIPS["idle"] = {"steps": [], "tail": 4.0}
        full = idle_bytes(record("idle", columns, lines), 2.5)
        off = idle_bytes(record("idle", columns, lines, motion="off"), 2.5)
        ok = full <= off + 600
        print(f"{'PASS' if ok else 'FAIL'} idle {size}: {full} bytes in the settled 1.5 s with motion, {off} without")
        ok or failures.append("idle")
    return 1 if failures else 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("clips", nargs="*", default=sorted(CLIPS))
    parser.add_argument("--out", type=Path, default=repo / "docs/screenshots/0.8/motion")
    parser.add_argument("--sizes", default="80x24,120x40")
    parser.add_argument("--fps", type=int, default=20)
    parser.add_argument("--no-gif", action="store_true", help="measure bytes only")
    parser.add_argument("--check", action="store_true", help="bandwidth/idle regression check")
    args = parser.parse_args()
    if args.check:
        raise SystemExit(check(args.sizes.split(",")))
    report = []
    for clip in args.clips:
        for size in args.sizes.split(","):
            columns, lines = (int(n) for n in size.split("x"))
            chunks = record(clip, columns, lines)
            baseline = record(clip, columns, lines, motion="off")
            row = {"clip": clip, "size": size, "bytes_full": sum(len(c) for _, c in chunks),
                   "bytes_off": sum(len(c) for _, c in baseline)}
            if not args.no_gif and size in GIF_SIZES.get(clip, ()):
                destination = args.out / f"{clip}-{size}.gif"
                row["frames"] = gif(chunks, columns, lines, args.fps, destination)
                row["gif_bytes"] = destination.stat().st_size
            if not args.no_gif and columns >= 120:
                wanted = sorted(STILLS.get(clip, {}).items(), key=lambda kv: kv[1])
                for t, screen in frames(chunks, columns, lines, 100):
                    while wanted and t >= wanted[0][1]:
                        name, _ = wanted.pop(0)
                        image(screen, cell=(10, 21), size=16).save(args.out / f"{clip}-{name}-{size}.png")
                    if not wanted:
                        break
            report.append(row)
            print(json.dumps(row), flush=True)
    if not args.no_gif:
        (args.out / "clips.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
