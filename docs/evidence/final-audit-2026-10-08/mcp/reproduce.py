#!/usr/bin/env python3
"""Bounded MCP audit fixtures. Never use a normal Alt data directory."""
import argparse
import ctypes
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time


def run(alt, state, *args):
    result = subprocess.run(
        [str(alt), '--data-dir', str(state), *args],
        capture_output=True, text=True, timeout=10,
    )
    return {'exit_code': result.returncode, 'stdout': result.stdout.strip(),
            'stderr': result.stderr.strip()}


def connection(name, fixture, marker):
    return {'name': name, 'transport': {'type': 'stdio', 'command': 'python3',
            'args': [str(fixture), str(marker)], 'env_names': []},
            'enabled': False, 'selected_tools': []}


def process_stat(pid):
    try:
        fields = Path('/proc', str(pid), 'stat').read_text().rsplit(') ', 1)[1].split()
        return {'state': fields[0], 'start': fields[19]}
    except FileNotFoundError:
        return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--alt', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True,
                        help='A new report file outside Alt state; never overwritten')
    args = parser.parse_args()
    alt = args.alt.resolve(strict=True)
    fixture_dir = Path(__file__).resolve().parent
    # Reap the intentionally orphaned helper on Linux after recording the bug.
    # The helper also has its own 30-second maximum lifetime.
    if os.name != 'posix' or not Path('/proc/self/stat').exists():
        raise SystemExit('This reproduction requires Linux.')
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
        raise SystemExit('Cannot enable local helper reaping; no probe ran.')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x', encoding='utf-8') as report_file:
        report = {'schema': 1, 'alt': str(alt), 'policy': [], 'helper': {}}
        with tempfile.TemporaryDirectory(prefix='alt-mcp-audit-', dir='/tmp') as tmp:
            root = Path(tmp)
            state = root / 'state'
            marker = root / 'marker'
            config = root / 'connection.json'
            config.write_text(json.dumps(connection(
                'marker', fixture_dir / 'mcp-marker-fixture.py', marker)))
            setup = run(alt, state, 'extensions', 'add', str(config))
            if setup['exit_code']:
                raise RuntimeError(setup)
            for policy in ['review-only', 'guided', 'trusted']:
                if marker.exists():
                    marker.unlink()
                result = run(alt, state, '--access', policy,
                             'extensions', 'check', 'marker')
                report['policy'].append(dict(result, policy=policy,
                                              host_marker_written=marker.exists()))
            pid_file = root / 'helper.pid'
            config.write_text(json.dumps(connection(
                'helper', fixture_dir / 'mcp-helper-fixture.py', pid_file)))
            setup = run(alt, state, 'extensions', 'add', str(config))
            if setup['exit_code']:
                raise RuntimeError(setup)
            pid = None
            observed = None
            try:
                result = run(alt, state, '--access', 'trusted',
                             'extensions', 'check', 'helper')
                pid = int(pid_file.read_text())
                observed = process_stat(pid)
                report['helper'] = dict(result, child_state_after_check=(
                    observed['state'] if observed else 'gone'),
                    survived_after_probe_returned=bool(observed and observed['state'] != 'Z'))
            finally:
                if pid is None and pid_file.exists():
                    pid = int(pid_file.read_text())
                    observed = process_stat(pid)
                if pid is not None:
                    current = process_stat(pid)
                    if observed and current and current['start'] == observed['start']:
                        try:
                            os.kill(pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                    deadline = time.monotonic() + 2
                    while time.monotonic() < deadline:
                        try:
                            reaped, _ = os.waitpid(pid, os.WNOHANG)
                        except ChildProcessError:
                            break
                        if reaped:
                            break
                        time.sleep(.02)
                    final = process_stat(pid)
                    report['helper']['cleanup_running'] = bool(final and final['state'] != 'Z')
        json.dump(report, report_file, indent=2)
        report_file.write('\n')
    print(str(args.output))


if __name__ == '__main__':
    main()
