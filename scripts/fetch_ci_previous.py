#!/usr/bin/env python3
"""Retrieve the pinned first public CI package for the first release upgrade gate."""
import argparse, hashlib, json, subprocess, tempfile, zipfile
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__); p.add_argument('--output',type=Path,required=True); a=p.parse_args()
repository='3d3dcanada/alt-cli'; run=37508006300; artifact=11434265059
expected_commit='07842ec52d5bfa470faecc7e0732f551ca0e8778'
expected_zip='f4139a9686d0585d8826753e28890cffae16083facb40978c01c334bd8e072fc'
result=subprocess.run(['gh','api',f'repos/{repository}/actions/runs/{run}'],capture_output=True,text=True)
assert result.returncode==0, 'Cannot read the pinned previous CI run'
metadata=json.loads(result.stdout)
assert metadata['head_sha']==expected_commit and metadata['conclusion']=='success', 'Unexpected previous run identity'
with tempfile.TemporaryDirectory(prefix='alt-previous-ci-') as d:
    archive=Path(d)/'previous.zip'
    with archive.open('wb') as output:
        result=subprocess.run(['gh','api',f'repos/{repository}/actions/artifacts/{artifact}/zip'],stdout=output,stderr=subprocess.PIPE)
    assert result.returncode==0, 'Previous CI artifact download unavailable; do not skip the upgrade gate'
    assert hashlib.sha256(archive.read_bytes()).hexdigest()==expected_zip, 'Previous CI artifact digest differs'
    with zipfile.ZipFile(archive) as z:
        body=z.read('alt-0.5.0-linux-x86_64.tar.gz')
        expected=z.read('alt-0.5.0-linux-x86_64.tar.gz.sha256').decode().split()[0]
        assert hashlib.sha256(body).hexdigest()==expected
        assert not a.output.exists(), 'Previous package destination already exists'
        a.output.parent.mkdir(parents=True,exist_ok=True); a.output.write_bytes(body)
        Path(str(a.output)+'.sha256').write_text(f'{expected}  {a.output.name}\n')
print(json.dumps({'run':run,'commit':expected_commit,'artifact_zip_sha256':expected_zip,'archive_sha256':expected}))
