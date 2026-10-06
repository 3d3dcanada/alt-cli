#!/usr/bin/env python3
"""Recheck retained source under the current oracle without changing original evidence."""
import argparse,hashlib,json,tempfile
from pathlib import Path
from acceptance_projects import CASES,ORACLE_VERSION,setup,check,write
p=argparse.ArgumentParser(description=__doc__);p.add_argument('cohort',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args();records=[]
for report in sorted(a.cohort.glob('*/report.json')):
 original=json.loads(report.read_text());name=original['case']
 with tempfile.TemporaryDirectory(prefix='alt-reassessment-')as t:
  project,command=setup(name,Path(t))
  # Start from the exact retained source, including any original-file deletions.
  for path in project.rglob('*'):
   if path.is_file():path.unlink()
  for name_in_project,body in original['resulting_source'].items():
   rel=Path(name_in_project);assert not rel.is_absolute() and '..' not in rel.parts
   write(project,{name_in_project:body})
  outcome=check(project,command)
  records.append({'attempt':report.parent.name,'original_report_sha256':hashlib.sha256(report.read_bytes()).hexdigest(),'original_oracle_passed':original['passed'],'current_oracle':outcome})
result={'oracle_version':ORACLE_VERSION,'method':'Recreated exact saved sources in disposable folders and ran stronger assertions. Original sources/events/oracles/reports preserved. No new model inference.','original_passed':sum(r['original_oracle_passed']for r in records),'current_passed':sum(r['current_oracle']['passed']for r in records),'total':len(records),'attempts':records}
a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n');print(a.cohort.name,result['original_passed'],'original ->',result['current_passed'],'current /',result['total'])
