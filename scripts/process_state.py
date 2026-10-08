"""Linux process probes for bounded cleanup checks.

A process may disappear either before opening procfs or while reading it. Both
ENOENT and ESRCH mean it exited; other read errors must still fail the check.
"""
from pathlib import Path


def running(pid):
    try:
        return "\nState:\tZ" not in Path(f"/proc/{pid}/status").read_text()
    except (FileNotFoundError, ProcessLookupError):
        return False


def identity_exists(identity):
    try:
        return Path(f"/proc/{identity[0]}/stat").read_text().rsplit(") ", 1)[1].split()[19] == identity[1]
    except (FileNotFoundError, ProcessLookupError):
        return False
