#!/usr/bin/env python3
"""Complete development and held-out matrices with bounded parallel workers.

Every child retains raw evidence and a frozen executable. Nonzero model scores
are results, not harness errors. Missing cells can never count as a complete run.
"""
import argparse, concurrent.futures, hashlib, json, os, shutil, subprocess, sys, time
from pathlib import Path
from acceptance_projects import DEVELOPMENT, HELD_OUT

p = argparse.ArgumentParser(description=__doc__)
for name in ['binary','engine','runtime','model','output']:
    p.add_argument('--'+name, type=Path, required=True)
p.add_argument('--sha256', required=True)
p.add_argument('--uncensored', action='store_true', required=True)
p.add_argument('--workers', type=int, choices=[1,2], default=2)
p.add_argument('--threads', type=int, default=2)
p.add_argument('--context', type=int, default=8192)
p.add_argument('--timeout', type=int, default=180)
p.add_argument('--repeats', type=int, default=5)
p.add_argument('--tool-profile', choices=['all','coding','compact','compact-lines'], default='coding')
p.add_argument('--thinking', choices=['default','on','off'], default='default')
p.add_argument('--resume', action='store_true')
a = p.parse_args()
assert a.threads > 0 and a.timeout > 0 and a.repeats > 0
root = a.output.resolve(); root.mkdir(parents=True, exist_ok=a.resume)
binary = root/'alt-under-test'
if binary.exists():
    assert a.resume and hashlib.sha256(binary.read_bytes()).digest()==hashlib.sha256(a.binary.read_bytes()).digest(), 'Resume executable changed'
else:
    shutil.copy2(a.binary.resolve(),binary)
identity = {'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
            'model_sha256':a.sha256, 'workers':a.workers, 'threads_per_worker':a.threads,
            'context':a.context, 'timeout_seconds':a.timeout, 'repeats':a.repeats,
            'tool_profile':a.tool_profile,'thinking':a.thinking,
            'development':DEVELOPMENT, 'held_out':HELD_OUT}
if (root/'campaign.json').exists():
    assert json.loads((root/'campaign.json').read_text()) == identity, 'Resume requires identical campaign'
else:
    (root/'campaign.json').write_text(json.dumps(identity, indent=2)+'\n')

def worker(partition, cases, index):
    destination = root/f'{partition}-{index+1}'
    command = [sys.executable, str(Path(__file__).with_name('live_acceptance.py'))]
    for name in ['binary','engine','runtime','model']:
        command += ['--'+name, str(binary if name=='binary' else getattr(a,name).resolve())]
    command += ['--sha256',a.sha256,'--uncensored','--contexts',str(a.context),
                '--repeats',str(a.repeats),'--threads',str(a.threads),'--timeout',str(a.timeout),
                '--tool-profile',a.tool_profile,'--thinking',a.thinking,'--verification-plan',
                '--partition',partition,'--cases',','.join(cases),'--output',str(destination)]
    if a.resume and destination.exists(): command.append('--resume')
    with (root/f'{partition}-{index+1}.log').open('a' if a.resume else 'w') as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
    reports = [json.loads(path.read_text()) for path in sorted(destination.glob('*/report.json'))]
    return {'partition':partition,'worker':index+1,'exit':result.returncode,
            'expected':len(cases)*a.repeats,'recorded':len(reports),
            'passed':sum(r['passed'] for r in reports), 'directory':destination.name}

rows = []
for partition, cases in [('development',DEVELOPMENT),('held-out',HELD_OUT)]:
    with concurrent.futures.ThreadPoolExecutor(max_workers=a.workers) as pool:
        futures = [pool.submit(worker,partition,cases[i::a.workers],i) for i in range(a.workers)]
        for future in concurrent.futures.as_completed(futures):
            row = future.result(); rows.append(row); print(json.dumps(row), flush=True)
            report = {'configuration':identity,'workers':rows,
                      'recorded':sum(r['recorded'] for r in rows),
                      'expected':(len(DEVELOPMENT)+len(HELD_OUT))*a.repeats,
                      'passed':sum(r['passed'] for r in rows),
                      'complete':len(rows)==2*a.workers and all(r['recorded']==r['expected'] for r in rows),
                      'preset_promoted':False,
                      'scope':'Concurrent CPU workers share the host. No physical GPU or novice acceptance. Completion is distinct from model success.'}
            temporary=root/'summary.tmp'; temporary.write_text(json.dumps(report,indent=2)+'\n'); temporary.replace(root/'summary.json')
if not report['complete']: raise SystemExit(2)
raise SystemExit(0 if report['passed']==report['expected'] else 1)
