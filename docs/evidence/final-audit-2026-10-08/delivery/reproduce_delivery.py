#!/usr/bin/env python3
"""Bounded published-artifact audit; all generated state is temporary and removed."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile


def sha(body):
    return hashlib.sha256(body).hexdigest()


def invoke(command, **kwargs):
    result = subprocess.run(command, capture_output=True, text=True, timeout=30, **kwargs)
    return dict(command=command, exit=result.returncode, stdout=result.stdout, stderr=result.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', type=Path, required=True)
    parser.add_argument('--previous', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    package = args.package.resolve()
    binary = package / 'alt'
    provenance = json.loads(subprocess.check_output([str(binary), 'build-info']))
    receipts = dict(schema=1, commit=provenance['commit'], release_tag=provenance['release_tag'],
                    binary_sha256=sha(binary.read_bytes()), scope='Disposable reproductions using actual released binaries. No network, model, or normal user-state access.', checks={})
    with tempfile.TemporaryDirectory(prefix='alt-audit-retention-', dir='/tmp') as directory:
        root = Path(directory)
        state = root / 'state'
        bucket = state / 'inference' / 'old-connection'
        bucket.mkdir(parents=True)
        stale = bucket / 'retained-response.raw'
        stale.write_bytes(b'old request/response evidence')
        os.utime(stale, (1, 1))
        os.utime(bucket, (1, 1))
        retained = invoke([str(binary), '--data-dir', str(state), 'state', 'retain', '--days', '1', '--apply'])
        retained['stale_inference_retained'] = stale.exists()
        retained['stale_timestamp'] = 1
        oversized = bucket / 'oversize.raw'
        with oversized.open('wb') as handle:
            handle.truncate(512 * 1024 * 1024 + 1)
        backup = invoke([str(binary), '--data-dir', str(state), 'state', 'backup', str(root / 'backup.tar.gz')])
        backup['sparse_test_file_bytes'] = oversized.stat().st_size
        backup['backup_exists'] = (root / 'backup.tar.gz').exists()
        receipts['checks']['retention_and_backup'] = dict(retention=retained, backup=backup,
            finding_reproduced=retained['exit'] == 0 and retained['stale_inference_retained'] and backup['exit'] == 1 and 'exceeds 512 MiB' in backup['stderr'])
    with tempfile.TemporaryDirectory(prefix='alt-audit-update-', dir='/tmp') as directory:
        root = Path(directory)
        prefix = root / 'prefix'
        (prefix / 'bin').mkdir(parents=True)
        docs = prefix / 'share/doc/alt'
        docs.mkdir(parents=True)
        with tarfile.open(args.previous) as archive:
            member = next(m for m in archive if m.name.endswith('/alt') and m.isfile())
            before = archive.extractfile(member).read()
        installed = prefix / 'bin/alt'
        installed.write_bytes(before)
        installed.chmod(0o755)
        (docs / 'docs').write_text('A file occupying the documentation directory path')
        env_overrides = dict(ALT_PREFIX=str(prefix), ALT_DATA_DIR=str(root / 'empty-state'))
        result = invoke(['bash', str(package / 'install.sh')], env={**os.environ, **env_overrides})
        result.update(environment_overrides=env_overrides,
                      setup='A genuine beta 1 binary installed at prefix/bin/alt; prefix/share/doc/alt/docs is a regular file.',
                      previous_archive_sha256=sha(args.previous.read_bytes()),
                      before_sha256=sha(before), after_sha256=sha(installed.read_bytes()),
                      saved_previous_sha256=sha((prefix / 'bin/alt.previous').read_bytes()),
                      installed_commit=json.loads(subprocess.check_output([str(installed), 'build-info']))['commit'],
                      binary_changed_despite_failed_installer=result['exit'] != 0 and installed.read_bytes() != before,
                      binary_equals_beta3=installed.read_bytes() == binary.read_bytes(),
                      previous_preserved=(prefix / 'bin/alt.previous').read_bytes() == before)
        receipts['checks']['partial_installation'] = result
    receipts['temporary_resources_removed'] = True
    args.output.write_text(json.dumps(receipts, indent=2) + '\n')
    print(args.output)


if __name__ == '__main__':
    main()
