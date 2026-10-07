#!/usr/bin/env python3
"""Capture a successful v5 live attempt as a pending-review native trajectory.

No automatic approval. Checks source, oracle and actual request/response receipts.
Use prepare.py only after correctness/rights/privacy/split review of this draft.
"""
import argparse, hashlib, json, shutil
from pathlib import Path
from prepare import canonical, require, strict_loads, sha256_file, validate_messages, REVIEW_FLAGS

def response(raw, streaming):
    if not streaming:
        value=strict_loads(raw);require('error' not in value,'Provider error');require(isinstance(value.get('choices'),list) and len(value['choices'])==1 and value['choices'][0].get('finish_reason') in ['stop','tool_calls'],'Incomplete native decision');return value['choices'][0]['message']
    content=[];reasoning=[];calls={};done=False;finished=False
    for line in raw.splitlines():
        if not line.startswith('data:'):continue
        data=line[5:].strip()
        if data=='[DONE]':done=True;continue
        require(not done,'Data after stream completion');value=strict_loads(data);require('error' not in value,'Stream error')
        for choice in value.get('choices',[]):
            require(choice.get('index',0)==0,'Multiple response choices need separate review')
            finished=finished or choice.get('finish_reason') is not None
            require(choice.get('finish_reason') in [None,'stop','tool_calls'],'Native decision ended before completion')
            delta=choice.get('delta',{})
            if isinstance(delta.get('content'),str):content.append(delta['content'])
            if isinstance(delta.get('reasoning_content',delta.get('reasoning')),str):reasoning.append(delta.get('reasoning_content',delta.get('reasoning')))
            for part in delta.get('tool_calls',[]):
                index=part.get('index');require(type(index) is int and 0<=index<16,'Invalid tool fragment index')
                call=calls.setdefault(index,{'id':'','type':'function','function':{'name':'','arguments':''}})
                if part.get('id'):
                    require(not call['id'] or call['id']==part['id'],'Tool id changed');call['id']=part['id']
                for field in ['name','arguments']:
                    if isinstance(part.get('function',{}).get(field),str):call['function'][field]+=part['function'][field]
    require(done and finished,'Incomplete stream')
    message={'role':'assistant','content':''.join(content)}
    if reasoning:message['reasoning_content']=''.join(reasoning)
    if calls:message['tool_calls']=[calls[n] for n in sorted(calls)]
    return message

def normalize(messages):
    out=[]
    for message in messages:
        message=dict(message)
        content=message.get('content')
        if isinstance(content,list):
            require(all(item.get('type')=='text' for item in content),'Multimodal trajectory requires separate review')
            message['content']=''.join(item['text'] for item in content)
        if message.get('tool_calls'):
            message['tool_calls']=[dict(call,function=dict(call['function'])) for call in message['tool_calls']]
            for call in message['tool_calls']:
                if isinstance(call['function']['arguments'],str):call['function']['arguments']=strict_loads(call['function']['arguments'])
        out.append(message)
    return out

def capture(attempt,actor,source,family,split,output):
    attempt=attempt.resolve();report=strict_loads((attempt/'report.json').read_text());fixture=strict_loads((attempt/'fixture.json').read_text())
    require(fixture['oracle_version']==5,'Historical passes require new independent requalification')
    config=strict_loads((attempt.parent/'configuration.json').read_text())
    require(actor.get('kind')=='model' and actor.get('uncensored') is True and actor.get('artifact_sha256')==config['sha256'],'Actor does not match the exact evaluated uncensored artifact')
    require(config['partition']=='development' and not report['case'].startswith('sealed-') and family==report.get('task_group',report['case']),'Only the actual development task family can be captured for training')
    require(report['passed'] is True and report['oracle_unchanged'] is True and report['after'].get('completion_observed') is True,'No independent passing behavior')
    manifest=strict_loads((attempt/'evidence-sha256.json').read_text())
    require(all(sha256_file(attempt/name)==digest for name,digest in manifest.items()),'Attempt evidence changed')
    for name,body in report['resulting_source'].items():
        path=Path(name);require(not path.is_absolute() and '..' not in path.parts,'Source path escape')
        require((attempt/'project'/path).read_text()==body,'Source changed after check')
    for name,digest in fixture['oracle_sha256_before'].items():require(sha256_file(attempt/'independent'/name)==digest,'Oracle changed')
    requests=sorted((attempt/'state').glob('inference/*/*-request.json'),key=lambda p:p.stat().st_mtime_ns)
    require(requests,'No actual provider requests were captured')
    chain=[]
    for request in requests:
        identity=request.name.removesuffix('-request.json');receipt=strict_loads(request.with_name(identity+'-receipt.json').read_text());raw=request.with_name(identity+'-response.raw')
        require(receipt.get('complete') is True and receipt.get('status')==200 and not receipt.get('response_truncated') and not receipt.get('provider_budget_violation'),'Interrupted, incomplete or over-budget provider exchange')
        require(sha256_file(request)==receipt['request_sha256'] and sha256_file(raw)==receipt['response_sha256'],'Provider evidence changed')
        chain.append({'request':str(request.relative_to(attempt)),'request_sha256':sha256_file(request),'response':str(raw.relative_to(attempt)),'response_sha256':sha256_file(raw),'receipt':receipt})
    request=strict_loads(requests[-1].read_text());raw=requests[-1].with_name(requests[-1].name.replace('-request.json','-response.raw'))
    messages=normalize(request['messages']+[response(raw.read_text(),request.get('stream',False))]);tools=request.get('tools',[]);validate_messages(messages,tools)
    # All earlier native calls must remain in the normalized trajectory; compaction cannot hide them.
    retained_ids={call['id'] for m in messages for call in m.get('tool_calls',[])}
    for previous in requests[:-1]:
        value=strict_loads(previous.read_text());oldraw=previous.with_name(previous.name.replace('-request.json','-response.raw'))
        old=response(oldraw.read_text(),value.get('stream',False))
        require(all(c['id'] in retained_ids for c in old.get('tool_calls',[])),'Context compaction omitted earlier native calls; review/export segments separately')
    output.mkdir(parents=True,exist_ok=False);evidence=output/'evidence';evidence.mkdir()
    shutil.copy2(attempt/'turn.jsonl',evidence/'turn.jsonl')
    (evidence/'provider-chain.json').write_text(canonical(chain)+'\n')
    shutil.copytree(attempt/'state'/'inference',evidence/'inference')
    (evidence/'transcript.json').write_text(canonical({'messages':messages,'tools':tools})+'\n')
    (evidence/'snapshot.json').write_text(canonical(report['resulting_source'])+'\n');snapshot_sha=sha256_file(evidence/'snapshot.json')
    (evidence/'oracle-output.txt').write_text(report['after'].get('stdout','')+'\n'+report['after'].get('stderr',''))
    shutil.copytree(attempt/'independent',evidence/'independent')
    shutil.copy2(attempt/'report.json',evidence/'attempt-report.json');shutil.copy2(attempt/'fixture.json',evidence/'fixture.json')
    check={'schema_version':1,'name':'v5-independent-behavior','kind':'independent_behavior','passed':True,'exit_code':report['after']['exit'],'tests_run':1,'test_count_scope':'One completed behavioral oracle containing its pinned assertions','source_sha256':snapshot_sha,'source_revision_in_harness':report.get('source_sha256'),'raw_output_path':'evidence/oracle-output.txt','raw_output_sha256':sha256_file(evidence/'oracle-output.txt'),'oracle_inputs':fixture['oracle_sha256_before']}
    (evidence/'check.json').write_text(canonical(check)+'\n')
    provenance={**source,'raw_trace_path':'evidence/turn.jsonl','raw_trace_sha256':sha256_file(evidence/'turn.jsonl'),'transcript_path':'evidence/transcript.json','transcript_sha256':sha256_file(evidence/'transcript.json'),'snapshot_path':'evidence/snapshot.json','snapshot_sha256':snapshot_sha}
    record={'schema_version':1,'id':'alt-'+snapshot_sha[:16]+'-'+sha256_file(evidence/'transcript.json')[:16],'task_group':family,'split':split,'response_style':'mixed' if any(m.get('reasoning_content') for m in messages) else 'concise','synthetic':True,'actor':actor,'messages':messages,'tools':tools,'source':provenance,'checks':[{'name':check['name'],'path':'evidence/check.json','sha256':sha256_file(evidence/'check.json')}],'review':{'status':'pending','reviewer':'',**{flag:False for flag in REVIEW_FLAGS}}}
    (output/'trajectory.pending.jsonl').write_text(canonical(record)+'\n');return record

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['attempt','actor','source','output']:p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--family',required=True);p.add_argument('--split',choices=['train','validation'],required=True)
    a=p.parse_args();row=capture(a.attempt,strict_loads(a.actor.read_text()),strict_loads(a.source.read_text()),a.family,a.split,a.output)
    print(json.dumps({'id':row['id'],'review':'pending','output':str(a.output),'training_eligible':False},indent=2))

if __name__=='__main__':main()
