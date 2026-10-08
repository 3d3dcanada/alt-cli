#!/usr/bin/env python3
"""Failures before packaging must still leave a source-indexed actionable receipt."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('ci-stage.py')


class StageReceipts(unittest.TestCase):
    def test_setup_package_and_preflight_failures_are_retained_without_dist(self):
        with tempfile.TemporaryDirectory(prefix='alt-ci-receipt-test-') as directory:
            root = Path(directory)
            for name in ['setup', 'package', 'preflight']:
                result = subprocess.run([sys.executable, str(SCRIPT), '--name', name, '--output', str(root / 'receipts'), '--', sys.executable, '-c', 'import sys; print("actionable failure for ' + name + '"); sys.exit(7)'], cwd=root, capture_output=True, text=True)
                self.assertEqual(result.returncode, 7)
                receipt = json.loads((root / 'receipts' / (name + '.json')).read_text())
                log = (root / 'receipts' / (name + '.log')).read_bytes()
                self.assertFalse(receipt['passed'])
                self.assertEqual(receipt['exit'], 7)
                self.assertEqual(receipt['status'], 'completed')
                self.assertEqual(receipt['log_sha256'], hashlib.sha256(log).hexdigest())
                self.assertIn(b'actionable failure', log)
                self.assertFalse((root / 'dist').exists())

    def test_success_and_missing_executable_are_distinct(self):
        with tempfile.TemporaryDirectory(prefix='alt-ci-receipt-test-') as directory:
            for name, command in [('success', [sys.executable, '-c', 'print("verified")']), ('missing', ['/not/an/installed/tool'])]:
                result = subprocess.run([sys.executable, str(SCRIPT), '--name', name, '--output', directory, '--', *command], capture_output=True)
                receipt = json.loads((Path(directory) / (name + '.json')).read_text())
                self.assertEqual(receipt['passed'], name == 'success')
                self.assertEqual(result.returncode == 0, name == 'success')
                if name == 'missing':
                    self.assertIn('error', receipt)


if __name__ == '__main__':
    unittest.main()
