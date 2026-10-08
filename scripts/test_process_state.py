#!/usr/bin/env python3
"""Regression checks for child exit during procfs cleanup probes."""
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

from process_state import identity_exists, running


class ProcessStateTests(unittest.TestCase):
    def test_exit_before_open_and_during_read_are_both_exits(self):
        for error in (FileNotFoundError(), ProcessLookupError()):
            with self.subTest(error=type(error).__name__), patch.object(Path, "read_text", side_effect=error):
                self.assertFalse(running(123))
                self.assertFalse(identity_exists((123, "99")))

    def test_unexpected_read_error_is_not_reported_as_cleanup_success(self):
        for error in (PermissionError(), OSError("unexpected procfs error")):
            with self.subTest(error=type(error).__name__), patch.object(Path, "read_text", side_effect=error):
                with self.assertRaises(type(error)):
                    running(123)
                with self.assertRaises(type(error)):
                    identity_exists((123, "99"))

    def test_zombie_is_exited_but_running_child_still_fails_cleanup(self):
        for state, expected in (("Z (zombie)", False), ("S (sleeping)", True), ("R (running)", True)):
            with self.subTest(state=state), patch.object(Path, "read_text", return_value=f"Name:\tchild\nState:\t{state}\n"):
                self.assertEqual(running(123), expected)

    def test_reused_pid_does_not_match_original_identity(self):
        fields = ["S"] + ["0"] * 18 + ["100"]
        with patch.object(Path, "read_text", return_value="123 (child with spaces) " + " ".join(fields)):
            self.assertTrue(identity_exists((123, "100")))
            self.assertFalse(identity_exists((123, "99")))

    @unittest.skipUnless(sys.platform == "linux", "Linux procfs cleanup")
    def test_actual_child_exists_then_disappears_after_reaping(self):
        child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
        try:
            start = Path(f"/proc/{child.pid}/stat").read_text().rsplit(") ", 1)[1].split()[19]
            self.assertTrue(running(child.pid))
            self.assertTrue(identity_exists((child.pid, start)))
        finally:
            child.terminate()
            child.wait(timeout=5)
        self.assertFalse(running(child.pid))
        self.assertFalse(identity_exists((child.pid, start)))


if __name__ == "__main__":
    unittest.main()
