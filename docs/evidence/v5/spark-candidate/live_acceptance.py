#!/usr/bin/env python3
"""Opt-in repeatable project evaluations; requires an exact uncensored artifact.
Every attempt, timeout, independent oracle and source diff is retained.
"""
import argparse,hashlib,json,os,signal,sqlite3,subprocess,time,urllib.request,shutil,sys
from pathlib import Path
from acceptance_projects import CASES,setup,check,ORACLE_VERSION,DEVELOPMENT,HELD_OUT
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--stop-after',type=int,help='Retain a bounded pilot and leave remaining matrix cells explicitly unmeasured');p.add_argument('--resume',action='store_true');p.add_argument('--tool-profile',choices=['all','inspect','coding','terminal'],default='all');p.add_argument('--partition',choices=['development','held-out','all'],default='development');p.add_argument('--threads',type=int,default=2)
p.add_argument('--verification-plan',action='store_true');p.add_argument('--history-notes',type=int,default=0)
p.add_argument('--binary',type=Path,default=Path('target/debug/alt'));p.add_argument('--engine',type=Path,required=True);p.add_argument('--model',type=Path,required=True);p.add_argument('--sha256',required=True);p.add_argument('--uncensored',action='store_true',required=True);p.add_argument('--runtime',type=Path);p.add_argument('--provider',choices=['openai','ollama'],default='openai');p.add_argument('--endpoint');p.add_argument('--model-id');p.add_argument('--contexts',default='8192');p.add_argument('--repeats',type=int,default=5);p.add_argument('--cases');p.add_argument('--timeout',type=int,default=240);p.add_argument('--output',type=Path,required=True)
a=p.parse_args();a.cases=a.cases or ','.join(DEVELOPMENT if a.partition=='development' else HELD_OUT if a.partition=='held-out' else CASES);assert a.runtime or (a.endpoint and a.model_id),'Choose managed runtime or explicit endpoint/model';assert a.repeats>=1
binary=a.binary.resolve();artifact=a.model.resolve();h=hashlib.sha256()
with artifact.open('rb') as f:
 for b in iter(lambda:f.read(8*1024*1024),b''):h.update(b)
assert h.hexdigest()==a.sha256,'Wrong model artifact'
root=a.output.resolve();root.mkdir(parents=True,exist_ok=a.resume)
frozen=root/'alt-under-test'
if frozen.exists():assert a.resume and hashlib.sha256(binary.read_bytes()).hexdigest()==hashlib.sha256(frozen.read_bytes()).hexdigest(),'Resume needs identical binary'
else:shutil.copy2(binary,frozen)
for source in ['live_acceptance.py','acceptance_projects.py','acceptance_extra.py']:
 target=root/source
 if not target.exists():shutil.copy2(Path(__file__).parent/source,target)
binary=frozen
config={'oracle_version':ORACLE_VERSION,'context_scope':'Managed runtime native window and engine budget' if a.runtime else 'Engine budget; external native window is configured on server/tag and inspected separately','model_file':artifact.name,'sha256':a.sha256,'bytes':artifact.stat().st_size,'uncensored':'Explicit user-selected publisher-labelled uncensored/abliterated artifact; no fallback','provider':a.provider,'endpoint':a.endpoint,'model_id':a.model_id,'runtime':str(a.runtime) if a.runtime else 'external','contexts':[int(x)for x in a.contexts.split(',')],'repeats':a.repeats,'timeout_seconds':a.timeout,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'physical_gpu_tested':False,'verification_plan':a.verification_plan,'history_notes':a.history_notes}
config.update({'cases':a.cases.split(','),'partition':a.partition,'tool_profile':a.tool_profile,'threads':a.threads,'engine_sha256':hashlib.sha256(a.engine.read_bytes()).hexdigest(),'runtime_sha256':hashlib.sha256(a.runtime.read_bytes()).hexdigest() if a.runtime else None,'fixture_sha256':hashlib.sha256(json.dumps({name:CASES[name] for name in a.cases.split(',')},sort_keys=True).encode()).hexdigest()})
if (root/'configuration.json').exists():assert json.loads((root/'configuration.json').read_text())==config,'Resume configuration or fixtures changed'
else:(root/'configuration.json').write_text(json.dumps(config,indent=2)+'\n')
if a.endpoint and a.provider=='ollama':
 req=urllib.request.Request(a.endpoint.rstrip('/')+'/api/show',data=json.dumps({'model':a.model_id}).encode(),headers={'Content-Type':'application/json'});(root/'runtime-model.json').write_text(json.dumps(json.load(urllib.request.urlopen(req)),indent=2))
results=[]
def memory_tree(pid):
 parents={};rss={}
 for d in Path('/proc').iterdir():
  if not d.name.isdigit():continue
  try:
   fields={l.split(':')[0]:l.split(':',1)[1].strip() for l in (d/'status').read_text().splitlines() if ':'in l};parents[int(d.name)]=int(fields['PPid']);rss[int(d.name)]=int(fields.get('VmRSS','0 kB').split()[0])*1024
  except (OSError,ValueError,KeyError):pass
 seen={pid};frontier={pid}
 while frontier:frontier={child for child,parent in parents.items() if parent in frontier}-seen;seen.update(frontier)
 return sum(rss.get(p,0)for p in seen)
for context in config['contexts']:
 for name in a.cases.split(','):
  for repeat in range(a.repeats):
   if a.stop_after and len(results)>=a.stop_after:
    (root/'PILOT_SCOPE.json').write_text(json.dumps({'measured_attempts':len(results),'expected_full_matrix':len(config['cases'])*len(config['contexts'])*config['repeats'],'preset_promoted':False,'remaining_cells':'unmeasured; resume without --stop-after to continue'},indent=2)+'\n');sys.exit(0 if all(r['passed']for r in results)else 1)
   label=f'{context}-{name}-{repeat+1}';run_dir=root/label
   if (run_dir/'report.json').exists():
    assert a.resume
    manifest=json.loads((run_dir/'evidence-sha256.json').read_text())
    assert all((run_dir/f).is_file() and hashlib.sha256((run_dir/f).read_bytes()).hexdigest()==digest for f,digest in manifest.items()),'Attempt evidence changed'
    results.append(json.loads((run_dir/'report.json').read_text()));continue
   if run_dir.exists():
    interrupted=root/'interrupted';interrupted.mkdir(exist_ok=True);run_dir.rename(interrupted/(label+'-'+str(time.time_ns())))
   run_dir.mkdir();project,oracle=setup(name,run_dir);state=run_dir/'state';state.mkdir()
   (state/'preferences.toml').write_text(f'project={json.dumps(str(project))}\ncontext_tokens={context}\nmax_turns=12\naccess_policy="trusted"\n'+(f'runtime_path={json.dumps(str(a.runtime.resolve()))}\n'if a.runtime else '')+(f'tool_profile={json.dumps(a.tool_profile)}\n[runtime]\nthreads={a.threads}\n' if subprocess.check_output([str(binary),'--version'],text=True).strip()!='alt 0.4.1' else ''))
   (state/'runtime.toml').write_text(f'threads={a.threads}\n')
   base=[str(binary),'--data-dir',str(state),'--engine',str(a.engine.resolve()),'--access','trusted']
   arguments=['models','import',str(artifact),'--uncensored','--use']if a.runtime else ['init','--model',a.model_id,'--provider',a.provider,'--endpoint',a.endpoint,'--context',str(context),'--max-turns','12','--uncensored']
   init=subprocess.run(base+arguments,cwd=project,capture_output=True,text=True,timeout=180);(run_dir/'setup.stdout').write_text(init.stdout);(run_dir/'setup.stderr').write_text(init.stderr);assert init.returncode==0,init.stderr
   before=check(project,oracle);assert not before['passed'],(name,'broken seed unexpectedly passed')
   if a.verification_plan:
    check_command=['env','ALT_PROJECT_URL='+project.as_uri(),'CARGO_TARGET_DIR='+str(run_dir/'oracle-target'),'PYTHONDONTWRITEBYTECODE=1',*oracle]
    subprocess.run(base+['task','configure-check','acceptance','--',*check_command],cwd=project,check=True,capture_output=True)
    subprocess.run(base+['task','require','requested-behavior','--check','acceptance','--description',CASES[name]['goal']],cwd=project,check=True,capture_output=True)
   if a.history_notes:
    subprocess.run(base+['task','remember','Cedar requirement: preserve the original public API and use the bundled helper when fixing dependency imports.'],cwd=project,check=True,capture_output=True)
    for n in range(a.history_notes):subprocess.run(base+['task','remember',f'Unrelated layout decision {n}: use spacing for panel {n}.'],cwd=project,check=True,capture_output=True)
    subprocess.run(base+['task','pin','Preserve the original public API.'],cwd=project,check=True,capture_output=True)
   oracle_hash={str(f.relative_to(run_dir/'independent')):hashlib.sha256(f.read_bytes()).hexdigest()for f in (run_dir/'independent').rglob('*')if f.is_file()}
   prompt=CASES[name]['goal']+' Inspect relevant files, save a brief plan, use native tools to implement the change, and test it. Report what you actually ran and any remaining uncertainty. Preserve existing public interfaces. Keep your explanation brief.'
   (run_dir/'fixture.json').write_text(json.dumps({'oracle_version':ORACLE_VERSION,'goal':CASES[name]['goal'],'prompt':prompt,'original_files':CASES[name]['files'],'oracle_command':oracle,'oracle_sha256_before':oracle_hash},indent=2)+'\n')
   started=time.monotonic();peak=0;timed_out=False
   with (run_dir/'turn.jsonl').open('w')as out,(run_dir/'turn.stderr').open('w')as err:
    proc=subprocess.Popen(base+['run',prompt,'--allow-tools','--json','--timeout',str(a.timeout)],cwd=project,stdout=out,stderr=err,start_new_session=True)
    try:
     while proc.poll()is None:
      peak=max(peak,memory_tree(proc.pid))
      if time.monotonic()-started>a.timeout+40:timed_out=True;proc.send_signal(signal.SIGINT);break
      time.sleep(.2)
     try:proc.wait(timeout=10)
     except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait();timed_out=True
    finally:
     if proc.poll()is None:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
   after=check(project,oracle);intact=all((run_dir/'independent'/f).is_file()and hashlib.sha256((run_dir/'independent'/f).read_bytes()).hexdigest()==h for f,h in oracle_hash.items())
   events=[]
   for line in (run_dir/'turn.jsonl').read_text().splitlines():
    try:events.append(json.loads(line))
    except json.JSONDecodeError:pass
   updates=[e.get('data',{}).get('update',{}) for e in events if e.get('type')=='update'];calls=[u for u in updates if u.get('sessionUpdate')=='tool_call'];failed=[u for u in updates if u.get('status')=='failed'];prose=''.join(u.get('content',{}).get('text','')for u in updates if u.get('sessionUpdate')=='agent_message_chunk')
   source={str(f.relative_to(project)):f.read_text(errors='replace')for f in project.rglob('*')if f.is_file() and not any(part in ['target','.git','__pycache__']for part in f.relative_to(project).parts) and f.stat().st_size<1024*1024}
   result={'case':name,'context':context,'repeat':repeat+1,'passed':after['passed']and intact,'cli_exit':proc.returncode,'wall_seconds':round(time.monotonic()-started,2),'outer_timeout':timed_out,'sampled_alt_tree_peak_rss_bytes':peak,'rss_scope':'Alt and descendant processes; external model server excluded'if a.endpoint else 'Alt, Goose and managed inference runtime','vram':None,'oracle_unchanged':intact,'before':before,'after':after,'tool_calls':calls,'failed_tool_updates':len(failed),'final_prose':prose,'prose_accuracy':'Retained for manual assessment; not inferred from process exit','resulting_source':source}
   context_views=[]
   for database in state.glob('projects/*/project.db'):
    with sqlite3.connect(database) as db:
     context_views.extend(json.loads(row[0]) for row in db.execute('SELECT payload FROM context_views'))
   result['context_views']=context_views
   if a.endpoint and a.provider=='ollama':
    try:
     with urllib.request.urlopen(a.endpoint.rstrip('/')+'/api/ps',timeout=10) as response:running=json.load(response)
     result['server_runtime_state']=[{key:model.get(key) for key in ['name','model','context_length','size','size_vram','digest']}for model in running.get('models',[]) if model.get('name','').removesuffix(':latest')==a.model_id.removesuffix(':latest')]
    except Exception as error:result['server_runtime_state_error']=str(error)
   if a.verification_plan:
    verification=subprocess.run(base+['task','verify','--run'],cwd=project,capture_output=True,text=True,timeout=90)
    result['independent_alt_verification']={'exit':verification.returncode,'stdout':verification.stdout,'stderr':verification.stderr}
   result['scores']={'source_behavior_passed':result['passed'],'turn_completed':proc.returncode==0 and any(e.get('type')=='turn_end' for e in events),'claim_accuracy':'unassessed: use retained prose and evidence; no automatic success-claim inference'}
   (run_dir/'report.json').write_text(json.dumps(result,indent=2)+'\n')
   retained=['report.json','fixture.json','turn.jsonl','turn.stderr','setup.stdout','setup.stderr']
   retained += [str(f.relative_to(run_dir)) for f in (run_dir/'independent').rglob('*') if f.is_file()]
   (run_dir/'evidence-sha256.json').write_text(json.dumps({f:hashlib.sha256((run_dir/f).read_bytes()).hexdigest() for f in retained},sort_keys=True,indent=2)+'\n');results.append(result)
   summary={'configuration':config,'attempts':[{k:v for k,v in r.items()if k not in ['tool_calls','before','after','resulting_source','final_prose','context_views','independent_alt_verification']}for r in results],'passed':sum(r['passed']for r in results),'total':len(results),'scope':'Seeded small projects; every attempt retained. These measurements do not establish arbitrary task reliability or GPU performance.'};(root/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(label,'PASS'if result['passed']else'FAIL',result['wall_seconds'],'seconds',flush=True)
print(json.dumps({'passed':sum(r['passed']for r in results),'total':len(results)},indent=2))
raise SystemExit(0 if all(r['passed']for r in results)else 1)
