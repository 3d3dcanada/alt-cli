#!/usr/bin/env python3
"""Keep the repository light (A09): no tracked file above 1 MiB and no model
weights or archives. Raw evidence over 1 MiB belongs in a release asset; git
keeps summaries. Files that predate this rule are listed explicitly."""
import subprocess
import sys
from pathlib import Path

LIMIT = 1024 * 1024
GRANDFATHERED = {"release-evidence/2026-10-07/CONTENTS.json"}
FORBIDDEN = (".gguf", ".safetensors", ".bin", ".pt", ".pth", ".onnx", ".tar.gz", ".zip", ".7z")


def main():
    files = subprocess.run(["git", "ls-files", "-z"], capture_output=True, check=True).stdout.split(b"\0")
    problems = []
    for name in filter(None, files):
        path = Path(name.decode())
        if not path.is_file():
            continue
        if path.name.lower().endswith(FORBIDDEN):
            problems.append(f"{path}: model weights and archives stay out of git")
        elif path.stat().st_size > LIMIT and str(path) not in GRANDFATHERED:
            problems.append(f"{path}: {path.stat().st_size} bytes; publish raw evidence over 1 MiB as a release asset and commit a summary")
    for line in problems:
        print(line)
    print(f"repository weight: {len(problems)} problem(s) across {len([f for f in files if f])} tracked files")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
