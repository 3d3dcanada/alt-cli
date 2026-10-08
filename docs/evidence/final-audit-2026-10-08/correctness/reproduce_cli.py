#!/usr/bin/env python3
"""Reproduce beta3 audit findings in new disposable project/state directories."""
import argparse, fcntl, hashlib, json, os, subprocess, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--alt', required=True, type=Path)
p.add_argument('--output', required=True, type=Path, help='Must not exist')
a = p.parse_args()
alt = a.alt.resolve()
root = a.output.resolve()
root.mkdir(parents=True, exist_ok=False)
provenance = json.loads(subprocess.check_output([str(alt), 'build-info'], text=True))
(root/'build-info.json').write_text(json.dumps(provenance, indent=2))

def command(cwd, data, args, overrides=None):
    call = [str(alt), '--data-dir', str(data), '--access', 'trusted', 'task'] + args
    result = subprocess.run(call, cwd=cwd, env={**os.environ, **(overrides or {})}, text=True, capture_output=True, timeout=15)
    return {'args': args, 'exit': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}

def save(name, result):
    (root/(name+'.json')).write_text(json.dumps(result, indent=2))

cwd=root/'mutating-source'; cwd.mkdir(); data=root/'mutating-state'
(cwd/'answer.py').write_text('def answer(): return 0\n')
(cwd/'prepare.py').write_text('from pathlib import Path\nPath("answer.py").write_text("def answer(): return 42\\n")\n')
oracle=root/'independent-assertion.py'
oracle.write_text('from pathlib import Path\nimport subprocess,json,sys,os\nsubprocess.run([sys.executable,"prepare.py"],check=True)\nscope={}\nexec(Path("answer.py").read_text(),scope)\nassert scope["answer"]()==42\nPath(os.environ["ALT_CHECK_REPORT"]).write_text(json.dumps({"schema":1,"complete":True,"tests":[{"name":"answer-is-42","status":"passed"}]}))\n')
steps=[['configure-check','independent','--','python3',str(oracle)],['contract','independent','--kind','tests','--format','json','--report','report.json','--assertion',str(oracle)],['require','returns-42','--check','independent'],['check','independent'],['verify']]
rows=[command(cwd,data,x) for x in steps]
save('mutating-source', {'original_source_after':(cwd/'answer.py').read_text(),'results':rows})

cwd=root/'environment-source';cwd.mkdir();data=root/'environment-state'
(cwd/'check.py').write_text('import os\nassert os.environ.get("APP_FEATURE_MODE")=="enabled"\nprint("Ran 1 test")\n')
steps=[('enabled',['configure-check','feature','--','python3','check.py']),('enabled',['require','feature-enabled','--check','feature']),('enabled',['check','feature']),('disabled',['verify']),('disabled',['check','feature'])]
save('environment', [{**command(cwd,data,args,{'APP_FEATURE_MODE':mode}),'APP_FEATURE_MODE':mode} for mode,args in steps])

cwd=root/'locked-source';cwd.mkdir();data=root/'locked-state'
marker=root/'check-started';release=root/'check-release'
(cwd/'check.py').write_text(f'from pathlib import Path\nimport time\nPath({str(marker)!r}).write_text("started")\nwhile not Path({str(release)!r}).exists(): time.sleep(.01)\nprint("Ran 1 test")\nprint("ACTUAL_SUCCESS_EVIDENCE")\n')
setup=command(cwd,data,['configure-check','delayed','--','python3','check.py'])
assert setup['exit']==0,setup
call=[str(alt),'--data-dir',str(data),'--access','trusted','task','check','delayed']
process=subprocess.Popen(call,cwd=cwd,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
try:
    until=time.monotonic()+10
    while not marker.exists() and time.monotonic()<until: time.sleep(.01)
    assert marker.exists(),'Check did not start'
    lockpath=data/'projects'/hashlib.sha256(str(cwd).encode()).hexdigest()/'project.lock'
    with lockpath.open('r+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        release.touch()
        out,err=process.communicate(timeout=10)
    save('locked-evidence',{'check_exit':process.returncode,'stdout':out,'stderr':err,'status':command(cwd,data,['status'])})
finally:
    if process.poll() is None:
        process.kill();process.wait()

cwd=root/'placeholder-source';cwd.mkdir();data=root/'placeholder-state'
save('placeholder-setup', [command(cwd,data,['configure-check','json-test','--','python3','check.py','{report}']),command(cwd,data,['contract','json-test','--kind','tests','--format','json','--report','report.json'])])
print(root)
