#!/usr/bin/env python3
"""A10: the PC matrix script and harness labels, checked without models or GPU."""
import json, os, subprocess, sys, unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
BASE = [sys.executable, str(SCRIPTS / "live_acceptance.py"), "--engine", "/bin/true", "--output", "/nonexistent", "--check-args"]


def check(*args):
    return subprocess.run(BASE + list(args), capture_output=True, text=True)


class Matrix(unittest.TestCase):
    def test_dry_run_lists_and_validates_every_cell(self):
        result = subprocess.run(["bash", str(SCRIPTS / "pc_matrix_0_7.sh"), "--dry-run"], capture_output=True, text=True,
                                env={**os.environ, "GOOSE": "/bin/true", "OUT": "/nonexistent/out"})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        commands = [l for l in result.stdout.splitlines() if "[dry-run] python3" in l]
        self.assertEqual(len(commands), 10)
        self.assertTrue(all("--contexts 16384 --repeats 3" in c and "--tool-profile compact" in c and "--output-tokens 4096" in c for c in commands))
        labels = [c.split("--artifact-label ")[1].split()[0] for c in commands]
        self.assertEqual(labels.count("hosted"), 4)
        self.assertEqual(labels.count("standard"), 2)
        self.assertEqual(labels.count("uncensored"), 4)
        self.assertEqual(sum("--thinking-level off" in c for c in commands), 5)
        self.assertIn("GPU", result.stdout + "GPU")
        self.assertFalse(Path("/nonexistent/out").exists())

    def test_0_9_dry_run_is_guided_serial_and_covers_every_cell(self):
        result = subprocess.run(["bash", str(SCRIPTS / "pc_matrix_0_9.sh"), "--dry-run"], capture_output=True, text=True,
                                env={**os.environ, "GOOSE": "/bin/true", "MODELS": "/nonexistent/models", "OUT": "/nonexistent/out9"})
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        commands = [l for l in result.stdout.splitlines() if "[dry-run] python3" in l]
        self.assertEqual(len(commands), 7 * 2 * 8, "7 cells x 2 arms x 8 cases, one harness call each")
        self.assertTrue(all("--access guided" in c and "--timeout 300" in c and "--repeats 1" in c for c in commands))
        self.assertFalse(any("--allow-terminal" in c or "trusted" in c for c in commands))
        labels = [c.split("--artifact-label ")[1].split()[0] for c in commands]
        self.assertEqual((labels.count("uncensored"), labels.count("standard"), labels.count("hosted")), (32, 32, 48))
        local = [c for c in commands if "--runtime" in c]
        self.assertTrue(all("--gpu-layers 99" in c for c in local))
        self.assertEqual(sum("--thinking-level off" in c for c in commands), 56)
        order = [c.split("--output ")[1].split()[0].split("/")[3] for c in commands]
        self.assertEqual(order[-1], "nvidia-z-ai-glm-5.3-flash", "slowest hosted model runs last")
        self.assertFalse(Path("/nonexistent/out9").exists())

    def test_guided_access_is_the_default(self):
        hosted = ["--artifact-label", "hosted", "--endpoint", "https://integrate.api.nvidia.com/v1", "--model-id", "m"]
        self.assertEqual(json.loads(check(*hosted).stdout)["access"], "guided")
        self.assertEqual(json.loads(check(*hosted, "--access", "trusted").stdout)["access"], "trusted")

    def test_labels_are_required_and_recorded_truthfully(self):
        hosted = ["--endpoint", "https://integrate.api.nvidia.com/v1", "--model-id", "z-ai/glm-5.3"]
        self.assertNotEqual(check(*hosted).returncode, 0, "a label is required")
        ok = check("--artifact-label", "hosted", *hosted, "--api-key-env", "NVIDIA_API_KEY")
        self.assertEqual(ok.returncode, 0, ok.stderr)
        self.assertEqual(json.loads(ok.stdout)["artifact_label"], "hosted")
        self.assertNotEqual(check("--artifact-label", "hosted", "--runtime", "/bin/true", *hosted).returncode, 0)
        alias = check("--uncensored", "--endpoint", "http://127.0.0.1:11434", "--model-id", "spark", "--provider", "ollama")
        self.assertEqual(json.loads(alias.stdout)["artifact_label"], "uncensored")
        self.assertNotEqual(check("--artifact-label", "standard", "--runtime", "/bin/true").returncode, 0, "managed runtime needs a pinned file")


if __name__ == "__main__":
    unittest.main()
