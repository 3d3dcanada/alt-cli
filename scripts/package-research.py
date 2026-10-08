#!/usr/bin/env python3
"""Preserve every historical document in a separate deterministic, hashed archive."""
import gzip
import hashlib
import json
import os
from pathlib import Path
import tarfile


def digest(path):
    result = hashlib.sha256()
    with path.open('rb') as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b''):
            result.update(chunk)
    return result.hexdigest()


def package_research(repo, dist, version, build):
    entries = sorted((repo / 'docs').rglob('*'))
    if any(p.is_symlink() for p in entries):
        raise ValueError('Research source contains a symlink; preserve its target as an explicit ordinary evidence file first')
    paths = [p for p in entries if p.is_file()]
    files = {p.relative_to(repo).as_posix(): {'sha256': digest(p), 'bytes': p.stat().st_size} for p in paths}
    index = dict(schema=1, source_commit=build['commit'], source_sha256=build['source_sha256'],
                 scope='Every source docs file, including all baseline failures, research and historical evidence. No file is silently selected out.',
                 files=files, total_bytes=sum(row['bytes'] for row in files.values()))
    archive = dist / ('research-' + version + '.tar.gz')
    epoch = int(os.environ.get('SOURCE_DATE_EPOCH', '0'))
    def normalized(info):
        info.uid = info.gid = 0
        info.uname = info.gname = ''
        info.mtime = epoch
        info.mode = 0o755 if info.isdir() else 0o644
        return info
    # The index is external as well as in the archive for inexpensive inspection.
    index_path = dist / ('research-' + version + '-index.json')
    index_path.write_text(json.dumps(index, sort_keys=True, indent=2) + '\n')
    with archive.open('wb') as raw:
        with gzip.GzipFile(filename='', mode='wb', fileobj=raw, mtime=epoch) as compressed:
            with tarfile.open(fileobj=compressed, mode='w') as output:
                for path in paths:
                    output.add(path, arcname=path.relative_to(repo).as_posix(), filter=normalized)
                output.add(index_path, arcname='RESEARCH-INDEX.json', filter=normalized)
    if any(digest(repo / name) != row['sha256'] for name, row in files.items()):
        raise ValueError('Historical evidence changed during packaging; retry from a frozen checkout')
    sha = digest(archive)
    Path(str(archive) + '.sha256').write_text('{}  {}\n'.format(sha, archive.name))
    return dict(archive=archive.name, sha256=sha, index=index_path.name,
                index_sha256=digest(index_path), files=len(files), uncompressed_bytes=index['total_bytes'],
                source_commit=build['commit'], source_sha256=build['source_sha256'])
