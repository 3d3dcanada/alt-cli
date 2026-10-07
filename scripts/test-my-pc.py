#!/usr/bin/env python3
"""Collect reproducible PC qualification results without changing model presets.

Run after selecting your own uncensored model in Alt. Without --live this does
not load weights or contact a model server. Reports stay on your computer.
"""
import argparse, datetime, hashlib, json, os, platform, shutil, subprocess, sys, tempfile, time
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--alt',default='alt',help='Installed Alt command or executable path')
p.add_argument('--data-dir',type=Path,help='Your existing Alt state folder')
p.add_argument('--profile',help='Explicit existing model profile')
p.add_argument('--engine',type=Path,help='Existing Goose executable, if not configured')
p.add_argument('--live',action='store_true',help='Run the selected uncensored model; no fallback or download')
p.add_argument('--contexts',default='2048,4096,8192')
p.add_argument('--output',type=Path,help='New report folder; never overwrites a previous run')
a=p.parse_args()
binary=Path(shutil.which(a.alt) or a.alt).resolve(); assert binary.is_file(), 'Install Alt first or supply --alt /path/to/alt'
contexts=[int(n) for n in a.contexts.split(',')]; assert contexts and all(512<=n<=1048576 for n in contexts)
root=(a.output or Path.cwd()/('alt-pc-results-'+datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ'))).resolve()
root.mkdir(mode=0o700,parents=True,exist_ok=False)
base=[str(binary)]
for name in ['data_dir','profile','engine']:
    value=getattr(a,name)
    if value: base+=['--'+name.replace('_','-'),str(value)]
report={'schema':1,'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'os':platform.platform(),'machine':platform.machine(),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'live_requested':a.live,'checks':[], 'scope':'Local application/hardware evidence; no information is uploaded. Review raw files before sharing.'}
def save(): (root/'summary.json').write_text(json.dumps(report,indent=2)+'\n')
def run(name,args,timeout=60):
    start=time.monotonic(); entry={'name':name,'passed':False}
    try:
        with (root/(name+'.stdout')).open('w') as out,(root/(name+'.stderr')).open('w') as err:
            child=subprocess.Popen(base+args,stdout=out,stderr=err,start_new_session=True)
            try: code=child.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                import signal
                child.send_signal(signal.SIGINT)
                try: child.wait(timeout=10)
                except subprocess.TimeoutExpired: os.killpg(child.pid,signal.SIGKILL); child.wait()
                raise
        entry.update(exit=code,passed=code==0)
    except Exception as error: entry['error']=str(error)
    entry['seconds']=round(time.monotonic()-start,3); report['checks'].append(entry); save()
    print(f'{name}: {"PASS" if entry["passed"] else "NEEDS ATTENTION"}',flush=True)
    return entry['passed']
run('build',['build-info'])
run('hardware',['hardware'])
if a.live:
    run('connection',['doctor'],120)
    run('model-qualification',['qualify','--contexts',','.join(map(str,contexts)),'--repeats','3'],max(900,len(contexts)*900))
    run('native-tool-use',['evaluate'],900)
else:
    report['live_status']='Unperformed. Select an uncensored model in Alt, then repeat with --live.'
report['passed']=all(c['passed'] for c in report['checks']); save()
print(f'Reports saved to {root}\nNext: open Alt → Home → Try a practice project; follow docs/PC_TESTING.md for the interactive checks.')
raise SystemExit(0 if report['passed'] else 1)
