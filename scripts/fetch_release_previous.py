#!/usr/bin/env python3
"""Use the latest attested public package, or the pinned CI bootstrap for release one."""
import argparse, hashlib, json, subprocess, sys, tempfile
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--output',type=Path,required=True); p.add_argument('--exclude-tag',required=True); a=p.parse_args()
repo='3d3dcanada/alt-cli'
response=subprocess.run(['gh','api',f'repos/{repo}/releases'],capture_output=True,text=True)
assert response.returncode==0, 'Cannot list published releases'
releases=[r for r in json.loads(response.stdout) if not r['draft'] and r['tag_name']!=a.exclude_tag and any(v['name'].endswith('.tar.gz.sigstore.jsonl') for v in r['assets'])]
if not releases:
    subprocess.run([sys.executable,str(Path(__file__).with_name('fetch_ci_previous.py')),'--output',str(a.output)],check=True)
else:
    previous=max(releases,key=lambda r:r['published_at'])
    with tempfile.TemporaryDirectory(prefix='alt-previous-release-') as d:
        root=Path(d)
        subprocess.run(['gh','release','download',previous['tag_name'],'--repo',repo,'--pattern','alt-*-linux-x86_64.tar.gz*','--dir',str(root)],check=True)
        archives=list(root.glob('*.tar.gz')); assert len(archives)==1, 'Expected exactly one previous Linux package'
        archive=archives[0]
        subprocess.run(['gh','attestation','verify',str(archive),'--repo',repo,'--bundle',str(archive)+'.sigstore.jsonl','--signer-workflow',repo+'/.github/workflows/publish.yml','--source-ref','refs/tags/'+previous['tag_name']],check=True)
        body=archive.read_bytes(); digest=hashlib.sha256(body).hexdigest()
        assert digest==Path(str(archive)+'.sha256').read_text().split()[0]
        assert not a.output.exists(); a.output.parent.mkdir(parents=True,exist_ok=True); a.output.write_bytes(body)
        Path(str(a.output)+'.sha256').write_text(f'{digest}  {a.output.name}\n')
        print(json.dumps({'previous_tag':previous['tag_name'],'archive_sha256':digest,'attestation_verified':True}))
