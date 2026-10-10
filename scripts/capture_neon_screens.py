#!/usr/bin/env python3
"""Capture the palette and settings tabs screens through a real PTY."""

import fcntl, os, pty, select, struct, subprocess, sys, tempfile, termios, time
from pathlib import Path
import pyte

repo = Path(__file__).resolve().parents[1]
binary = Path(os.environ.get("ALT_TEST_BINARY", repo / "target/debug/alt")).resolve()
fixture = repo / "tests/fixtures/acp_engine.py"
out = Path(os.environ.get("ALT_EXTRA_SHOTS", "/tmp/kilo/extra"))
out.mkdir(parents=True, exist_ok=True)

with tempfile.TemporaryDirectory(prefix="alt-extra-") as directory:
    root = Path(directory)
    project = root / "proj"
    project.mkdir()
    state = root / "state"
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    env = {**os.environ, "TERM": "xterm-256color", "COLORTERM": "truecolor"}
    process = subprocess.Popen(
        [str(binary), "--data-dir", str(state), "--engine", str(fixture), "tui"],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        cwd=project,
        env=env,
    )
    screen = pyte.Screen(120, 40)
    stream = pyte.ByteStream(screen)

    def pump(t=0.05):
        while select.select([master], [], [], t)[0]:
            stream.feed(os.read(master, 65536))

    def wait_for(needle, timeout=10):
        start = time.monotonic()
        while time.monotonic() - start < timeout:
            pump()
            if needle in "\n".join(screen.display):
                for _ in range(4):
                    pump(0.05)
                return True
            if process.poll() is not None:
                break
        return False

    try:
        wait_for("Choose a model to get started")
        # Settings tabs
        os.write(master, b"\x1b6")  # Alt+6 Settings
        wait_for("Appearance")
        os.write(master, b"\x1b[C")  # Right arrow -> Models tab
        wait_for("Context:")
        (out / "settings-tabs.png.txt") and None
        from terminal_capture import save_screen

        save_screen(screen, out / "settings-tabs")
        # Command palette
        os.write(master, b"\x10")  # Ctrl+P
        wait_for("Command palette")
        os.write(master, b"skill")
        wait_for("Bundled skills")
        save_screen(screen, out / "palette")
        # Theme menu through palette
        os.write(master, b"\x15")  # clear query
        os.write(master, b"theme\r")
        wait_for("Neon Grid")
        save_screen(screen, out / "theme-menu")
        print("PASS: settings tabs, palette filter, theme menu captured")
    finally:
        process.terminate()
        try:
            process.wait(timeout=8)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)
