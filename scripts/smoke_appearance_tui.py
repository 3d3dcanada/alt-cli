#!/usr/bin/env python3
"""Real-terminal appearance journeys, with a weight-free ACP fixture.

Drives the public appearance menus, checks persisted settings after restarting,
visits every page in every preset, and checks actual terminal text contrast.
ALT_TEST_BINARY selects the build; ALT_TUI_SCREENSHOTS saves the observed buffers
and a receipt identifying the binary. No model weights or downloads are used.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import signal
import struct
import sys
import tempfile
import termios
import time
import tomllib

from smoke_product_tui import BINARY, PAGES, Terminal


THEMES = [
    ("Lagoon", "lagoon"),
    ("Graphite", "graphite"),
    ("Aurora", "aurora"),
    ("Ember", "ember"),
    ("Daylight", "daylight"),
    ("High contrast", "high-contrast"),
]
LAYOUTS = [("Automatic", "auto"), ("Sidebar", "sidebar"), ("Top tabs", "tabs"), ("Focus", "focus")]


def appearance(term):
    term.palette("Appearance")
    term.wait("Decorative graphics")


def choose(term, label):
    # Menu selection wraps; keep the target in view by following the actual
    # selection marker rather than assuming a fixed scroll offset or row.
    for _ in range(20):
        if any(("› " + label) in row for row in term.screen.display):
            term.send(b"\r")
            return
        term.send(b"\x1b[B")
    raise AssertionError(f"Could not select {label!r}\n{term.text}")


def preferences(term):
    return tomllib.loads((term.state / "preferences.toml").read_text())["appearance"]


def apply_theme(term, label, key):
    appearance(term)
    choose(term, "Color theme")
    term.wait("Color theme")
    choose(term, label)
    assert preferences(term)["theme"] == key
    term.capture("theme-" + key + "-picker")
    term.click_text("Close", range(term.rows - 1, -1, -1))
    assert preferences(term)["theme"] == key, "Closing the picker reverted the saved theme"
    term.page("Home")


def apply_layout(term, label, key):
    appearance(term)
    choose(term, "Layout")
    term.wait("Layout")
    choose(term, label)
    assert preferences(term)["layout"] == key
    term.click_text("Close", range(term.rows - 1, -1, -1))
    assert preferences(term)["layout"] == key, "Closing the picker reverted the saved layout"
    term.page("Home")


def quit_cleanly(term):
    term.send(b"\x11")
    term.process.wait(timeout=10)
    assert term.process.returncode == 0, term.text
    assert termios.tcgetattr(term.slave) == term.original, "Terminal attributes were not restored"


def resize(term, columns, rows):
    fcntl.ioctl(term.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))
    term.screen.resize(rows, columns)
    term.columns, term.rows = columns, rows
    term.process.send_signal(signal.SIGWINCH)
    term.pump(0.25)


def rgb(value):
    names = {"default": "0f141d", "black": "000000", "white": "ffffff"}
    value = names.get(value, value)
    assert len(value) == 6, f"Unsupported observed terminal color {value!r}"
    return tuple(int(value[index:index + 2], 16) / 255 for index in (0, 2, 4))


def luminance(value):
    channels = [channel / 12.92 if channel <= 0.04045 else ((channel + 0.055) / 1.055) ** 2.4 for channel in rgb(value)]
    return sum(channel * weight for channel, weight in zip(channels, (0.2126, 0.7152, 0.0722)))


def contrast(term):
    """Check alphanumeric screen text, excluding decorative lines and spaces."""
    minimum = 21.0
    colors = set()
    for y in range(term.rows):
        for x in range(term.columns):
            cell = term.screen.buffer[y][x]
            if not any(character.isalnum() for character in cell.data):
                continue
            foreground, background = cell.fg, cell.bg
            colors.add((foreground, background))
            light, dark = sorted((luminance(foreground), luminance(background)), reverse=True)
            value = (light + 0.05) / (dark + 0.05)
            minimum = min(minimum, value)
            assert value >= 4.5, f"Low-contrast text {cell.data!r} at {x},{y}: {foreground}/{background}, {value:.2f}:1\n{term.text}"
    brand = term.screen.display[0].find("ALT")
    return {"minimum_text_contrast": round(minimum, 2), "observed_text_color_pairs": sorted(colors), "shell_background": term.screen.buffer[0][0].bg, "brand_foreground": term.screen.buffer[0][max(brand, 0)].fg}


def navigation_mode(term):
    if any("Context & memory" in row[:24] for row in term.screen.display[3:]):
        return "sidebar"
    if "Chat" in "\n".join(term.screen.display[3:5]):
        return "tabs"
    return "focus"


def mouse_page_from_header(term, current, target):
    term.click_text(current, range(1, 2))
    term.wait("Quick actions")
    for _ in range(35):
        if any(target in row for row in term.screen.display[4:-2]):
            term.click_text(target, range(4, term.rows - 2))
            term.wait_page(target)
            return
        term.wheel(1)
    raise AssertionError(f"No mouse-accessible page {target!r}\n{term.text}")


def seed_fixture_profile(term):
    # This engine is a deterministic ACP peer, never a language model. Keeping
    # its name visible in screenshots prevents evidence being read as inference.
    (term.state / "config.toml").write_text(
        'default_profile = "UI fixture"\n'
        '[profiles."UI fixture"]\nprovider = "openai"\n'
        'endpoint = "http://127.0.0.1:1/v1"\nmodel = "UI fixture — no model weights"\n'
        'context_tokens = 8192\nmax_turns = 12\n'
    )


def themed_startup(root, saved_preferences, expected_background):
    directory = root / "aurora-startup"
    directory.mkdir()
    state = directory / "state"
    state.mkdir()
    (state / "preferences.toml").write_text(saved_preferences)
    database = sqlite3.connect(state / "sessions.db")
    database.execute("CREATE TABLE screenshot_lock (value INTEGER)")
    database.execute("BEGIN EXCLUSIVE")
    term = Terminal(directory, 80, 24)
    try:
        term.wait("Opening settings", timeout=3)
        assert term.screen.buffer[0][0].bg == expected_background, "Startup did not use the saved Aurora palette"
        contrast(term)
        term.capture("theme-aurora-startup")
        database.rollback()
        database.close()
        term.wait_page("Home")
        quit_cleanly(term)
    finally:
        try:
            database.rollback()
            database.close()
        except sqlite3.ProgrammingError:
            pass
        term.stop()


def permission_journey(term, theme):
    term.page("Workspace")
    term.wait("Describe what you want")
    term.paste("tools")
    term.send(b"\r")
    term.wait("Review this action")
    term.wait("Reject")
    term.wait("Allow once")
    stats = contrast(term)
    term.capture(theme + "-permission-fixture")
    # Arbitrary typing must never approve a tool, irrespective of appearance.
    term.send(b"y")
    term.wait("Review this action")
    with sqlite3.connect(term.state / "sessions.db") as database:
        rows = [json.loads(row[0]) for row in database.execute("SELECT payload FROM events ORDER BY seq")]
    assert not any(row["type"] == "permission_decision" for row in rows)
    # The explanatory paragraph also contains "Reject". Click the actual
    # bottom action, not the first mention in the dialog's help text.
    term.click_text("Reject", range(term.rows - 1, -1, -1))
    term.wait("denied")
    term.wait("final chunk")
    return stats


def theme_journeys(root, receipts):
    identities = []
    for label, key in THEMES:
        directory = root / key
        directory.mkdir()
        term = Terminal(directory, 80, 24)
        try:
            term.wait_page("Home")
            apply_theme(term, label, key)
            term.capture("theme-" + key)
            stats = contrast(term)
            identities.append(tuple(tuple(pair) for pair in stats["observed_text_color_pairs"]))
            term.click_text(label, range(2, 3))
            term.wait("Decorative graphics")
            term.send(b"\x1b")
            term.wait_page("Home")
            resize(term, 120, 40)
            term.wait_page("Home")
            term.capture("theme-" + key + "-dashboard")
            contrast(term)
            if key == "lagoon":
                # The adjacent card in the right column changes Connections;
                # going left restores the first-run card on Home. This checks
                # the action reached, not an assumed highlight color.
                term.send(b"\x1b[C\r")
                term.wait("Where will your model run?")
                term.send(b"\x1b")
                term.wait_page("Connections")
                term.page("Home")
                term.send(b"\x1b[D\r")
                term.wait("Where will your model run?")
                term.send(b"\x1b")
                term.wait_page("Home")
                term.click_text("Keep a project brief")
                term.wait("Project brief")
                term.send(b"\x1b")
                term.wait_page("Home")
            resize(term, 80, 24)
            term.wait_page("Home")
            for title, _ in PAGES:
                term.page(title)
                contrast(term)
            term.page("Home")
            quit_cleanly(term)
            seed_fixture_profile(term)
        finally:
            if sys.exc_info()[0] is not None:
                term.capture(key + "-FAILURE")
            term.stop()
        if key == "aurora":
            themed_startup(root, (term.state / "preferences.toml").read_text(), stats["shell_background"])
        # Reopen the same persisted installation. A render-only preview cannot
        # satisfy this check: we inspect both actual colors and stored settings.
        term = Terminal(directory, 80, 24)
        try:
            term.wait_page("Home")
            assert preferences(term)["theme"] == key
            restored = contrast(term)
            for field in ("shell_background", "brand_foreground"):
                assert restored[field] == stats[field], f"Theme colors did not survive restart: {key} {field}"
            approval_stats = permission_journey(term, key)
            term.capture("theme-" + key + "-workspace-fixture")
            quit_cleanly(term)
            receipts.append({"theme": key, "size": [80, 24], "persisted_restart": True, "picker_close_keeps_saved_choice": True, "all_pages": len(PAGES), "header_appearance_chip_mouse": True, "wide_dashboard_captured": True, "saved_theme_startup_captured": key == "aurora", "dashboard_left_right_and_mouse_actions": key == "lagoon", "permission_typing_did_not_approve": True, "mouse_rejected_permission": True, "home": stats, "permission": approval_stats})
            print(f"PASS appearance preset: {label}; restart, all pages, permission controls", flush=True)
        finally:
            if sys.exc_info()[0] is not None:
                term.capture(key + "-restart-FAILURE")
            term.stop()
    assert len(set(identities)) == len(THEMES), "Two presets rendered identical visible text palettes"


def layout_journeys(root, receipts):
    for columns, rows in [(120, 40), (80, 24), (60, 18)]:
        directory = root / f"layouts-{columns}x{rows}"
        directory.mkdir()
        term = Terminal(directory, columns, rows)
        try:
            term.wait_page("Home")
            for label, key in LAYOUTS:
                apply_layout(term, label, key)
                expected_mode = key
                if key == "auto":
                    expected_mode = "sidebar" if columns >= 85 else "tabs"
                elif key == "sidebar" and (columns < 80 or rows < 22):
                    expected_mode = "tabs"
                assert navigation_mode(term) == expected_mode, f"Requested {key}, expected {expected_mode}\n{term.text}"
                term.capture("layout-" + key)
                if key == "focus":
                    mouse_page_from_header(term, "Home", "Context & memory")
                    mouse_page_from_header(term, "Context & memory", "Home")
                # Every page remains reachable even when Focus hides persistent
                # navigation or a requested sidebar must adapt to a small window.
                for title, _ in PAGES:
                    term.page(title)
                    contrast(term)
                term.page("Workspace")
                term.capture("layout-" + key + "-workspace")
                term.page("Home")
                term.send(b"\t" + b"\x1b[B" * 10)
                term.wait("Context & memory" if expected_mode == "sidebar" else "Memory")
                term.send(b"\r")
                term.wait_page("Context & memory")
                term.page("Home")
                receipts.append({"layout": key, "rendered_navigation": expected_mode, "size": [columns, rows], "all_pages": len(PAGES), "keyboard_navigation_to_memory": True, "focus_mouse_only_header_navigation": key == "focus"})
                print(f"PASS appearance layout: {label} at {columns}×{rows}; rendered {expected_mode}", flush=True)
            # Leave Focus saved and prove restart does not trap the user without
            # a path to Appearance, navigation, or their draft.
            term.page("Workspace")
            draft = "Keep this draft while I change how Alt looks."
            term.paste(draft)
            appearance(term)
            choose(term, "Decorative graphics")
            assert preferences(term)["decorations"] is False
            term.send(b"\x1b")
            term.page("Workspace")
            term.wait(draft)
            quit_cleanly(term)
        finally:
            if sys.exc_info()[0] is not None:
                term.capture("layouts-FAILURE")
            term.stop()
        term = Terminal(directory, columns, rows)
        try:
            term.wait_page("Home")
            assert preferences(term)["layout"] == "focus"
            assert preferences(term)["decorations"] is False
            term.capture("plain-focus")
            term.page("Workspace")
            term.wait(draft)
            appearance(term)
            choose(term, "Accent color")
            term.wait("Accent color")
            term.send(b"\x15")
            term.paste("#GG1122")
            term.send(b"\r")
            term.wait("six hexadecimal digits")
            term.send(b"\r")
            term.wait("#GG1122")
            assert "accent" not in preferences(term), "Invalid color was persisted"
            term.capture("invalid-accent-recovery")
            term.send(b"\x15")
            term.paste("#7AA2F7")
            term.send(b"\r")
            term.wait("Decorative graphics")
            assert preferences(term)["accent"].lower() == "#7aa2f7"
            term.capture("custom-accent")
            choose(term, "Reset appearance")
            saved = preferences(term)
            assert saved["theme"] == "lagoon" and saved["layout"] == "auto"
            assert saved["decorations"] is True and "accent" not in saved
            term.send(b"\x1b")
            term.page("Home")
            term.capture("appearance-reset")
            quit_cleanly(term)
            receipts.append({"size": [columns, rows], "focus_and_plain_persisted_restart": True, "draft_retained_across_appearance_and_restart": True, "invalid_accent_not_saved": True, "custom_accent_saved": True, "reset_restores_defaults": True})
        finally:
            if sys.exc_info()[0] is not None:
                term.capture("controls-FAILURE")
            term.stop()


def main():
    receipts = []
    started = time.monotonic()
    with BINARY.open("rb") as binary:
        initial_digest = hashlib.file_digest(binary, "sha256").hexdigest()
    with tempfile.TemporaryDirectory(prefix="alt-appearance-tui-") as directory:
        root = Path(directory)
        theme_journeys(root, receipts)
        layout_journeys(root, receipts)
    with BINARY.open("rb") as binary:
        digest = hashlib.file_digest(binary, "sha256").hexdigest()
    assert digest == initial_digest, "The tested binary changed during the journeys; rerun against an immutable build"
    destination = os.environ.get("ALT_TUI_SCREENSHOTS")
    if destination:
        path = Path(destination)
        path.mkdir(parents=True, exist_ok=True)
        screenshots = {image.name: hashlib.sha256(image.read_bytes()).hexdigest() for image in sorted(path.glob("*.png"))}
        (path / "appearance-journey-receipt.json").write_text(json.dumps({
            "binary": str(BINARY), "binary_sha256": digest,
            "capture_method": "Actual PTY output parsed by pyte and rendered with terminal_capture; no mockups",
            "model_weights": False, "engine": "Deterministic ACP UI fixture; no inference quality claim",
            "elapsed_seconds": round(time.monotonic() - started, 2),
            "journeys": receipts, "screenshot_sha256": screenshots,
        }, indent=2) + "\n")
    print(f"PASS: six distinct persisted themes, every page and permission dialogs; four layouts at 120×40, 80×24, 60×18; draft-safe graphics/accent/reset controls; {len(receipts)} journey groups; real PTY and clean terminal restoration.")


if __name__ == "__main__":
    main()
