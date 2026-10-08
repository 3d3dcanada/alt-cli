#!/usr/bin/env python3
"""Gate publication on successful Verify CI for the exact pushed release tag."""
import argparse
import json
import re
import subprocess
import sys
import time


def select_run(runs, tag, sha):
    matching = [r for r in runs if r.get("head_sha") == sha and
                r.get("head_branch") == tag and r.get("event") == "push" and
                r.get("path") == ".github/workflows/verify.yml"]
    return max(matching, key=lambda r: r["id"], default=None)


def wait_for_verification(fetch, tag, sha, timeout, now=time.monotonic, pause=time.sleep):
    deadline = now() + timeout
    run = None
    while True:
        run = select_run(fetch(), tag, sha)
        if run and run["status"] == "completed":
            return {"passed": run.get("conclusion") == "success", "tag": tag,
                    "source_sha": sha, "run_id": run["id"],
                    "conclusion": run.get("conclusion"), "url": run.get("html_url")}
        remaining = deadline - now()
        if remaining <= 0:
            return {"passed": False, "tag": tag, "source_sha": sha,
                    "run_id": run["id"] if run else None,
                    "error": "Exact-tag verification did not finish before the deadline"}
        pause(min(15, remaining))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--sha", required=True)
    parser.add_argument("--timeout", type=int, default=1800)
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repo) or not re.fullmatch(r"[0-9a-f]{40}", args.sha):
        parser.error("A repository identity and full source SHA are required")
    if not re.fullmatch(r"v[0-9.]+-beta\.[0-9]+", args.tag) or not 1 <= args.timeout <= 1800:
        parser.error("A beta tag and deadline of 1–1800 seconds are required")

    def fetch():
        result = subprocess.run(["gh", "api", f"repos/{args.repo}/actions/workflows/verify.yml/runs?per_page=100&head_sha={args.sha}"],
                                capture_output=True, text=True, timeout=30)
        if result.returncode:
            raise RuntimeError(f"Verification API request failed (exit {result.returncode})")
        return json.loads(result.stdout)["workflow_runs"]

    try:
        receipt = wait_for_verification(fetch, args.tag, args.sha, args.timeout)
    except (RuntimeError, subprocess.TimeoutExpired, ValueError, KeyError) as error:
        receipt = {"passed": False, "tag": args.tag, "source_sha": args.sha, "error": str(error)}
    print(json.dumps(receipt, indent=2))
    return 0 if receipt["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
