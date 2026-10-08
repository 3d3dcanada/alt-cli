#!/usr/bin/env python3
"""Verify complete historical evidence, index and application-package binding."""
import argparse
import hashlib
import json
from pathlib import Path
import tarfile


def digest_stream(handle):
    value = hashlib.sha256()
    for chunk in iter(lambda: handle.read(1024 * 1024), b''):
        value.update(chunk)
    return value.hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify(archive, index, package=None):
    external_bytes = index.read_bytes()
    require(len(external_bytes) <= 64 * 1024 * 1024, 'Research index exceeds 64 MiB')
    manifest = json.loads(external_bytes)
    files = manifest['files']
    observed = set()
    embedded = False
    total = 0
    with tarfile.open(archive, 'r|gz') as tar:
        for member in tar:
            require(member.isfile(), 'Research archive contains an unexpected special entry')
            if member.name == 'RESEARCH-INDEX.json':
                require(not embedded and member.size == len(external_bytes), 'Unexpected research index')
                require(tar.extractfile(member).read() == external_bytes, 'Research index differs from archive')
                embedded = True
                continue
            require(member.name in files and member.name not in observed, 'Unindexed or duplicate evidence entry')
            relative = Path(member.name)
            require(not relative.is_absolute() and '..' not in relative.parts, 'Unsafe research path')
            expected = files[member.name]
            require(member.size == expected['bytes'], 'Evidence length differs: ' + member.name)
            require(digest_stream(tar.extractfile(member)) == expected['sha256'], 'Evidence checksum differs: ' + member.name)
            observed.add(member.name)
            total += member.size
    require(embedded and observed == set(files), 'Historical evidence omitted from research archive')
    require(total == manifest['total_bytes'], 'Research byte count differs')
    if package:
        with tarfile.open(package) as tar:
            member = next(m for m in tar if m.name.endswith('/RESEARCH.json'))
            require(member.size <= 65536, 'Research binding exceeds 64 KiB')
            binding = json.load(tar.extractfile(member))
        with archive.open('rb') as handle:
            require(digest_stream(handle) == binding['sha256'], 'Application package research digest mismatch')
        require(hashlib.sha256(external_bytes).hexdigest() == binding['index_sha256'], 'Research index binding mismatch')
        require(binding['source_commit'] == manifest['source_commit'] and binding['source_sha256'] == manifest['source_sha256'], 'Research source identity mismatch')
    return dict(passed=True, files=len(files), uncompressed_bytes=total, source_commit=manifest['source_commit'], package_binding_checked=bool(package))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('--index', required=True, type=Path)
    parser.add_argument('--package', type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(verify(args.archive, args.index, args.package), indent=2))
    except (OSError, ValueError, KeyError, StopIteration, tarfile.TarError) as error:
        raise SystemExit('Research integrity verification failed: ' + str(error))
