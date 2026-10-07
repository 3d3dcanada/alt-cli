#!/usr/bin/env python3
"""Execute an optional adapter in fresh v5 source states and check actual behavior.

An adapter command receives ALT_ADAPTER_INPUT/OUTPUT JSON paths and a project cwd.
Language-model resource claims need captured, exact-model Alt receipts. Text search
scores, a dialogue tree or a PRM score never substitute for independent acceptance.
No adapters are installed, enabled or promoted by this runner.
"""
import argparse,hashlib,json,os,signal,subprocess,sys,time
from pathlib import Path
if sys.flags.optimize:raise RuntimeError('Run without Python optimization; evidence assertions must remain enabled')
from acceptance_v5 import CASES,DEVELOPMENT,setup,check,oracle_inputs

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def resources(result,roles,root):
    if not isinstance(result,dict):return {'captured_calls':0,'charged_generated_tokens':None,'errors':['Adapter output is not an object'],'complete_visibility':False}
    rows=result.get('inference',[]);errors=[];total=0
    if not isinstance(rows,list):return {'captured_calls':0,'charged_generated_tokens':None,'errors':['Inference references are not a list'],'complete_visibility':False}
    if roles and not rows:errors.append('Model roles have no captured inference receipts')
    for ref in rows:
        if not isinstance(ref,dict) or not all(isinstance(ref.get(k),str) for k in ['role','request','response','receipt','request_sha256','response_sha256','receipt_sha256']):errors.append('Malformed inference reference');continue
        role=next((r for r in roles if r['id']==ref.get('role')),None)
        if role is None:errors.append('Undeclared model role');continue
        for field in ['request','response','receipt']:
            p=(root/ref[field]).resolve()
            if not p.is_relative_to(root.resolve()) or not p.is_file() or sha(p)!=ref[field+'_sha256']:errors.append('Changed/outside-root inference evidence');break
        else:
            try:request=json.loads((root/ref['request']).read_text());receipt=json.loads((root/ref['receipt']).read_text())
            except (ValueError,OSError):errors.append('Invalid inference JSON');continue
            if request.get('model')!=role['model_id'] or receipt.get('request_sha256')!=ref['request_sha256'] or receipt.get('response_sha256')!=ref['response_sha256']:errors.append('Inference identity mismatch')
            if receipt.get('complete') is not True or receipt.get('provider_budget_violation'):errors.append('Incomplete/over-budget inference')
            charged=receipt.get('charged_generated_tokens');
            if type(charged) is not int or charged<0:errors.append('Missing actual/reserved inference cost')
            else:total+=charged
    return {'captured_calls':len(rows),'charged_generated_tokens':total,'errors':errors,'complete_visibility':result.get('all_model_calls_captured') is True and not errors,'scope':'Receipts verify recorded calls. Review pinned adapter source for hidden calls; source review remains an explicit gate.'}

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--manifest',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--cases',default='python-feature,python-multifile');p.add_argument('--timeout',type=int,default=180);p.add_argument('--allow-host',action='store_true',required=True);a=p.parse_args()
    manifest=json.loads(a.manifest.read_text());assert manifest['schema']==1 and manifest['kind'] in ['source-executor','deterministic-solver']
    command=manifest['command'];assert command and all(isinstance(s,str) for s in command)
    program=Path(command[0]).resolve();assert program.is_file() and sha(program)==manifest['executable_sha256']
    assert manifest['source_reviewed'] is True and manifest['license_reviewed'] is True
    source_files=manifest.get('source_files',{})
    assert isinstance(source_files,dict) and all(Path(name).is_absolute() and sha(Path(name))==digest for name,digest in source_files.items()),'Pin reviewed adapter files'
    for arg in command[1:]:
        if Path(arg).is_file():assert str(Path(arg).resolve()) in source_files,'Every executable script/input file in the command must be pinned'
    roles=manifest.get('roles',[]);assert len({r['id'] for r in roles})==len(roles)
    for role in roles:
        assert role['uncensored'] is True and role['repository'] and len(role['revision']) in [40,64]
        assert sha(Path(role['artifact_path']))==role['artifact_sha256'],'Exact local model identity required'
    if manifest['kind']=='deterministic-solver':assert not roles,'A solver with a model is a disclosed model adapter'
    cases=a.cases.split(',');assert all(c in DEVELOPMENT for c in cases) and len(set(cases))==len(cases) and a.timeout>0
    a.output.mkdir(parents=True,exist_ok=False);identity={'manifest':manifest,'manifest_sha256':sha(a.manifest),'cases':cases,'oracle_version':5,'timeout':a.timeout,'oracle_code_sha256':sha(Path(__file__).with_name('acceptance_v5.py'))}
    (a.output/'campaign.json').write_text(json.dumps(identity,indent=2)+'\n');rows=[]
    for case in cases:
        assert sha(program)==manifest['executable_sha256'] and all(sha(Path(name))==digest for name,digest in source_files.items()),'Adapter changed during campaign'
        root=a.output/case;root.mkdir();project,oracle=setup(case,root);inputs=oracle_inputs(root);before=check(project,oracle);assert not before['passed']
        input_path=root/'input.json';output_path=root/'adapter-output.json';input_path.write_text(json.dumps({'schema':1,'goal':CASES[case]['goal'],'workspace':str(project.resolve()),'seconds_limit':a.timeout,'permitted_roles':[r['id'] for r in roles],'response_contract':'Actual source writes, inference evidence references, all_model_calls_captured; model scores are unverified'},indent=2))
        started=time.monotonic();timed_out=False
        with (root/'stdout.log').open('w') as out,(root/'stderr.log').open('w') as err:
            proc=subprocess.Popen(command,cwd=project,env={**os.environ,'ALT_ADAPTER_INPUT':str(input_path.resolve()),'ALT_ADAPTER_OUTPUT':str(output_path.resolve())},stdout=out,stderr=err,start_new_session=True)
            try:proc.wait(timeout=a.timeout)
            except subprocess.TimeoutExpired:timed_out=True;os.killpg(proc.pid,signal.SIGKILL);proc.wait()
            finally:
                try:os.killpg(proc.pid,signal.SIGKILL)
                except ProcessLookupError:pass
        after=check(project,oracle);intact=all(sha(root/'independent'/name)==digest for name,digest in inputs.items())
        output_error=None
        try:output=json.loads(output_path.read_text()) if output_path.exists() else {}
        except (ValueError,OSError) as error:output={};output_error=type(error).__name__
        cost=resources(output,roles,root)
        if output_error:cost['errors'].append('Malformed adapter output: '+output_error);cost['complete_visibility']=False
        source={str(f.relative_to(project)):sha(f) for f in project.rglob('*') if f.is_file() and not any(part in ['target','__pycache__','.git'] for part in f.parts)}
        row={'case':case,'passed':after['passed'] and intact and not timed_out,'before':before,'after':after,'oracle_unchanged':intact,'timeout':timed_out,'exit':proc.returncode,'wall_seconds':time.monotonic()-started,'source_manifest':source,'resources':cost,'qualified':after['passed'] and intact and not timed_out and cost['complete_visibility'],'promoted':False}
        (root/'report.json').write_text(json.dumps(row,indent=2)+'\n');rows.append(row)
        (a.output/'summary.json').write_text(json.dumps({'identity':identity,'rows':rows,'complete':len(rows)==len(cases),'all_behavior_passed':all(r['passed'] for r in rows),'resource_visibility':all(r['resources']['complete_visibility'] for r in rows),'promoted':False},indent=2)+'\n')
        print(json.dumps({'case':case,'passed':row['passed'],'qualified':row['qualified']}),flush=True)
    return 0 if all(r['qualified'] for r in rows) else 1

if __name__=='__main__':raise SystemExit(main())
