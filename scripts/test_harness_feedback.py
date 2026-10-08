#!/usr/bin/env python3
"""Actual observed failure packets; no weights or fabricated successful repairs."""
import json
import tempfile
import unittest
from pathlib import Path

import acceptance_projects as acceptance
from optimize_instructions import failure_feedback


class FeedbackTests(unittest.TestCase):
    def test_stdout_assertion_and_stderr_remain_distinct(self):
        row = {'after': {'stdout': 'left: Some(0.0)\nright: Some(-0.5)\n',
                         'stderr': 'compiler warning\n', 'exit': 1}}
        packet = failure_feedback(row)
        self.assertIn('Some(-0.5)', packet['stdout_tail'])
        self.assertEqual(packet['stderr_tail'], 'compiler warning\n')
        self.assertEqual(packet['exit'], 1)

    def test_quote_only_valid_observations_and_mark_truncation(self):
        row = {'after': {'stdout': 'progress' * 800 + '\nALT_OBSERVATION {"input":[1,2]}\nALT_OBSERVATION malformed', 'stderr': ''}}
        packet = failure_feedback(row)
        self.assertTrue(packet['stdout_truncated'])
        self.assertEqual(packet['check_emitted_observations'], [{'stream': 'stdout', 'observed': {'input': [1, 2]}}])

    def test_real_rust_failure_quotes_unchanged_assertion_then_gold_still_passes(self):
        with tempfile.TemporaryDirectory(prefix='alt-feedback-rust-') as directory:
            root = Path(directory)
            project, command = acceptance.setup('rust-feature', root)
            hashes = acceptance.oracle_inputs(root)
            before = acceptance.check(project, command)
            self.assertFalse(before['passed'])
            observations = [json.loads(line[len('ALT_OBSERVATION '):]) for line in before['stdout'].splitlines() if line.startswith('ALT_OBSERVATION ')]
            self.assertEqual(len(observations), 1, before)
            self.assertIn('i64::MIN,i64::MAX', observations[0]['assertion_source_excerpt'])
            self.assertIn('Some(-0.5)', observations[0]['assertion_source_excerpt'])
            acceptance.write(project, acceptance.CASES['rust-feature']['fixed'])
            after = acceptance.check(project, command)
            self.assertTrue(after['passed'], after)
            self.assertEqual(acceptance.oracle_inputs(root), hashes)


if __name__ == '__main__':
    unittest.main()
