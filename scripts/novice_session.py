#!/usr/bin/env python3
"""Record a real uncoached usability session; never manufactures participant results."""
import argparse,datetime,json,uuid
from pathlib import Path
TASKS=[('connect','Choose an existing local model or add your own endpoint.'),('project','Select a disposable project and explain where changes will be written.'),('draft','Write a Unicode request, change pages, and recover it after a connection error.'),('evidence','Configure a check, run it, and explain whether its result establishes behavior.'),('undo','Edit a disposable file, inspect its diff, undo it, and rerun the check.'),('recover','Make a state backup and restore it into a new empty folder.'),('access','Explain Guided, Review only and Full access in your own words, then choose one.'),('cancel','Stop a long operation and identify what was stopped and what was saved.')]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--template',action='store_true');a=p.parse_args()
report={'schema':1,'id':str(uuid.uuid4()),'date':datetime.datetime.now(datetime.timezone.utc).isoformat(),'assessment':'unperformed-template','participant_alias':None,'tasks':[{'id':id,'instruction':text,'completed_without_coaching':None,'seconds':None,'observed_confusion':None,'recovery_succeeded':None} for id,text in TASKS]}
if not a.template:
 report['participant_alias']=input('Participant alias (no real name or private information): ').strip();assert report['participant_alias']
 report['assessment']='human-observed'
 for task in report['tasks']:
  print('\n'+task['instruction']);answer=input('Completed without coaching? [y/n]: ').strip().lower();assert answer in ['y','n'];task['completed_without_coaching']=answer=='y';task['seconds']=float(input('Observed seconds: '));assert task['seconds']>=0;task['observed_confusion']=input('Observed confusion, or none: ').strip();answer=input('Recovery succeeded or not applicable? [y/n/na]: ').strip().lower();assert answer in ['y','n','na'];task['recovery_succeeded']=None if answer=='na' else answer=='y'
a.output.parent.mkdir(parents=True,exist_ok=True)
with a.output.open('x') as f:json.dump(report,f,indent=2);f.write('\n')
print(a.output)
