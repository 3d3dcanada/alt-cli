#!/usr/bin/env python3
"""Scan added lines (or whole files) for credentials before a push. Stdlib only.

Usage: secret_scan.py [--base REF] [--all]
  --base REF  scan lines added since REF (default origin/main)
  --all       scan every tracked file instead of a diff
Exit 1 when a likely secret is found. Fixture placeholders such as
`sk-test-fixture` or values built from repeated characters are ignored.
"""
import argparse
import re
import subprocess
import sys

PATTERNS = [
    ("private key block", re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----")),
    ("AWS access key", re.compile(r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b")),
    ("GitHub token", re.compile(r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36,}\b|\bgithub_pat_[A-Za-z0-9_]{60,}\b")),
    ("NVIDIA API key", re.compile(r"\bnvapi-[A-Za-z0-9_-]{30,}\b")),
    ("OpenAI-style key", re.compile(r"\bsk-(?:proj-|ant-)?[A-Za-z0-9_-]{32,}\b")),
    ("Hugging Face token", re.compile(r"\bhf_[A-Za-z0-9]{30,}\b")),
    ("Slack token", re.compile(r"\bxox[abprs]-[A-Za-z0-9-]{10,}\b")),
    ("Google API key", re.compile(r"\bAIza[0-9A-Za-z_-]{35}\b")),
    ("Groq key", re.compile(r"\bgsk_[A-Za-z0-9]{40,}\b")),
    ("bearer header", re.compile(r"Authorization:\s*Bearer\s+[A-Za-z0-9._-]{24,}", re.I)),
    ("assigned secret", re.compile(r"(?i)\b(?:api[_-]?key|secret|token|password)\b\s*[=:]\s*['\"]([A-Za-z0-9/+_=-]{24,})['\"]")),
]
ALLOW = re.compile(r"(?i)fixture|example|placeholder|dummy|redacted|xxxx|test-key|not-a-real")


def suspicious(value: str) -> bool:
    # Repeated characters and obvious placeholders are fixtures, not credentials.
    return len(set(value)) > 6 and not ALLOW.search(value)


def scan(lines):
    found = []
    for origin, number, text in lines:
        for label, pattern in PATTERNS:
            for match in pattern.finditer(text):
                value = match.group(match.lastindex or 0)
                if suspicious(value) and not ALLOW.search(text):
                    found.append((origin, number, label, value[:6] + "…"))
    return found


def diff_lines(base):
    out = subprocess.run(["git", "diff", "--unified=0", "--no-color", f"{base}...HEAD"],
                         capture_output=True, text=True, check=True).stdout
    out += subprocess.run(["git", "diff", "--unified=0", "--no-color", "HEAD"],
                          capture_output=True, text=True, check=True).stdout
    path, number = "?", 0
    for line in out.splitlines():
        if line.startswith("+++ "):
            path = line[6:] if line.startswith("+++ b/") else line[4:]
        elif line.startswith("@@"):
            number = int(re.search(r"\+(\d+)", line).group(1)) - 1
        elif line.startswith("+") and not line.startswith("+++"):
            number += 1
            yield path, number, line[1:]


def all_lines():
    files = subprocess.run(["git", "ls-files"], capture_output=True, text=True, check=True).stdout.split("\n")
    for path in filter(None, files):
        try:
            with open(path, encoding="utf-8") as handle:
                for number, text in enumerate(handle, 1):
                    yield path, number, text
        except (UnicodeDecodeError, IsADirectoryError, FileNotFoundError):
            continue


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="origin/main")
    parser.add_argument("--all", action="store_true")
    args = parser.parse_args()
    found = scan(all_lines() if args.all else diff_lines(args.base))
    for path, number, label, preview in found:
        print(f"{path}:{number}: possible {label} ({preview})")
    print(f"secret scan: {len(found)} finding(s)")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
