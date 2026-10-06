#!/usr/bin/env python3
"""Resume an uncensored live-model evaluation through the real terminal UI.

Run smoke_live_model.py first. This explicitly approves tools in its disposable
project and checks observed events, the original check, and terminal restoration.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import sqlite3
import struct
import subprocess
import termios
import time
import uuid
import pyte

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--evaluation", type=Path, required=True)
parser.add_argument("--engine", type=Path, required=True)
parser.add_argument("--binary", type=Path, default=Path(__file__).resolve().parents[1] / "target/release/alt")
parser.add_argument("--allow-tools", action="store_true", required=True)
parser.add_argument("--screenshot", type=Path)
args = parser.parse_args()
root = args.evaluation.resolve()
report = json.loads((root / "report.json").read_text())
state = root / "state"
with sqlite3.connect(state / "sessions.db") as db:
    session = db.execute("SELECT cwd,profile FROM sessions WHERE id=?", [report["session"]]).fetchone()
    assert json.loads(session[1])["uncensored"], "Live UI evaluation must use the chosen uncensored model"
    project = Path(session[0])
    assert project == root / "project", "Use the disposable smoke_live_model project"
    checkpoint = db.execute("SELECT max(seq) FROM events").fetchone()[0]
original = (project / "calculator.py").read_text()
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
term_state = termios.tcgetattr(slave)
env = {**os.environ, "TERM": "xterm-256color", "COLORTERM": "truecolor"}
env.pop("NO_COLOR", None)
process = subprocess.Popen([str(args.binary.resolve()), "--data-dir", str(state), "--engine", str(args.engine.resolve()),
                            "tui", "--resume", report["session"]], cwd=project, env=env,
                           stdin=slave, stdout=slave, stderr=slave)
screen = pyte.Screen(120, 40)
stream = pyte.ByteStream(screen)
started = time.monotonic()

def pump(timeout=0.1):
    if select.select([master], [], [], timeout)[0]:
        stream.feed(os.read(master, 65536))

def wait_text(text, timeout=180):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        pump()
        if text in "\n".join(screen.display):
            return
        assert process.poll() is None, "Alt exited during live UI evaluation"
    raise AssertionError("Terminal did not show " + text + "\n" + "\n".join(screen.display))

try:
    wait_text("Ready. Describe")
    marker = "ALT_FRESH_" + uuid.uuid4().hex[:12]
    prompt = f"Use the shell tool to execute this command now: python3 calculator.py && printf '{marker}\\n'. Report its exact output. This is a new check; earlier results do not count. Do not change files.\r"
    os.write(master, prompt.encode())
    replied = set()
    deadline = time.monotonic() + 600
    while time.monotonic() < deadline:
        pump()
        with sqlite3.connect(state / "sessions.db") as db:
            events = [json.loads(r[0]) for r in db.execute("SELECT payload FROM events WHERE seq>? ORDER BY seq", [checkpoint])]
        requests = [e for e in events if e["type"] == "permission_request"]
        for request in requests:
            key = json.dumps(request["id"])
            if key not in replied and "Review this action" in "\n".join(screen.display):
                os.write(master, b"\x19")
                replied.add(key)
        if any(e["type"] == "turn_end" for e in events):
            break
        assert process.poll() is None, "Alt exited before completing the live task"
    else:
        raise AssertionError("Live UI task timed out")
    results = [e["data"]["update"].get("rawOutput", {}) for e in events if e["type"] == "update"]
    assert any(r.get("exit_code") == 0 and "ALT_MANAGED_CHECK_OK" in r.get("stdout", "") and marker in r.get("stdout", "") for r in results), "The model did not produce fresh tool evidence"
    wait_text("Response ready")
    for _ in range(5): pump(0.1)
    if args.screenshot:
        from terminal_capture import save_screen
        save_screen(screen, args.screenshot)
    os.write(master, b"\x11")
    process.wait(timeout=12)
    assert process.returncode == 0
    assert termios.tcgetattr(slave) == term_state, "Terminal mode was not restored"
    assert (project / "calculator.py").read_text() == original
    after = subprocess.run(["python3", "calculator.py"], cwd=project, capture_output=True, text=True)
    assert after.returncode == 0 and "ALT_MANAGED_CHECK_OK" in after.stdout
    result = {"result": "passed", "elapsed_seconds": round(time.monotonic()-started,2),
              "real_ui_resume": True, "tool_check_observed": True, "terminal_restored": True,
              "files_unchanged": True, "permission_requests_answered": len(replied)}
    (root / "tui-report.json").write_text(json.dumps(result, indent=2)+"\n")
    print(json.dumps(result, indent=2))
finally:
    if process.poll() is None:
        process.terminate()
        process.wait(timeout=12)
    os.close(master)
    os.close(slave)
