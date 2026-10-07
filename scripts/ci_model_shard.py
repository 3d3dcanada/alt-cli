#!/usr/bin/env python3
"""Run one disjoint shard of a frozen uncensored model evaluation."""
import argparse, hashlib, json, os, platform, subprocess, sys
from pathlib import Path
from acceptance_projects import DEVELOPMENT, HELD_OUT

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',type=Path,required=True); p.add_argument('--root',type=Path,required=True)
p.add_argument('--shard',type=int,required=True); p.add_argument('--output',type=Path,required=True)
a=p.parse_args(); assert 0<=a.shard<8
cases=(DEVELOPMENT+HELD_OUT)[a.shard::8]; root=a.root.resolve()
engine=next((root/'bootstrap').rglob('goose')); runtime=next((root/'bootstrap').rglob('llama-server'))
model=root/'Spark-X2.5-1.7B-Abliterated-Q4_K_M.gguf'; campaign=root/'campaign'
scripts=Path(__file__).resolve().parent
command=[sys.executable,str(scripts/'live_acceptance.py'),'--binary',str(a.binary.resolve()),'--engine',str(engine),'--runtime',str(runtime),'--model',str(model),'--sha256','1e4d920aaff1248b68751e49b16dae6104c5f4d3924a86a290e24b31db08113c','--uncensored','--contexts','8192','--repeats','5','--timeout','240','--threads','2','--tool-profile','coding','--thinking','off','--verification-plan','--partition','all','--cases',','.join(cases),'--output',str(campaign)]
a.output.mkdir(parents=True,exist_ok=True)
with (a.output/'campaign.log').open('w') as log:
    result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
subprocess.run([sys.executable,str(scripts/'collect_live_evidence.py'),str(campaign),str(a.output)],check=True)
build=json.loads(subprocess.check_output([str(a.binary.resolve()),'build-info']))
(a.output/'BUILD.json').write_text(json.dumps(build,indent=2)+'\n')
(a.output/'host.json').write_text(json.dumps({'platform':platform.platform(),'cpu_count':os.cpu_count(),'github_run_id':os.environ.get('GITHUB_RUN_ID'),'github_sha':os.environ.get('GITHUB_SHA'),'shard':a.shard,'assigned_cases':cases,'harness_exit':result.returncode,'harness_inputs':{name:hashlib.sha256((scripts/name).read_bytes()).hexdigest() for name in ['live_acceptance.py','acceptance_projects.py','acceptance_extra.py']}},indent=2)+'\n')
reports=list(a.output.glob('*/report.json'))
assert len(reports)==len(cases)*5, f'Incomplete shard: recorded {len(reports)} of {len(cases)*5}'
print(json.dumps({'recorded':len(reports),'passed':sum(json.loads(p.read_text())['passed'] for p in reports),'scope':'Measurement completed; this is not a model acceptance pass.'}))
