#!/usr/bin/env python3
"""Validate an exact local package, old/current Linux and optional actual prior-version recovery."""
import argparse,hashlib,json,os,subprocess,tarfile,tempfile,time
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('archive',type=Path);p.add_argument('--previous',type=Path);p.add_argument('--verify-key',type=Path);p.add_argument('--require-upgrade',action='store_true');p.add_argument('--output',type=Path,required=True);a=p.parse_args()
assert not a.require_upgrade or a.previous,'Release promotion requires an actual previous artifact'
repo=Path(__file__).resolve().parents[1];rows=[];archive=a.archive.resolve()
def run(name,cmd,**kwargs):
 start=time.monotonic()
 try:
  r=subprocess.run(cmd,capture_output=True,text=True,timeout=180,**kwargs);row={'name':name,'passed':r.returncode==0,'exit':r.returncode,'seconds':time.monotonic()-start,'stdout':r.stdout,'stderr':r.stderr}
 except subprocess.TimeoutExpired as error:
  row={'name':name,'passed':False,'exit':None,'seconds':time.monotonic()-start,'stdout':str(error.stdout or ''),'stderr':str(error.stderr or ''),'error':'Timed out after 180 seconds'}
 rows.append(row);return row
command=['python3',str(repo/'scripts/smoke_package.py'),str(archive),'--tui']
if a.previous:command+=['--previous',str(a.previous.resolve())]
if a.verify_key:command+=['--verify-key',str(a.verify_key.resolve())]
run('installed-integrity-upgrade-rollback',command)
run('official-cyclonedx-schema',['python3',str(repo/'scripts/validate_sbom.py'),str(archive)])
with tempfile.TemporaryDirectory(prefix='alt-release-',dir='/workspace' if Path('/workspace').is_dir() else None) as d:
 root=Path(d)
 with tarfile.open(archive) as tar:tar.extractall(root,filter='data')
 package=next(root.iterdir());manifest=json.loads((package/'CONTENTS.json').read_text());assert all(hashlib.sha256((package/name).read_bytes()).hexdigest()==sha for name,sha in manifest.items())
 old=None
 if a.previous:
  old_root=root/'previous';old_root.mkdir()
  with tarfile.open(a.previous) as tar:tar.extractall(old_root,filter='data')
  old=next(old_root.iterdir())
 build=json.loads((package/'BUILD.json').read_text());assert json.loads(subprocess.check_output([str(package/'alt'),'build-info']))==build
 assert (package/'SBOM.cdx.json').is_file()
 # Both tests mount and execute exactly the packaged bytes; they do not rebuild Alt.
 for image in ['rust:1.85.1-bullseye@sha256:033a8da97a1e4d53d51fcef8f8eefaba69ff2208fbee583c4ea84f704b307db2','ubuntu:24.04']:
  row=run('container-'+image,['docker','run','--rm','--network','none','-v',str(package)+':/package:ro',image,'/package/alt','--data-dir','/tmp/alt-state','hardware'])
  identity=subprocess.run(['docker','image','inspect',image,'--format','{{.Id}}'],capture_output=True,text=True)
  row['image_id']=identity.stdout.strip() if identity.returncode==0 else None
 host=run('prepare-current-install-host',['python3',str(repo/'scripts/build-release-host.py')])
 for image in ['rust:1.85.1-bullseye@sha256:033a8da97a1e4d53d51fcef8f8eefaba69ff2208fbee583c4ea84f704b307db2','alt-release-current']:
  if image=='alt-release-current' and not host['passed']:continue
  command=['docker','run','--rm','--network','none','-v',str(package)+':/package:ro','-v',str(repo)+':/repo:ro','-v','/workspace/.alt-tools/python:/python:ro','-e','PYTHONPATH=/python']
  if old:command+=['-v',str(old)+':/previous:ro']
  command += [image,'python3','/repo/scripts/smoke_installed_host.py','/package']
  if old:command+=['--previous','/previous']
  row=run('installed-container-'+image,command)
  identity=subprocess.run(['docker','image','inspect',image,'--format','{{.Id}}'],capture_output=True,text=True);row['image_id']=identity.stdout.strip() if identity.returncode==0 else None
report={'schema':1,'archive':archive.name,'archive_sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),'previous_sha256':hashlib.sha256(a.previous.read_bytes()).hexdigest() if a.previous else None,'upgrade_gate':'measured' if a.previous else 'pending: no prior artifact supplied','checks':rows,'passed':all(r['passed'] for r in rows),'published':False,'signed':Path(str(archive)+'.sig').exists(),'signature_verified':bool(a.verify_key) and rows[0]['passed']}
a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':report['passed'],'evidence':str(a.output)}));raise SystemExit(0 if report['passed'] else 1)
