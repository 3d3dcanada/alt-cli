#!/usr/bin/env python3
"""Capture work-order screens through a real PTY at 80x24 and one wide size.

Usage: capture_screens.py ORDER [--out DIR] [--sizes 80x24,120x40]
Each ORDER below is a list of steps run against the deterministic fixture engine
(no model weights, no network). Text captures are always written; PNGs need the
optional Pillow + DejaVu fonts used by terminal_capture.py.
"""
import argparse, fcntl, os, pty, select, struct, subprocess, sys, tempfile, termios, time
from pathlib import Path

import pyte

repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / "scripts"))
binary = Path(os.environ.get("ALT_TEST_BINARY", repo / "target/debug/alt")).resolve()
fixture = repo / "tests/fixtures/acp_engine.py"

# A saved connection that is never contacted while capturing menus.
FIXTURE_CONFIG = """default_profile = "local"

[profiles.local]
provider = "openai"
endpoint = "http://127.0.0.1:9/v1"
model = "fixture-no-weights"
context_tokens = 8192
max_turns = 12
uncensored = true
"""


def palette(query):
    return [("send", b"\x10"), ("wait", "Command palette"), ("send", query.encode()), ("wait", "result(s)"), ("send", b"\r")]


# Orders whose screens need a reachable fixture server (no weights).
NEEDS_SERVER = {"a05"}

PLANS = {
    "a05": [("wait", "Alt"), *palette("probe this model"), ("wait", "Probe results"), ("shot", "probe-results"),
            ("send", b"\x1b[B\r"), ("wait", "applied to this connection"), ("shot", "probe-applied")],
    "a07": [("wait", "Alt"), ("send", b"\x1b2"), ("wait", "Workspace"), ("send", b"tools"), ("send", b"\r"),
            ("wait", "Review this action"), ("shot", "terminal-approval"), ("send", b"\x0e")],
    "a01": [("wait", "Alt"), *palette("model thinking"), ("wait", "Model thinking"), ("shot", "thinking-menu"), ("send", b"\x1b"),
            *palette("recover from a thinking"), ("wait", "whole answer allowance"), ("shot", "cutoff-recovery"), ("send", b"\x1b"),
            ("send", b"\x1b6"), ("wait", "Preferences"), ("send", b"\x1b[C"), ("wait", "Thinking:"), ("shot", "settings-models")],
    "a02": [("wait", "Alt"), *palette("advanced parameters"), ("wait", "Show raw JSON"), ("shot", "params-menu"),
            ("send", b"\x1b[B\r"), ("wait", "Set Repeat penalty"), ("shot", "params-field"), ("send", b"\x1b"), ("send", b"\x1b"),
            *palette("advanced parameters"), ("wait", "Show raw JSON"), ("send", b"\r"), ("wait", "raw JSON"), ("shot", "params-raw")],
    "a03": [("wait", "Alt"), *palette("connect a hosted api"), ("wait", "Choose a provider"), ("shot", "provider-presets"),
            ("send", b"\r"), ("wait", "NVIDIA_API_KEY is not set"), ("shot", "missing-key"), ("send", b"\x1b"),
            ("wait", "integrate.api.nvidia.com"), ("shot", "nvidia-form"), ("send", b"\x1b")],
    "a04": [("wait", "Alt"), *palette("hosted output guard"), ("wait", "Always stream live"), ("shot", "output-guard")],
    "a06": [("wait", "Alt"), *palette("setting: tool focus"), ("wait", "Compact: smaller context"), ("shot", "tool-focus")],
    "a08": [("wait", "Alt"), *palette("gpu runtime"), ("wait", "CPU remains the default"), ("shot", "gpu-runtime")],
    "b07": [("wait", "Alt"), *palette("connect a model"), ("wait", "Where will your model run?"), ("send", b"\x1b[B\r"),
            ("wait", "Type model ID"), ("shot", "connection-form"), ("send", b"\x1bm"), ("wait", "Enter a model identifier"), ("shot", "manual-model")],
    "h01": [("wait", "Alt"), ("wait", "Tip: your assistant"), ("shot", "home"), ("send", b"\x1b[B\x1b[B"), ("shot", "home-selected")],
    "board": [("wait", "Alt"), ("send", b"\x10"), ("wait", "Command palette"), ("send", b"setting:"), ("wait", "Setting: Context"),
              ("shot", "palette-settings"), ("send", b"\x1b"), *palette("setting: tool focus"), ("wait", "Which tools fit"),
              ("shot", "palette-setting-opened"), ("send", b"\x1b")],
}


def run(order, out, columns, lines):
    steps = PLANS[order]
    with tempfile.TemporaryDirectory(prefix="alt-capture-") as directory:
        root = Path(directory)
        project = root / "project"
        project.mkdir()
        (project / "README.md").write_text("Fixture project for screenshots.\n")
        state = root / "state"
        state.mkdir(mode=0o700)
        config = FIXTURE_CONFIG
        server = None
        if order in NEEDS_SERVER:
            import socket
            with socket.socket() as probe_port:
                probe_port.bind(("127.0.0.1", 0))
                port = probe_port.getsockname()[1]
            server = subprocess.Popen([sys.executable, str(repo / "tests/fixtures/fake_llama_server.py"), "--port", str(port), "--alias", "fixture-no-weights"])
            config = config.replace("http://127.0.0.1:9/v1", f"http://127.0.0.1:{port}/v1")
            for _ in range(50):
                try:
                    with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                        break
                except OSError:
                    time.sleep(0.1)
        (state / "config.toml").write_text(config)
        (state / "preferences.toml").write_text(f'project = "{project}"\nmouse = false\n')
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", lines, columns, 0, 0))
        env = {**os.environ, "TERM": "xterm-256color", "COLORTERM": "truecolor"}
        process = subprocess.Popen([str(binary), "--data-dir", str(state), "--engine", str(fixture), "tui"],
                                   stdin=slave, stdout=slave, stderr=slave, cwd=project, env=env)
        screen = pyte.Screen(columns, lines)
        stream = pyte.ByteStream(screen)

        def pump(t=0.05):
            # Bounded: animated indicators redraw continuously.
            end = time.monotonic() + 0.5
            while time.monotonic() < end and select.select([master], [], [], t)[0]:
                stream.feed(os.read(master, 65536))

        def wait(needle, timeout=15):
            end = time.monotonic() + timeout
            while time.monotonic() < end:
                pump()
                if needle in "\n".join(screen.display):
                    for _ in range(4):
                        pump(0.05)
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(f"{order} {columns}x{lines}: missing {needle!r}\n" + "\n".join(screen.display))

        saved = []
        try:
            for kind, value in steps:
                if kind == "send":
                    os.write(master, value)
                    pump(0.1)
                elif kind == "wait":
                    wait(value)
                elif kind == "shot":
                    destination = out / f"{value}-{columns}x{lines}"
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    destination.with_suffix(".txt").write_text("\n".join(screen.display) + "\n")
                    try:
                        from terminal_capture import save_screen
                        save_screen(screen, destination)
                    except Exception as error:  # Pillow/fonts are optional
                        print(f"text only for {destination.name}: {error}")
                    saved.append(destination.name)
        finally:
            if server is not None:
                server.terminate()
                server.wait()
            process.terminate()
            try:
                process.wait(timeout=8)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)
        return saved


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("order", choices=sorted(PLANS))
    parser.add_argument("--out", type=Path)
    parser.add_argument("--sizes", default="80x24,120x40")
    args = parser.parse_args()
    out = args.out or repo / "docs/screenshots/0.7" / args.order
    for size in args.sizes.split(","):
        columns, lines = (int(n) for n in size.split("x"))
        print(f"PASS {args.order} {size}: {', '.join(run(args.order, out, columns, lines))}")


if __name__ == "__main__":
    main()
