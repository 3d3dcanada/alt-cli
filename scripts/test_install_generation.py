#!/usr/bin/env python3
"""Exercise package generations, negative updates and prior documentation recovery."""
import hashlib
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import shlex
import shutil
import struct
import subprocess
import tempfile
import termios
import time
import unittest

REPO = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get('ALT_TEST_BINARY', str(REPO / 'target/debug/alt')))
HELPERS = Path(os.environ.get('ALT_TEST_PACKAGE', str(REPO / 'scripts')))


def package(root, name, marker):
    destination = root / name
    destination.mkdir()
    shutil.copy2(BINARY, destination / 'alt')
    for helper in ['install-generation.py', 'verify-package.py']:
        shutil.copy2(HELPERS / helper, destination / helper)
    shutil.copy2(HELPERS / ('install.sh' if (HELPERS / 'install.sh').exists() else 'install-package.sh'), destination / 'install.sh')
    for name in ['README.md', 'LICENSE', 'THIRD_PARTY.md', 'DEPENDENCIES.md']:
        (destination / name).write_text(marker)
    for name in ['BUILD.json', 'SBOM.cdx.json']:
        (destination / name).write_text('{}')
    (destination / 'PLATFORM.json').write_text(json.dumps({'sha256': hashlib.sha256((destination / 'alt').read_bytes()).hexdigest()}))
    (destination / 'docs').mkdir()
    (destination / 'docs/guide.md').write_text(marker)
    manifest = {p.relative_to(destination).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in destination.rglob('*') if p.is_file()}
    (destination / 'CONTENTS.json').write_text(json.dumps(manifest, sort_keys=True))
    return destination


class Generations(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='alt-generation-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.prefix = self.root / 'prefix'
        self.env = dict(os.environ, ALT_PREFIX=str(self.prefix), ALT_DATA_DIR=str(self.root / 'unused-state'))
        self.old = package(self.root, 'old', 'old generation')
        self.new = package(self.root, 'new', 'new generation')

    def install(self, package, *args, **env):
        return subprocess.run(['bash', str(package / 'install.sh'), *args], env={**self.env, **env}, capture_output=True, text=True, timeout=30)

    def test_complete_update_rollback_and_stale_docs_removed(self):
        self.assertEqual(self.install(self.old).returncode, 0)
        old_target = os.readlink(self.prefix / 'share/alt/current')
        self.assertEqual(self.install(self.new).returncode, 0)
        self.assertNotEqual(os.readlink(self.prefix / 'share/alt/current'), old_target)
        self.assertEqual((self.prefix / 'share/doc/alt/docs/guide.md').read_text(), 'new generation')
        self.assertEqual(self.install(self.new, '--rollback').returncode, 0)
        self.assertEqual((self.prefix / 'share/doc/alt/docs/guide.md').read_text(), 'old generation')
        self.assertEqual(os.readlink(self.prefix / 'share/alt/current'), old_target)
        self.assertEqual(self.install(self.new).returncode, 0)
        self.assertEqual(self.install(self.new).returncode, 0)

    def test_missing_path_help_quotes_custom_prefix_and_state_without_editing_shell(self):
        self.prefix = self.root / "prefix with spaces and 'quotes'"
        self.env['ALT_PREFIX'] = str(self.prefix)
        home = self.root / 'home'
        home.mkdir()
        startup = home / '.bashrc'
        startup.write_text('# keep my shell settings\n')
        state = self.root / 'custom state'
        result = self.install(self.new, '--data-dir', str(state), HOME=str(home), SHELL='/bin/bash', PATH='/usr/bin:/bin')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('does not yet find this installation as alt', result.stdout)
        self.assertIn('No shell files were changed', result.stdout)
        self.assertIn(shlex.quote(str(state)), result.stdout)
        launch = result.stdout.split("Start Alt's terminal workspace:\n  ", 1)[1].splitlines()[0]
        self.assertEqual(shlex.split(launch), [str(self.prefix / 'bin/alt'), '--data-dir', str(state)])
        setting = next(line.strip() for line in result.stdout.splitlines() if line.startswith('  export PATH='))
        checked = subprocess.run(['bash', '-c', setting + '\nhash -r\ncommand -v alt\nalt --version'], env={**self.env, 'PATH': '/usr/bin:/bin'}, capture_output=True, text=True, check=True)
        self.assertEqual(checked.stdout.splitlines()[0], str(self.prefix / 'bin/alt'))
        self.assertEqual(startup.read_text(), '# keep my shell settings\n')

    def test_shadowing_alt_is_identified_without_running_or_replacing_it(self):
        existing = self.root / 'other-bin'
        existing.mkdir()
        marker = self.root / 'other-command-ran'
        other = existing / 'alt'
        payload = '#!/bin/sh\ntouch ' + shlex.quote(str(marker)) + '\n'
        other.write_text(payload)
        other.chmod(0o755)
        path = str(existing) + ':' + str(self.prefix / 'bin') + ':/usr/bin:/bin'
        result = self.install(self.new, PATH=path)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('Another alt command comes first on PATH: ' + str(other), result.stdout)
        self.assertFalse(marker.exists())
        self.assertEqual(other.read_text(), payload)

    def test_fish_path_help_and_repeat_install_still_explain_launch(self):
        self.prefix = self.root / "fish prefix's back\\slash"
        self.env['ALT_PREFIX'] = str(self.prefix)
        for _ in range(2):
            result = self.install(self.new, SHELL='/usr/bin/fish', PATH='/usr/bin:/bin')
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn('set -gx PATH ', result.stdout)
            self.assertIn("prefix\\'s back\\\\slash/bin' $PATH", result.stdout)
            self.assertIn('~/.config/fish/config.fish', result.stdout)
            self.assertNotIn('export PATH=', result.stdout)

    def test_installed_alt_command_opens_tui_from_path_and_restores_terminal(self):
        path = str(self.prefix / 'bin') + ':' + os.environ['PATH']
        result = self.install(self.new, PATH=path)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('The command is the word alt, then Enter', result.stdout)
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 80, 0, 0))
        original = termios.tcgetattr(slave)
        environment = {**self.env, 'PATH': path, 'TERM': 'xterm-256color', 'XDG_DATA_HOME': str(self.root / 'tui-data')}
        environment.pop('ALT_DATA_DIR', None)
        process = subprocess.Popen(['alt'], cwd=self.root, stdin=slave, stdout=slave, stderr=slave, env=environment)
        output = bytearray()
        try:
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                if select.select([master], [], [], 0.05)[0]:
                    output.extend(os.read(master, 65536))
                if b'Connect your first model' in output:
                    break
                self.assertIsNone(process.poll(), output.decode(errors='replace'))
            self.assertIn(b'Connect your first model', output, output.decode(errors='replace'))
            os.write(master, b'\x11')
            deadline = time.monotonic() + 10
            closing = bytearray()
            confirmed = False
            while process.poll() is None and time.monotonic() < deadline:
                if select.select([master], [], [], 0.05)[0]:
                    closing.extend(os.read(master, 65536))
                if not confirmed and b'Leave Alt?' in closing:
                    os.write(master, b'\t\r')
                    confirmed = True
            self.assertIsNotNone(process.poll(), closing.decode(errors='replace'))
            self.assertEqual(process.returncode, 0)
            self.assertEqual(termios.tcgetattr(slave), original)
            self.assertTrue((self.root / 'tui-data/alt-cli').is_dir())
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            os.close(master)
            os.close(slave)

    def test_tamper_and_failed_backup_never_activate(self):
        self.assertEqual(self.install(self.old).returncode, 0)
        old_target = os.readlink(self.prefix / 'share/alt/current')
        (self.new / 'docs/guide.md').write_text('tampered')
        for optimization in ['0', '1', '2']:
            self.assertNotEqual(self.install(self.new, PYTHONOPTIMIZE=optimization).returncode, 0)
            self.assertEqual(os.readlink(self.prefix / 'share/alt/current'), old_target)
        (self.new / 'docs/guide.md').write_text('new generation')
        Path(self.env['ALT_DATA_DIR']).mkdir()
        (self.prefix / 'share/alt/backups').write_text('blocked')
        self.assertNotEqual(self.install(self.new).returncode, 0)
        self.assertEqual(os.readlink(self.prefix / 'share/alt/current'), old_target)
        self.assertEqual((self.prefix / 'share/doc/alt/docs/guide.md').read_text(), 'old generation')

    def test_legacy_document_conflict_is_resolved_before_switch_and_rollback_is_complete(self):
        (self.prefix / 'bin').mkdir(parents=True)
        shutil.copy2(BINARY, self.prefix / 'bin/alt')
        docs = self.prefix / 'share/doc/alt'
        docs.mkdir(parents=True)
        (docs / 'docs').write_text('legacy conflict must be retained')
        self.assertEqual(self.install(self.new).returncode, 0)
        self.assertEqual((docs / 'docs/guide.md').read_text(), 'new generation')
        self.assertEqual(self.install(self.new, '--rollback').returncode, 0)
        self.assertEqual((docs / 'docs').read_text(), 'legacy conflict must be retained')

    def test_destination_conflict_prevents_binary_change(self):
        (self.prefix / 'bin').mkdir(parents=True)
        shutil.copy2(BINARY, self.prefix / 'bin/alt')
        (self.prefix / 'share/doc').mkdir(parents=True)
        (self.prefix / 'share/doc/alt').write_text('not a directory')
        before = (self.prefix / 'bin/alt').read_bytes()
        self.assertNotEqual(self.install(self.new).returncode, 0)
        self.assertEqual((self.prefix / 'bin/alt').read_bytes(), before)
        self.assertFalse((self.prefix / 'bin/alt').is_symlink())

    def test_changed_verifier_is_rejected_before_import_or_binary_execution(self):
        marker = self.root / 'verifier-executed'
        (self.new / 'verify-package.py').write_text('from pathlib import Path\nPath({!r}).write_text("executed")\n'.format(str(marker)))
        result = self.install(self.new)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(marker.exists())
        self.assertFalse((self.prefix / 'bin/alt').exists())

    def test_out_of_band_binary_is_preserved_instead_of_overwritten(self):
        self.assertEqual(self.install(self.old).returncode, 0)
        installed = self.prefix / 'bin/alt'
        installed.unlink()
        installed.write_bytes(b'custom executable preserve me')
        self.assertNotEqual(self.install(self.new).returncode, 0)
        self.assertEqual(installed.read_bytes(), b'custom executable preserve me')

    def test_abrupt_exit_around_every_pointer_keeps_a_complete_generation(self):
        runner = r"""
import importlib.util,os,sys
from pathlib import Path
# The helper can come from an immutable extracted release package. Importing the
# fault target must not add unmanifested __pycache__ files to that package.
sys.dont_write_bytecode=True
spec=importlib.util.spec_from_file_location('installer',sys.argv[1])
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
original=module.link
boundary,phase=sys.argv[3:5]
def interrupted(path,target):
    match=str(path).endswith(boundary)
    if match and phase=='before':os._exit(86)
    original(path,target)
    if match and phase=='after':os._exit(87)
module.link=interrupted
sys.argv=['installer','--package',sys.argv[2]]
module.main()
"""
        for index, boundary in enumerate(['/share/doc/alt', '/bin/alt', '/bin/alt.previous', '/share/alt/previous', '/share/alt/current']):
            for phase in ['before', 'after']:
                with self.subTest(boundary=boundary, phase=phase):
                    self.prefix = self.root / ('boundary-{}-{}'.format(index, phase))
                    self.env['ALT_PREFIX'] = str(self.prefix)
                    self.assertEqual(self.install(self.old).returncode, 0)
                    result = subprocess.run(['python3', '-c', runner, str(HELPERS / 'install-generation.py'), str(self.new), boundary, phase], env=self.env, capture_output=True, text=True, timeout=30)
                    self.assertIn(result.returncode, [86, 87], result.stdout + result.stderr)
                    self.assertIn((self.prefix / 'share/doc/alt/docs/guide.md').read_text(), ['old generation', 'new generation'])
                    self.assertEqual(self.install(self.new, '--status').returncode, 0)
                    self.assertEqual(self.install(self.new).returncode, 0)
                    self.assertEqual((self.prefix / 'share/doc/alt/docs/guide.md').read_text(), 'new generation')

    def test_io_failure_during_receipt_keeps_existing_generation(self):
        self.assertEqual(self.install(self.old).returncode, 0)
        old_target = os.readlink(self.prefix / 'share/alt/current')
        library = self.root / 'fault.so'
        subprocess.run(['cc', '-shared', '-fPIC', '-O2', str(REPO / 'tests/fixtures/fault_io.c'), '-ldl', '-o', str(library)], check=True)
        for mode in ['enospc', 'crash-before-rename', 'crash-after-rename']:
            result = self.install(self.new, LD_PRELOAD=str(library), ALT_FAULT_PROJECT=str(self.prefix / 'share/alt'), ALT_FAULT_MATCH='/.alt-install-', ALT_FAULT_MODE=mode)
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(os.readlink(self.prefix / 'share/alt/current'), old_target)
            self.assertEqual((self.prefix / 'share/doc/alt/docs/guide.md').read_text(), 'old generation')
        self.assertEqual(self.install(self.new).returncode, 0)


if __name__ == '__main__':
    unittest.main()
