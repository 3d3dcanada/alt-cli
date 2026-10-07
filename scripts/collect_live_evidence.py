#!/usr/bin/env python3
"""Copy completed live attempts without weights, executables or private state databases.

Unfinished attempts stay in the source folder; never copy a changing stream into
an immutable report collection. Failed completed attempts are always retained.
"""
import argparse,hashlib,json,shutil,sqlite3,sys
from pathlib import Path
if sys.flags.optimize:raise RuntimeError('Run without Python optimization; evidence assertions must remain enabled')
p=argparse.ArgumentParser(description=__doc__);p.add_argument('source',type=Path);p.add_argument('destination',type=Path);a=p.parse_args();a.destination.mkdir(parents=True,exist_ok=True)
manifest={}
def retain(source,target):
 assert source.is_file() and not source.is_symlink() and source.resolve().is_relative_to(a.source.resolve())
 assert source.stat().st_size<32*1024*1024,'Inspect unexpectedly large evidence before copying'
 target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,target)
 manifest[str(target.relative_to(a.destination))]=hashlib.sha256(source.read_bytes()).hexdigest()
for name in ['configuration.json','summary.json','runtime-model.json','interrupted-run.json','live_acceptance.py','live_tui_turn.py','acceptance_projects.py','acceptance_extra.py','acceptance_v5.py','BUILD.json']:
 source=a.source/name
 if source.exists():retain(source,a.destination/name)
for run in sorted(a.source.iterdir()):
 if not run.is_dir() or not (run/'turn.jsonl').exists() or not (run/'report.json').exists():continue
 dest=a.destination/run.name;dest.mkdir(exist_ok=True)
 original=run/'evidence-sha256.json'
 if original.exists():
  for name,digest in json.loads(original.read_text()).items():
   relative=Path(name);assert not relative.is_absolute() and '..' not in relative.parts
   source=run/relative
   assert source.resolve().is_relative_to(run.resolve()) and hashlib.sha256(source.read_bytes()).hexdigest()==digest,'Original evidence changed'
 for name in ['report.json','turn.jsonl','turn.stderr','setup.stdout','setup.stderr','fixture.json','assertion.py','evidence-sha256.json','terminal.raw','terminal.txt','terminal-after-exit.txt','tui-turn.json']:
  source=run/name
  if source.exists():
   retain(source,dest/name)
 for source in sorted((run/'state'/'inference').rglob('*')):
  if source.is_file():retain(source,dest/source.relative_to(run))
 fixture=json.loads((run/'fixture.json').read_text())
 for name in sorted(fixture['oracle_sha256_before']):
  relative=Path(name);assert not relative.is_absolute() and '..' not in relative.parts
  source=run/'independent'/relative
  assert not source.is_symlink() and source.stat().st_size<4*1024*1024
  target=dest/'independent'/source.relative_to(run/'independent');target.parent.mkdir(parents=True,exist_ok=True)
  retain(source,target)
 contexts=[]
 for database in (run/'state').glob('projects/*/project.db'):
  with sqlite3.connect('file:'+str(database)+'?mode=ro',uri=True)as db:
   try:contexts.extend(json.loads(row[0]) for row in db.execute('SELECT payload FROM context_views'))
   except sqlite3.OperationalError:pass
 if contexts:
  path=dest/'context-views.json';path.write_text(json.dumps(contexts,indent=2)+'\n');manifest[str(path.relative_to(a.destination))]=hashlib.sha256(path.read_bytes()).hexdigest()
(root_manifest:=a.destination/'RAW-SHA256.json').write_text(json.dumps(manifest,sort_keys=True,indent=2)+'\n')
print(f'Retained {len(manifest)} raw files in {a.destination}')
