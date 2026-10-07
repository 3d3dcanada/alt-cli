#!/usr/bin/env python3
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from evaluation_preflight import inspect


class PreflightTests(unittest.TestCase):
    def test_python_project_does_not_require_unselected_language_tools(self):
        result = inspect([{'files': {'main.py': ''}}], dict(os.environ, PATH=''))
        self.assertTrue(result['passed'])
        self.assertEqual(len(result['checks']), 1)

    def test_executable_rust_proxy_with_broken_environment_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            for name, text in [('cargo', 'echo cargo-fixture; exit 0'),
                               ('rustc', 'echo "toolchain home unavailable" >&2; exit 7')]:
                path = Path(directory) / name
                path.write_text('#!/bin/sh\n' + text + '\n'); path.chmod(0o755)
            result = inspect([{'files': {'src/lib.rs': ''}}], dict(os.environ, PATH=directory))
            self.assertFalse(result['passed'])
            self.assertTrue(result['checks'][1]['passed'])
            self.assertEqual(result['checks'][2]['exit'], 7)

    def test_live_runner_stops_before_invoking_engine_when_rust_is_missing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); model = root / 'test.gguf'; model.write_bytes(b'weight-free fixture')
            import hashlib
            result = subprocess.run([sys.executable, str(Path(__file__).with_name('live_acceptance.py')),
                                     '--suite', 'v5', '--cases', 'rust-feature', '--contexts', '8192',
                                     '--repeats', '1', '--engine', str(root/'missing-engine'),
                                     '--runtime', str(root/'missing-runtime'), '--model', str(model),
                                     '--sha256', hashlib.sha256(model.read_bytes()).hexdigest(),
                                     '--uncensored', '--output', str(root/'report')],
                                    env=dict(os.environ, PATH=''), capture_output=True, text=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('No model requests were issued', result.stderr)
            report = json.loads((root/'report/environment-preflight.json').read_text())
            self.assertFalse(report['passed'])
            self.assertFalse((root/'report/alt-under-test').exists())


if __name__ == '__main__':
    unittest.main()
