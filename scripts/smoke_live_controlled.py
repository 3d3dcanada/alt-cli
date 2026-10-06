#!/usr/bin/env python3
"""Opt-in 0.3 controlled-tools evaluation. Only an explicit uncensored artifact.
Keeps raw outcomes even if the model fails; independent assertions decide pass.
"""
import argparse,hashlib,json,os,sqlite3,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary',type=Path,default=Path('target/debug/alt'))
p.add_argument('--engine',type=Path,required=True)
p.add_argument('--runtime',type=Path,required=True)
p.add_argument('--model',type=Path,required=True)
p.add_argument('--sha256',required=True)
p.add_argument('--uncensored',action='store_true',required=True)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--direct-edit-instructions',action='store_true',help='Test tool execution with explicit one-line edit arguments; not independent repair reasoning')
a=p.parse_args();root=a.output.resolve();root.mkdir(parents=True,exist_ok=False)
project=root/'project';project.mkdir();state=root/'state';state.mkdir()
(state/'preferences.toml').write_text(f'project = {json.dumps(str(project))}\nruntime_path = {json.dumps(str(a.runtime.resolve()))}\ncontext_tokens = 8192\nmax_turns = 16\naccess_policy = "trusted"\n')
base=[str(a.binary.resolve()),'--data-dir',str(state),'--engine',str(a.engine.resolve()),'--access','trusted']
def run(name,arguments,timeout=780):
 start=time.monotonic();r=subprocess.run(base+arguments,cwd=project,capture_output=True,text=True,timeout=timeout)
 (root/(name+'.stdout')).write_text(r.stdout);(root/(name+'.stderr')).write_text(r.stderr)
 return r,round(time.monotonic()-start,2)
original="def total(items):\n    return sum(items) + 1\n\nif __name__ == '__main__':\n    assert total([2, 3, 5]) == 10\n    assert total([]) == 0\n    print('ALT_CONTROLLED_CHECK_OK')\n"
(project/'calculator.py').write_text(original)
r,_=run('import',['models','import',str(a.model.resolve()),'--uncensored','--use'],240)
assert r.returncode==0,r.stderr
artifact=json.loads(r.stdout);assert artifact['sha256']==a.sha256
r,_=run('configure',['task','configure-check','Calculator checks','--','python3','calculator.py']);assert r.returncode==0,r.stderr
prompt='Repair calculator.py without changing assertions or the success message. Use list and read, save a short plan with remember(kind=plan), run_check with name "Calculator checks" to reproduce the failure, invoke edit as a native tool call to fix only total (Alt remembers the read hash; omit optional expected_sha256), and run_check again. Do not use terminal for this evaluation. Use native tools, not JSON examples in chat. Keep explanations brief; report actual evidence.'
if a.direct_edit_instructions:
 prompt='Use native tools in this order: read calculator.py; remember a brief plan; run_check name "Calculator checks"; edit with path="calculator.py", operation="replace", old_text="return sum(items) + 1", new_text="return sum(items)", reason="Remove the off-by-one bug"; run_check name "Calculator checks" again. Do not include newline characters in the edit arguments. Do not use terminal. Preserve all assertions. Report only the final observed check result.'
r,elapsed=run('repair',['run',prompt,'--allow-tools','--json','--timeout','600'])
report={'direct_edit_instructions':a.direct_edit_instructions,'model':artifact,'context_tokens':8192,'mode':'managed CPU; explicit trusted check execution in disposable source copy','gtx_1070_tested':False,'pass_scope':'Native edit/check, independent assertions, direct rerun, and read-only resume completion; prose accuracy requires separate review.','repair_seconds':elapsed,'cli_exit':r.returncode,'assertions_preserved':False,'independent_check_passed':False,'tracked_edit':False,'failed_then_passed_check':False}
try:
 events=[json.loads(line) for line in r.stdout.splitlines()];session=events[0]['session']['id'];report['session']=session
 changed=(project/'calculator.py').read_text();report['assertions_preserved']=all(line in changed for line in original.splitlines() if 'assert ' in line or 'print(' in line)
 independent=subprocess.run(['python3','calculator.py'],cwd=project,capture_output=True,text=True)
 (root/'independent.stdout').write_text(independent.stdout);(root/'independent.stderr').write_text(independent.stderr)
 report['independent_check_passed']=independent.returncode==0 and 'ALT_CONTROLLED_CHECK_OK' in independent.stdout
 database=next((state/'projects').glob('*/project.db'))
 with sqlite3.connect(database) as db:
  edits=[json.loads(row[0]) for row in db.execute('SELECT payload FROM changes')];checks=[json.loads(row[0]) for row in db.execute('SELECT payload FROM checks')]
 report['tracked_edit']=any(c['status']=='applied' for c in edits)
 report['failed_then_passed_check']=len(checks)>=2 and checks[0]['exit_code']!=0 and checks[-1]['exit_code']==0
 report['changes']=edits;report['checks']=checks
 # Direct application action proves a fresh check without needing another inference decision.
 direct,direct_time=run('direct-rerun',['task','check','Calculator checks'])
 report['direct_rerun_exit']=direct.returncode;report['direct_rerun_seconds']=direct_time
 resumed,resume_time=run('resume',['run','State the saved goal and the latest actual check evidence. Do not edit files or rerun checks. Identify any remaining uncertainty.','--resume',session,'--json','--timeout','300'])
 report['resume_exit']=resumed.returncode;report['resume_seconds']=resume_time
 report['resume_preserved_files']=(project/'calculator.py').read_text()==changed
except Exception as e:report['evaluation_error']=repr(e)
report['passed']=all(report.get(k) for k in ['assertions_preserved','independent_check_passed','tracked_edit','failed_then_passed_check','resume_preserved_files']) and report.get('direct_rerun_exit')==0 and report.get('resume_exit')==0
(root/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k not in ['changes','checks','model']},indent=2))
raise SystemExit(0 if report['passed'] else 1)
