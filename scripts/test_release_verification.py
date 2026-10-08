#!/usr/bin/env python3
"""Publication must reject unrelated, failed, missing or unfinished CI runs."""
import unittest
from wait_release_verification import select_run, wait_for_verification

TAG = "v0.6.0-beta.3"
SHA = "a" * 40


def run(**changes):
    return dict(dict(id=1, head_sha=SHA, head_branch=TAG, event="push",
                     path=".github/workflows/verify.yml", status="completed",
                     conclusion="success", html_url="https://github.com/example/run/1"), **changes)


class ReleaseVerificationTests(unittest.TestCase):
    def test_only_exact_tag_source_and_verification_workflow_match(self):
        for changes in (dict(head_sha="b" * 40), dict(head_branch="main"),
                        dict(event="workflow_dispatch"), dict(path=".github/workflows/publish.yml")):
            with self.subTest(changes=changes):
                self.assertIsNone(select_run([run(**changes)], TAG, SHA))
        self.assertEqual(select_run([run()], TAG, SHA)["id"], 1)

    def test_newer_failure_cannot_be_hidden_by_older_pass(self):
        result = wait_for_verification(lambda: [run(), run(id=2, conclusion="failure")], TAG, SHA, 1)
        self.assertFalse(result["passed"])
        self.assertEqual(result["run_id"], 2)

    def test_only_successful_completion_permits_publication(self):
        for conclusion in ("failure", "cancelled", "skipped", "neutral", "timed_out", None):
            with self.subTest(conclusion=conclusion):
                self.assertFalse(wait_for_verification(lambda: [run(conclusion=conclusion)], TAG, SHA, 1)["passed"])
        self.assertTrue(wait_for_verification(lambda: [run()], TAG, SHA, 1)["passed"])

    def test_pending_run_can_complete_within_deadline(self):
        ticks = [0]
        states = iter([[run(status="queued", conclusion=None)], [run(status="in_progress", conclusion=None)], [run()]])
        result = wait_for_verification(lambda: next(states), TAG, SHA, 60,
                                       now=lambda: ticks[0], pause=lambda seconds: ticks.__setitem__(0, ticks[0] + seconds))
        self.assertTrue(result["passed"])
        self.assertEqual(ticks[0], 30)

    def test_missing_or_unfinished_run_fails_at_deadline(self):
        for rows in ([], [run(status="in_progress", conclusion=None)]):
            with self.subTest(rows=rows):
                ticks = [0]
                result = wait_for_verification(lambda: rows, TAG, SHA, 1,
                                               now=lambda: ticks[0], pause=lambda seconds: ticks.__setitem__(0, ticks[0] + seconds))
                self.assertFalse(result["passed"])
                self.assertIn("deadline", result["error"])


if __name__ == "__main__":
    unittest.main()
