#!/usr/bin/env python3
"""Install complete generations; one atomic pointer selects executable and docs.

Legacy migration is journaled and repeatable. Python 3.8+ standard library only.
"""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import uuid


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sync(directory):
    descriptor = os.open(str(directory), os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def write(path, data):
    descriptor, temporary = tempfile.mkstemp(prefix='.alt-install-', dir=str(path.parent))
    try:
        with os.fdopen(descriptor, 'wb') as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        sync(path.parent)
    finally:
        if os.path.lexists(temporary):
            os.unlink(temporary)


def link(path, target):
    temporary = path.parent / ('.alt-install-' + uuid.uuid4().hex)
    try:
        temporary.symlink_to(target)
        os.replace(str(temporary), str(path))
        sync(path.parent)
    finally:
        if temporary.is_symlink():
            temporary.unlink()


def current_identity(control):
    current = control / 'current'
    return os.readlink(current) if current.is_symlink() else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', type=Path, required=True)
    parser.add_argument('--rollback', action='store_true')
    parser.add_argument('--status', action='store_true', help='Inspect actual active and previous generations and recovery receipt')
    parser.add_argument('--data-dir', type=Path)
    parser.add_argument('--verify-key', type=Path)
    args = parser.parse_args()
    package = args.package.resolve()
    prefix = Path(os.environ.get('ALT_PREFIX', str(Path.home() / '.local'))).absolute()
    state = args.data_dir or Path(os.environ.get('ALT_DATA_DIR', str(Path(os.environ.get('XDG_DATA_HOME', str(Path.home() / '.local/share'))) / 'alt-cli')))
    control = prefix / 'share/alt'
    generations = control / 'installations'
    bindir = prefix / 'bin'
    docdir = prefix / 'share/doc'
    for directory in (control, generations, bindir, docdir):
        directory.mkdir(parents=True, exist_ok=True)
    lock_path = control / 'install.lock'
    require(not lock_path.is_symlink(), 'Installation lock is a symlink')
    with lock_path.open('a+b') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        journal = control / 'installation.json'
        def record(stage, **details):
            write(journal, (json.dumps(dict(schema=1, stage=stage, active=current_identity(control), **details), indent=2) + '\n').encode())
        # Authenticate the manifest first when a trusted key was supplied, then
        # validate the verifier module with stdlib hashing before importing it.
        # Published users authenticate the complete archive before running install.sh.
        if args.verify_key:
            subprocess.run(['openssl', 'dgst', '-sha256', '-verify', str(args.verify_key), '-signature', str(package / 'CONTENTS.json.sig'), str(package / 'CONTENTS.json')], check=True)
        require((package / 'CONTENTS.json').stat().st_size <= 64 * 1024 * 1024, 'Package manifest exceeds 64 MiB')
        manifest = json.loads((package / 'CONTENTS.json').read_text())
        helper = package / 'verify-package.py'
        require(not helper.is_symlink() and helper.is_file() and helper.stat().st_size <= 1024 * 1024, 'Missing, linked or oversized package verifier')
        require(hashlib.sha256(helper.read_bytes()).hexdigest() == manifest.get('verify-package.py'), 'Package verifier integrity failed before import')
        spec = importlib.util.spec_from_file_location('alt_package_verifier', package / 'verify-package.py')
        module = importlib.util.module_from_spec(spec)
        sys.dont_write_bytecode = True
        spec.loader.exec_module(module)
        if args.status:
            identity = current_identity(control)
            active = (control / identity).resolve() if identity else None
            if active:
                require(active.parent == generations.resolve(), 'Active generation escaped installation folder')
                if (active / 'LEGACY.json').is_file():
                    legacy = json.loads((active / 'LEGACY.json').read_text())
                    require(module.digest(active / 'alt') == legacy['binary_sha256'], 'Legacy generation integrity failed')
                    for name, expected in legacy.get('files', {}).items():
                        require(module.digest(active / name) == expected, 'Legacy generation documentation integrity failed: ' + name)
                else:
                    module.verify(active)
            print(json.dumps({'schema': 1, 'active': identity, 'previous': os.readlink(control / 'previous') if (control / 'previous').is_symlink() else None, 'active_integrity_verified': bool(active), 'recovery_receipt': json.loads(journal.read_text()) if journal.exists() else None}, indent=2))
            return
        if args.rollback:
            previous = control / 'previous'
            require(previous.is_symlink() and (previous / 'alt').is_file(), 'No complete previous generation was saved')
            target = os.readlink(previous)
            require((control / target).resolve().parent == generations.resolve(), 'Previous generation escaped installation folder')
            prior_path = (control / target).resolve()
            if not (prior_path / 'LEGACY.json').is_file():
                module.verify(prior_path)
            else:
                legacy = json.loads((prior_path / 'LEGACY.json').read_text())
                require(module.digest(prior_path / 'alt') == legacy['binary_sha256'], 'Legacy rollback binary integrity failed')
                for name, expected in legacy.get('files', {}).items():
                    require(module.digest(prior_path / name) == expected, 'Legacy rollback documentation integrity failed: ' + name)
            subprocess.run([str(previous / 'alt'), '--version'], check=True)
            old = current_identity(control)
            record('rollback-prepared', target=target, previous=old)
            link(control / 'current', target)
            if old:
                link(previous, old)
            record('complete', operation='rollback', previous=old)
            print('Previous executable and documentation generation restored. Restore matching pre-upgrade state into a new folder if a schema downgrade is needed.')
            return
        require(sys.platform.startswith('linux') and os.uname().machine == 'x86_64', 'This package requires Linux x86_64')
        contents = module.verify(package)
        subprocess.run([str(package / 'alt'), '--version'], check=True)
        destination_docs = docdir / 'alt'
        require(not destination_docs.exists() or destination_docs.is_dir(), 'Documentation destination is a regular file; installation remains unchanged: ' + str(destination_docs))
        if destination_docs.is_symlink():
            require(os.readlink(destination_docs) == '../alt/current', 'Documentation symlink points outside the managed generation')
        executable = bindir / 'alt'
        if executable.is_symlink():
            require(os.readlink(executable) == '../share/alt/current/alt', 'Executable symlink points outside the managed generation')
        package_id = hashlib.sha256((package / 'CONTENTS.json').read_bytes()).hexdigest()
        generation = generations / package_id
        previous_id = current_identity(control)
        require(not (control / 'current').exists() or (control / 'current').is_symlink(), 'Current installation pointer is not a symlink')
        if previous_id:
            require((control / previous_id).resolve().parent == generations.resolve(), 'Current generation escaped installation folder')
            if executable.exists() and not executable.is_symlink():
                prior_executable = control / previous_id / 'alt'
                require(prior_executable.is_file() and module.digest(executable) == module.digest(prior_executable), 'The managed executable was replaced outside this installer. Preserve that binary and choose a fresh prefix or restore the managed layout before upgrading')
        if not generation.exists():
            with tempfile.TemporaryDirectory(prefix='.alt-install-stage-', dir=str(generations)) as staging:
                staged = Path(staging) / 'package'
                shutil.copytree(package, staged)
                module.verify(staged)
                for item in staged.rglob('*'):
                    if item.is_file():
                        with item.open('rb') as handle:
                            os.fsync(handle.fileno())
                for item in sorted((p for p in staged.rglob('*') if p.is_dir()), key=lambda p: len(p.parts), reverse=True):
                    sync(item)
                sync(staged)
                os.rename(str(staged), str(generation))
                sync(generations)
        else:
            module.verify(generation)
        target = 'installations/' + package_id
        if previous_id == target:
            record('complete', operation='already-installed')
            print('This complete Alt generation is already installed: ' + str(executable))
            return
        backup = None
        if executable.is_file() and state.is_dir():
            backups = control / 'backups'
            backups.mkdir(mode=0o700, exist_ok=True)
            backup = backups / ('pre-upgrade-' + str(time.time_ns()) + '.tar.gz')
            result = subprocess.run([str(package / 'alt'), '--data-dir', str(state), 'state', 'backup', str(backup)], capture_output=True, text=True)
            require(result.returncode == 0, 'Pre-upgrade backup failed; active generation unchanged: ' + result.stderr)
            write(Path(str(backup) + '.report.json'), result.stdout.encode())
        if previous_id is None and executable.is_file():
            legacy = generations / ('legacy-' + uuid.uuid4().hex)
            legacy.mkdir()
            if destination_docs.is_dir():
                shutil.copytree(destination_docs, legacy, dirs_exist_ok=True)
            shutil.copy2(executable, legacy / 'alt')
            write(legacy / 'LEGACY.json', json.dumps({'source': 'Pre-generation installation', 'binary_sha256': module.digest(legacy / 'alt'), 'files': {p.relative_to(legacy).as_posix(): module.digest(p) for p in legacy.rglob('*') if p.is_file()}}).encode())
            sync(legacy)
            previous_id = 'installations/' + legacy.name
            record('legacy-prepared', legacy=previous_id, target=target)
            link(control / 'current', previous_id)
        record('prepared', target=target, previous=previous_id, state_backup=str(backup) if backup else None)
        if destination_docs.is_dir() and not destination_docs.is_symlink():
            retained = docdir / ('alt.legacy-' + uuid.uuid4().hex)
            record('migrating-legacy-docs', target=target, previous=previous_id, retained_docs=str(retained))
            os.rename(str(destination_docs), str(retained))
            sync(docdir)
        link(destination_docs, '../alt/current')
        link(executable, '../share/alt/current/alt')
        link(bindir / 'alt.previous', '../share/alt/previous/alt')
        if previous_id:
            link(control / 'previous', previous_id)
        record('activation-prepared', target=target, previous=previous_id)
        link(control / 'current', target)
        record('complete', operation='install', previous=previous_id, files=len(contents), state_backup=str(backup) if backup else None)
        print('Installed complete Alt generation: ' + str(executable))
        if backup:
            print('Saved pre-upgrade state backup: ' + str(backup))


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError, ValueError, subprocess.CalledProcessError) as error:
        prefix = Path(os.environ.get('ALT_PREFIX', str(Path.home() / '.local'))).absolute()
        control = prefix / 'share/alt'
        print('Installation stopped: {}\nActive generation: {}\nRecovery receipt: {}'.format(error, current_identity(control) or 'legacy layout or no installation', control / 'installation.json'), file=sys.stderr)
        raise SystemExit(1)
