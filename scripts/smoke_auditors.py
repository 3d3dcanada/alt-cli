#!/usr/bin/env python3
"""Use real dependency auditors with disposable manifests; no dependency code executes."""
import argparse,json,os,subprocess,tempfile
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--alt',type=Path,default=Path('target/debug/alt'));p.add_argument('--output',type=Path,required=True);p.add_argument('--skill',choices=['dependency-review']);a=p.parse_args();binary=a.alt.resolve();records=[]
with tempfile.TemporaryDirectory(prefix='alt-dependency-audit-') as t:
 root=Path(t);project=root/'project';project.mkdir();state=root/'state'
 def run(pack):
  action=['skills','run',a.skill,pack] if a.skill else ['packs','run',pack]
  r=subprocess.run([str(binary),'--data-dir',str(state),'--access','trusted',*action,'--input','{}'],cwd=project,capture_output=True,text=True,timeout=330)
  assert r.stdout.strip(),r.stderr
  result=json.loads(r.stdout);records.append(result);return result
 try:
  (project/'requirements.txt').write_text('requests==2.19.1\n')
  python=run('pip-audit');assert python['status']=='findings' and python['findings'],python
  for version in ['1.2.0','1.2.8']:
   (project/'package.json').write_text(json.dumps({'name':'alt-audit-fixture','version':'1.0.0','private':True,'dependencies':{'minimist':version}}))
   subprocess.run(['npm','install','--package-lock-only','--ignore-scripts','--audit=false'],cwd=project,check=True,capture_output=True,timeout=120)
   result=run('npm-audit')
   assert result['status']==('findings' if version=='1.2.0' else 'passed'),result
  print('PASS: actual pip-audit detects vulnerable pinned dependencies; npm audit detects vulnerable minimist and accepts corrected lockfile')
 finally:
  a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps({'scope':'Real registry/advisory responses; manifests only, no dependency code executed','skill':a.skill,'records':records},indent=2)+'\n')
