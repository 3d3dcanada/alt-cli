#!/usr/bin/env python3
"""Copy bounded live evidence without weights, executables or private state databases."""
import argparse,hashlib,json,shutil,sqlite3
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('source',type=Path);p.add_argument('destination',type=Path);a=p.parse_args();a.destination.mkdir(parents=True,exist_ok=True)
for name in ['configuration.json','summary.json','runtime-model.json','interrupted-run.json']:
 source=a.source/name
 if source.exists():shutil.copy2(source,a.destination/name)
manifest={}
for run in sorted(a.source.iterdir()):
 if not run.is_dir() or not (run/'turn.jsonl').exists():continue
 dest=a.destination/run.name;dest.mkdir(exist_ok=True)
 for name in ['report.json','turn.jsonl','turn.stderr','setup.stdout','setup.stderr','fixture.json']:
  source=run/name
  if source.exists():
   assert source.stat().st_size<32*1024*1024,'Inspect unexpectedly large evidence before copying'
   shutil.copy2(source,dest/name);manifest[str((dest/name).relative_to(a.destination))]=hashlib.sha256(source.read_bytes()).hexdigest()
 for source in sorted((run/'independent').rglob('*')):
  if not source.is_file():continue
  assert not source.is_symlink() and source.stat().st_size<4*1024*1024
  target=dest/'independent'/source.relative_to(run/'independent');target.parent.mkdir(parents=True,exist_ok=True)
  shutil.copy2(source,target);manifest[str(target.relative_to(a.destination))]=hashlib.sha256(source.read_bytes()).hexdigest()
 contexts=[]
 for database in (run/'state').glob('projects/*/project.db'):
  with sqlite3.connect('file:'+str(database)+'?mode=ro',uri=True)as db:
   try:contexts.extend(json.loads(row[0]) for row in db.execute('SELECT payload FROM context_views'))
   except sqlite3.OperationalError:pass
 if contexts:(dest/'context-views.json').write_text(json.dumps(contexts,indent=2)+'\n')
(root_manifest:=a.destination/'RAW-SHA256.json').write_text(json.dumps(manifest,sort_keys=True,indent=2)+'\n')
print(f'Retained {len(manifest)} raw files in {a.destination}')
