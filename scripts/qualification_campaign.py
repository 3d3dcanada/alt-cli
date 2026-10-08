#!/usr/bin/env python3
"""Frozen, family-blocked baseline/candidate qualification with immutable attempts.

A pilot is not a completed matrix. Final results cannot be opened before sealing
an exact candidate binary. Failed/interrupted claims are never silently retried.
"""
import argparse
import collections
import hashlib
import json
import math
import os
from pathlib import Path
import random
import shutil
import signal
import subprocess
import sys
import time
sys.dont_write_bytecode = True

MODELS = {
    'mimo9b-heretic-q4': ('MiMo-V2.6-Distill-Qwen-9B-heretic.Q4_K_M.gguf',
                         '00877ff79174b79f72c5ec704901fc958400a0b0d390fb61a068281618116dd1'),
    'josiefied7b-abliterated-q4': ('Josiefied-Qwen2.5-7B-Instruct-abliterated.Q4_K_M.gguf',
                                  'd7d626d96cc2d3567e8266101b4211b17634012dd99a0c9b28f475ce4eb620b6'),
}
EVALUATORS = ['live_acceptance.py','live_tui_turn.py','acceptance_projects.py',
              'acceptance_extra.py','acceptance_v5.py','acceptance_final.py','evaluation_preflight.py',
              'qualification_campaign.py']
CONDITIONS = {'context':8192,'output_tokens':1024,'generated_tokens':8192,
              'requests':12,'temperature':0.2,'top_p':0.95,'thinking':'default',
              'threads':2,'batch':128,'timeout_seconds':600,'tool_profile':'compact',
              'workflow':'model-plan','gpu_layers':0,'provider':'openai'}


class InvalidCampaign(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise InvalidCampaign(message)


def canonical(value):
    return json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':'))


def sha(path):
    digest=hashlib.sha256()
    with Path(path).open('rb') as source:
        for chunk in iter(lambda:source.read(4*1024*1024),b''):
            digest.update(chunk)
    return digest.hexdigest()


def write_new(path, value):
    """Exclusive durable receipt; the caller never overwrites measured evidence."""
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('x',encoding='utf-8') as stream:
        stream.write(json.dumps(value,indent=2,ensure_ascii=False)+'\n')
        stream.flush();os.fsync(stream.fileno())
    directory=os.open(path.parent,os.O_RDONLY)
    try:os.fsync(directory)
    finally:os.close(directory)


def load(path):
    return json.loads(Path(path).read_text())


def file_manifest(root):
    return {str(p.relative_to(root)):sha(p) for p in sorted(Path(root).rglob('*')) if p.is_file()}


def task_manifest():
    from acceptance_final import CASES
    return [{'case':name,'family':case['task_group'],'partition':case['partition'],
             'category':case.get('category','historical-development'),
             'interface':case.get('interface','cli'),
             'contract_sha256':hashlib.sha256(canonical(case).encode()).hexdigest()}
            for name,case in sorted(CASES.items())]


def attempt_slots(tasks, repeats):
    """Alternate arm order within a family/model/repetition block."""
    result=[]
    for task in tasks:
        for model in sorted(MODELS):
            for repetition in range(1,repeats+1):
                arms=['baseline','candidate'] if repetition%2 else ['candidate','baseline']
                for arm in arms:
                    result.append({'id':f'{task["case"]}--{model}--r{repetition}--{arm}',
                                   'case':task['case'],'family':task['family'],
                                   'partition':task['partition'],'model':model,
                                   'repetition':repetition,'arm':arm})
    return result


def create(root, baseline, engine, runtime, model_dir, repeats=3):
    require(repeats>=3,'Final qualification requires at least three repetitions')
    tasks=task_manifest();families=[t['family'] for t in tasks]
    require(len(families)==len(set(families)),'Task family reused across partitions')
    require(sum(t['partition']=='final-holdout' for t in tasks)>=30,'Need30 final families')
    root=Path(root);root.mkdir(parents=True,exist_ok=False)
    binaries=root/'binaries';binaries.mkdir()
    shutil.copy2(baseline,binaries/'baseline')
    evaluator=root/'evaluator';evaluator.mkdir()
    for name in EVALUATORS:shutil.copy2(Path(__file__).with_name(name),evaluator/name)
    models={}
    for name,(filename,digest) in MODELS.items():
        path=Path(model_dir).resolve()/filename
        require(path.is_file() and sha(path)==digest,f'Wrong/missing exact artifact: {name}')
        models[name]={'path':str(path),'sha256':digest,'uncensored':True,
                      'selection':'Explicit publisher-labelled uncensored/abliterated Q4; never substituted'}
    identity={'schema':1,'created_unix':time.time(),'baseline_sha256':sha(binaries/'baseline'),
              'engine':{'path':str(Path(engine).resolve()),'sha256':sha(engine)},
              'runtime':{'path':str(Path(runtime).resolve()),'sha256':sha(runtime)},
              'evaluator_sha256':file_manifest(evaluator),'models':models,'conditions':CONDITIONS,
              'repeats':repeats,'tasks':tasks,'attempts':attempt_slots(tasks,repeats),
              'freeze_rule':'Candidate and presets must be sealed before either final baseline or candidate is inspected.',
              'training_rule':'Every final-holdout family is forbidden for corpus/prompt/adapter selection.',
              'preset_promoted':False}
    write_new(root/'campaign.json',identity)
    return identity


def verify(root, execution=True):
    root=Path(root);manifest=load(root/'campaign.json')
    require(manifest['schema']==1,'Unknown campaign version')
    require(sha(root/'binaries/baseline')==manifest['baseline_sha256'],'Frozen baseline changed')
    require(file_manifest(root/'evaluator')==manifest['evaluator_sha256'],'Frozen evaluator changed')
    require(sha(__file__)==manifest['evaluator_sha256']['qualification_campaign.py'],
            'Campaign controller changed; run the frozen evaluator/qualification_campaign.py')
    if execution:
        for kind in ['engine','runtime']:
            require(sha(manifest[kind]['path'])==manifest[kind]['sha256'],f'{kind} changed')
    return manifest


def seal(root, candidate, source_commit, source_dirty):
    root=Path(root);manifest=verify(root)
    require(not any((root/'attempts'/s['id']/'started.json').exists()
                    for s in manifest['attempts'] if s['partition']=='final-holdout'),
            'Final inspection already started; cannot change candidate')
    destination=root/'binaries/candidate'
    require(not destination.exists(),'Candidate already copied; inspect prior incomplete seal')
    shutil.copy2(candidate,destination)
    receipt={'schema':1,'candidate_sha256':sha(destination),'campaign_sha256':sha(root/'campaign.json'),
             'conditions_sha256':hashlib.sha256(canonical(manifest['conditions']).encode()).hexdigest(),
             'source_commit':source_commit,'source_dirty':source_dirty,'sealed_unix':time.time(),
             'preset_promoted':False,'scope':'Immutable candidate/preset freeze, not a quality result'}
    write_new(root/'candidate-seal.json',receipt)
    return receipt


def checked_seal(root, manifest):
    receipt=load(root/'candidate-seal.json')
    require(receipt['campaign_sha256']==sha(root/'campaign.json'),'Candidate belongs to another plan')
    require(receipt['candidate_sha256']==sha(root/'binaries/candidate'),'Frozen candidate changed')
    require(receipt['conditions_sha256']==hashlib.sha256(canonical(manifest['conditions']).encode()).hexdigest(),
            'Candidate effort conditions changed')
    return receipt


def run_evaluator(command, out, err, seconds):
    """Bound outer failure cleanup even when the evaluator owns new sessions."""
    process=subprocess.Popen(command,stdout=out,stderr=err,start_new_session=True,
                             env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
    try:
        code=process.wait(timeout=seconds)
        return subprocess.CompletedProcess(command,code)
    except BaseException:
        # Snapshot descendants before killing the leader so reparenting cannot hide them.
        entries={}
        for item in Path('/proc').iterdir():
            if not item.name.isdigit():continue
            try:
                parts=(item/'stat').read_text().rsplit(') ',1)[1].split()
                entries[int(item.name)]=(int(parts[1]),parts[19])
            except (OSError,ValueError,IndexError):continue
        owned={process.pid};changed=True
        while changed:
            added={pid for pid,(parent,_) in entries.items() if parent in owned}-owned
            changed=bool(added);owned.update(added)
        for sig in [signal.SIGTERM,signal.SIGKILL]:
            for pid in sorted(owned,reverse=True):
                try:
                    parts=Path('/proc',str(pid),'stat').read_text().rsplit(') ',1)[1].split()
                    if pid==process.pid or entries.get(pid,(None,None))[1]==parts[19]:os.kill(pid,sig)
                except (OSError,ValueError,IndexError):pass
            if sig==signal.SIGTERM:time.sleep(.2)
        process.wait(timeout=3)
        raise


def observed_outcome(report, slot):
    """Re-derive model outcomes from the retained independent report."""
    valid=bool(report and report.get('case')==slot['case'] and
               report.get('oracle_unchanged') is True and
               report.get('before',{}).get('passed') is False and
               report.get('before',{}).get('exit') is not None and
               type(report.get('after',{}).get('passed')) is bool and
               report.get('after',{}).get('exit') is not None)
    passed=bool(valid and report['after']['passed'] is True and
                report['after']['exit']==0 and report['after'].get('completion_observed') is True)
    valid=valid and report.get('passed') is passed
    return {'status':'measured' if valid else 'infrastructure-failed',
            'behavior_passed':bool(valid and passed),
            'native_completed':report.get('scores',{}).get('turn_completed') if report else None,
            'native_stop_reason':report.get('scores',{}).get('stop_reason') if report else None,
            'failed_native_tool_updates':report.get('failed_tool_updates') if report else None,
            'tool_calls':len(report.get('tool_calls',[])) if report else None,
            'model_wall_seconds':report.get('wall_seconds') if report else None,
            'inference_cost':report.get('inference_cost') if report else None,
            'runtime_timings':report.get('runtime_timings') if report else None,
            'sampled_peak_rss_bytes':report.get('sampled_alt_tree_peak_rss_bytes') if report else None}


def check_started(root, slot, sealed=None):
    started=load(root/'attempts'/slot['id']/'started.json')
    require(started.get('slot')==slot,'Started receipt belongs to another slot')
    require(started.get('campaign_sha256')==sha(root/'campaign.json'),'Attempt belongs to another campaign')
    require(started.get('attempt_number')==1,'Attempt number changed')
    require(isinstance(started.get('started_unix'),(int,float)),'Missing start time')
    if slot['partition']=='final-holdout' or slot['arm']=='candidate':
        require(sealed is not None and started['started_unix']>=sealed['sealed_unix'],
                'Candidate/final inspection predates candidate freeze')
    return started


def run_slots(root, partition, max_attempts, arms=('baseline','candidate'), shard_index=0, shard_count=1):
    root=Path(root);manifest=verify(root)
    require(max_attempts>0,'Choose an explicit positive shard size')
    require(0<=shard_index<shard_count<=64,'Invalid shard index/count')
    require(partition in {'development','validation','final-holdout'},'Unknown partition')
    if partition=='final-holdout' or 'candidate' in arms:
        checked_seal(root,manifest)
    # Import only the immutable evaluator source, not current checkout changes.
    import importlib.util
    saved=list(sys.path);sys.path.insert(0,str(root/'evaluator'))
    for name in ['acceptance_final','acceptance_v5','acceptance_projects','acceptance_extra']:
        sys.modules.pop(name,None)
    try:
        spec=importlib.util.spec_from_file_location('frozen_final',root/'evaluator/acceptance_final.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    finally:sys.path[:]=saved
    count=0
    for index,slot in enumerate(manifest['attempts']):
        if (index//2)%shard_count!=shard_index:continue
        if slot['partition']!=partition or slot['arm'] not in arms:continue
        attempt=root/'attempts'/slot['id']
        if (attempt/'started.json').exists():continue  # includes interrupted; never replay
        if count>=max_attempts:break
        model=manifest['models'][slot['model']]
        require(sha(model['path'])==model['sha256'],'Model bytes changed; no request issued')
        write_new(attempt/'started.json',{'schema':1,'slot':slot,'campaign_sha256':sha(root/'campaign.json'),
                                        'started_unix':time.time(), 'attempt_number':1})
        count+=1;c=manifest['conditions'];case=module.CASES[slot['case']]
        command=[sys.executable,str(root/'evaluator/live_acceptance.py'),'--suite','final-pass',
                 '--binary',str(root/'binaries'/slot['arm']),'--engine',manifest['engine']['path'],
                 '--runtime',manifest['runtime']['path'],'--model',model['path'],'--sha256',model['sha256'],
                 '--uncensored','--partition',slot['partition'],'--cases',slot['case'],'--contexts',str(c['context']),'--repeats','1',
                 '--timeout',str(c['timeout_seconds']),'--threads',str(c['threads']),'--batch',str(c['batch']),
                 '--tool-profile',c['tool_profile'],'--workflow',c['workflow'],'--thinking',c['thinking'],
                 '--output-tokens',str(c['output_tokens']),'--generated-tokens',str(c['generated_tokens']),
                 '--requests',str(c['requests']),'--temperature',str(c['temperature']),'--top-p',str(c['top_p']),
                 '--verification-plan','--interface',case.get('interface','cli'),'--output',str(attempt/'raw')]
        if case.get('followup'):command+=['--followup-prompt',case['followup']]
        write_new(attempt/'invocation.json',{'argv':command,'scope':'No secret values recorded'})
        started=time.monotonic();error=None;exit_code=None
        try:
            with (attempt/'runner.stdout').open('x') as out,(attempt/'runner.stderr').open('x') as err:
                result=run_evaluator(command,out,err,c['timeout_seconds']+360)
                exit_code=result.returncode
        except (OSError,subprocess.TimeoutExpired) as problem:
            error=type(problem).__name__
        reports=list((attempt/'raw').glob('*/report.json'))
        report=load(reports[0]) if len(reports)==1 else None
        receipt={'schema':1,'slot':slot,**observed_outcome(report,slot),
                 'runner_exit':exit_code,'runner_error':error,'wall_seconds':time.monotonic()-started,
                 'unsupported_completion_claim':None,'claim_review':'Unreviewed; not inferred from prose/exit',
                 'interventions':0,'evidence_sha256':file_manifest(attempt),
                 'finished_unix':time.time(),'preset_promoted':False}
        write_new(attempt/'result.json',receipt)
        print(json.dumps({'slot':slot['id'],'status':receipt['status'],'behavior_passed':receipt['behavior_passed']}),flush=True)
    return count


def paired_family_interval(deltas, seed=1729, samples=10000):
    require(bool(deltas),'No complete family pairs')
    mean=sum(deltas)/len(deltas)
    rng=random.Random(seed)
    draws=sorted(sum(rng.choice(deltas) for _ in deltas)/len(deltas) for _ in range(samples))
    return {'families':len(deltas),'mean_lift':mean,'bootstrap_95_percent':[draws[int(.025*samples)],draws[int(.975*samples)-1]],
            'unit':'paired task-family mean; repetitions are within-family observations',
            'seed':seed,'resamples':samples}


def summarize(root):
    root=Path(root);manifest=verify(root,execution=False);rows=[];sealed=None
    if any((root/'attempts'/s['id']/'started.json').exists()
           for s in manifest['attempts'] if s['partition']=='final-holdout' or s['arm']=='candidate'):
        sealed=checked_seal(root,manifest)
    for slot in manifest['attempts']:
        folder=root/'attempts'/slot['id'];result=folder/'result.json'
        if (folder/'started.json').exists():check_started(root,slot,sealed)
        if result.exists():
            require((folder/'started.json').exists(),'Attempt result has no original started receipt')
            row=load(result)
            require(row['slot']==slot,'Attempt substituted')
            for name,digest in row['evidence_sha256'].items():
                path=folder/name
                require(path.is_file() and sha(path)==digest,'Attempt evidence changed')
            expected=set(row['evidence_sha256'])|{'result.json'}
            require(set(file_manifest(folder))==expected,'Unmanifested attempt evidence')
            reports=list((folder/'raw').glob('*/report.json'))
            observed=observed_outcome(load(reports[0]) if len(reports)==1 else None,slot)
            require(all(row.get(key)==value for key,value in observed.items()),
                    'Attempt outcome differs from retained independent report')
            rows.append(row)
        else:rows.append({'slot':slot,'status':'interrupted' if (folder/'started.json').exists() else 'unperformed',
                          'behavior_passed':None})
    states=collections.Counter(r['status'] for r in rows);comparisons={};costs={}
    for model in manifest['models']:
        for partition in ['development','validation','final-holdout']:
            for arm in ['baseline','candidate']:
                selected=[r for r in rows if r['slot']['model']==model and r['slot']['partition']==partition and r['slot']['arm']==arm]
                observed=[r for r in selected if r['status']=='measured']
                costs[f'{model}/{partition}/{arm}']={
                    'planned':len(selected),'measured':len(observed),
                    'behavior_passed':sum(r['behavior_passed'] is True for r in observed),
                    'native_completed':sum(r.get('native_completed') is True for r in observed),
                    'completion_claims_unreviewed':sum(r.get('unsupported_completion_claim') is None for r in observed),
                    'failed_native_tool_updates':sum(r.get('failed_native_tool_updates') or 0 for r in observed),
                    'wall_seconds':sum(r.get('wall_seconds') or 0 for r in observed),
                    'requests':sum((r.get('inference_cost') or {}).get('requests',0) for r in observed),
                    'charged_generated_tokens':sum((r.get('inference_cost') or {}).get('charged_generated_tokens',0) for r in observed),
                    'output_cost_note':'Charged tokens can include conservative allowance reservations; known token counts remain in each attempt.',
                    'prompt_cost_note':'Actual server prompt timings/token fields, when supplied, remain in runtime_timings; missing values are not estimated.',
                    'peak_sampled_rss_bytes':max((r.get('sampled_peak_rss_bytes') or 0 for r in observed),default=None),
                    'interventions':sum(r.get('interventions') or 0 for r in observed),
                    'complete':len(observed)==len(selected)}
    for model in manifest['models']:
        relevant=[r for r in rows if r['slot']['model']==model and r['slot']['partition']=='final-holdout']
        groups=collections.defaultdict(list)
        for row in relevant:groups[row['slot']['family']].append(row)
        deltas=[]
        for family,attempts in groups.items():
            if len(attempts)==manifest['repeats']*2 and all(r['status']=='measured' for r in attempts):
                rates={arm:sum(r['behavior_passed'] for r in attempts if r['slot']['arm']==arm)/manifest['repeats'] for arm in ['baseline','candidate']}
                deltas.append(rates['candidate']-rates['baseline'])
        complete=len(deltas)==len(groups) and len(groups)>=30
        comparisons[model]={'complete':complete,'expected_families':len(groups),'measured_complete_families':len(deltas),
                            'uncertainty':paired_family_interval(deltas) if complete else None,
                            'partial_subset_not_promoted':True,'quality_target_lift':.15}
    return {'schema':1,'campaign_sha256':sha(root/'campaign.json'),'expected':len(rows),'status_counts':dict(states),
            'attempts':rows,'comparisons':comparisons,'resources_by_arm':costs,'preset_promoted':False,
            'software_readiness':'Separate software gates; this matrix does not certify the application',
            'scope':'All intended slots included. No selective successful-subset score; final family intervals require the complete matrix.'}


def main():
    parser=argparse.ArgumentParser(description=__doc__);sub=parser.add_subparsers(dest='action',required=True)
    p=sub.add_parser('create')
    for name in ['output','baseline','engine','runtime','model-dir']:p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--repeats',type=int,default=3)
    p=sub.add_parser('seal');p.add_argument('--root',type=Path,required=True);p.add_argument('--candidate',type=Path,required=True)
    p.add_argument('--source-commit',required=True);p.add_argument('--source-dirty',action='store_true')
    p=sub.add_parser('run');p.add_argument('--root',type=Path,required=True);p.add_argument('--partition',choices=['development','validation','final-holdout'],required=True)
    p.add_argument('--shard-index',type=int,default=0);p.add_argument('--shard-count',type=int,default=1)
    p.add_argument('--max-attempts',type=int,required=True);p.add_argument('--arms',default='baseline,candidate',choices=['baseline','candidate','baseline,candidate'])
    p=sub.add_parser('summary');p.add_argument('--root',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if args.action=='create':value=create(args.output,args.baseline,args.engine,args.runtime,args.model_dir,args.repeats)
    elif args.action=='seal':value=seal(args.root,args.candidate,args.source_commit,args.source_dirty)
    elif args.action=='run':value={'new_attempts':run_slots(args.root,args.partition,args.max_attempts,args.arms.split(','),args.shard_index,args.shard_count)}
    else:value=summarize(args.root);write_new(args.output,value)
    print(json.dumps({k:v for k,v in value.items() if k not in ['attempts','tasks']},indent=2))


if __name__=='__main__':main()
