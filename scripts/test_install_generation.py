#!/usr/bin/env python3
"""Exercise package generations, negative updates and prior documentation recovery."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
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
