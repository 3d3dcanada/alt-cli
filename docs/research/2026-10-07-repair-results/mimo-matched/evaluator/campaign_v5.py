#!/usr/bin/env python3
"""Serial, blocked matched campaigns. Resumes validate every executable and cell.

--arms is JSON: [{"name":"baseline","output_tokens":2048,"thinking":"off"},...].
Every arm shares the same exact model, total allowance and task/time limits.
Development screening and sealed confirmation require separate output folders.
"""
import argparse
import hashlib
import json
import random
import shutil
import subprocess
import sys
from pathlib import Path
if sys.flags.optimize:raise RuntimeError('Run without Python optimization; evidence assertions must remain enabled')
from acceptance_v5 import CASES, DEVELOPMENT, HELD_OUT, ORACLE_VERSION

def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda:f.read(1024*1024),b''):h.update(chunk)
    return h.hexdigest()

def freeze_evaluators(root,inputs,resume,source=None):
    source=source or Path(__file__).parent
    frozen=root/'evaluator';frozen.mkdir(exist_ok=True)
    for name,digest in inputs.items():
        assert Path(name).name==name,'Invalid evaluator file name'
        target=frozen/name
        if not target.exists():
            assert not resume,'Frozen evaluator missing; retain evidence and start a separate campaign'
            assert not (source/name).is_symlink() and sha(source/name)==digest,'Evaluator changed before freezing'
            shutil.copy2(source/name,target)
        assert not target.is_symlink() and sha(target)==digest,'Frozen evaluator changed'
    return frozen

def metrics(rows, expected):
    groups={}
    for r in rows:groups.setdefault(r['case'],[]).append(r)
    summaries=[]
    for name,rs in sorted(groups.items()):
        summaries.append({'task_group':name,'attempts':len(rs),'passed':sum(bool(r['passed']) for r in rs),'timeouts':sum(bool(r['outer_timeout']) or r['cli_exit']==124 for r in rs),'cancellations':sum(r.get('scores',{}).get('stop_reason')=='cancelled' for r in rs),'wall_seconds':sum(r['wall_seconds'] for r in rs),'requests':sum(r.get('inference_cost',{}).get('requests',0) for r in rs),'charged_generated_tokens':sum(r.get('inference_cost',{}).get('charged_generated_tokens',0) for r in rs)})
    return {'recorded':len(rows),'expected':expected,'complete':len(rows)==expected,'passed':sum(bool(r['passed']) for r in rows),'groups':summaries,'timeout_scope':'Outer evaluator timeouts or explicit exit 124; observed cancelled engine turns are counted separately, without inferring their cause','preset_promoted':False}

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for n in ['binary','engine','runtime','model','arms','output']:p.add_argument('--'+n,type=Path,required=True)
    p.add_argument('--sha256',required=True);p.add_argument('--uncensored',action='store_true',required=True)
    p.add_argument('--partition',choices=['development','held-out'],default='development')
    p.add_argument('--cases');p.add_argument('--repeats',type=int,default=3);p.add_argument('--context',type=int,default=8192)
    p.add_argument('--generated-tokens',type=int,default=8192);p.add_argument('--requests',type=int,default=12)
    p.add_argument('--timeout',type=int,default=240);p.add_argument('--threads',type=int,default=2);p.add_argument('--batch',type=int,default=128)
    p.add_argument('--seed',type=int,default=42);p.add_argument('--stop-after',type=int);p.add_argument('--resume',action='store_true')
    a=p.parse_args();assert a.repeats>0 and a.timeout>0 and a.threads>0 and 16<=a.batch<=8192
    cases=a.cases.split(',') if a.cases else (DEVELOPMENT if a.partition=='development' else HELD_OUT)
    assert len(set(cases))==len(cases) and all(n in CASES and CASES[n]['partition']==a.partition for n in cases)
    arms=json.loads(a.arms.read_text());assert arms and len({r['name'] for r in arms})==len(arms)
    for arm in arms:
        assert set(arm)<= {'name','output_tokens','reasoning_tokens','thinking','temperature','top_p','tool_profile','workflow','skill'}
        assert arm['name'].isascii() and arm['name'].replace('-','').replace('_','').isalnum()
        assert arm.get('thinking','default') in ('default','on','off')
    assert len({r.get('output_tokens',2048) for r in arms})==1,'Matched arms need equal total output allowance'
    assert sha(a.model)==a.sha256,'Exact artifact mismatch'
    root=a.output.resolve();root.mkdir(parents=True,exist_ok=a.resume)
    files={n:getattr(a,n).resolve() for n in ['binary','engine','runtime','model']}
    identity={'schema':1,'oracle_version':ORACLE_VERSION,'artifacts':{n:sha(f) for n,f in files.items()},'arms':arms,'cases':cases,'partition':a.partition,'repeats':a.repeats,'context':a.context,'timeout':a.timeout,'threads':a.threads,'batch':a.batch,'seed':a.seed,'generated_tokens':a.generated_tokens,'requests':a.requests,'uncensored':True,'oracle_code':{n:sha(Path(__file__).with_name(n)) for n in ['acceptance_v5.py','acceptance_projects.py','acceptance_extra.py','live_acceptance.py','campaign_v5.py']}}
    schedule=[];rng=random.Random(a.seed)
    for rep in range(a.repeats):
        tasks=cases.copy();rng.shuffle(tasks)
        for case in tasks:
            order=list(range(len(arms)));rng.shuffle(order)
            schedule.extend({'case':case,'repeat':rep+1,'arm':arms[i]['name']} for i in order)
    identity['schedule']=schedule;manifest=root/'campaign.json'
    if manifest.exists():assert a.resume and json.loads(manifest.read_text())==identity,'Immutable campaign changed'
    else:
        manifest.write_text(json.dumps(identity,indent=2)+'\n');shutil.copy2(files['binary'],root/'alt-under-test')
    assert sha(root/'alt-under-test')==identity['artifacts']['binary']
    evaluator=freeze_evaluators(root,identity['oracle_code'],a.resume)
    executed=0;all_rows={arm['name']:[] for arm in arms}
    for cell in schedule:
        destination=root/f"{cell['arm']}-{cell['repeat']}-{cell['case']}"
        if a.stop_after and executed>=a.stop_after:break
        arm=next(r for r in arms if r['name']==cell['arm'])
        freeze_evaluators(root,identity['oracle_code'],True)
        cmd=[sys.executable,str(evaluator/'live_acceptance.py'),'--suite','v5','--binary',str(root/'alt-under-test'),'--engine',str(files['engine']),'--runtime',str(files['runtime']),'--model',str(files['model']),'--sha256',a.sha256,'--uncensored','--verification-plan','--generated-tokens',str(a.generated_tokens),'--requests',str(a.requests),'--contexts',str(a.context),'--timeout',str(a.timeout),'--threads',str(a.threads),'--batch',str(a.batch),'--partition',a.partition,'--cases',cell['case'],'--repeats','1','--output',str(destination)]
        for n in ['output_tokens','reasoning_tokens','thinking','temperature','top_p','tool_profile','workflow','skill']:
            if n in arm:cmd+=['--'+n.replace('_','-'),str(arm[n])]
        if destination.exists():cmd.append('--resume')
        with (root/(destination.name+'.log')).open('a') as log:
            result=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
        reports=list(destination.glob('*/report.json'));assert len(reports)==1, f'Incomplete cell {destination}: exit {result.returncode}; evidence retained'
        row=json.loads(reports[0].read_text());row['campaign_repeat']=cell['repeat'];all_rows[cell['arm']].append(row);executed+=1
        summary={'configuration':identity,'arms':{name:metrics(rs,len(cases)*a.repeats) for name,rs in all_rows.items()},'attempts':executed,'expected':len(schedule),'complete':executed==len(schedule),'scope':'Serial CPU campaign; independent behavior only. No GPU qualification or confidence claim from a pilot.'}
        temporary=root/'summary.tmp';temporary.write_text(json.dumps(summary,indent=2)+'\n');temporary.replace(root/'summary.json')
        print(json.dumps({'cell':cell,'passed':row['passed'],'wall_seconds':row['wall_seconds']}),flush=True)
    return 0 if executed==len(schedule) else 2

if __name__=='__main__':raise SystemExit(main())
