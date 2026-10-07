#!/usr/bin/env python3
"""Offline verified behavior rewards. Invalid gold is excluded, never rewarded.

This scorer is a prerequisite for distillation/RL, not a training run or a PRM.
Formatting, fluent claims, agreement and shorter wrong answers cannot earn reward.
"""
import argparse,json
from pathlib import Path
from prepare import strict_loads,verified_file,require,DataError

def observation(root,reference):
    path=verified_file(root,reference['path'],reference['sha256']);row=strict_loads(path.read_text())
    require(row.get('schema_version')==1 and row.get('kind')=='independent_behavior','Unsupported oracle evidence')
    verified_file(root,row['raw_output_path'],row['raw_output_sha256'])
    return row

def valid_pass(row):
    return (row.get('passed') is True and type(row.get('exit_code')) is int and row['exit_code']==0
            and type(row.get('tests_run')) is int and row['tests_run']>0
            and not row.get('timed_out',False) and not row.get('cancelled',False)
            and not row.get('error') and row.get('complete',True) is True)

def score(record,root):
    result={'schema_version':1,'id':record.get('id'),'eligible':False,'reward':None,'reason':None}
    try:
        require(record.get('schema_version')==1,'Unsupported reward record')
        require(record.get('rights_reviewed') is True,'Training rights unresolved')
        gold=observation(root,record['gold'])
        require(valid_pass(gold),'Invalid gold: independent reference does not pass')
        require(isinstance(gold.get('oracle_inputs'),dict) and bool(gold['oracle_inputs']),'Gold has no pinned oracle identity')
        candidate=observation(root,record['candidate'])
        source=record['source'];verified_file(root,source['path'],source['sha256'])
        require(candidate.get('source_sha256')==source['sha256'],'Candidate check is stale for source')
        require(gold['oracle_inputs']==candidate.get('oracle_inputs')==record['oracle_before']==record['oracle_after'],'Tests/oracle changed')
        require(source['sha256']!=record['baseline_sha256'],'No-op source cannot earn a repair reward')
        for name,digest in record['oracle_after'].items():
            verified_file(root,record['oracle_directory']+'/'+name,digest)
        result['eligible']=True
        if not valid_pass(candidate):result.update(reward=0.0,reason='Independent behavior failed, timed out, cancelled or ran no tests');return result
        require(record.get('completion_claim_supported') is True,'Unsupported completion claim is not a positive trajectory')
        cost=record.get('cost',{});used=cost.get('generated_tokens');budget=cost.get('generated_token_budget')
        bonus=0.0
        if type(used) is int and type(budget) is int and 0<=used<=budget and budget>0:bonus=0.1*(1-used/budget)
        result.update(reward=1.0+bonus,reason='Current source passed the unchanged independent behavioral oracle',behavior_reward=1.0,efficiency_bonus=bonus)
    except (DataError,KeyError,ValueError,OSError) as e:result.update(reason=str(e),eligible=False,reward=None)
    return result

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('records',type=Path);p.add_argument('--evidence-root',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    rows=[score(strict_loads(line),a.evidence_root) for line in a.records.read_text().splitlines() if line.strip()]
    with a.output.open('x') as f:
        for row in rows:f.write(json.dumps(row,allow_nan=False)+'\n')
    print(json.dumps({'records':len(rows),'eligible':sum(r['eligible'] for r in rows),'excluded':sum(not r['eligible'] for r in rows),'training_run_performed':False},indent=2))

if __name__=='__main__':main()
