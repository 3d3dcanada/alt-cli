#!/usr/bin/env python3
"""Exact package update/rollback gate over the persistent synthetic aged workspace.

This does not open new-schema state using the old binary, or assert model quality.
Retains only JSON/text evidence; test installations remain outside the repository.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tarfile
import time
import traceback


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def database(path):
    with sqlite3.connect(path.resolve().as_uri() + '?mode=ro', uri=True) as db:
        integrity = db.execute('PRAGMA integrity_check').fetchone()[0]
        if integrity != 'ok':
            raise RuntimeError('SQLite integrity failure: ' + str(path))
        digest = hashlib.sha256()
        for line in db.iterdump():
            digest.update(line.encode()); digest.update(b'\n')
        tables = sorted(r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table'"))
        counts = {table: db.execute('SELECT count(*) FROM "' + table.replace('"', '""') + '"').fetchone()[0] for table in tables}
        return {'logical_sha256': digest.hexdigest(), 'integrity_check': integrity, 'rows': counts}


def snapshot(root):
    entries = {}; databases = {}; total = 0
    for path in sorted(root.rglob('*')):
        if not path.is_file() or path.name.endswith(('.lock', '-wal', '-shm')):
            continue
        relative = path.relative_to(root).as_posix()
        total += path.stat().st_size
        if path.suffix in ('.db', '.sqlite'):
            databases[relative] = database(path)
            entries[relative] = databases[relative]['logical_sha256']
        else:
            entries[relative] = sha(path)
    return {'sha256': hashlib.sha256(json.dumps(entries, sort_keys=True).encode()).hexdigest(), 'files': len(entries), 'disk_bytes': total, 'databases': databases}, entries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--previous', type=Path, required=True)
    parser.add_argument('--state', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--cleanup-work', action='store_true', help='After success, remove only this run\'s disposable extracted packages, installed generations and restored state; retain receipt')
    args = parser.parse_args()
    work = args.output_dir.resolve(); work.mkdir(parents=True, exist_ok=False)
    state = args.state.resolve(); prefix = work / 'prefix'
    receipt = {'schema': 1, 'scope': 'Real published beta3 -> frozen candidate generation -> complete rollback -> candidate re-upgrade over persistent synthetic aged state; no live models, GPU measurements or human usability claim. Old binary is never asked to open the new-schema longevity state. Independent actual old-schema migration is measured by release_gate.py.', 'archive': str(args.archive.resolve()), 'archive_sha256': sha(args.archive), 'previous': str(args.previous.resolve()), 'previous_sha256': sha(args.previous), 'state': str(state), 'commands': [], 'passed': False}
    def run(name, command):
        started = time.monotonic()
        result = subprocess.run(list(map(str, command)), env={**os.environ, 'ALT_PREFIX': str(prefix)}, capture_output=True, text=True, timeout=180)
        row = {'name': name, 'argv': list(map(str, command)), 'env_overrides': {'ALT_PREFIX': str(prefix)}, 'exit': result.returncode, 'seconds': round(time.monotonic()-started, 4), 'stdout': result.stdout, 'stderr': result.stderr}
        receipt['commands'].append(row)
        if result.returncode:
            raise RuntimeError(name + ' failed with exit ' + str(result.returncode))
        return result.stdout
    def require(value, message):
        if not value:
            raise RuntimeError(message)
    try:
        initial, original_entries = snapshot(state); receipt['state_before'] = initial
        packages = []
        for label, archive in [('previous', args.previous), ('candidate', args.archive)]:
            destination = work / label; destination.mkdir()
            with tarfile.open(archive) as tar:
                tar.extractall(destination, filter='data')
            package = next(destination.iterdir()); packages.append(package)
        old, new = packages
        receipt['binary_before_sha256'] = sha(old / 'alt'); receipt['binary_candidate_sha256'] = sha(new / 'alt')
        run('install-real-published-beta3', ['bash', old / 'install.sh', '--data-dir', state])
        require(sha(prefix / 'bin/alt') == receipt['binary_before_sha256'], 'Old installed bytes differ')
        old_docs = {p.relative_to(prefix / 'share/doc/alt').as_posix(): sha(p) for p in (prefix / 'share/doc/alt').rglob('*') if p.is_file()}
        receipt['old_document_count'] = len(old_docs)
        run('update-with-persistent-aged-state', ['bash', new / 'install.sh', '--data-dir', state])
        updated = json.loads(run('verify-complete-candidate-generation', ['bash', new / 'install.sh', '--status']))
        require(updated['active_integrity_verified'], 'Candidate generation was not verified')
        require(sha(prefix / 'bin/alt') == receipt['binary_candidate_sha256'], 'Candidate installed bytes differ')
        new_manifest = json.loads((new / 'CONTENTS.json').read_text())
        require(all(sha(prefix / 'share/doc/alt' / name) == digest for name, digest in new_manifest.items()), 'Installed complete candidate docs/binary differ')
        backup = Path(updated['recovery_receipt']['state_backup'])
        receipt['automatic_backup'] = {'path': str(backup), 'sha256': sha(backup), 'bytes': backup.stat().st_size}
        restored = work / 'restored-aged-state'
        restore_output = run('restore-automatic-pre-upgrade-backup', [new / 'alt', '--data-dir', state, 'state', 'restore', backup, restored])
        receipt['automatic_backup']['restore'] = json.loads(restore_output)
        restored_summary, restored_entries = snapshot(restored)
        receipt['restored_state'] = restored_summary
        require(restored_summary['databases'] == initial['databases'], 'Restored SQLite content differs from aged source')
        require(all(original_entries.get(name) == digest for name, digest in restored_entries.items()), 'Restored core file differs from aged source')
        run('rollback-entire-published-beta3-generation', ['bash', new / 'install.sh', '--rollback'])
        rolled = json.loads(run('verify-complete-rollback-generation', ['bash', new / 'install.sh', '--status']))
        require(rolled['active_integrity_verified'], 'Rollback generation was not verified')
        receipt['binary_rollback_sha256'] = sha(prefix / 'bin/alt')
        require(receipt['binary_rollback_sha256'] == receipt['binary_before_sha256'], 'Rollback did not restore published beta3 bytes')
        require(all(sha(prefix / 'share/doc/alt' / name) == digest for name, digest in old_docs.items()), 'Rollback did not restore original documentation')
        run('re-upgrade-complete-generation', ['bash', new / 'install.sh', '--data-dir', state])
        final_status = json.loads(run('verify-final-complete-generation', ['bash', new / 'install.sh', '--status']))
        require(final_status['active_integrity_verified'] and final_status['active'] == updated['active'], 'Re-upgrade current pointer differs')
        receipt['binary_after_sha256'] = sha(prefix / 'bin/alt')
        require(receipt['binary_after_sha256'] == receipt['binary_candidate_sha256'], 'Re-upgrade executable differs')
        final, final_entries = snapshot(state); receipt['state_after'] = final
        require(final_entries == original_entries, 'Persistent aged state changed during installation/rollback')
        receipt['state_unchanged'] = True; receipt['rollback_documentation_verified'] = True
        if args.cleanup_work:
            for generated in [work / 'previous', work / 'candidate', prefix, restored]:
                shutil.rmtree(generated)
            receipt['disposable_work_removed'] = True
        receipt['passed'] = True
    except Exception as error:
        receipt['error'] = str(error); receipt['traceback'] = traceback.format_exc()
    finally:
        report = work / 'aged-install-receipt.json'
        report.write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps({'passed': receipt['passed'], 'receipt': str(report), 'error': receipt.get('error')}))
    return 0 if receipt['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
