import argparse,hashlib,json,pathlib,shutil,sys,tempfile
sys.path.insert(0,'/workspace/alt-cli/scripts')
from acceptance_projects import check
p=argparse.ArgumentParser();p.add_argument('source',type=pathlib.Path);p.add_argument('--output',type=pathlib.Path,required=True);a=p.parse_args()
summary=json.loads((a.source/'summary.json').read_text())
if not summary['complete'] or summary['recorded']!=120:raise ValueError('Complete matrix required')
rows=[]
for report in sorted(a.source.glob('uncensored-matrix-*/*/report.json')):
 row=json.loads(report.read_text());manifest=json.loads((report.parent.parent/'RAW-SHA256.json').read_text())
 names=[str(report.relative_to(report.parent.parent))]+[k for k in manifest if k.startswith(report.parent.name+'/independent/')]
 for name in names:
  if hashlib.sha256((report.parent.parent/name).read_bytes()).hexdigest()!=manifest[name]:raise ValueError('Retained bytes changed')
 with tempfile.TemporaryDirectory(prefix='alt-retained-replay-') as d:
  root=pathlib.Path(d);project=root/'project';project.mkdir()
  for name,content in row['resulting_source'].items():
   rel=pathlib.Path(name)
   if rel.is_absolute() or '..' in rel.parts:raise ValueError('Unsafe source path')
   target=project/rel;target.parent.mkdir(parents=True,exist_ok=True);target.write_text(content)
  shutil.copytree(report.parent/'independent',root/'independent')
  command=['node',str(root/'independent/check.mjs')] if (root/'independent/check.mjs').exists() else ['python3',str(root/'independent/check.py')]
  observed=check(project,command)
  rows.append({'case':row['case'],'repeat':row['repeat'],'original_report_sha256':hashlib.sha256(report.read_bytes()).hexdigest(),'original_behavior_passed':row['passed'],'replay':observed,'matches':observed['passed']==row['passed']})
if len(rows)!=120:raise ValueError('Missing attempts')
result={'schema':1,'summary_sha256':hashlib.sha256((a.source/'summary.json').read_bytes()).hexdigest(),'script_sha256':hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),'recorded':len(rows),'matching_outcomes':sum(r['matches'] for r in rows),'rows':rows,'scope':'Post-collection replay of all retained final sources using exact retained independent oracle bytes, fresh temporary project folders and this cloud host. No inference, model retries or changes to original campaign scores. This checks collection/result consistency, not new model attempts or adversarial isolation.'}
a.output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:result[k] for k in ['recorded','matching_outcomes']}))
raise SystemExit(0 if all(r['matches'] for r in rows) else 1)
