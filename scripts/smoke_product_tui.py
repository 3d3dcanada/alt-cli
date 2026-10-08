#!/usr/bin/env python3
"""Product navigation through a real PTY: no model weights or network required.

Exercises every page by keyboard and mouse, every Settings entry, narrow-screen
focus, list scrolling, contextual actions, resize and terminal restoration.
ALT_TEST_BINARY selects the build; ALT_TUI_SCREENSHOTS saves observed buffers.
"""
import fcntl
import hashlib
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
import termios
import time

import pyte


REPO = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get("ALT_TEST_BINARY", REPO / "target/debug/alt")).resolve()
PAGES = [
    ("Home", "Home"),
    ("Workspace", "Chat"),
    ("Models", "Models"),
    ("Connections", "Links"),
    ("Conversations", "Saved"),
    ("Settings", "Settings"),
    ("Help", "Help"),
    ("Task progress", "Task"),
    ("Project files", "Files"),
    ("Terminal jobs", "Jobs"),
    ("Context & memory", "Memory"),
]
SETTINGS = [
    "Settings saved",
    "Settings saved",
    "Settings saved",
    "Choose engine",
    "Choose runtime",
    "Project brief",
    "Choose the access",
    "Fit Alt to this computer",
    "Choose a model first",
    "Choose a model first",
    "Tools and workflows",
    "Storage and recovery",
    "Model files and cache",
    "Model and runtime settings",
    "Select a model first",
    "Which tools fit this task?",
    "Model qualification",
]


class Terminal:
    def __init__(self, root, columns, rows):
        self.columns, self.rows = columns, rows
        self.project = root / "Example project"
        self.project.mkdir()
        for index in range(50):
            (self.project / f"folder_{index:02}").mkdir()
        self.state = root / "state"
        self.master, self.slave = pty.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))
        self.original = termios.tcgetattr(self.slave)
        self.screen = pyte.Screen(columns, rows)
        self.stream = pyte.ByteStream(self.screen)
        terminal_env = {**os.environ, "TERM": "xterm-256color", "COLORTERM": "truecolor"}
        terminal_env.pop("NO_COLOR", None)
        self.process = subprocess.Popen(
            [str(BINARY), "--data-dir", str(self.state), "--engine", str(REPO / "tests/fixtures/acp_engine.py")],
            cwd=self.project,
            stdin=self.slave,
            stdout=self.slave,
            stderr=self.slave,
            env=terminal_env,
        )
        self.checks = []

    @property
    def text(self):
        return "\n".join(self.screen.display)

    def pump(self, duration=0.12):
        until = time.monotonic() + duration
        while time.monotonic() < until:
            if select.select([self.master], [], [], 0.02)[0]:
                try:
                    self.stream.feed(os.read(self.master, 65536))
                except OSError:
                    return

    def send(self, data):
        self.pump(0.03)
        os.write(self.master, data)
        self.pump()

    def paste(self, text):
        self.send(b"\x1b[200~" + text.encode() + b"\x1b[201~")

    def wait(self, text, timeout=12):
        until = time.monotonic() + timeout
        while time.monotonic() < until:
            self.pump(0.04)
            if text in self.text:
                self.pump(0.04)
                return
            if self.process.poll() is not None:
                break
        raise AssertionError(f"Missing {text!r}; exit={self.process.poll()}\n{self.text}")

    def wait_page(self, title):
        until = time.monotonic() + 8
        while time.monotonic() < until:
            self.pump(0.04)
            if self.screen.display[1].lstrip().startswith(title):
                return
        raise AssertionError(f"Page did not become {title!r}\n{self.text}")

    def palette(self, query):
        self.send(b"\x10")
        self.wait("Quick actions")
        self.paste(query)
        self.send(b"\r")

    def page(self, title):
        self.palette(title)
        self.wait_page(title)

    def click(self, x, y):
        # SGR coordinates are one-based; both press and release are real events.
        self.send(f"\x1b[<0;{x + 1};{y + 1}M\x1b[<0;{x + 1};{y + 1}m".encode())

    def click_text(self, text, row_range=None):
        for y in row_range or range(self.rows):
            x = self.screen.display[y].find(text)
            if x >= 0:
                self.click(x, y)
                return
        raise AssertionError(f"No visible click target {text!r}\n{self.text}")

    def wheel(self, count, down=True):
        button = 65 if down else 64
        x = self.columns // 2
        y = self.rows // 2
        for _ in range(count):
            self.send(f"\x1b[<{button};{x};{y}M".encode())

    def capture(self, name):
        destination = os.environ.get("ALT_TUI_SCREENSHOTS")
        if destination:
            from terminal_capture import save_screen
            self.pump()
            save_screen(self.screen, Path(destination) / f"{self.columns}x{self.rows}-{name}")

    def check(self, name):
        self.checks.append(name)

    def stop(self):
        if self.process.poll() is None:
            self.process.terminate()
            self.process.wait(timeout=10)
        os.close(self.master)
        os.close(self.slave)


def exercise(term):
    term.wait_page("Home")
    term.wait("Connect your first model")
    term.wait("Ctrl+Q Quit")
    term.capture("home")
    # A first launch is useful before configuring a provider or downloading weights.
    assert not (term.state / "config.toml").exists()
    term.check("plain-alt-first-run")

    term.click_text("Connect your first model")
    term.wait("Where will your model run?")
    term.click_text("› Ollama")
    term.wait("Server address")
    term.capture("connection-form")
    term.click_text("Server address")
    term.send(b"\x15")
    term.paste("not-a-server-address")
    term.click_text("Check connection")
    term.wait("Invalid endpoint URL")
    term.capture("connection-error")
    term.send(b"\x1b")
    term.wait("Server address")
    term.wait("not-a-server-address")
    term.capture("connection-error-recovery")
    term.click_text("Cancel")
    term.wait_page("Home")
    assert not (term.state / "config.toml").exists(), "Cancelling an invalid connection persisted it"
    term.check("mouse-setup-validation-keeps-form-and-cancel-does-not-save")

    # Focus must be visible even when its page starts outside the narrow tab strip.
    term.send(b"\t" + b"\x1b[B" * 10)
    if term.columns < 85:
        assert "Memory" in "\n".join(term.screen.display[3:5]), term.text
    term.capture("navigation-focus")
    term.send(b"\r")
    term.wait_page("Context & memory")
    term.check("keyboard-focus-all-the-way-to-memory")

    for index, (title, short) in enumerate(PAGES):
        term.page(title)
        term.capture("page-" + title.lower().replace(" ", "-").replace("&", "and"))
        # Move to a neighbour, then click the original page. The destination is
        # visible in both the sidebar and the centred narrow strip.
        neighbour = PAGES[1 if index == 0 else index - 1][0]
        term.page(neighbour)
        if term.columns >= 85:
            term.click_text(title[:10], range(3, min(term.rows - 3, 26)))
        else:
            term.click_text(short, range(3, 5))
        term.wait_page(title)
        term.check("keyboard-and-mouse-page:" + title)

    # Form-only actions must not appear as dead global commands.
    term.send(b"\x10")
    term.wait("Quick actions")
    term.paste("Enter model ID")
    term.wait("No matching actions")
    term.send(b"\r")
    term.wait("Quick actions")
    assert "› Enter model ID" not in term.text, "Form-only action leaked into the global palette"
    term.send(b"\x1b")
    term.check("contextual-palette-hides-form-only-action")

    term.page("Settings")
    before = term.text
    term.wheel(8)
    assert term.text != before, "Settings ignored the mouse wheel"
    term.wait("Qualify model")
    term.capture("settings-wheel")
    term.wheel(8, down=False)
    term.wait("Context:")
    term.check("settings-mouse-wheel-both-directions")

    # Every Settings entry must open its actual wizard, expose its missing
    # prerequisite, or persist its immediate change. No live probe is run.
    for index, expected in enumerate(SETTINGS):
        term.page("Settings")
        term.send(b"\x1b[A" * 20 + b"\x1b[B" * index + b"\r")
        term.wait(expected)
        term.capture(f"setting-{index:02}")
        if index <= 2:
            if index == 2:
                term.send(b"\r")  # Restore mouse capture for remaining journeys.
        else:
            term.send(b"\x1b")
        term.check(f"settings-entry:{index}")

    term.palette("Choose project")
    term.wait("Choose a project folder")
    term.wait("folder_00")
    term.wheel(20)
    term.wait("folder_49")
    term.wheel(20, down=False)
    term.wait("folder_00")
    term.capture("project-browser-wheel")
    term.click_text("Cancel")
    term.wait_page("Settings")
    term.check("project-browser-mouse-wheel-and-cancel")

    # Resize below the documented floor and back; Ctrl+Q must still work and
    # restore the caller's terminal attributes.
    fcntl.ioctl(term.slave, termios.TIOCSWINSZ, struct.pack("HHHH", 14, 45, 0, 0))
    term.screen.resize(14, 45)
    term.process.send_signal(signal.SIGWINCH)
    term.wait("Resize to continue")
    fcntl.ioctl(term.slave, termios.TIOCSWINSZ, struct.pack("HHHH", term.rows, term.columns, 0, 0))
    term.screen.resize(term.rows, term.columns)
    term.process.send_signal(signal.SIGWINCH)
    term.wait_page("Settings")
    term.send(b"\x11")
    term.process.wait(timeout=10)
    assert term.process.returncode == 0
    assert termios.tcgetattr(term.slave) == term.original
    term.check("resize-roundtrip-clean-exit-and-terminal-restoration")


def main():
    receipts = []
    for columns, rows in [(120, 40), (80, 24), (60, 18)]:
        with tempfile.TemporaryDirectory(prefix="alt-product-tui-") as directory:
            root = Path(directory)
            (root / "state").mkdir()
            # Hold a real SQLite lock until the opening frame is observed. This
            # captures the actual startup renderer without an artificial splash
            # delay or a reconstructed screen. Releasing the lock resumes load.
            database = sqlite3.connect(root / "state/sessions.db")
            database.execute("CREATE TABLE screenshot_lock (value INTEGER)")
            database.execute("BEGIN EXCLUSIVE")
            term = Terminal(root, columns, rows)
            try:
                term.wait("Opening settings", timeout=3)
                term.capture("startup")
                database.rollback()
                database.close()
                term.check("actual-startup-frame-during-state-open")
                exercise(term)
                receipts.append({"columns": columns, "rows": rows, "checks": term.checks, "status": "passed"})
            except BaseException:
                term.capture("FAILURE")
                raise
            finally:
                try:
                    database.rollback()
                    database.close()
                except sqlite3.ProgrammingError:
                    pass
                term.stop()
    with tempfile.TemporaryDirectory(prefix="alt-product-startup-error-") as directory:
        root = Path(directory)
        (root / "state").mkdir()
        (root / "state/sessions.db").write_bytes(b"not a SQLite database\n")
        term = Terminal(root, 80, 24)
        try:
            # Continue draining the PTY while startup reports its real error.
            until = time.monotonic() + 8
            while term.process.poll() is None and time.monotonic() < until:
                term.pump()
            assert term.process.wait(timeout=1) != 0, "Invalid conversation storage unexpectedly loaded"
            term.pump()
            assert termios.tcgetattr(term.slave) == term.original, "Startup error left the terminal in raw mode"
            assert "database" in term.text.lower(), term.text
        finally:
            term.stop()
    destination = os.environ.get("ALT_TUI_SCREENSHOTS")
    if destination:
        with BINARY.open("rb") as binary:
            digest = hashlib.file_digest(binary, "sha256").hexdigest()
        Path(destination, "product-journey-receipt.json").write_text(json.dumps({"binary": str(BINARY), "binary_sha256": digest, "model_weights": False, "engine": "weight-free ACP protocol fixture; no model conversation in these journeys", "startup_capture": "actual startup frame held while opening a SQLite database under exclusive lock; lock released before Home", "startup_error_restored_terminal": True, "journeys": receipts}, indent=2) + "\n")
    print(f"PASS: product navigation at 120×40, 80×24, 60×18; {sum(len(r['checks']) for r in receipts) + 1} asserted checks; actual startup, all 11 pages by keyboard and mouse, 17 Settings entries, scoped palette, wheel scrolling, resize and terminal restoration on normal exit and startup error.")


if __name__ == "__main__":
    main()
