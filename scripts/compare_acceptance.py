#!/usr/bin/env python3
"""Compare matched acceptance matrices; incomplete cells remain explicitly unmeasured."""
import argparse,json,math
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('baseline',type=Path);p.add_argument('candidate',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
def load(root):
 c=json.loads((root/'configuration.json').read_text());r=[]
 for f in root.glob('*/report.json'):
  row=json.loads(f.read_text());events=[json.loads(line) for line in (f.parent/'turn.jsonl').read_text().splitlines() if line.strip()]
  endings=[e.get('data',{}) for e in events if e.get('type')=='turn_end']
  row['natural_turn_ended']=row['cli_exit']==0 and bool(endings) and endings[-1].get('stopReason')=='end_turn'
  r.append(row)
 return c,r
bc,br=load(a.baseline);cc,cr=load(a.candidate)
keys=['sha256','provider','endpoint','model_id','runtime_sha256','engine_sha256','contexts','repeats','timeout_seconds','cases','fixture_sha256','threads','verification_plan','history_notes','thinking']
assert all(bc.get(k)==cc.get(k) for k in keys), 'Comparison requires identical models, fixtures, runtime and budgets'
def score(rows):
 n=len(rows);k=sum(r['passed'] for r in rows);z=1.96
 if n:
  center=(k/n+z*z/(2*n))/(1+z*z/n);half=z*math.sqrt(k/n*(1-k/n)/n+z*z/(4*n*n))/(1+z*z/n)
  interval=[max(0,center-half),min(1,center+half)]
 else:interval=None
 return {'source_passes':k,'attempts':n,'wilson_95_percent_interval':interval,'naturally_ended_turns':sum(r['natural_turn_ended'] for r in rows),'claim_accuracy':'unassessed'}
expected=len(cc['cases'])*len(cc['contexts'])*cc['repeats'];report={'baseline':score(br),'candidate':score(cr),'expected_attempts_each':expected,'complete':len(br)==len(cr)==expected,'baseline_config':bc,'candidate_config':cc,'preset_promotion':'Not automatic. Requires complete development and held-out matrices plus manually assessed claims and matching physical hardware.'}
a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
