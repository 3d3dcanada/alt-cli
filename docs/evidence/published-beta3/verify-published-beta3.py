#!/usr/bin/env python3
"""Independently verify the published beta and install in disposable cloud state."""
import datetime, hashlib, json, os
from pathlib import Path
import subprocess, tarfile

TAG='v0.6.0-beta.3'
SHA='fa0c8e4130ec4d13c59193436679c4e059c22dee'
SOURCE='54bb6215b1e87579f3534c347f38019cf569e91386237352e314ba5d025976f1'
ROOT=Path('/workspace/.alt-pc-ready/published-beta3')
ROOT.mkdir(exist_ok=False)
GH='/workspace/.alt-tools/gh'

def invoke(name,args,env=None):
    with (ROOT/(name+'.stdout')).open('w') as stdout,(ROOT/(name+'.stderr')).open('w') as stderr:
        result=subprocess.run(args,stdout=stdout,stderr=stderr,env=env,timeout=240)
    if result.returncode: raise RuntimeError(f'{name} failed with exit {result.returncode}; retained stdout/stderr')

invoke('download',[GH,'release','download',TAG,'--repo','3d3dcanada/alt-cli','--dir',str(ROOT)])
archive=ROOT/'alt-0.6.0-linux-x86_64.tar.gz'
archive_sha=hashlib.sha256(archive.read_bytes()).hexdigest()
assert archive_sha==Path(str(archive)+'.sha256').read_text().split()[0]
invoke('signature-verification',[GH,'attestation','verify',str(archive),'--repo','3d3dcanada/alt-cli','--bundle',str(archive)+'.sigstore.jsonl','--custom-trusted-root','/workspace/.alt-pc-ready/tuf-mirror/trusted-root.jsonl','--signer-workflow','3d3dcanada/alt-cli/.github/workflows/publish.yml','--source-ref','refs/tags/'+TAG,'--source-digest',SHA,'--deny-self-hosted-runners','--format','json'],env=dict(os.environ,XDG_CACHE_HOME='/workspace/.alt-tools/cache'))
gate=json.loads((ROOT/'release-gate.json').read_text())
assert gate['passed'] and gate['archive_sha256']==archive_sha and gate['upgrade_gate']=='measured'
assert len(gate['checks'])==7 and all(row['passed'] for row in gate['checks'])
audit=json.loads((ROOT/'dependency-audit.json').read_text())
assert audit['vulnerabilities']['count']==0 and not any(audit.get('warnings',{}).values())
tag_gate=json.loads((ROOT/'tag-verification.json').read_text())
assert tag_gate['passed'] and tag_gate['tag']==TAG and tag_gate['source_sha']==SHA and tag_gate['conclusion']=='success'
unpack=ROOT/'unpack'
with tarfile.open(archive) as artifact:
    artifact.extractall(unpack,filter='data')
package=unpack/'alt-0.6.0-linux-x86_64'
manifest=json.loads((package/'CONTENTS.json').read_text())
actual={str(p.relative_to(package)) for p in package.rglob('*') if p.is_file()}
assert actual==set(manifest)|{'CONTENTS.json'}
for name,digest in manifest.items():
    assert hashlib.sha256((package/name).read_bytes()).hexdigest()==digest,name
build=json.loads((package/'BUILD.json').read_text())
assert build['release_tag']==TAG and build['commit']==SHA and build['dirty'] is False and build['source_sha256']==SOURCE
prefix=ROOT/'installed'
state=ROOT/'disposable-state'
invoke('install',['bash',str(package/'install.sh')],env=dict(os.environ,ALT_PREFIX=str(prefix),ALT_DATA_DIR=str(state)))
binary=prefix/'bin/alt'
assert binary.read_bytes()==(package/'alt').read_bytes()
invoke('installed-build',[str(binary),'--data-dir',str(state),'build-info'])
installed=json.loads((ROOT/'installed-build.stdout').read_text())
assert installed==build
invoke('pc-recorder',['python3',str(package/'test-my-pc.py'),'--alt',str(binary),'--data-dir',str(state),'--output',str(ROOT/'pc-recorder')])
pc=json.loads((ROOT/'pc-recorder/summary.json').read_text())
assert pc['passed'] and pc['live_requested'] is False
report={'schema':1,'verified_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'passed':True,'tag':TAG,'commit':SHA,'archive_sha256':archive_sha,'source_sha256':SOURCE,'sigstore_bundle_verified':True,'signer_workflow':'3d3dcanada/alt-cli/.github/workflows/publish.yml','source_ref':'refs/tags/'+TAG,'self_hosted_runners_denied':True,'public_trust_root_sha256':hashlib.sha256(Path('/workspace/.alt-pc-ready/tuf-mirror/trusted-root.jsonl').read_bytes()).hexdigest(),'trust_root_source':'Official Sigstore TUF signature chain via the documented official GitHub metadata mirror; TLS and signature checks retained','manifest_files_verified':len(manifest),'package_gates_passed':len(gate['checks']),'actual_upgrade_prior_sha256':gate['previous_sha256'],'dependency_advisories':0,'tag_verification_run_id':tag_gate['run_id'],'installed_build_identical':True,'installed_pc_inventory_recorder_passed':True,'live_model_trials':'Separate retained 7B/9B development outcomes in repository; no new model trial in this consumer verification','scope':'Independent download/signature/checksum/contents/install/PC-inventory verification on cloud Linux. Physical GTX 1070 fit/speed and beginner usability remain unmeasured.'}
(ROOT/'consumer-verification.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
