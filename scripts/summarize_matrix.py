#!/usr/bin/env python3
"""Reject missing, duplicated or mismatched cells before reporting a full matrix."""
import argparse, hashlib, json, os
from pathlib import Path
from acceptance_projects import DEVELOPMENT, HELD_OUT, ORACLE_VERSION
from evaluation_models import MODELS

p=argparse.ArgumentParser(description=__doc__); p.add_argument('source',type=Path); p.add_argument('--output',type=Path,required=True); p.add_argument('--model',choices=MODELS,default='spark'); a=p.parse_args()
expected={(name,repeat) for name in DEVELOPMENT+HELD_OUT for repeat in range(1,6)}
seen=set(); rows=[]; identities=[]; errors=[]
for shard in sorted(a.source.glob('*')):
    if not shard.is_dir(): continue
    try:
        config=json.loads((shard/'configuration.json').read_text())
        identity={key:config.get(key) for key in ['binary_sha256','sha256','engine_sha256','runtime_sha256','contexts','repeats','timeout_seconds','tool_profile','thinking','verification_plan','threads','oracle_version']}
        host=json.loads((shard/'host.json').read_text()); identity['harness_inputs']=host['harness_inputs']; identities.append(identity)
        assert identity['contexts']==[8192] and identity['repeats']==5 and identity['verification_plan'] is True
        assert identity['oracle_version']==ORACLE_VERSION, 'Different oracle execution contract; retain as a separate historical cohort'
        assert identity['sha256']==MODELS[a.model]['sha256']
        manifest=json.loads((shard/'RAW-SHA256.json').read_text())
        assert all(hashlib.sha256((shard/name).read_bytes()).hexdigest()==digest for name,digest in manifest.items()), 'Retained evidence changed'
        for path in sorted(shard.glob('*/report.json')):
            assert str(path.relative_to(shard)) in manifest, 'Unhashed report'
            row=json.loads(path.read_text()); cell=(row['case'],row['repeat'])
            assert cell in expected and cell not in seen, f'Unexpected or duplicate cell {cell}'
            assert row['context']==8192 and row['oracle_unchanged'], 'Context mismatch or changed oracle'
            seen.add(cell); rows.append({key:row[key] for key in ['case','repeat','passed','wall_seconds','scores']})
    except Exception as error: errors.append(f'{shard.name}: {error}')
if identities and any(i!=identities[0] for i in identities): errors.append('Shard configuration/compiled executable mismatch')
report={'schema':1,'expected':len(expected),'recorded':len(seen),'complete':seen==expected and not errors,'passed':sum(row['passed'] for row in rows),'missing':sorted(expected-seen),'errors':errors,'identity':identities[0] if identities else None,'attempts':rows,'preset_promoted':False,'claim_accuracy':'Unassessed until retained prose is reviewed against actual sources and checks','scope':'Seeded tasks on GitHub-hosted CPU runners; no physical GTX 1070 or human usability qualification.'}
for name,cases in [('development',DEVELOPMENT),('held_out',HELD_OUT)]:
    subset=[r for r in rows if r['case'] in cases]; report[name]={'passed':sum(r['passed'] for r in subset),'recorded':len(subset),'expected':len(cases)*5}
a.output.write_text(json.dumps(report,indent=2)+'\n')
summary=f"Matrix recorded {len(seen)}/{len(expected)} attempts; behavioral successes {report['passed']}. Complete: {report['complete']}. No preset promotion.\n"
print(summary)
if os.environ.get('GITHUB_STEP_SUMMARY'): Path(os.environ['GITHUB_STEP_SUMMARY']).write_text(summary)
raise SystemExit(0 if report['complete'] else 2)
