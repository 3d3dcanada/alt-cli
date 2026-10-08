#!/usr/bin/env python3
"""Validate every package file without trusting Python assertion settings."""
import argparse
import hashlib
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    result = hashlib.sha256()
    with path.open('rb') as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b''):
            result.update(chunk)
    return result.hexdigest()


def verify(package):
    package = Path(package)
    require(not package.is_symlink(), 'Linked package root')
    contents = json.loads((package / 'CONTENTS.json').read_text())
    required = {'alt', 'install.sh', 'README.md', 'LICENSE', 'THIRD_PARTY.md',
                'DEPENDENCIES.md', 'PLATFORM.json', 'BUILD.json', 'SBOM.cdx.json'}
    require(isinstance(contents, dict) and required <= contents.keys(), 'Incomplete package manifest')
    allowed = set(contents) | {'CONTENTS.json', 'CONTENTS.json.sig', 'PLATFORM.json.sig'}
    for path in package.rglob('*'):
        require(not path.is_symlink(), 'Linked package entry: ' + str(path))
        require(path.is_dir() or (path.is_file() and path.relative_to(package).as_posix() in allowed),
                'Unexpected package entry: ' + str(path))
    for name, expected in contents.items():
        relative = Path(name)
        require(not relative.is_absolute() and '..' not in relative.parts, 'Invalid content path')
        path = package / relative
        require(path.is_file() and not path.is_symlink(), 'Missing or linked package file: ' + name)
        require(digest(path) == expected, 'Package integrity failed: ' + name)
    platform = json.loads((package / 'PLATFORM.json').read_text())
    require(digest(package / 'alt') == platform['sha256'], 'Binary checksum failed')
    return contents


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    args = parser.parse_args()
    try:
        manifest = verify(args.package)
        print(json.dumps({'passed': True, 'files': len(manifest)}))
    except (OSError, ValueError, KeyError) as error:
        raise SystemExit(str(error))
