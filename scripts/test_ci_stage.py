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
                    self.assertIn(b'::error title=missing::', result.stdout)
                    self.assertIn(b'Stage runner error:', result.stdout)
                else:
                    self.assertNotIn(b'::error title=', result.stdout)

    def test_failure_annotation_retains_bounded_escaped_tail_and_full_raw_log(self):
        with tempfile.TemporaryDirectory(prefix='alt-ci-annotation-test-') as directory:
            root = Path(directory)
            # Cross several read chunks, include multi-byte text, and finish with
            # both workflow-command syntax and an unterminated diagnostic line.
            payload = ('DISCARD_OLD_PREFIX\n' + 'old output\n' * 10000 + '漢😀' * 3000
                       + '\n100% done\r\n::error title=forged::literal %0A %25\nFINAL_DIAGNOSTIC')
            fixture = root / 'output.bin'
            fixture.write_bytes(payload.encode())
            command = [sys.executable, '-c', 'import pathlib,sys;sys.stdout.buffer.write(pathlib.Path(sys.argv[1]).read_bytes());sys.exit(23)', str(fixture)]
            result = subprocess.run([sys.executable, str(SCRIPT), '--name', 'bounded-tail', '--output', str(root / 'receipts'), '--', *command], capture_output=True)
            self.assertEqual(result.returncode, 23)
            raw = (root / 'receipts/bounded-tail.log').read_bytes()
            self.assertEqual(raw, fixture.read_bytes(), 'Annotations must never truncate the retained raw evidence')
            marker = b'\n::error title=bounded-tail::'
            self.assertEqual(result.stdout.count(marker), 1)
            encoded = result.stdout.split(marker)[1].rstrip(b'\n')
            self.assertNotIn(b'\n', encoded)
            self.assertNotIn(b'\r', encoded)
            self.assertNotIn(b'DISCARD_OLD_PREFIX', encoded)
            self.assertIn(b'100%25 done%0D%0A::error title=forged::literal %250A %2525', encoded)
            self.assertTrue(encoded.endswith(b'FINAL_DIAGNOSTIC'))
            # GitHub unescapes a workflow command once; literal %0A must survive.
            decoded = encoded.decode().replace('%0D', '\r').replace('%0A', '\n').replace('%25', '%')
            self.assertLessEqual(len(decoded), 4000)
            self.assertIn('漢😀', decoded)
            self.assertTrue(decoded.endswith('literal %0A %25\nFINAL_DIAGNOSTIC'))
            receipt = json.loads((root / 'receipts/bounded-tail.json').read_text())
            self.assertEqual(receipt['log_sha256'], hashlib.sha256(raw).hexdigest())


if __name__ == '__main__':
    unittest.main()
