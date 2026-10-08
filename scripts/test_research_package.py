#!/usr/bin/env python3
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(path))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


PACK = module('research_pack', 'package-research.py')
VERIFY = module('research_verify', 'verify-research.py')


class ResearchArchive(unittest.TestCase):
    def test_all_nested_baseline_files_are_accounted_for_and_index_tamper_fails(self):
        with tempfile.TemporaryDirectory(prefix='alt-research-test-') as directory:
            root = Path(directory)
            (root / 'docs/evidence/failed').mkdir(parents=True)
            (root / 'docs/INSTALLATION.md').write_text('offline help')
            (root / 'docs/evidence/failed/raw.json').write_text('{"passed":false}')
            (root / 'dist').mkdir()
            receipt = PACK.package_research(root, root / 'dist', 'test', {'commit': 'abc', 'source_sha256': 'def'})
            archive = root / 'dist' / receipt['archive']
            index = root / 'dist' / receipt['index']
            self.assertEqual(VERIFY.verify(archive, index)['files'], 2)
            payload = json.loads(index.read_text())
            self.assertIn('docs/evidence/failed/raw.json', payload['files'])
            del payload['files']['docs/evidence/failed/raw.json']
            index.write_text(json.dumps(payload))
            with self.assertRaises(ValueError):
                VERIFY.verify(archive, index)

    def test_linked_evidence_is_not_silently_followed_or_omitted(self):
        with tempfile.TemporaryDirectory(prefix='alt-research-test-') as directory:
            root = Path(directory)
            (root / 'docs').mkdir()
            (root / 'outside').write_text('outside')
            (root / 'docs/link').symlink_to(root / 'outside')
            with self.assertRaises(ValueError):
                PACK.package_research(root, root, 'test', {'commit': 'abc', 'source_sha256': 'def'})


if __name__ == '__main__':
    unittest.main()
