#!/usr/bin/env python3
"""Verify archive integrity, failed updates, prior-version state and atomic rollback."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('archive', type=Path)
parser.add_argument('--previous', type=Path)
parser.add_argument('--verify-key', type=Path)
parser.add_argument('--tui', action='store_true')
a = parser.parse_args()
repo = Path(__file__).resolve().parents[1]
assert hashlib.sha256(a.archive.read_bytes()).hexdigest() == Path(str(a.archive)+'.sha256').read_text().split()[0]
if a.verify_key:
    subprocess.run(['openssl', 'dgst', '-sha256', '-verify', str(a.verify_key), '-signature', str(a.archive)+'.sig', str(a.archive)], check=True)
with tempfile.TemporaryDirectory(prefix='alt-installed-') as temp:
    root = Path(temp)
    unpack = root/'unpack'
    with tarfile.open(a.archive) as archive:
        archive.extractall(unpack, filter='data')
    package = next(unpack.iterdir())
    platform = json.loads((package/'PLATFORM.json').read_text())
    assert hashlib.sha256((package/'alt').read_bytes()).hexdigest() == platform['sha256']
    prefix = root/'installed'
    env = {**os.environ, 'ALT_PREFIX': str(prefix)}
    binary = prefix/'bin/alt'
    project = root/'project'
    project.mkdir()
    (project/'check.py').write_text("print('package behavior checked')\n")
    state = root/'state'
    env['ALT_DATA_DIR'] = str(state)
    def invoke(*args):
        return subprocess.run([str(binary), '--data-dir', str(state), *args], cwd=project, check=True, capture_output=True, text=True)
    installer = ['bash', str(package/'install.sh')]
    if a.verify_key:
        installer += ['--verify-key', str(a.verify_key.resolve())]
    if a.previous:
        previous_root = root/'previous'
        with tarfile.open(a.previous) as archive:
            archive.extractall(previous_root, filter='data')
        old = next(previous_root.iterdir())/'alt'
        binary.parent.mkdir(parents=True)
        shutil.copy2(old, binary)
        prior_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
        invoke('task', 'configure-check', 'package-check', '--', 'python3', 'check.py')
        invoke('--access', 'trusted', 'task', 'check', 'package-check')
        invoke('state','backup',str(root/'pre-upgrade-state.tar.gz'))
        # Failure to create the automatic backup must leave the old installation intact.
        blocked = prefix/'share/alt/backups'
        blocked.parent.mkdir(parents=True,exist_ok=True)
        blocked.write_text('A file blocks the backup directory')
        rejected=subprocess.run(installer,env=env,capture_output=True)
        assert rejected.returncode!=0 and hashlib.sha256(binary.read_bytes()).hexdigest()==prior_hash
        blocked.unlink()
    else:
        prior_hash = None
    if a.previous:
        library=root/'fault.so'
        subprocess.run(['cc','-shared','-fPIC','-O2',str(repo/'tests/fixtures/fault_io.c'),'-ldl','-o',str(library)],check=True)
        old_bytes=binary.read_bytes()
        for mode in ['enospc','crash-before-rename','crash-after-rename']:
            binary.write_bytes(old_bytes);binary.chmod(0o755)
            interrupted=subprocess.run(installer,env={**env,'LD_PRELOAD':str(library),'ALT_FAULT_PROJECT':str(binary.parent),'ALT_FAULT_MATCH':'/.alt-install-','ALT_FAULT_MODE':mode},capture_output=True)
            assert interrupted.returncode!=0,(mode,'injector did not interrupt install')
            assert binary.read_bytes() in [old_bytes,(package/'alt').read_bytes()],(mode,'partial binary installed')
            subprocess.run([str(binary),'--version'],check=True,capture_output=True)
        binary.write_bytes(old_bytes);binary.chmod(0o755)
    for _ in range(2):
        subprocess.run(installer, env=env, check=True, capture_output=True)
    assert binary.read_bytes() == (package/'alt').read_bytes()
    if a.previous:
        backups=sorted((prefix/'share/alt/backups').glob('pre-upgrade-*.tar.gz'))
        assert backups, 'Installer did not preserve pre-upgrade state'
        restored_auto=root/'automatic-backup-restored'
        invoke('state','restore',str(backups[-1]),str(restored_auto))
        prior_checks=subprocess.check_output([str(old),'--data-dir',str(restored_auto),'task','checks'],cwd=project,text=True)
        assert 'package-check' in prior_checks, 'Automatic backup lost the previous check configuration'
    pc_report=root/'pc-report'
    subprocess.run(['python3',str(package/'test-my-pc.py'),'--alt',str(binary),'--data-dir',str(state),'--output',str(pc_report)],check=True,capture_output=True)
    assert json.loads((pc_report/'summary.json').read_text())['passed']
    invoke('--version')
    assert (prefix/'share/doc/alt/DEPENDENCIES.md').is_file()
    assert len(list((prefix/'share/doc/alt/licenses').iterdir())) > 100
    invoke('runtime', '--gpu-layers', '0', '--threads', '2')
    invoke('task', 'configure-check', 'package-check', '--', 'python3', 'check.py')
    invoke('task', 'require', 'behavior', '--check', 'package-check')
    invoke('--access', 'trusted', 'task', 'verify', '--run')
    status = json.loads(invoke('task', 'status').stdout)
    assert status['verification']['complete'], status
    invoke('state', 'backup', str(root/'backup.tar.gz'))
    invoke('state', 'restore', str(root/'backup.tar.gz'), str(root/'restored-state'))
    assert (root/'restored-state').is_dir()
    # A changed documentation payload must reject the entire update before replacing Alt.
    untouched = binary.read_bytes()
    documentation = package/'README.md'
    original = documentation.read_bytes()
    documentation.write_bytes(original+b'\nTAMPERED\n')
    rejected = subprocess.run(installer, env=env, capture_output=True)
    assert rejected.returncode != 0 and binary.read_bytes() == untouched
    documentation.write_bytes(original)
    unexpected = package/'docs/unmanifested.txt'
    unexpected.write_text('Unlisted files must not be installed.\n')
    assert subprocess.run(installer, env=env, capture_output=True).returncode != 0
    assert binary.read_bytes() == untouched
    unexpected.unlink()
    unexpected.symlink_to(documentation)
    assert subprocess.run(installer, env=env, capture_output=True).returncode != 0
    assert binary.read_bytes() == untouched
    unexpected.unlink()
    if a.verify_key:
        manifest = package/'CONTENTS.json'
        original_manifest = manifest.read_bytes()
        manifest.write_bytes(original_manifest+b' ')
        assert subprocess.run(installer, env=env, capture_output=True).returncode != 0
        assert binary.read_bytes() == untouched
        manifest.write_bytes(original_manifest)
    if a.tui:
        subprocess.run(['python3', str(repo/'scripts/smoke_workbench_tui.py')], env={**env, 'ALT_TEST_BINARY': str(binary)}, cwd=project, check=True)
    if prior_hash:
        assert hashlib.sha256((prefix/'bin/alt.previous').read_bytes()).hexdigest() == prior_hash
        subprocess.run(['bash', str(package/'install.sh'), '--rollback'], env=env, check=True, capture_output=True)
        assert hashlib.sha256(binary.read_bytes()).hexdigest() == prior_hash
        # A binary rollback must use its matching state generation. Keep newer state recoverable.
        state.rename(root/'upgraded-state-retained')
        invoke('state','restore',str(root/'pre-upgrade-state.tar.gz'),str(state))
        invoke('task', 'status')
        assert 'package-check' in invoke('task', 'checks').stdout
        invoke('--access', 'trusted', 'task', 'check', 'package-check')
        previous_contract=json.loads(invoke('task','export').stdout)['checks'][-1].get('environment',{}).get('verification_contract')
        subprocess.run(installer, env=env, check=True, capture_output=True)
        invoke('task','require','behavior','--check','package-check')
        # 0.5-compatible evidence stays current; pre-0.5 evidence requires a rerun.
        assert json.loads(invoke('task', 'status').stdout)['verification']['complete'] == (previous_contract=='3')
        invoke('--access', 'trusted', 'task', 'verify', '--run')
        assert json.loads(invoke('task', 'status').stdout)['verification']['complete']
    invoke('hardware')
    print('PASS: archive, every-file integrity, isolated repeated install, failed update preserves binary, state backup/restore, docs/licenses'+('; actual previous-version upgrade/rollback/re-upgrade' if prior_hash else '')+('; signature/tamper rejection' if a.verify_key else '')+('; installed TUI' if a.tui else ''))
