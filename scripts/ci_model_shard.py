#!/usr/bin/env python3
"""Run one disjoint shard of a frozen uncensored model evaluation."""
import argparse, hashlib, json, os, platform, subprocess, sys
from pathlib import Path
from acceptance_projects import DEVELOPMENT, HELD_OUT
from evaluation_models import MODELS

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',type=Path,required=True); p.add_argument('--root',type=Path,required=True)
p.add_argument('--shard',type=int,required=True); p.add_argument('--output',type=Path,required=True)
p.add_argument('--model',choices=MODELS,default='spark')
a=p.parse_args(); assert 0<=a.shard<8
cases=(DEVELOPMENT+HELD_OUT)[a.shard::8]; root=a.root.resolve()
engine=next((root/'bootstrap').rglob('goose')); runtime=next((root/'bootstrap').rglob('llama-server'))
selected=MODELS[a.model]; model=root/selected['filename']; campaign=root/'campaign'
scripts=Path(__file__).resolve().parent
command=[sys.executable,str(scripts/'live_acceptance.py'),'--binary',str(a.binary.resolve()),'--engine',str(engine),'--runtime',str(runtime),'--model',str(model),'--sha256',selected['sha256'],'--uncensored','--contexts','8192','--repeats','5','--timeout',str(selected['timeout']),'--threads','2','--tool-profile','coding','--thinking',selected['thinking'],'--verification-plan','--partition','all','--cases',','.join(cases),'--output',str(campaign)]
a.output.mkdir(parents=True,exist_ok=True)
with (a.output/'campaign.log').open('w') as log:
    result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
build=json.loads(subprocess.check_output([str(a.binary.resolve()),'build-info']))
(a.output/'BUILD.json').write_text(json.dumps(build,indent=2)+'\n')
(a.output/'host.json').write_text(json.dumps({'platform':platform.platform(),'cpu_count':os.cpu_count(),'github_run_id':os.environ.get('GITHUB_RUN_ID'),'github_sha':os.environ.get('GITHUB_SHA'),'shard':a.shard,'assigned_cases':cases,'harness_exit':result.returncode,'harness_inputs':{name:hashlib.sha256((scripts/name).read_bytes()).hexdigest() for name in ['live_acceptance.py','acceptance_projects.py','acceptance_extra.py','collect_live_evidence.py']}},indent=2)+'\n')
subprocess.run([sys.executable,str(scripts/'collect_live_evidence.py'),str(campaign),str(a.output)],check=True)
reports=list(a.output.glob('*/report.json'))
assert len(reports)==len(cases)*5, f'Incomplete shard: recorded {len(reports)} of {len(cases)*5}'
print(json.dumps({'recorded':len(reports),'passed':sum(json.loads(p.read_text())['passed'] for p in reports),'scope':'Measurement completed; this is not a model acceptance pass.'}))
