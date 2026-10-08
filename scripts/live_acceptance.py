#!/usr/bin/env python3
"""Opt-in repeatable project evaluations; requires an exact uncensored artifact.
Every attempt, timeout, independent oracle and source diff is retained.
"""
import argparse,hashlib,json,os,signal,sqlite3,subprocess,time,urllib.request,shutil,sys
from pathlib import Path
if sys.flags.optimize:raise RuntimeError('Run without Python optimization; evidence assertions must remain enabled')
from acceptance_projects import CASES,setup,check,ORACLE_VERSION,DEVELOPMENT,HELD_OUT,oracle_inputs,assertion_script
p=argparse.ArgumentParser(description=__doc__)
p.add_argument("--suite",choices=["v4","v5","final-pass"],default="v4")
p.add_argument("--output-tokens",type=int)
p.add_argument("--reasoning-tokens",type=int)
p.add_argument("--temperature",type=float)
p.add_argument("--top-p",type=float)
p.add_argument("--workflow",choices=["model-plan","host"],default="model-plan")
p.add_argument("--skill")
p.add_argument('--thinking',choices=['default','on','off'],default='default');p.add_argument('--stop-after',type=int,help='Retain a bounded pilot and leave remaining matrix cells explicitly unmeasured');p.add_argument('--resume',action='store_true');p.add_argument('--tool-profile',choices=['all','inspect','coding','terminal','compact','compact-lines'],default='all');p.add_argument('--partition',choices=['development','validation','final-holdout','held-out','all'],default='development');p.add_argument('--threads',type=int,default=2);p.add_argument('--batch',type=int,default=128)
p.add_argument('--verification-plan',action='store_true');p.add_argument('--history-notes',type=int,default=0)
p.add_argument('--binary',type=Path,default=Path('target/debug/alt'));p.add_argument('--engine',type=Path,required=True);p.add_argument('--model',type=Path,required=True);p.add_argument('--sha256',required=True);p.add_argument('--uncensored',action='store_true',required=True);p.add_argument('--runtime',type=Path);p.add_argument('--provider',choices=['openai','ollama'],default='openai');p.add_argument('--endpoint');p.add_argument('--model-id');p.add_argument('--contexts',default='8192');p.add_argument('--repeats',type=int,default=5);p.add_argument('--cases');p.add_argument('--timeout',type=int,default=240);p.add_argument('--output',type=Path,required=True)
p.add_argument('--generated-tokens',type=int);p.add_argument('--requests',type=int)
p.add_argument('--instruction-draft',type=Path,help='Trial this bounded offline supplement for this campaign only')
p.add_argument('--interface',choices=['cli','tui'],default='cli');p.add_argument('--width',type=int,default=80);p.add_argument('--height',type=int,default=24)
p.add_argument('--followup-prompt',action='append',default=[],help='Send a follow-up in the same actual TUI conversation after a normal turn; the total deadline/allowance remains shared')
a=p.parse_args()
if a.suite=='final-pass':
 from acceptance_final import CASES,setup,check,ORACLE_VERSION,DEVELOPMENT,HELD_OUT,oracle_inputs,assertion_script
if a.suite=='v5':
 from acceptance_v5 import CASES,setup,check,ORACLE_VERSION,DEVELOPMENT,HELD_OUT,oracle_inputs,assertion_script
assert a.timeout>0 and a.threads>0 and 16<=a.batch<=8192
assert 60<=a.width<=240 and 20<=a.height<=100
assert a.interface=='cli' or a.instruction_draft is None,'TUI draft trials need a separately reviewed activation; this runner does not activate drafts'
assert not a.followup_prompt or a.interface=='tui','Follow-up qualification uses the actual persistent TUI conversation'
assert len(a.followup_prompt)<=3 and all(0<len(p.encode())<=64000 for p in a.followup_prompt)
assert a.cases is None or all(n in CASES for n in a.cases.split(',')), 'Unknown fixture'
if a.partition in ['validation','final-holdout']:
 assert a.suite=='final-pass','The selected partition belongs to final-pass'
 selected=[name for name,case in CASES.items() if case.get('partition')==a.partition]
else:selected=DEVELOPMENT if a.partition=='development' else HELD_OUT if a.partition=='held-out' else CASES
a.cases=a.cases or ','.join(selected)
if a.suite=='final-pass' and a.partition!='all':
 assert all(CASES[n]['partition']==a.partition for n in a.cases.split(',')), 'Case crosses the declared partition'
assert a.runtime or (a.endpoint and a.model_id),'Choose managed runtime or explicit endpoint/model';assert a.repeats>=1
assert a.runtime or a.thinking=='default','Configure reasoning on the external server; this switch controls only an owned runtime'
binary=a.binary.resolve();artifact=a.model.resolve();h=hashlib.sha256()
with artifact.open('rb') as f:
 for b in iter(lambda:f.read(8*1024*1024),b''):h.update(b)
assert h.hexdigest()==a.sha256,'Wrong model artifact'
root=a.output.resolve();root.mkdir(parents=True,exist_ok=a.resume)
from evaluation_preflight import inspect
preflight=inspect([CASES[name] for name in a.cases.split(',')])
(root/('environment-preflight-'+str(time.time_ns())+'.json' if a.resume else 'environment-preflight.json')).write_text(json.dumps(preflight,indent=2)+'\n')
if not preflight['passed']:raise SystemExit('Language tool preflight failed. No model requests were issued; inspect environment-preflight.json and activate the documented toolchain environment.')
frozen=root/'alt-under-test'
if frozen.exists():assert a.resume and hashlib.sha256(binary.read_bytes()).hexdigest()==hashlib.sha256(frozen.read_bytes()).hexdigest(),'Resume needs identical binary'
else:shutil.copy2(binary,frozen)
for source in ['live_acceptance.py','live_tui_turn.py','acceptance_projects.py','acceptance_extra.py','acceptance_v5.py','acceptance_final.py','evaluation_preflight.py']:
 target=root/source
 if not target.exists():shutil.copy2(Path(__file__).parent/source,target)
 else:assert hashlib.sha256(target.read_bytes()).hexdigest()==hashlib.sha256((Path(__file__).parent/source).read_bytes()).hexdigest(), "Resume evaluator source changed"
binary=frozen
config={'oracle_version':ORACLE_VERSION,'context_scope':'Managed runtime native window and engine budget' if a.runtime else 'Engine budget; external native window is configured on server/tag and inspected separately','model_file':artifact.name,'sha256':a.sha256,'bytes':artifact.stat().st_size,'uncensored':'Explicit user-selected publisher-labelled uncensored/abliterated artifact; no fallback','provider':a.provider,'endpoint':a.endpoint,'model_id':a.model_id,'runtime':str(a.runtime) if a.runtime else 'external','contexts':[int(x)for x in a.contexts.split(',')],'repeats':a.repeats,'timeout_seconds':a.timeout,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'physical_gpu_tested':False,'verification_plan':a.verification_plan,'history_notes':a.history_notes}
config.update({'cases':a.cases.split(','),'partition':a.partition,'tool_profile':a.tool_profile,'threads':a.threads,'batch':a.batch,'engine_sha256':hashlib.sha256(a.engine.read_bytes()).hexdigest(),'runtime_sha256':hashlib.sha256(a.runtime.read_bytes()).hexdigest() if a.runtime else None,'fixture_sha256':hashlib.sha256(json.dumps({name:CASES[name] for name in a.cases.split(',')},sort_keys=True).encode()).hexdigest()})
config['suite']=a.suite
config['interface']=a.interface
if a.followup_prompt:config['followup_prompts']=a.followup_prompt
if a.interface=='tui':config.update(width=a.width,height=a.height)
config['instruction_sha256']=hashlib.sha256(a.instruction_draft.read_bytes()).hexdigest() if a.instruction_draft else None
if a.instruction_draft:assert 0<len(a.instruction_draft.read_bytes())<=5000,'Use a bounded nonempty instruction draft'
config['inference']={'output_tokens':a.output_tokens,'reasoning_tokens':a.reasoning_tokens,'temperature':a.temperature,'top_p':a.top_p,'workflow':a.workflow,'skill':a.skill,'generated_tokens':a.generated_tokens,'requests':a.requests}
config['oracle_code_sha256']={name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in ['acceptance_projects.py','acceptance_extra.py','acceptance_v5.py','acceptance_final.py','live_tui_turn.py']}
if a.thinking!='default':config['thinking']=a.thinking
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
   run_dir.mkdir();project,oracle=setup(name,run_dir);oracle_hash=oracle_inputs(run_dir);state=run_dir/'state';state.mkdir()
   (state/'preferences.toml').write_text(f'project={json.dumps(str(project))}\ncontext_tokens={context}\nmax_turns=12\naccess_policy="trusted"\n'+(f'runtime_path={json.dumps(str(a.runtime.resolve()))}\n'if a.runtime else '')+(f'tool_profile={json.dumps(a.tool_profile)}\n[runtime]\nthreads={a.threads}\nbatch={a.batch}\n' if subprocess.check_output([str(binary),'--version'],text=True).strip()!='alt 0.4.1' else ''))
   (state/'runtime.toml').write_text(f'threads={a.threads}\n')
   if a.thinking!='default':
    with (state/'preferences.toml').open('a') as settings:settings.write('thinking='+('true' if a.thinking=='on' else 'false')+'\n')
   base=[str(binary),'--data-dir',str(state),'--engine',str(a.engine.resolve()),'--access','trusted']
   arguments=['models','import',str(artifact),'--uncensored','--use']if a.runtime else ['init','--model',a.model_id,'--provider',a.provider,'--endpoint',a.endpoint,'--context',str(context),'--max-turns','12','--uncensored']
   init=subprocess.run(base+arguments,cwd=project,capture_output=True,text=True,timeout=180);(run_dir/'setup.stdout').write_text(init.stdout);(run_dir/'setup.stderr').write_text(init.stderr);assert init.returncode==0,init.stderr
   allocation=[]
   for field in ['output_tokens','reasoning_tokens','temperature','top_p','generated_tokens','requests']:
    if getattr(a,field) is not None:allocation+=['--'+field.replace('_','-'),str(getattr(a,field))]
   if allocation:subprocess.run(base+['inference',*allocation],cwd=project,check=True,capture_output=True)
   subprocess.run(base+['workflow',a.workflow],cwd=project,check=True,capture_output=True)
   if a.skill:subprocess.run(base+['skills','use',a.skill],cwd=project,check=True,capture_output=True)
   before=check(project,oracle);assert not before['passed'],(name,'broken seed unexpectedly passed')
   if a.verification_plan:
    # A pinned wrapper invokes the actual oracle, emits structured evidence and
    # hashes every external oracle input before execution. It never embeds a repair.
    assertion=run_dir/'assertion.py'
    assertion.write_text(assertion_script(run_dir,oracle,oracle_hash))
    subprocess.run(base+['task','configure-check','acceptance','--','python3',str(assertion)],cwd=project,check=True,capture_output=True)
    subprocess.run(base+['task','contract','acceptance','--kind','tests','--format','json','--report','.alt-acceptance.json','--assertion',str(assertion)],cwd=project,check=True,capture_output=True)
    subprocess.run(base+['task','require','requested-behavior','--check','acceptance','--description',CASES[name]['goal']],cwd=project,check=True,capture_output=True)
   if a.history_notes:
    subprocess.run(base+['task','remember','Cedar requirement: preserve the original public API and use the bundled helper when fixing dependency imports.'],cwd=project,check=True,capture_output=True)
    for n in range(a.history_notes):subprocess.run(base+['task','remember',f'Unrelated layout decision {n}: use spacing for panel {n}.'],cwd=project,check=True,capture_output=True)
    subprocess.run(base+['task','pin','Preserve the original public API.'],cwd=project,check=True,capture_output=True)
   prompt=CASES[name]['goal']+CASES[name].get('prompt_suffix',' Inspect relevant files, save a brief plan, use native tools to implement the change, and test it. Report what you actually ran and any remaining uncertainty. Preserve existing public interfaces. Keep your explanation brief.')
   (run_dir/'fixture.json').write_text(json.dumps({'oracle_version':ORACLE_VERSION,'goal':CASES[name]['goal'],'prompt':prompt,'original_files':CASES[name]['files'],'oracle_command':oracle,'oracle_sha256_before':oracle_hash},indent=2)+'\n')
   started=time.monotonic();peak=0;timed_out=False
   tui=None
   if a.interface=='tui':
    import importlib.util
    helper=root/'live_tui_turn.py'
    assert hashlib.sha256(helper.read_bytes()).hexdigest()==config['oracle_code_sha256']['live_tui_turn.py'],'Frozen TUI evaluator changed'
    spec=importlib.util.spec_from_file_location('alt_frozen_live_tui_turn',helper);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    tui=module.run_turn(base,project,state,prompt,a.timeout,run_dir,memory_tree,width=a.width,height=a.height,allow_tools=True,followup_prompts=a.followup_prompt)
    peak=tui['sampled_alt_tree_peak_rss_bytes'];timed_out=tui['timed_out'];exit_code=tui['process_exit']
   else:
    with (run_dir/'turn.jsonl').open('w')as out,(run_dir/'turn.stderr').open('w')as err:
     proc=subprocess.Popen(base+['run',prompt,'--allow-tools','--json','--timeout',str(a.timeout)]+(['--instruction-draft',str(a.instruction_draft.resolve())] if a.instruction_draft else []),cwd=project,stdout=out,stderr=err,start_new_session=True)
     try:
      while proc.poll()is None:
       peak=max(peak,memory_tree(proc.pid))
       if time.monotonic()-started>a.timeout+40:timed_out=True;proc.send_signal(signal.SIGINT);break
       time.sleep(.2)
      try:proc.wait(timeout=10)
      except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait();timed_out=True
     finally:
      if proc.poll()is None:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
    exit_code=proc.returncode
   after=check(project,oracle);intact=all((run_dir/'independent'/f).is_file()and hashlib.sha256((run_dir/'independent'/f).read_bytes()).hexdigest()==h for f,h in oracle_hash.items())
   events=[]
   for line in (run_dir/'turn.jsonl').read_text().splitlines():
    try:events.append(json.loads(line))
    except json.JSONDecodeError:pass
   updates=[e.get('data',{}).get('update',{}) for e in events if e.get('type')=='update'];calls=[u for u in updates if u.get('sessionUpdate')=='tool_call'];failed=[u for u in updates if u.get('status')=='failed'];prose=''.join(u.get('content',{}).get('text','')for u in updates if u.get('sessionUpdate')=='agent_message_chunk')
   source={str(f.relative_to(project)):f.read_text(errors='replace')for f in project.rglob('*')if f.is_file() and not any(part in ['target','.git','__pycache__']for part in f.relative_to(project).parts) and f.stat().st_size<1024*1024}
   result={'case':name,'context':context,'repeat':repeat+1,'passed':after['passed']and intact,'cli_exit':exit_code if a.interface=='cli' else None,'process_exit':exit_code,'interface':a.interface,'wall_seconds':round(time.monotonic()-started,2),'outer_timeout':timed_out,'sampled_alt_tree_peak_rss_bytes':peak,'rss_scope':'Alt and descendant processes; external model server excluded'if a.endpoint else 'Alt, Goose and managed inference runtime','vram':None,'oracle_unchanged':intact,'before':before,'after':after,'tool_calls':calls,'failed_tool_updates':len(failed),'final_prose':prose,'prose_accuracy':'Retained for manual assessment; not inferred from process exit','resulting_source':source}
   if tui:result['tui']=tui
   context_views=[]
   for database in state.glob('projects/*/project.db'):
    with sqlite3.connect(database) as db:
     context_views.extend(json.loads(row[0]) for row in db.execute('SELECT payload FROM context_views'))
   result['context_views']=context_views
   receipts=[json.loads(f.read_text()) for f in state.glob('inference/*/*-receipt.json')]
   result['inference_cost']={'requests':len(receipts),'charged_generated_tokens':sum(r.get('charged_generated_tokens',0) for r in receipts),'known_generated_tokens':sum(r['generated_tokens'] for r in receipts) if all(r.get('generated_tokens') is not None for r in receipts) else None}
   # Actual llama.cpp timings distinguish initial prefill from cached follow-ups.
   # Missing/interrupted/external responses stay explicitly unmeasured.
   timings=[]
   for raw in sorted(state.glob('inference/*/*-response.raw'),key=lambda f:f.stat().st_mtime_ns):
    observed=None
    for line in raw.read_text(errors='replace').splitlines():
     if not line.startswith('data: {'):continue
     try:chunk=json.loads(line[6:])
     except json.JSONDecodeError:continue
     if isinstance(chunk.get('timings'),dict):observed=chunk['timings']
    if observed:timings.append({'response':str(raw.relative_to(run_dir)),'timings':observed})
   result['runtime_timings']={'scope':'Actual server-reported timings for completed responses, not inferred elapsed time','measured_responses':len(timings),'responses':timings,'prefill_ms':sum(t['timings'].get('prompt_ms',0)for t in timings),'generation_ms':sum(t['timings'].get('predicted_ms',0)for t in timings)}
   result['task_group']=CASES[name].get('task_group',name)
   result['source_sha256']=hashlib.sha256(json.dumps(source,sort_keys=True,ensure_ascii=False).encode()).hexdigest()
   result['inference_requests']=len(list(state.glob('inference/*/*-request.json')))
   if a.endpoint and a.provider=='ollama':
    try:
     with urllib.request.urlopen(a.endpoint.rstrip('/')+'/api/ps',timeout=10) as response:running=json.load(response)
     result['server_runtime_state']=[{key:model.get(key) for key in ['name','model','context_length','size','size_vram','digest']}for model in running.get('models',[]) if model.get('name','').removesuffix(':latest')==a.model_id.removesuffix(':latest')]
    except Exception as error:result['server_runtime_state_error']=str(error)
   if a.verification_plan:
    verification=subprocess.run(base+['task','verify','--run'],cwd=project,capture_output=True,text=True,timeout=90)
    result['independent_alt_verification']={'exit':verification.returncode,'stdout':verification.stdout,'stderr':verification.stderr}
   endings=[e.get('data',{}) for e in events if e.get('type')=='turn_end']
   normal=exit_code==0 and bool(endings) and endings[-1].get('stopReason')=='end_turn'
   if tui:normal=normal and not timed_out and tui['error'] is None and tui['turns_completed']==len(a.followup_prompt)+1 and tui['prompts_sent']==len(a.followup_prompt)+1
   result['scores']={'source_behavior_passed':result['passed'],'turn_completed':normal,'transport_turn_ended':exit_code==0 and bool(endings),'stop_reason':endings[-1].get('stopReason') if endings else None,'claim_accuracy':'unassessed: use retained prose and evidence; no automatic success-claim inference'}
   (run_dir/'report.json').write_text(json.dumps(result,indent=2)+'\n')
   retained=['report.json','fixture.json','turn.jsonl','turn.stderr','setup.stdout','setup.stderr']
   if (run_dir/'assertion.py').exists():retained.append('assertion.py')
   if tui:retained+=['terminal.raw','terminal.txt','terminal-after-exit.txt','tui-turn.json']
   retained += ['independent/'+name for name in oracle_hash]
   retained += [str(f.relative_to(run_dir)) for f in state.glob('inference/**/*') if f.is_file()]
   (run_dir/'evidence-sha256.json').write_text(json.dumps({f:hashlib.sha256((run_dir/f).read_bytes()).hexdigest() for f in retained},sort_keys=True,indent=2)+'\n');results.append(result)
   summary={'configuration':config,'attempts':[{k:v for k,v in r.items()if k not in ['tool_calls','before','after','resulting_source','final_prose','context_views','independent_alt_verification']}for r in results],'passed':sum(r['passed']for r in results),'total':len(results),'scope':'Seeded small projects; every attempt retained. These measurements do not establish arbitrary task reliability or GPU performance.'};(root/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(label,'PASS'if result['passed']else'FAIL',result['wall_seconds'],'seconds',flush=True)
print(json.dumps({'passed':sum(r['passed']for r in results),'total':len(results)},indent=2))
raise SystemExit(0 if all(r['passed']for r in results)else 1)
