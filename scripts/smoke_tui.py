#!/usr/bin/env python3
"""First-run acceptance through a real terminal and deterministic ACP/HTTP fixtures."""
import fcntl
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import pty
import select
import signal
import sqlite3
import struct
import subprocess
import tempfile
import sys
import termios
import threading
import time
import tomllib
import pyte
from process_state import running

repo = Path(__file__).resolve().parents[1]
binary = Path(os.environ.get("ALT_TEST_BINARY", repo / "target/debug/alt")).resolve()
fixture = repo / "tests/fixtures/acp_engine.py"
sys.path.insert(0, str(repo / "tests/fixtures"))
from gguf import fixture as gguf_fixture

class Inventory(BaseHTTPRequestHandler):
    def log_message(self, *_): pass
    def do_GET(self):
        body = json.dumps({"data": [{"id": "fixture"}, {"id": "fixture-second"}]}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

server = ThreadingHTTPServer(("127.0.0.1", 0), Inventory)
threading.Thread(target=server.serve_forever, daemon=True).start()
with tempfile.TemporaryDirectory(prefix="alt-tui-smoke-") as directory:
    root = Path(directory)
    state = root / "state"
    project = root / "my project"
    project.mkdir()
    (project / "sample.gguf").write_bytes(gguf_fixture())
    trace = root / "acp.jsonl"
    base = [str(binary), "--data-dir", str(state), "--engine", str(fixture)]
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    original = termios.tcgetattr(slave)
    terminal_env = {**os.environ, "TERM": "xterm-256color", "COLORTERM": "truecolor", "ALT_FIXTURE_TRACE": str(trace)}
    terminal_env.pop("NO_COLOR", None)
    process = subprocess.Popen(base + ["tui"], stdin=slave, stdout=slave, stderr=slave,
        cwd=project, env=terminal_env)
    output = bytearray()
    screen = pyte.Screen(120, 40)
    terminal_stream = pyte.ByteStream(screen)
    checkpoint = 0

    def pump(timeout=0.05):
        if select.select([master], [], [], timeout)[0]:
            data = os.read(master, 65536)
            output.extend(data)
            terminal_stream.feed(data)

    def wait_for(needle, timeout=10):
        global checkpoint
        start = time.monotonic()
        while time.monotonic() - start < timeout:
            pump()
            if needle.decode() in "\n".join(screen.display):
                checkpoint = len(output)
                return
            if process.poll() is not None: break
        raise AssertionError(f"Missing terminal text {needle!r}; exit={process.poll()}; screen:\n" + "\n".join(screen.display))

    def send(data):
        global checkpoint
        for _ in range(2): pump(0.02)
        checkpoint = len(output)
        os.write(master, data)

    def paste(text): send(b"\x1b[200~" + text.encode() + b"\x1b[201~")

    def capture(name):
        destination = os.environ.get("ALT_TUI_SCREENSHOTS")
        if not destination:
            return
        # A matching string can arrive halfway through a frame. Drain its tail.
        for _ in range(5): pump(0.05)
        from terminal_capture import save_screen
        save_screen(screen, Path(destination) / name)

    def stored():
        if not (state / "sessions.db").exists(): return []
        with sqlite3.connect(state / "sessions.db") as db:
            return [json.loads(row[0]) for row in db.execute("SELECT payload FROM events ORDER BY seq")]

    def wait_saved(predicate, timeout=10):
        start = time.monotonic()
        while time.monotonic() - start < timeout:
            pump()
            rows = stored()
            if predicate(rows):
                for _ in range(3): pump(0.06)
                return rows
        raise AssertionError("Expected saved state did not arrive: " + str(stored()[-3:]))

    try:
        wait_for(b"Connect your first model")
        capture("home")
        assert not (state / "config.toml").exists()
        send(b"\r")
        wait_for(b"Where will your model run?")
        send(b"\x1b[B\r")
        wait_for(b"Server address")
        capture("connection")
        send(b"\x15")
        paste(f"http://127.0.0.1:{server.server_port}/v1")
        send(b"\r")
        wait_for(b"Choose your model")
        send(b"\r")
        wait_for(b"Connection saved")
        configuration = tomllib.loads((state / "config.toml").read_text())
        assert configuration["profiles"]["LM Studio"]["model"] == "fixture"
        send(b"\x1b[B\x1b[B\r")
        wait_for(b"Choose a project folder")
        send(b"\x13")
        wait_for(b"Project folder selected")
        assert tomllib.loads((state / "preferences.toml").read_text())["project"] == str(project)
        send(b"\x1b2")
        wait_for(b"Describe what you want")
        send(b"tools\r")
        wait_for(b"Review this action")
        capture("approval")
        send(b"y")
        pump(0.1)
        assert not any(e["type"] == "permission_decision" for e in stored())
        send(b"\x0e")
        wait_saved(lambda rows: any(e["type"] == "turn_end" for e in rows))
        assert any(e["type"] == "permission_decision" and not e["allow"] for e in stored())
        capture("workspace")
        send(b"\x02")
        wait_for(b"Project brief")
        paste("Goal: make the project understandable.\nKeep the public API unchanged.")
        send(b"\x13")
        wait_for(b"Project brief saved")
        paste("Explain this in plain language.\nUnicode: café 日本語 🙂")
        send(b"\r")
        wait_saved(lambda rows: sum(e["type"] == "turn_end" for e in rows) >= 2)
        assert any(e["type"] == "user" and "日本語 🙂" in e.get("text", "") for e in stored())
        requests = [json.loads(line) for line in trace.read_text().splitlines()]
        assert any("Keep the public API unchanged" in json.dumps(r, ensure_ascii=False) for r in requests)
        send(b"wait\r")
        wait_for(b"waiting")
        send(b"\x1b")
        wait_saved(lambda rows: any(e["type"] == "turn_end" and e["data"]["stopReason"] == "cancelled" for e in rows))
        send(b"\x1b5")
        wait_for(b"Saved work")
        send(b"/tools\r")
        send(b"r")
        wait_for(b"Name this conversation")
        send(b"\x15")
        paste("Project check")
        send(b"\r")
        wait_for(b"Conversation name saved.")
        send(b"/\x15\r")
        # Renaming removes this row from the old /tools filter. Wait for the
        # asynchronously refreshed, unfiltered list before exporting its row.
        wait_for(b"Project check")
        send(b"e")
        wait_for(b"Conversation exported")
        assert list((state / "exports").glob("*/conversation.md"))
        send(b"\r")
        send(b"\r")
        wait_for(b"Ready. Describe")
        assert sum(json.loads(line).get("method") == "session/new" for line in trace.read_text().splitlines()) >= 4, "Resume must rebuild a bounded context without restoring legacy tools"
        send(b"\x1b4")
        wait_for(b"Saved connections")
        send(b"e")
        wait_for(b"Server address")
        send(b"\t\x15")
        paste("http://127.0.0.1:1/v1")
        send(b"\r")
        # Connection failures remain inline with the original editable form.
        wait_for(b"Could not connect")
        wait_for(b"Check connection")
        send(b"\x15")
        paste(f"http://127.0.0.1:{server.server_port}/v1")
        send(b"\r")
        wait_for(b"Choose your model")
        send(b"\r")
        wait_for(b"Connection saved")
        send(b"\x1b3")
        wait_for(b"On this computer")
        send(b"i")
        wait_for(b"Import a GGUF model")
        send(b"/")
        wait_for(b"Enter a path")
        send(b"\x15")
        paste(str(project / "sample.gguf"))
        send(b"\r")
        wait_for(b"Model verified and added")
        assert len(list((state / "models").glob("*.json"))) == 1
        assert (project / "sample.gguf").exists()
        for page in [b"\x1b6", b"\x1b7", b"\x1b1"]:
            send(page)
            pump(0.1)
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
        process.send_signal(signal.SIGWINCH)
        pump(0.1)
        send(b"\x11")
        process.wait(timeout=8)
        assert process.returncode == 0
        assert termios.tcgetattr(slave) == original
        with sqlite3.connect(state / "sessions.db") as db:
            session_id = db.execute("SELECT id FROM sessions LIMIT 1").fetchone()[0]
        screen = pyte.Screen(80, 24)
        terminal_stream = pyte.ByteStream(screen)
        process = subprocess.Popen(base + ["tui", "--resume", session_id], stdin=slave, stdout=slave, stderr=slave,
                                   cwd=project, env=terminal_env)
        wait_for(b"Ready. Describe")
        children = [int(pid) for pid, parent in (line.split() for line in subprocess.check_output(["ps", "-eo", "pid=,ppid="], text=True).splitlines()) if int(parent) == process.pid]
        assert children
        process.send_signal(signal.SIGTERM)
        process.wait(timeout=8)
        assert process.returncode == 0
        assert termios.tcgetattr(slave) == original
        assert not any(Path(f"/proc/{pid}").exists() for pid in children), "SIGTERM left the engine behind"
        process = subprocess.Popen(base + ["run", "wait", "--timeout", "60"], cwd=project,
                                   stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                                   env=terminal_env)
        assert b"Session:" in process.stderr.readline()
        children = [int(pid) for pid, parent in (line.split() for line in subprocess.check_output(["ps", "-eo", "pid=,ppid="], text=True).splitlines()) if int(parent) == process.pid]
        assert children
        process.kill()
        process.wait(timeout=8)
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline and any(running(pid) for pid in children): time.sleep(0.05)
        assert not any(running(pid) for pid in children), "Abrupt parent death left a running engine"
        print("PASS: first-run wizard, model selection, project picker, Unicode/multiline paste, brief delivery, tool rejection, cancellation, search/rename/export/resume, connection repair, GGUF import, navigation, resize, terminal restoration, SIGTERM cleanup, Linux parent-death cleanup.")
    finally:
        if process.poll() is None:
            process.send_signal(signal.SIGINT)
            try: process.wait(timeout=8)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        os.close(master)
        os.close(slave)
        server.shutdown()
        server.server_close()
