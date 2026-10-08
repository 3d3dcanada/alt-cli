#!/usr/bin/env python3
"""Retain commit-indexed stage receipts/logs even when setup fails before dist."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time


ANNOTATION_CHAR_LIMIT = 4000
TAIL_BYTE_LIMIT = ANNOTATION_CHAR_LIMIT * 4


def workflow_escape(text, property_value=False):
    """Escape workflow-command data without interpreting child output as commands."""
    text = text.replace('%', '%25').replace('\r', '%0D').replace('\n', '%0A')
    if property_value:
        text = text.replace(':', '%3A').replace(',', '%2C')
    return text


def failure_annotation(name, receipt, tail):
    prefix = 'Stage failed (exit {}). Full output remains in the stage log and receipt.\n'.format(receipt['exit'])
    detail = tail.decode('utf-8', errors='replace').rstrip('\n')
    if receipt.get('error'):
        detail += '\nStage runner error: ' + receipt['error']
    if not detail:
        detail = 'No command output was captured.'
    message = prefix + detail[-(ANNOTATION_CHAR_LIMIT - len(prefix)):]
    title = workflow_escape(name[:128], property_value=True)
    # Start a fresh line even if the child ended with an unterminated progress line.
    return '\n::error title={}::{}\n'.format(title, workflow_escape(message))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--name', required=True)
    parser.add_argument('--output', type=Path, default=Path('ci-evidence'))
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not command or not all(c.isalnum() or c in '-_' for c in args.name):
        parser.error('Supply a stable stage name and command after --')
    args.output.mkdir(parents=True, exist_ok=True)
    base = args.output / args.name
    source = subprocess.run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True)
    receipt = dict(schema=1, stage=args.name, source_commit=source.stdout.strip() if source.returncode == 0 else None,
                   workflow_run=os.environ.get('GITHUB_RUN_ID'), attempt=os.environ.get('GITHUB_RUN_ATTEMPT'),
                   ref=os.environ.get('GITHUB_REF'), command=command,
                   started_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(), status='running')
    def save():
        temporary = base.with_suffix('.json.tmp')
        temporary.write_text(json.dumps(receipt, indent=2) + '\n')
        temporary.replace(base.with_suffix('.json'))
    save()
    child = None
    interrupted = []
    started = time.monotonic()
    def stop(number, _frame):
        interrupted.append(number)
        if child is not None and child.poll() is None:
            os.killpg(child.pid, number)
    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    code = 1
    tail = b''
    try:
        with base.with_suffix('.log').open('wb') as log:
            child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
            digest = hashlib.sha256()
            while True:
                chunk = child.stdout.read1(65536)
                if not chunk:
                    break
                # Four UTF-8 bytes per character retain a bounded final 4K-character
                # annotation without loading the potentially very large full log.
                tail = (tail + chunk)[-TAIL_BYTE_LIMIT:]
                log.write(chunk)
                log.flush()
                digest.update(chunk)
                sys.stdout.buffer.write(chunk)
                sys.stdout.buffer.flush()
            code = child.wait()
            log.flush()
            os.fsync(log.fileno())
            receipt['log_sha256'] = digest.hexdigest()
    except OSError as error:
        receipt['error'] = str(error)
    finally:
        receipt.update(status='completed', exit=code, passed=code == 0 and not interrupted,
                       seconds=round(time.monotonic() - started, 3), interrupted=interrupted)
        save()
        if not receipt['passed']:
            sys.stdout.write(failure_annotation(args.name, receipt, tail))
            sys.stdout.flush()
    return code if code >= 0 else 128 - code


if __name__ == '__main__':
    raise SystemExit(main())
